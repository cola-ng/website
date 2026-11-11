use super::*;
use horme::prelude::*;
use tokio::sync::oneshot;
use tokio::time::timeout;

async fn one(db: &mut Database) -> horme::Result<i32> {
    diesel::select(1_i32.into_sql::<diesel::sql_types::Integer>())
        .get_result(db)
        .await
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn pool_leases_are_released_between_queries_and_after_failures() {
    let url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL is required");
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url);
    let pool = horme_pool::Pool::builder(manager)
        .runtime(deadpool::Runtime::Tokio1)
        .max_size(1)
        .timeouts(Timeouts {
            wait: Some(Duration::from_secs(2)),
            create: Some(Duration::from_secs(2)),
            recycle: Some(Duration::from_secs(2)),
        })
        .build()
        .unwrap();
    let db = Db::new(pool);

    // Keep the business callback alive between queries. Another callback must
    // still be able to use the only connection in the pool.
    let (ready_tx, ready_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    let first = tokio::spawn(run_with(db.clone(), move |conn| {
        Box::pin(async move {
            assert_eq!(one(conn).await?, 1);
            ready_tx.send(()).unwrap();
            resume_rx.await.unwrap();
            one(conn).await
        })
    }));
    ready_rx.await.unwrap();
    assert_eq!(db.pool().status().available, 1);
    assert_eq!(
        run_with(db.clone(), |conn| Box::pin(one(conn)))
            .await
            .unwrap(),
        1
    );
    resume_tx.send(()).unwrap();
    assert_eq!(first.await.unwrap().unwrap(), 1);

    // More callers than pool slots must finish and return every lease.
    let queries = (0..24).map(|_| run_with(db.clone(), |conn| Box::pin(one(conn))));
    for result in futures_util::future::join_all(queries).await {
        assert_eq!(result.unwrap(), 1);
    }
    assert!(
        run_with(db.clone(), |conn| Box::pin(async move {
            diesel::sql_query("SELECT 1 / 0").execute(conn).await
        }))
        .await
        .is_err()
    );
    assert_eq!(
        run_with(db.clone(), |conn| Box::pin(one(conn)))
            .await
            .unwrap(),
        1
    );

    // A transaction must pin one connection and roll back on callback failure.
    assert!(
        run_with(db.clone(), |conn| Box::pin(async move {
            horme::transaction(conn, async |tx| {
                let first: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
                    "pg_backend_pid()",
                ))
                .get_result(tx)
                .await?;
                let second: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
                    "pg_backend_pid()",
                ))
                .get_result(tx)
                .await?;
                assert_eq!(first, second);
                diesel::sql_query("SELECT set_config('horme.lease_test', 'inside', true)")
                    .execute(tx)
                    .await?;
                Err::<(), horme::Error>(diesel::result::Error::RollbackTransaction.into())
            })
            .await
        }))
        .await
        .is_err()
    );
    run_with(db.clone(), |conn| {
        Box::pin(async move {
            let value: Option<String> = diesel::select(diesel::dsl::sql::<
                diesel::sql_types::Nullable<diesel::sql_types::Text>,
            >(
                "current_setting('horme.lease_test', true)",
            ))
            .get_result(conn)
            .await?;
            assert_ne!(value.as_deref(), Some("inside"));
            Ok(())
        })
    })
    .await
    .unwrap();

    // Cancellation while queued for a saturated pool must not leak a slot.
    let mut holder = db.clone();
    let lease = holder.get_mut_conn().await.unwrap();
    let (started_tx, started_rx) = oneshot::channel();
    let queued = tokio::spawn(run_with(db.clone(), move |conn| {
        Box::pin(async move {
            started_tx.send(()).unwrap();
            one(conn).await
        })
    }));
    started_rx.await.unwrap();
    tokio::task::yield_now().await;
    queued.abort();
    assert!(queued.await.unwrap_err().is_cancelled());
    drop(lease);
    assert_eq!(
        timeout(
            Duration::from_secs(3),
            run_with(db.clone(), |conn| Box::pin(one(conn)))
        )
        .await
        .unwrap()
        .unwrap(),
        1
    );
    assert_eq!(db.pool().status().available, 1);
}
