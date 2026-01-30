use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::db::schema::*;

// ============================================================================
// AI Service Type
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiServiceType {
    Chat,
    Asr,
    Tts,
    Translation,
}

impl AiServiceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AiServiceType::Chat => "chat",
            AiServiceType::Asr => "asr",
            AiServiceType::Tts => "tts",
            AiServiceType::Translation => "translation",
        }
    }
}

impl std::fmt::Display for AiServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// AI Usage Record
// ============================================================================

#[derive(Identifiable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = ai_usage_records)]
pub struct AiUsageRecord {
    pub id: i64,
    pub user_id: i64,
    pub service_type: String,
    pub provider_type: String,
    pub is_user_provider: bool,
    pub tokens_used: Option<i32>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub duration_seconds: Option<f64>,
    pub characters_count: Option<i32>,
    pub model_name: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = ai_usage_records)]
pub struct NewAiUsageRecord {
    pub user_id: i64,
    pub service_type: String,
    pub provider_type: String,
    pub is_user_provider: bool,
    pub tokens_used: Option<i32>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub duration_seconds: Option<f64>,
    pub characters_count: Option<i32>,
    pub model_name: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub metadata: Option<JsonValue>,
}

// ============================================================================
// AI Usage Quota
// ============================================================================

#[derive(Identifiable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = ai_usage_quotas)]
pub struct AiUsageQuota {
    pub id: i64,
    pub user_id: i64,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub chat_tokens_used: i64,
    pub asr_seconds_used: f64,
    pub tts_seconds_used: f64,
    pub translation_chars_used: i64,
    pub chat_requests: i32,
    pub asr_requests: i32,
    pub tts_requests: i32,
    pub translation_requests: i32,
    pub limits_snapshot: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = ai_usage_quotas)]
pub struct NewAiUsageQuota {
    pub user_id: i64,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub limits_snapshot: Option<JsonValue>,
}

#[derive(AsChangeset, Debug, Default)]
#[diesel(table_name = ai_usage_quotas)]
pub struct UpdateAiUsageQuota {
    pub chat_tokens_used: Option<i64>,
    pub asr_seconds_used: Option<f64>,
    pub tts_seconds_used: Option<f64>,
    pub translation_chars_used: Option<i64>,
    pub chat_requests: Option<i32>,
    pub asr_requests: Option<i32>,
    pub tts_requests: Option<i32>,
    pub translation_requests: Option<i32>,
    pub updated_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Response Types
// ============================================================================

/// Service usage details for a single service type
#[derive(Serialize, ToSchema, Debug, Clone)]
pub struct ServiceUsage {
    pub used: i64,
    pub limit: i64, // -1 for unlimited
    pub unit: String,
    pub requests: i32,
    pub percentage: Option<f64>,
}

impl ServiceUsage {
    pub fn new(used: i64, limit: i64, unit: &str, requests: i32) -> Self {
        let percentage = if limit > 0 {
            Some((used as f64 / limit as f64) * 100.0)
        } else if limit == -1 {
            None // Unlimited
        } else {
            Some(100.0) // Zero limit, 100% used
        };

        Self {
            used,
            limit,
            unit: unit.to_string(),
            requests,
            percentage,
        }
    }
}

/// Complete usage summary for current billing period
#[derive(Serialize, ToSchema, Debug)]
pub struct UsageSummary {
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub days_remaining: i64,
    pub chat: ServiceUsage,
    pub asr: ServiceUsage,
    pub tts: ServiceUsage,
    pub translation: ServiceUsage,
}

/// Quota check result
#[derive(Serialize, ToSchema, Debug)]
pub struct QuotaCheckResult {
    pub allowed: bool,
    pub service_type: String,
    pub used: i64,
    pub limit: i64,
    pub remaining: i64,
    pub unit: String,
    pub message: Option<String>,
}

/// AI limits from subscription plan
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone, Default)]
pub struct AiLimits {
    #[serde(default)]
    pub chat: ServiceLimits,
    #[serde(default)]
    pub asr: ServiceLimits,
    #[serde(default)]
    pub tts: ServiceLimits,
    #[serde(default)]
    pub translation: ServiceLimits,
}

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone, Default)]
pub struct ServiceLimits {
    #[serde(default)]
    pub daily_requests: i32,
    #[serde(default)]
    pub monthly_tokens: i64,
    #[serde(default)]
    pub monthly_seconds: f64,
    #[serde(default)]
    pub monthly_chars: i64,
}

/// Admin view: User usage with user info
#[derive(Serialize, ToSchema, Debug)]
pub struct UserUsageInfo {
    pub user_id: i64,
    pub user_name: String,
    pub user_email: Option<String>,
    pub plan_name: Option<String>,
    pub usage: UsageSummary,
}

/// Admin stats: System-wide usage statistics
#[derive(Serialize, ToSchema, Debug)]
pub struct SystemUsageStats {
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub total_users: i64,
    pub active_users: i64,
    pub total_chat_tokens: i64,
    pub total_asr_seconds: f64,
    pub total_tts_seconds: f64,
    pub total_translation_chars: i64,
    pub total_requests: i64,
    pub by_service: Vec<ServiceStats>,
}

#[derive(Serialize, ToSchema, Debug)]
pub struct ServiceStats {
    pub service_type: String,
    pub total_requests: i64,
    pub total_usage: f64,
    pub unit: String,
    pub unique_users: i64,
}
