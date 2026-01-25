use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::schema::*;

// ============================================================================
// Issue Words (user-specific vocabulary issues)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_issue_words)]
pub struct IssueWord {
    pub id: i64,
    pub user_id: i64,
    pub word: String,
    pub issue_type: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub last_picked_at: Option<DateTime<Utc>>,
    pub pick_count: i32,
    pub next_review_at: Option<DateTime<Utc>>,
    pub review_interval_days: Option<i32>,
    pub difficulty: Option<i32>,
    pub context: Option<String>,
    pub audio_timestamp: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_issue_words)]
pub struct NewIssueWord {
    pub user_id: i64,
    pub word: String,
    pub issue_type: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub context: Option<String>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = learn_issue_words)]
pub struct UpdateIssueWord {
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub next_review_at: Option<DateTime<Utc>>,
    pub review_interval_days: Option<i32>,
    pub difficulty: Option<i32>,
}

// ============================================================================
// Chat (conversation sessions)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_chats)]
pub struct Chat {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub context_id: Option<i64>,
    pub duration_ms: Option<i32>,
    pub issues_count: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_chats)]
pub struct NewChat {
    pub user_id: i64,
    pub title: String,
    pub context_id: Option<i64>,
    pub duration_ms: Option<i32>,
    pub issues_count: Option<i32>,
}

// ============================================================================
// Chat Turns (conversation messages)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_chat_turns)]
pub struct ChatTurn {
    pub id: i64,
    pub user_id: i64,
    pub chat_id: i64,
    pub speaker: String,
    pub use_lang: String,
    pub content_en: String,
    pub content_zh: String,
    pub audio_path: Option<String>,
    pub duration_ms: Option<i32>,
    pub words_per_minute: Option<f32>,
    pub issues_count: i32,
    pub hesitation_count: Option<i32>,
    pub status: String,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_chat_turns)]
pub struct NewChatTurn {
    pub user_id: i64,
    pub chat_id: i64,
    pub speaker: String,
    pub use_lang: String,
    pub content_en: String,
    pub content_zh: String,
    pub audio_path: Option<String>,
    pub duration_ms: Option<i32>,
    pub words_per_minute: Option<f32>,
    pub issues_count: Option<i32>,
    pub hesitation_count: Option<i32>,
    pub status: String,
}

// ============================================================================
// Chat Issues (feedback on chat turns)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_chat_issues)]
pub struct ChatIssue {
    pub id: i64,
    pub user_id: i64,
    pub chat_id: i64,
    pub chat_turn_id: i64,
    pub issue_type: String,
    pub start_position: Option<i32>,
    pub end_position: Option<i32>,
    pub original_text: Option<String>,
    pub suggested_text: Option<String>,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub severity: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_chat_issues)]
pub struct NewChatIssue {
    pub user_id: i64,
    pub chat_id: i64,
    pub chat_turn_id: i64,
    pub issue_type: String,
    pub start_position: Option<i32>,
    pub end_position: Option<i32>,
    pub original_text: Option<String>,
    pub suggested_text: Option<String>,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub severity: Option<String>,
}

// ============================================================================
// Practices (general practice sessions)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_practices)]
pub struct Practice {
    pub id: i64,
    pub user_id: i64,
    pub success_level: Option<i32>,
    pub notes: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_practices)]
pub struct NewPractice {
    pub user_id: i64,
    pub success_level: Option<i32>,
    pub notes: Option<String>,
}

// ============================================================================
// Write Practices (writing practice logs)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_write_practices)]
pub struct WritePractice {
    pub id: i64,
    pub user_id: i64,
    pub word_id: i64,
    pub practice_id: String,
    pub success_level: Option<i32>,
    pub notes: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_write_practices)]
pub struct NewWritePractice {
    pub user_id: i64,
    pub word_id: i64,
    pub practice_id: String,
    pub success_level: Option<i32>,
    pub notes: Option<String>,
}

// ============================================================================
// Reading Practice Attempts
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_read_practices)]
pub struct ReadPractice {
    pub id: i64,
    pub user_id: i64,
    pub sentence_id: Option<i64>,
    pub practice_id: String,
    pub user_audio_path: Option<String>,
    pub pronunciation_score: Option<i32>,
    pub fluency_score: Option<i32>,
    pub intonation_score: Option<i32>,
    pub overall_score: Option<i32>,
    pub detected_errors: Option<Value>,
    pub ai_feedback_en: Option<String>,
    pub ai_feedback_zh: Option<String>,
    pub waveform_data: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_read_practices)]
pub struct NewReadPractice {
    pub user_id: i64,
    pub sentence_id: Option<i64>,
    pub practice_id: String,
    pub user_audio_path: Option<String>,
    pub pronunciation_score: Option<i32>,
    pub fluency_score: Option<i32>,
    pub intonation_score: Option<i32>,
    pub overall_score: Option<i32>,
    pub detected_errors: Option<Value>,
    pub ai_feedback_en: Option<String>,
    pub ai_feedback_zh: Option<String>,
    pub waveform_data: Option<Value>,
}

