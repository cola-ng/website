use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::db::schema::user_activities;

/// User activity record
#[derive(Identifiable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = user_activities)]
pub struct UserActivity {
    pub id: i64,
    pub user_id: i64,
    pub activity_type: String,
    pub description: Option<String>,
    pub metadata: Option<JsonValue>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// New user activity for insertion
#[derive(Insertable, Debug)]
#[diesel(table_name = user_activities)]
pub struct NewUserActivity {
    pub user_id: i64,
    pub activity_type: String,
    pub description: Option<String>,
    pub metadata: Option<JsonValue>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Activity type constants
pub mod activity_types {
    pub const LOGIN: &str = "login";
    pub const LOGOUT: &str = "logout";
    pub const PAYMENT: &str = "payment";
    pub const SUBSCRIPTION: &str = "subscription";
    pub const PASSWORD_CHANGE: &str = "password_change";
    pub const PROFILE_UPDATE: &str = "profile_update";
    pub const ADMIN_GRANT: &str = "admin_grant";
    pub const ADMIN_REVOKE: &str = "admin_revoke";
    pub const REGISTER: &str = "register";
}

/// Response type for activity list
#[derive(Serialize, ToSchema)]
pub struct UserActivityResponse {
    pub id: i64,
    pub activity_type: String,
    pub description: Option<String>,
    pub metadata: Option<JsonValue>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<UserActivity> for UserActivityResponse {
    fn from(a: UserActivity) -> Self {
        Self {
            id: a.id,
            activity_type: a.activity_type,
            description: a.description,
            metadata: a.metadata,
            ip_address: a.ip_address,
            created_at: a.created_at,
        }
    }
}
