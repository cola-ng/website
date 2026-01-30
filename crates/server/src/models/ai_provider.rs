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
}

#[derive(Insertable)]
#[diesel(table_name = user_ai_providers)]
pub struct NewUserAiProvider {
    pub user_id: i64,
    pub provider_type: String,
    pub is_active: bool,
    pub settings: Value,
}

#[derive(AsChangeset, Default)]
#[diesel(table_name = user_ai_providers)]
pub struct UpdateUserAiProvider {
    pub is_active: Option<bool>,
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
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAiProviderRequest {
    pub settings: Option<Value>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TestProviderResponse {
    pub success: bool,
    pub error: Option<String>,
    pub tested_at: DateTime<Utc>,
}

// ============================================================================
// Provider Settings Types (for JSON validation)
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZhipuSettings {
    pub api_key: String,
    #[serde(default = "default_zhipu_chat_model")]
    pub chat_model: String,
}

fn default_zhipu_chat_model() -> String {
    "glm-4-flash".to_string()
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DoubaoSettings {
    pub app_id: String,
    pub access_token: String,
    pub api_key: String,
    #[serde(default = "default_doubao_chat_model")]
    pub chat_model: String,
    pub tts_resource_id: Option<String>,
    pub voice_type: Option<String>,
}

fn default_doubao_chat_model() -> String {
    "doubao-1-5-pro-32k-250115".to_string()
}

// ============================================================================
// Provider Type Enum
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Zhipu,
    Doubao,
}

impl ProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderType::Zhipu => "zhipu",
            ProviderType::Doubao => "doubao",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "zhipu" => Some(ProviderType::Zhipu),
            "doubao" => Some(ProviderType::Doubao),
            _ => None,
        }
    }
}
