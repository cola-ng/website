use salvo::http::StatusError;
use salvo::oapi::ToSchema;
use serde::Serialize;
use salvo::prelude::*;

use crate::AppError;

#[derive(Serialize, ToSchema, Clone, Copy, Debug)]
pub struct EmptyObject {}

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

pub trait DepotExt {
    fn user_id(&self) -> AppResult<i64>;
}
impl DepotExt for salvo::prelude::Depot {
    fn user_id(&self) -> AppResult<i64> {
        self.get::<i64>("user_id").copied().map_err(|_| {
            StatusError::unauthorized()
                .brief("missing user_id in depot")
                .into()
        })
    }
}

#[macro_export]
macro_rules! diesel_exists {
    ($query:expr, $conn:expr) => {{
        // tracing::info!( sql = %debug_query!(&$query), "diesel_exists");
        diesel::select(diesel::dsl::exists($query)).get_result::<bool>($conn)
    }};
    ($query:expr, $default:expr, $conn:expr) => {{
        // tracing::info!( sql = debug_query!(&$query), "diesel_exists");
        diesel::select(diesel::dsl::exists($query))
            .get_result::<bool>($conn)
            .unwrap_or($default)
    }};
}

#[macro_export]
macro_rules! print_query {
    ($query:expr) => {
        println!("{}", diesel::debug_query::<diesel::pg::Pg, _>($query));
    };
}

#[macro_export]
macro_rules! debug_query {
    ($query:expr) => {{ format!("{}", diesel::debug_query::<diesel::pg::Pg, _>($query)) }};
}
