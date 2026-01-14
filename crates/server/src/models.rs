use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::db::schema::{
    desktop_auth_codes, learning_records, oauth_identities, oauth_login_sessions, role_permissions,
    roles, user_roles, users,
};

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = users)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub name: Option<String>,
    pub phone: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
    pub name: Option<String>,
    pub phone: Option<String>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = users)]
pub struct UpdateUserProfile {
    pub name: Option<String>,
    pub phone: Option<String>,
}

#[derive(Queryable, Identifiable, Associations, Serialize, Debug, Clone)]
#[diesel(table_name = learning_records)]
#[diesel(belongs_to(User))]
pub struct LearningRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub record_type: String,
    pub content: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learning_records)]
pub struct NewLearningRecord {
    pub user_id: Uuid,
    pub record_type: String,
    pub content: Value,
}

#[derive(Queryable, Identifiable, Associations, Serialize, Debug, Clone)]
#[diesel(table_name = desktop_auth_codes)]
#[diesel(belongs_to(User))]
pub struct DesktopAuthCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code_hash: String,
    pub redirect_uri: String,
    pub state: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = desktop_auth_codes)]
pub struct NewDesktopAuthCode {
    pub user_id: Uuid,
    pub code_hash: String,
    pub redirect_uri: String,
    pub state: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = roles)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = roles)]
pub struct NewRole {
    pub name: String,
}

#[derive(Queryable, Identifiable, Associations, Serialize, Debug, Clone)]
#[diesel(table_name = role_permissions)]
#[diesel(belongs_to(Role))]
pub struct RolePermission {
    pub id: Uuid,
    pub role_id: Uuid,
    pub operation: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = role_permissions)]
pub struct NewRolePermission {
    pub role_id: Uuid,
    pub operation: String,
}

#[derive(Queryable, Associations, Serialize, Debug, Clone)]
#[diesel(table_name = user_roles)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Role))]
#[allow(dead_code)]
pub struct UserRole {
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = user_roles)]
pub struct NewUserRole {
    pub user_id: Uuid,
    pub role_id: Uuid,
}

#[derive(Queryable, Identifiable, Associations, Serialize, Debug, Clone)]
#[diesel(table_name = oauth_identities)]
#[diesel(belongs_to(User))]
pub struct OauthIdentity {
    pub id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = oauth_identities)]
pub struct NewOauthIdentity {
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub user_id: Option<Uuid>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = oauth_login_sessions)]
pub struct OauthLoginSession {
    pub id: Uuid,
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
