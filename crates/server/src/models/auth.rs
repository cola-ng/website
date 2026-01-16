
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use crate::db::schema::*;

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = auth_codes)]
pub struct AuthCode {
    pub id: i64,
    pub user_id: i64,
    pub code_hash: String,
    pub redirect_uri: String,
    pub state: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = auth_codes)]
pub struct NewAuthCode {
    pub user_id: i64,
    pub code_hash: String,
    pub redirect_uri: String,
    pub state: String,
    pub expires_at: DateTime<Utc>,
}
