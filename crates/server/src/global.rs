use std::collections::{BTreeMap, HashMap, HashSet};
use std::future::Future;
use std::net::IpAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock, OnceLock, RwLock};
use std::time::Instant;

use diesel::prelude::*;
use hickory_resolver::Resolver as HickoryResolver;
use hickory_resolver::config::*;
use hickory_resolver::name_server::TokioConnectionProvider;
use salvo::oapi::ToSchema;
use serde::Serialize;
use tokio::sync::{Semaphore, broadcast};

use crate::AppResult;
use crate::db::connect;
use crate::db::schema::*;
use crate::models::user::{NewUser, NewUserDevice};
use crate::utils::{MutexMap, MutexMapGuard};

pub const DEVICE_ID_LENGTH: usize = 10;
pub const TOKEN_LENGTH: usize = 32;
pub const SESSION_ID_LENGTH: usize = 32;
pub const AUTO_GEN_PASSWORD_LENGTH: usize = 15;
pub const RANDOM_USER_ID_LENGTH: usize = 10;

pub type TlsNameMap = HashMap<String, (Vec<IpAddr>, u16)>;
type RateLimitState = (Instant, u32); // Time if last failed try, number of failed tries

pub type LazyRwLock<T> = LazyLock<RwLock<T>>;
pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, ToSchema, Clone, Copy, Debug)]
pub struct EmptyObject {}

pub fn shutdown() {
    SHUTDOWN.store(true, std::sync::atomic::Ordering::Relaxed);
    // On shutdown
    info!(target: "shutdown-sync", "received shutdown notification, notifying sync helpers...");
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
