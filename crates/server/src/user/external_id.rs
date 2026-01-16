use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::AppResult;
use crate::db::connect;
use crate::db::schema::*;

#[derive(Insertable, Identifiable, Queryable, Debug, Clone)]
#[diesel(table_name = user_external_ids)]
pub struct UserExternalId {
    pub id: i64,
    pub auth_provider: String,
    pub external_id: String,
    pub user_id: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = user_external_ids)]
pub struct NewUserExternalId {
    pub auth_provider: String,
    pub external_id: String,
    pub user_id: i64,
    pub created_at: DateTime<Utc>,
}

/// Get user_id by external auth provider and external_id
pub fn get_user_by_external_id(auth_provider: &str, external_id: &str) -> AppResult<Option<i64>> {
    user_external_ids::table
        .filter(user_external_ids::auth_provider.eq(auth_provider))
        .filter(user_external_ids::external_id.eq(external_id))
        .select(user_external_ids::user_id)
        .first::<i64>(&mut conn()?)
        .optional()
        .map_err(Into::into)
}

/// Get all external IDs for a user
pub fn get_external_ids_by_user(user_id: i64) -> AppResult<Vec<UserExternalId>> {
    user_external_ids::table
        .filter(user_external_ids::user_id.eq(user_id))
        .load::<UserExternalId>(&mut conn()?)
        .map_err(Into::into)
}

/// Record a new external ID for a user
pub fn record_external_id(auth_provider: &str, external_id: &str, user_id: i64) -> AppResult<()> {
    diesel::insert_into(user_external_ids::table)
        .values(NewUserExternalId {
            auth_provider: auth_provider.to_owned(),
            external_id: external_id.to_owned(),
            user_id: user_id.to_owned(),
            created_at: Utc::now(),
        })
        .execute(&mut conn()?)?;
    Ok(())
}

/// Replace all external IDs for a user
pub fn replace_external_ids(
    user_id: i64,
    new_external_ids: &[(String, String)], // (auth_provider, external_id)
) -> AppResult<()> {
    let mut conn = conn()?;

    // Delete existing external IDs for this user
    diesel::delete(user_external_ids::table.filter(user_external_ids::user_id.eq(user_id)))
        .execute(&mut conn)?;

    // Insert new external IDs
    let now = Utc::now();
    for (auth_provider, external_id) in new_external_ids {
        diesel::insert_into(user_external_ids::table)
            .values(NewUserExternalId {
                auth_provider: auth_provider.clone(),
                external_id: external_id.clone(),
                user_id: user_id.to_owned(),
                created_at: now,
            })
            .execute(&mut conn)?;
    }

    Ok(())
}

/// Delete a specific external ID
pub fn delete_external_id(auth_provider: &str, external_id: &str) -> AppResult<()> {
    diesel::delete(
        user_external_ids::table
            .filter(user_external_ids::auth_provider.eq(auth_provider))
            .filter(user_external_ids::external_id.eq(external_id)),
    )
    .execute(&mut conn()?)?;
    Ok(())
}
