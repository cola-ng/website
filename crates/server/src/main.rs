mod auth;
mod config;
mod db;
mod models;
mod routing;

use salvo::prelude::*;

use crate::config::AppConfig;
use crate::db::DbConfig;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let app_config = AppConfig::from_env().expect("invalid config");
    let bind_addr = app_config.bind_addr.clone();

    let db_config = DbConfig {
        url: app_config.database_url.clone(),
        pool_size: 15,
        min_idle: 5,
        connection_timeout: 30000,
        helper_threads: 4,
        statement_timeout: 5000,
        tcp_timeout: 30000,
        enforce_tls: false,
    };

    db::init(&db_config);

    let router = routing::router(app_config);

    let acceptor = TcpListener::new(bind_addr).bind().await;
    Server::new(acceptor).serve(router).await;
}
