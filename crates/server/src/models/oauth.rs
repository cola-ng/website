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


#[derive(Queryable, Identifiable, Associations, Serialize, Debug, Clone)]
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
