use diesel::prelude::*;

use crate::AppResult;
use crate::db::connect;
use crate::db::schema::*;

#[derive(Identifiable, Queryable, Debug, Clone)]
#[diesel(table_name = user_profiles, primary_key(user_id))]
pub struct UserProfile {
    pub user_id: i64,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub blurhash: Option<String>,
}

pub fn get_profile(user_id: i64) -> AppResult<Option<UserProfile>> {
    let profile = user_profiles::table
        .filter(user_profiles::user_id.eq(user_id))
        .first::<UserProfile>(&mut conn()?)
        .optional()?;
    Ok(profile)
}
