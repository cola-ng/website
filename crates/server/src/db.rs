use std::sync::OnceLock;
use std::time::Duration;

use deadpool::managed::{Status, Timeouts};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use futures_util::future::BoxFuture;
use horme::diesel_async::{AsyncMigrationHarness, AsyncPgConnection};
use horme::pool::{AsyncDieselConnectionManager, deadpool as horme_pool};
use horme::{Db, GetConnection};
use url::Url;

extern crate tracing;
pub mod schema;

pub use crate::config::DbConfig;

// pub mod full_text_search;

pub mod pool;
pub use pool::PoolError;

type HormePool = horme_pool::Pool<AsyncPgConnection>;
pub type Database = Db<HormePool>;
pub type PgPooledConnection = horme_pool::Object<AsyncPgConnection>;

pub static DIESEL_POOL: OnceLock<Db<HormePool>> = OnceLock::new();

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub async fn init(config: &DbConfig) {
    let timeout = Duration::from_millis(config.connection_timeout);
    let manager =
        AsyncDieselConnectionManager::<AsyncPgConnection>::new(connection_url(config, &config.url));
    let pool = horme_pool::Pool::builder(manager)
        .runtime(deadpool::Runtime::Tokio1)
        .max_size(config.pool_size as usize)
        .timeouts(Timeouts {
            wait: Some(timeout),
            create: Some(timeout),
            recycle: Some(timeout),
        })
        .build()
        .expect("horme database pool should be created");
    DIESEL_POOL
        .set(Db::new(pool))
        .map_err(|_| "horme database pool should be set once")
        .unwrap();
    migrate().await;
}

pub async fn migrate() {
    let conn = conn().await.expect("db connect should work");
    AsyncMigrationHarness::new(conn)
        .run_pending_migrations(MIGRATIONS)
        .expect("migrate db should work");
}

pub async fn conn() -> Result<PgPooledConnection, PoolError> {
    let mut db = DIESEL_POOL.get().expect("horme pool should set").clone();
    db.get_mut_conn()
        .await
        .map_err(|e| PoolError::Queue(e.to_string()))
}

pub async fn with_conn<F, R>(f: F) -> Result<R, String>
where
    F: for<'a> FnOnce(&'a mut Database) -> BoxFuture<'a, horme::Result<R>> + Send + 'static,
    R: Send + 'static,
{
    let db = DIESEL_POOL.get().expect("horme pool should set").clone();
    run_with(db, f).await
}

async fn run_with<F, R>(mut db: Database, f: F) -> Result<R, String>
where
    F: for<'a> FnOnce(&'a mut Database) -> BoxFuture<'a, horme::Result<R>> + Send + 'static,
    R: Send + 'static,
{
    // A Db handle holds no lease. Horme releases each query's lease before
    // the callback continues. A transaction keeps one lease until it ends.
    f(&mut db).await.map_err(|e| e.to_string())
}

#[macro_export]
macro_rules! with_conn {
    (move |$conn:ident| horme::transaction($($body:tt)+)) => {
        $crate::db::with_conn(move |$conn| std::boxed::Box::pin(async move { horme::transaction($($body)+).await }))
    };
    ({ move |$conn:ident| $body:expr }) => {
        $crate::db::with_conn(move |$conn| std::boxed::Box::pin(async move { $body }))
    };
    ({ $pre:stmt; move |$conn:ident| $body:expr }) => {
        $crate::db::with_conn(move |$conn| std::boxed::Box::pin(async move { $pre $body }))
    };
    ({ $pre1:stmt; $pre2:stmt; move |$conn:ident| $body:expr }) => {
        $crate::db::with_conn(move |$conn| std::boxed::Box::pin(async move { $pre1 $pre2 $body }))
    };
    ({ $pre1:stmt; $pre2:stmt; $pre3:stmt; move |$conn:ident| $body:expr }) => {
        $crate::db::with_conn(move |$conn| std::boxed::Box::pin(async move { $pre1 $pre2 $pre3 $body }))
    };
    (move |$conn:ident| async $body:block) => {
        $crate::db::with_conn(move |$conn| std::boxed::Box::pin(async move $body))
    };
    (move |$conn:ident| $body:expr) => {
        $crate::db::with_conn(move |$conn| std::boxed::Box::pin(async move { $body }))
    };
}

pub use crate::with_conn;

pub fn state() -> Status {
    DIESEL_POOL
        .get()
        .expect("horme pool should set")
        .pool()
        .status()
}

pub fn connection_url(config: &DbConfig, url: &str) -> String {
    let mut url = Url::parse(url).expect("Invalid database URL");

    if config.enforce_tls {
        maybe_append_url_param(&mut url, "sslmode", "require");
    }

    // Configure the time it takes for diesel to return an error when there is full packet loss
    // between the application and the database.
    maybe_append_url_param(
        &mut url,
        "tcp_user_timeout",
        &config.tcp_timeout.to_string(),
    );
    maybe_append_encoded_url_param(
        &mut url,
        "options",
        &format!("-c statement_timeout={}", config.statement_timeout),
    );

    url.into()
}

fn maybe_append_url_param(url: &mut Url, key: &str, value: &str) {
    if !url.query_pairs().any(|(k, _)| k == key) {
        url.query_pairs_mut().append_pair(key, value);
    }
}

fn maybe_append_encoded_url_param(url: &mut Url, key: &str, value: &str) {
    if url.query_pairs().any(|(k, _)| k == key) {
        return;
    }
    let value = url::form_urlencoded::byte_serialize(value.as_bytes())
        .collect::<String>()
        .replace('+', "%20");
    let prefix = url
        .query()
        .map(|query| format!("{query}&"))
        .unwrap_or_default();
    url.set_query(Some(&format!("{prefix}{key}={value}")));
}

#[cfg(test)]
mod url_tests {
    use super::*;

    #[test]
    fn connection_url_preserves_existing_options_and_adds_timeouts() {
        let config = DbConfig::default();
        let url = connection_url(
            &config,
            "postgresql://user@127.0.0.1/db?options=-c%20search_path%3Dapp",
        );
        assert!(url.contains("search_path%3Dapp"));
        assert!(!url.contains("statement_timeout"));
        let url = connection_url(&config, "postgresql://user@127.0.0.1/db");
        assert!(url.contains("statement_timeout"));
        assert!(url.contains("tcp_user_timeout"));
    }
}

#[cfg(test)]
#[path = "db/tests.rs"]
mod tests;
