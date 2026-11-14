//! Background task to flush study time cache to database

use std::time::Duration;

use horme::prelude::*;
use std::result::Result;
use tokio::time;

use crate::db::schema::learn_study_time_daily;
use crate::db::with_conn;
use crate::models::learn::NewStudyTimeDaily;
use crate::services::study_time::STUDY_TIME_TRACKER;

/// Start the background flush task
///
/// This task runs every 5 minutes to persist cached study time to the database.
pub fn start() -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let (stop, mut shutdown) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(300));
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                _ = interval.tick() => {
                    if let Err(error) = flush_pending_increments().await { tracing::error!(%error, "Study time flush failed; batch retained"); }
                    STUDY_TIME_TRACKER.cleanup_stale_states().await;
                }
            }
        }
        // Drain the retained batch, then increments received while it was in flight.
        for _ in 0..3 {
            if let Err(error) = flush_pending_increments().await {
                tracing::error!(%error, "Final study time flush failed");
            }
        }
    });
    (stop, task)
}

/// Flush pending increments to database
async fn flush_pending_increments() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    static FLUSH: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = FLUSH.lock().await;
    let Some(batch) = STUDY_TIME_TRACKER.begin_flush().await else {
        return Ok(());
    };
    let id = batch.id;
    with_conn!(move |conn| horme::transaction(conn, async |conn| {
        let inserted = diesel::sql_query(
            "INSERT INTO study_time_flush_batches(id) VALUES ($1) ON CONFLICT DO NOTHING",
        )
        .bind::<diesel::sql_types::Uuid, _>(id)
        .execute(conn)
        .await?;
        if inserted == 0 {
            return Ok(());
        }
        let pending = batch.increments;
        for ((user_id, stat_date), increment) in pending {
            if increment.is_empty() {
                continue;
            }

            let total = increment.total_seconds();
            let chat_seconds = increment.chat_seconds;
            let review_seconds = increment.review_seconds;
            let stage_seconds = increment.stage_seconds;
            let read_seconds = increment.read_seconds;
            let dictionary_seconds = increment.dictionary_seconds;
            let translate_seconds = increment.translate_seconds;
            let heartbeat_count = increment.heartbeat_count;

            {
                // Try upsert: insert or update on conflict
                diesel::insert_into(learn_study_time_daily::table)
                    .values(&NewStudyTimeDaily {
                        user_id,
                        stat_date,
                        chat_seconds,
                        review_seconds,
                        stage_seconds,
                        read_seconds,
                        dictionary_seconds,
                        translate_seconds,
                        total_seconds: total,
                        heartbeat_count,
                    })
                    .on_conflict((
                        learn_study_time_daily::user_id,
                        learn_study_time_daily::stat_date,
                    ))
                    .do_update()
                    .set((
                        learn_study_time_daily::chat_seconds
                            .eq(learn_study_time_daily::chat_seconds + chat_seconds),
                        learn_study_time_daily::review_seconds
                            .eq(learn_study_time_daily::review_seconds + review_seconds),
                        learn_study_time_daily::stage_seconds
                            .eq(learn_study_time_daily::stage_seconds + stage_seconds),
                        learn_study_time_daily::read_seconds
                            .eq(learn_study_time_daily::read_seconds + read_seconds),
                        learn_study_time_daily::dictionary_seconds
                            .eq(learn_study_time_daily::dictionary_seconds + dictionary_seconds),
                        learn_study_time_daily::translate_seconds
                            .eq(learn_study_time_daily::translate_seconds + translate_seconds),
                        learn_study_time_daily::total_seconds
                            .eq(learn_study_time_daily::total_seconds + total),
                        learn_study_time_daily::heartbeat_count
                            .eq(learn_study_time_daily::heartbeat_count + heartbeat_count),
                    ))
                    .execute(conn)
                    .await
            }?;
        }
        // Keep only a bounded retry window; UUID batches are acknowledged immediately.
        diesel::sql_query(
            "DELETE FROM study_time_flush_batches WHERE created_at < NOW() - INTERVAL '7 days'",
        )
        .execute(conn)
        .await?;
        Ok(())
    }))
    .await?;
    STUDY_TIME_TRACKER.acknowledge(id).await;

    tracing::info!("Study time flush completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "requires explicitly isolated local QA_DATABASE_URL"]
    async fn study_flush_database_flow() {
        let database = std::env::var("QA_DATABASE_URL").unwrap();
        let url = url::Url::parse(&database).unwrap();
        assert!(
            matches!(url.host_str(), Some("localhost" | "127.0.0.1")) && url.path() == "/colang_qa"
        );
        crate::db::init(&crate::DbConfig {
            url: database,
            ..Default::default()
        })
        .await;
        let user = with_conn!(move |conn| {
            diesel::insert_into(crate::db::schema::base_users::table)
                .values(
                    crate::db::schema::base_users::name
                        .eq(format!("flush-{}", uuid::Uuid::new_v4())),
                )
                .returning(crate::db::schema::base_users::id)
                .get_result::<i64>(conn)
                .await
        })
        .await
        .unwrap();
        STUDY_TIME_TRACKER
            .process_heartbeat(user, crate::models::learn::PageType::Chat)
            .await;
        tokio::time::sleep(Duration::from_millis(1100)).await;
        STUDY_TIME_TRACKER
            .process_heartbeat(user, crate::models::learn::PageType::Chat)
            .await;
        let batch = STUDY_TIME_TRACKER.begin_flush().await.unwrap();
        with_conn!(move |conn| {
            diesel::sql_query(format!("CREATE FUNCTION qa_flush_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.user_id={user} THEN RAISE EXCEPTION 'QA failure'; END IF; RETURN NEW; END $$"))
                .execute(conn).await?;
            diesel::sql_query("CREATE TRIGGER qa_flush_failure BEFORE INSERT OR UPDATE ON learn_study_time_daily FOR EACH ROW EXECUTE FUNCTION qa_flush_failure()")
                .execute(conn).await
        }).await.unwrap();
        assert!(flush_pending_increments().await.is_err());
        assert_eq!(STUDY_TIME_TRACKER.begin_flush().await.unwrap().id, batch.id);
        with_conn!(move |conn| {
            diesel::sql_query("DROP TRIGGER qa_flush_failure ON learn_study_time_daily")
                .execute(conn)
                .await?;
            diesel::sql_query("DROP FUNCTION qa_flush_failure()")
                .execute(conn)
                .await
        })
        .await
        .unwrap();
        flush_pending_increments().await.unwrap();
        assert!(STUDY_TIME_TRACKER.begin_flush().await.is_none());
        // Simulate losing the commit acknowledgement: resend the exact UUID batch.
        let id = batch.id;
        let total = with_conn!(move |conn| {
            let total = learn_study_time_daily::table
                .filter(learn_study_time_daily::user_id.eq(user))
                .select(diesel::dsl::sum(learn_study_time_daily::total_seconds))
                .first::<Option<i64>>(conn)
                .await?;
            let rows = diesel::sql_query(
                "INSERT INTO study_time_flush_batches(id) VALUES ($1) ON CONFLICT DO NOTHING",
            )
            .bind::<diesel::sql_types::Uuid, _>(id)
            .execute(conn)
            .await?;
            assert_eq!(rows, 0);
            Ok(total)
        })
        .await
        .unwrap();
        assert_eq!(
            total,
            Some(
                batch
                    .increments
                    .values()
                    .map(|v| v.total_seconds() as i64)
                    .sum()
            )
        );
    }
}
