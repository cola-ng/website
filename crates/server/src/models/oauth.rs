use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use salvo::http::StatusError;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AppResult;
use crate::db::url_filter::JoinedOption;
use crate::db::schema::*;


#[derive(Queryable, Identifiable,  Serialize, Debug, Clone)]
#[diesel(table_name = oauth_identities)]
#[diesel(belongs_to(User))]
pub struct OauthIdentity {
    pub id: i64,
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub user_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = oauth_identities)]
pub struct NewOauthIdentity {
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub user_id: Option<i64>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = oauth_login_sessions)]
pub struct OauthLoginSession {
    pub id: i64,
    pub provider: String,
    pub state: String,
    pub redirect_uri: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = oauth_login_sessions)]
pub struct NewOauthLoginSession {
    pub provider: String,
    pub state: String,
    pub redirect_uri: String,
    pub expires_at: DateTime<Utc>,
}



#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
#[diesel(table_name = oauth_accesses)]
pub struct OauthAccess {
    pub id: i64,
    pub owner_id: i64,
    pub platform: String,
    pub access_token: String,
    pub token_type: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub shopify_shop: Option<String>,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Debug)]
#[diesel(table_name = oauth_accesses)]
pub struct NewOauthAccess<'a> {
    pub owner_id: i64,
    pub platform: &'a str,
    pub access_token: &'a str,
    pub token_type: &'a str,
    pub refresh_token: Option<&'a str>,
    pub expires_at: DateTime<Utc>,
    pub shopify_shop: Option<&'a str>,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}
pub static OAUTH_USER_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "user_id", "platform", "token_type", "updated_by", "created_by"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static OAUTH_USERS_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
#[diesel(table_name = oauth_users)]
pub struct OauthUser {
    pub id: i64,
    pub user_id: Option<i64>,
    pub platform: String,
    pub me_id: String,
    pub me_full_name: Option<String>,
    pub me_email: Option<String>,
    pub me_phone: Option<String>,
    pub access_token: String,
    pub token_type: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub shopify_shop: Option<String>,

    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, AsChangeset, Clone, Debug)]
#[diesel(table_name = oauth_users)]
pub struct NewOauthUser<'a> {
    pub user_id: Option<i64>,
    pub platform: &'a str,
    pub me_id: &'a str,
    pub me_full_name: Option<&'a str>,
    pub me_email: Option<&'a str>,
    pub me_phone: Option<&'a str>,
    pub access_token: &'a str,
    pub token_type: &'a str,
    pub refresh_token: Option<&'a str>,
    pub expires_at: DateTime<Utc>,
    pub shopify_shop: Option<&'a str>,

    pub created_at: DateTime<Utc>,
}