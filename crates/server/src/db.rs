use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub fn create_pool(database_url: &str) -> Result<DbPool, diesel::r2d2::PoolError> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder().build(manager)
}

pub async fn with_conn<T, F>(pool: DbPool, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&mut PgConnection) -> Result<T, diesel::result::Error> + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|_| "database connection error".to_string())?;
        f(&mut conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "database task error".to_string())?
}
