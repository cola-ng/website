use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db::schema::*;

#[derive(Insertable, Identifiable, Debug, Clone)]
#[diesel(table_name = user_sessions)]
pub struct DbSession {
    pub id: i64,
    pub user_id: i64,
    pub value: String,
    pub expires_at: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = user_sessions)]
pub struct NewDbSession {
    pub user_id: i64,
    pub value: String,
    pub expires_at: i64,
    pub created_at: DateTime<Utc>,
}
