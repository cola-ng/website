use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use salvo::http::StatusError;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AppResult;
use crate::db::schema::*;
use crate::db::url_filter::JoinedOption;

#[derive(Queryable, Identifiable, Associations, Serialize, Debug, Clone)]
#[diesel(table_name = auth_codes)]
#[diesel(belongs_to(User))]
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
