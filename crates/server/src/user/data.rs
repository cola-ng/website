use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::de::DeserializeOwned;

use crate::db::{connect, schema::*};
use crate::{AppResult, JsonValue};

#[derive(Identifiable, Queryable, Debug, Clone)]
#[diesel(table_name = user_datas)]
pub struct UserData {
    pub id: i64,
    pub user_id: i64,
    pub data_type: String,
    pub json_data: JsonValue,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = user_datas)]
pub struct NewUserData {
    pub user_id: i64,
    pub data_type: String,
    pub json_data: JsonValue,
    pub created_at: DateTime<Utc>,
}