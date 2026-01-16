#[macro_use]
extern crate tracing;

pub mod auth;
mod config;
mod db;
mod error;
mod global;
mod hoops;
mod models;
mod routing;
pub use global::*;
pub mod user;
pub use error::AppError;
use salvo::prelude::*;

use crate::config::AppConfig;
use crate::db::DbConfig;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    AppConfig::init();
    let app_config = AppConfig::get();
    let bind_addr = app_config.bind_addr.clone();

    db::init(&app_config.database);

    let router = routing::router();

    println!("router:::::{:?}", router);

    let acceptor = TcpListener::new(bind_addr).bind().await;
    Server::new(acceptor).serve(router).await;
}
