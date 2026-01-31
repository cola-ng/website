//! Background task to flush study time cache to database

use std::time::Duration;

use diesel::prelude::*;
use tokio::time;

use crate::db::schema::learn_study_time_daily;
use crate::db::with_conn;
use crate::models::learn::NewStudyTimeDaily;
use crate::services::study_time::STUDY_TIME_TRACKER;

/// Start the background flush task
///
/// This task runs every 5 minutes to persist cached study time to the database.
pub fn start() {
    tokio::spawn(async move {
        // Initial delay to let the server fully start
        time::sleep(Duration::from_secs(10)).await;

        let mut interval = time::interval(Duration::from_secs(300)); // 5 minutes
        tracing::info!("Study time flush task started (interval: 5 minutes)");

        loop {
            interval.tick().await;

            if let Err(e) = flush_pending_increments().await {
                tracing::error!("Failed to flush study time increments: {:?}", e);
            }

            // Clean up stale user states
            STUDY_TIME_TRACKER.cleanup_stale_states().await;
        }
    });
}

/// Flush pending increments to database
async fn flush_pending_increments() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pending = STUDY_TIME_TRACKER.take_pending_increments().await;

    if pending.is_empty() {
        return Ok(());
    }

    tracing::info!("Flushing {} study time records to database", pending.len());

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

        let result = with_conn(move |conn| {
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
        })
        .await;

        if let Err(e) = result {
            tracing::error!(
                "Failed to upsert study time for user {} on {}: {:?}",
                user_id,
                stat_date,
                e
            );
        }
    }

    tracing::info!("Study time flush completed");
    Ok(())
}
