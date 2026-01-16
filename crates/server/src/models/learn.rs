use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::schema::*;

// ============================================================================
// User-specific models (with user_id)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
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
    pub difficulty_level: Option<i32>,
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
    pub difficulty_level: Option<i32>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = learn_sessions)]
pub struct LearningSession {
    pub id: i64,
    pub session_id: String,
    pub user_id: i64,
    pub session_type: Option<String>,
    pub scene_id: Option<i64>,
    pub dialogue_id: Option<i64>,
    pub classic_clip_id: Option<i64>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
    pub total_words_spoken: Option<i32>,
    pub average_wpm: Option<f32>,
    pub error_count: Option<i32>,
    pub correction_count: Option<i32>,
    pub notes: Option<String>,
    pub ai_summary_en: Option<String>,
    pub ai_summary_zh: Option<String>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_sessions)]
pub struct NewLearningSession {
    pub session_id: String,
    pub user_id: i64,
    pub session_type: Option<String>,
    pub scene_id: Option<i64>,
    pub dialogue_id: Option<i64>,
    pub classic_clip_id: Option<i64>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = learn_sessions)]
pub struct UpdateLearningSession {
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
    pub total_words_spoken: Option<i32>,
    pub average_wpm: Option<f32>,
    pub error_count: Option<i32>,
    pub correction_count: Option<i32>,
    pub notes: Option<String>,
    pub ai_summary_en: Option<String>,
    pub ai_summary_zh: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = learn_conversations)]
pub struct Conversation {
    pub id: i64,
    pub user_id: i64,
    pub session_id: String,
    pub speaker: String,
    pub use_lang: String,
    pub content_en: String,
    pub content_zh: String,
    pub audio_path: Option<String>,
    pub duration_ms: Option<i32>,
    pub words_per_minute: Option<f32>,
    pub pause_count: Option<i32>,
    pub hesitation_count: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_conversations)]
pub struct NewConversation {
    pub user_id: i64,
    pub session_id: String,
    pub speaker: String,
    pub use_lang: String,
    pub content_en: String,
    pub content_zh: String,
    pub audio_path: Option<String>,
    pub duration_ms: Option<i32>,
    pub words_per_minute: Option<f32>,
    pub pause_count: Option<i32>,
    pub hesitation_count: Option<i32>,
}

#[derive(Queryable, Identifiable, Associations, Serialize, Debug, Clone)]
#[diesel(table_name = learn_conversation_annotations)]
#[diesel(belongs_to(Conversation))]
pub struct ConversationAnnotation {
    pub id: i64,
    pub user_id: i64,
    pub conversation_id: i64,
    pub annotation_type: String,
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
#[diesel(table_name = learn_conversation_annotations)]
pub struct NewConversationAnnotation {
    pub user_id: i64,
    pub conversation_id: i64,
    pub annotation_type: String,
    pub start_position: Option<i32>,
    pub end_position: Option<i32>,
    pub original_text: Option<String>,
    pub suggested_text: Option<String>,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub severity: Option<String>,
}

#[derive(Queryable, Identifiable, Associations, Serialize, Debug, Clone)]
#[diesel(table_name = learn_word_practices)]
#[diesel(belongs_to(IssueWord, foreign_key = word_id))]
pub struct WordPracticeLog {
    pub id: i64,
    pub user_id: i64,
    pub word_id: i64,
    pub session_id: String,
    pub success_level: Option<i32>,
    pub notes: Option<String>,
    pub practiced_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_word_practices)]
pub struct NewWordPracticeLog {
    pub user_id: i64,
    pub word_id: i64,
    pub session_id: String,
    pub success_level: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Queryable, Identifiable, Associations, Serialize, Debug, Clone)]
#[diesel(table_name = learn_read_practices)]
#[diesel(belongs_to(ReadingSentence, foreign_key = sentence_id))]
pub struct ReadingPracticeAttempt {
    pub id: i64,
    pub user_id: i64,
    pub sentence_id: i64,
    pub session_id: String,
    pub user_audio_path: Option<String>,
    pub pronunciation_score: Option<i32>,
    pub fluency_score: Option<i32>,
    pub intonation_score: Option<i32>,
    pub overall_score: Option<i32>,
    pub detected_errors: Option<Value>,
    pub ai_feedback_en: Option<String>,
    pub ai_feedback_zh: Option<String>,
    pub waveform_data: Option<Value>,
    pub attempted_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_read_practices)]
pub struct NewReadingPracticeAttempt {
    pub user_id: i64,
    pub sentence_id: i64,
    pub session_id: String,
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

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
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

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
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

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
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

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
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
