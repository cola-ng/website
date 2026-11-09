use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::schema::*;

// ============================================================================
// User AI Provider Settings
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = user_ai_providers)]
pub struct UserAiProvider {
    pub id: i64,
    pub user_id: i64,
    pub provider_type: String,
    pub is_active: bool,
    pub settings: Value,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub test_status: Option<String>,
    pub test_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tts_active: bool,
    pub asr_active: bool,
}

#[derive(Insertable)]
#[diesel(table_name = user_ai_providers)]
pub struct NewUserAiProvider {
    pub user_id: i64,
    pub provider_type: String,
    pub is_active: bool,
    pub settings: Value,
    pub tts_active: bool,
    pub asr_active: bool,
}

#[derive(AsChangeset, Default)]
#[diesel(table_name = user_ai_providers)]
pub struct UpdateUserAiProvider {
    pub is_active: Option<bool>,
    pub tts_active: Option<bool>,
    pub asr_active: Option<bool>,
    pub settings: Option<Value>,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub test_status: Option<String>,
    pub test_error: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

// ============================================================================
// API Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAiProviderRequest {
    pub provider_type: String,
    pub settings: Value,
    #[serde(default)]
    pub set_active: bool,
    pub activate_for: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAiProviderRequest {
    pub activate_for: Option<Vec<String>>,
    pub settings: Option<Value>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TestProviderResponse {
    pub success: bool,
    pub error: Option<String>,
    pub tested_at: DateTime<Utc>,
}
