use diesel::prelude::*;
use diesel::result::Error as DieselError;

use chrono::{DateTime, Utc};

use crate::db::connect;
use crate::db::schema::*;
use crate::models::user::{NewAccessToken, NewRefreshToken};
use crate::{AppResult, diesel_exists};

#[derive(Identifiable, Queryable, Debug, Clone)]
#[diesel(table_name = user_devices)]
pub struct UserDevice {
    pub id: i64,

    pub user_id: i64,

    /// Public display name of the device.
    pub name: Option<String>,

    pub user_agent: Option<String>,

    pub is_hidden: bool,
    /// Most recently seen IP address of the session.
    pub last_seen_ip: Option<String>,

    /// Unix timestamp that the session was last active.
    pub last_seen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = user_devices)]
pub struct NewUserDevice {
    pub user_id: i64,

    /// Public display name of the device.
    pub name: Option<String>,

    pub user_agent: Option<String>,

    pub is_hidden: bool,
    /// Most recently seen IP address of the session.
    pub last_seen_ip: Option<String>,

    /// Unix timestamp that the session was last active.
    pub last_seen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

pub fn create_device(
    user_id: i64,
    token: &str,
    initial_name: Option<String>,
    last_seen_ip: Option<String>,
) -> AppResult<UserDevice> {
    let device = diesel::insert_into(user_devices::table)
        .values(NewUserDevice {
            user_id,
            name: initial_name,
            user_agent: None,
            is_hidden: false,
            last_seen_ip,
            last_seen_at: Utc::now(),
            created_at: Utc::now(),
        })
        .get_result::<UserDevice>(&mut connect()?)?;

    diesel::insert_into(user_access_tokens::table)
        .values(NewAccessToken::new(
            user_id,
            device.id,
            token.to_owned(),
            None,
        ))
        .execute(&mut connect()?)?;
    Ok(device)
}

pub fn get_device(device_id: i64) -> AppResult<UserDevice> {
    user_devices::table
        .find(device_id)
        .first::<UserDevice>(&mut connect()?)
        .map_err(Into::into)
}

pub fn is_device_exists(device_id: i64) -> AppResult<bool> {
    let query = user_devices::table.filter(user_devices::id.eq(device_id));
    diesel_exists!(query, &mut connect()?).map_err(Into::into)
}

pub fn set_refresh_token(
    user_id: i64,
    device_id: i64,
    token: &str,
    expires_at: DateTime<Utc>,
    ultimate_session_expires_at: DateTime<Utc>,
) -> AppResult<i64> {
    let id = connect()?.transaction::<_, DieselError, _>(|conn| {
        diesel::delete(
            user_refresh_tokens::table
                .filter(user_refresh_tokens::user_id.eq(user_id))
                .filter(user_refresh_tokens::device_id.eq(device_id)),
        )
        .execute(conn)?;
        diesel::insert_into(user_refresh_tokens::table)
            .values(NewRefreshToken::new(
                user_id.to_owned(),
                device_id.to_owned(),
                token.to_owned(),
                expires_at,
                ultimate_session_expires_at,
            ))
            .returning(user_refresh_tokens::id)
            .get_result::<i64>(conn)
    })?;

    Ok(id)
}

pub fn set_access_token(
    user_id: i64,
    device_id: i64,
    token: &str,
    refresh_token_id: Option<i64>,
) -> AppResult<()> {
    diesel::insert_into(user_access_tokens::table)
        .values(NewAccessToken::new(
            user_id.to_owned(),
            device_id.to_owned(),
            token.to_owned(),
            refresh_token_id,
        ))
        .on_conflict((user_access_tokens::user_id, user_access_tokens::device_id))
        .do_update()
        .set(user_access_tokens::token.eq(token))
        .execute(&mut connect()?)?;
    Ok(())
}

pub fn delete_access_tokens(user_id: i64, device_id: i64) -> AppResult<()> {
    diesel::delete(
        user_access_tokens::table
            .filter(user_access_tokens::user_id.eq(user_id))
            .filter(user_access_tokens::device_id.eq(device_id)),
    )
    .execute(&mut connect()?)?;
    Ok(())
}

pub fn delete_refresh_tokens(user_id: i64, device_id: i64) -> AppResult<()> {
    diesel::delete(
        user_refresh_tokens::table
            .filter(user_refresh_tokens::user_id.eq(user_id))
            .filter(user_refresh_tokens::device_id.eq(device_id)),
    )
    .execute(&mut connect()?)?;
    Ok(())
}
