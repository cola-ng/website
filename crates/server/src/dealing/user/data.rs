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

/// Places one event in the account data of the user and removes the previous entry.
#[tracing::instrument(skip(user_id, event_type, json_data))]
pub fn set_data(user_id: i64, event_type: &str, json_data: JsonValue) -> AppResult<UserData> {
    let user_data = user_datas::table
        .filter(user_datas::user_id.eq(user_id))
        .filter(user_datas::data_type.eq(event_type))
        .first::<UserData>(&mut connect()?)
        .optional()?;
    if let Some(user_data) = user_data
        && user_data.json_data == json_data
    {
        return Ok(user_data);
    }

    let new_data = NewUserData {
        user_id: user_id.to_owned(),
        data_type: event_type.to_owned(),
        json_data,
        created_at: Utc::now(),
    };
    diesel::insert_into(user_datas::table)
        .values(&new_data)
        .on_conflict((user_datas::user_id, user_datas::data_type))
        .do_update()
        .set(&new_data)
        .get_result::<UserData>(&mut connect()?)
        .map_err(Into::into)
}

#[tracing::instrument]
pub fn get_data<E: DeserializeOwned>(user_id: i64, kind: &str) -> AppResult<E> {
    let row = user_datas::table
        .filter(user_datas::user_id.eq(user_id))
        .filter(user_datas::data_type.eq(kind))
        .order_by(user_datas::id.desc())
        .first::<UserData>(&mut connect()?)?;
    Ok(serde_json::from_value(row.json_data)?)
}
