mod auth;
mod config;
mod db;
mod models;
mod routing;

use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use salvo::prelude::*;

use crate::config::AppConfig;
use crate::db::create_pool;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env().expect("invalid config");
    let bind_addr = config.bind_addr.clone();
    let pool = create_pool(&config.database_url).expect("failed to create db pool");
    {
        let mut conn = pool.get().expect("failed to get db connection");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("failed to run migrations");
    }

    let router = routing::router(pool, config);

    let acceptor = TcpListener::new(bind_addr).bind().await;
    Server::new(acceptor).serve(router).await;
}
