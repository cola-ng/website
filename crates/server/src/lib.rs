#[macro_use]
extern crate tracing;

pub mod auth;
pub mod config;
pub mod db;
mod error;
mod global;
mod hoops;
pub mod models;
mod routing;
pub mod user;

pub use config::{AppConfig, DbConfig};
pub use error::AppError;
pub use global::*;

use salvo::prelude::*;

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
