use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::schema::{desktop_auth_codes, learning_records, users};

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
