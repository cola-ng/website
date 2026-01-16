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

pub type AppResult<T> = Result<T, AppError>;
pub type DieselResult<T> = Result<T, diesel::result::Error>;
pub type JsonResult<T> = Result<Json<T>, AppError>;
pub type EmptyResult = Result<Json<EmptyObject>, AppError>;

pub fn json_ok<T>(data: T) -> JsonResult<T> {
    Ok(Json(data))
}
pub fn empty_ok() -> JsonResult<EmptyObject> {
    Ok(Json(EmptyObject {}))
}

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
