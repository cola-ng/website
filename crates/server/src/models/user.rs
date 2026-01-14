pub mod device;
pub use device::{NewUserDevice, UserDevice};
mod profile;
pub use profile::*;
mod access_token;
pub use access_token::*;
mod refresh_token;
pub use refresh_token::*;
mod data;
pub use data::*;
pub mod session;
pub use session::*;
pub mod external_id;
pub use external_id::*;

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db::connect;
use crate::db::schema::*;
use crate::{AppResult, diesel_exists};

#[derive(Insertable, Identifiable, Queryable, Debug, Clone)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub is_admin: bool,
    pub is_guest: bool,
    pub approved_at: Option<DateTime<Utc>>,
    pub approved_by: Option<i64>,
    pub deactivated_at: Option<DateTime<Utc>>,
    pub deactivated_by: Option<i64>,
    pub locked_at: Option<DateTime<Utc>>,
    pub locked_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub name: String,
    pub is_admin: bool,
    pub is_guest: bool,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn is_deactivated(&self) -> bool {
        self.deactivated_at.is_some()
    }
}

#[derive(Identifiable, Debug, Clone)]
#[diesel(table_name = user_passwords)]
pub struct Password {
    pub id: i64,
    pub user_id: i64,
    pub hash: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Queryable, Debug, Clone)]
#[diesel(table_name = user_passwords)]
pub struct NewPassword {
    pub user_id: i64,
    pub hash: String,
    pub created_at: DateTime<Utc>,
}