// ============================================================================
// Reading Progress
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_read_progress)]
pub struct ReadProgress {
    pub id: i64,
    pub user_id: i64,
    pub exercise_id: i64,
    pub current_sentence_order: Option<i32>,
    pub progress_percent: Option<i32>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_practiced_at: Option<DateTime<Utc>>,
    pub practice_count: Option<i32>,
    pub average_score: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_read_progress)]
pub struct NewReadProgress {
    pub user_id: i64,
    pub exercise_id: i64,
    pub current_sentence_order: Option<i32>,
    pub progress_percent: Option<i32>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = learn_read_progress)]
pub struct UpdateReadProgress {
    pub current_sentence_order: Option<i32>,
    pub progress_percent: Option<i32>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_practiced_at: Option<DateTime<Utc>>,
    pub practice_count: Option<i32>,
    pub average_score: Option<i32>,
}

// ============================================================================
// Script Progress
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_script_progress)]
pub struct ScriptProgress {
    pub id: i64,
    pub user_id: i64,
    pub stage_id: i64,
    pub script_id: i64,
    pub progress_percent: Option<i32>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_practiced_at: Option<DateTime<Utc>>,
    pub practice_count: Option<i32>,
    pub best_score: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_script_progress)]
pub struct NewScriptProgress {
    pub user_id: i64,
    pub stage_id: i64,
    pub script_id: i64,
    pub progress_percent: Option<i32>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = learn_script_progress)]
pub struct UpdateScriptProgress {
    pub progress_percent: Option<i32>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_practiced_at: Option<DateTime<Utc>>,
    pub practice_count: Option<i32>,
    pub best_score: Option<i32>,
}

// ============================================================================
// User Achievements
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_achievements)]
pub struct UserAchievement {
    pub id: i64,
    pub user_id: i64,
    pub achievement_type: String,
    pub achievement_name: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub metadata: Option<Value>,
    pub earned_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_achievements)]
pub struct NewUserAchievement {
    pub user_id: i64,
    pub achievement_type: String,
    pub achievement_name: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub metadata: Option<Value>,
}

// ============================================================================
// Daily Stats
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_daily_stats)]
pub struct DailyStat {
    pub id: i64,
    pub user_id: i64,
    pub stat_date: NaiveDate,
    pub minutes_studied: Option<i32>,
    pub words_practiced: Option<i32>,
    pub sessions_completed: Option<i32>,
    pub errors_corrected: Option<i32>,
    pub new_words_learned: Option<i32>,
    pub review_words_count: Option<i32>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_daily_stats)]
pub struct NewDailyStat {
    pub user_id: i64,
    pub stat_date: NaiveDate,
    pub minutes_studied: Option<i32>,
    pub words_practiced: Option<i32>,
    pub sessions_completed: Option<i32>,
    pub errors_corrected: Option<i32>,
    pub new_words_learned: Option<i32>,
    pub review_words_count: Option<i32>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = learn_daily_stats)]
pub struct UpdateDailyStat {
    pub minutes_studied: Option<i32>,
    pub words_practiced: Option<i32>,
    pub sessions_completed: Option<i32>,
    pub errors_corrected: Option<i32>,
    pub new_words_learned: Option<i32>,
    pub review_words_count: Option<i32>,
}

// ============================================================================
// User Vocabularies
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_vocabularies)]
pub struct UserVocabulary {
    pub id: i64,
    pub user_id: i64,
    pub word: String,
    pub word_zh: Option<String>,
    pub mastery_level: Option<i32>,
    pub first_seen_at: DateTime<Utc>,
    pub last_practiced_at: Option<DateTime<Utc>>,
    pub practice_count: Option<i32>,
    pub correct_count: Option<i32>,
    pub next_review_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_vocabularies)]
pub struct NewUserVocabulary {
    pub user_id: i64,
    pub word: String,
    pub word_zh: Option<String>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = learn_vocabularies)]
pub struct UpdateUserVocabulary {
    pub word_zh: Option<String>,
    pub mastery_level: Option<i32>,
    pub last_practiced_at: Option<DateTime<Utc>>,
    pub practice_count: Option<i32>,
    pub correct_count: Option<i32>,
    pub next_review_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Suggestions
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_suggestions)]
pub struct Suggestion {
    pub id: i64,
    pub user_id: i64,
    pub suggestion_type: Option<String>,
    pub suggested_text: String,
    pub was_accepted: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_suggestions)]
pub struct NewSuggestion {
    pub user_id: i64,
    pub suggestion_type: Option<String>,
    pub suggested_text: String,
    pub was_accepted: Option<bool>,
}
