use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db::schema::*;

#[derive(Identifiable, Queryable, Debug, Clone)]
#[diesel(table_name = user_refresh_tokens)]
pub struct RefreshToken {
    pub id: i64,
    pub user_id: i64,
    pub device_id: i64,
    pub token: String,
    pub next_token_id: Option<i64>,
    pub expires_at: DateTime<Utc>,
    pub ultimate_session_expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = user_refresh_tokens)]
pub struct NewRefreshToken {
    pub user_id: i64,
    pub device_id: i64,
    pub token: String,
    pub next_token_id: Option<i64>,
    pub expires_at: DateTime<Utc>,
    pub ultimate_session_expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl NewRefreshToken {
    pub fn new(
        user_id: i64,
        device_id: i64,
        token: String,
        expires_at: DateTime<Utc>,
        ultimate_session_expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            user_id,
            device_id,
            token,
            next_token_id: None,
            expires_at,
            ultimate_session_expires_at,
            created_at: Utc::now(),
        }
    }
}
