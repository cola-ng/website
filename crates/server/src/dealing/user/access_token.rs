use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db::schema::*;

#[derive(Identifiable, Queryable, Debug, Clone)]
#[diesel(table_name = user_access_tokens)]
pub struct AccessToken {
    pub id: i64,
    pub user_id: i64,
    pub device_id: i64,
    pub token: String,
    pub puppets_user_id: Option<i64>,
    pub last_validated: Option<DateTime<Utc>>,
    pub refresh_token_id: Option<i64>,
    pub is_used: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = user_access_tokens)]
pub struct NewAccessToken {
    pub user_id: i64,
    pub device_id: i64,
    pub token: String,
    pub puppets_user_id: Option<i64>,
    pub last_validated: Option<DateTime<Utc>>,
    pub refresh_token_id: Option<i64>,
    pub is_used: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl NewAccessToken {
    pub fn new(user_id: i64, device_id: i64, token: String, refresh_token_id: Option<i64>) -> Self {
        Self {
            user_id,
            device_id,
            token,
            puppets_user_id: None,
            last_validated: None,
            refresh_token_id,
            is_used: false,
            expires_at: None,
            created_at: Utc::now(),
        }
    }
}
