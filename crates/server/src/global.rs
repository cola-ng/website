use std::collections::{BTreeMap, HashMap, HashSet};
use std::future::Future;
use std::net::IpAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock, OnceLock, RwLock};

use diesel::prelude::*;
use salvo::http::StatusError;
use salvo::oapi::ToSchema;
use serde::Serialize;

use crate::{AppError, AppResult};

#[derive(Serialize, ToSchema, Clone, Copy, Debug)]
pub struct EmptyObject {}

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
