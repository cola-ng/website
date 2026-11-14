use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::schema::*;

// ============================================================================
// User Skill Levels
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_levels)]
pub struct UserLevel {
    pub id: i64,
    pub user_id: i64,
    pub skill_type: String,
    pub level: i32,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_levels)]
pub struct NewUserLevel {
    pub user_id: i64,
    pub skill_type: String,
    pub level: i32,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = learn_levels)]
pub struct UpdateUserLevel {
    pub level: Option<i32>,
}

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
// Study Time Daily (heartbeat-based tracking)
// ============================================================================

/// 页面类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PageType {
    Chat,
    Review,
    Stage,
    Read,
    Dictionary,
    Translate,
}

impl PageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PageType::Chat => "chat",
            PageType::Review => "review",
            PageType::Stage => "stage",
            PageType::Read => "read",
            PageType::Dictionary => "dictionary",
            PageType::Translate => "translate",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "chat" => Some(PageType::Chat),
            "review" => Some(PageType::Review),
            "stage" => Some(PageType::Stage),
            "read" => Some(PageType::Read),
            "dictionary" => Some(PageType::Dictionary),
            "translate" => Some(PageType::Translate),
            _ => None,
        }
    }
}

/// 每日学习时间统计
#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_study_time_daily)]
pub struct StudyTimeDaily {
    pub id: i64,
    pub user_id: i64,
    pub stat_date: NaiveDate,
    pub chat_seconds: i32,
    pub review_seconds: i32,
    pub stage_seconds: i32,
    pub read_seconds: i32,
    pub dictionary_seconds: i32,
    pub translate_seconds: i32,
    pub total_seconds: i32,
    pub heartbeat_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 新建每日学习时间记录
#[derive(Insertable)]
#[diesel(table_name = learn_study_time_daily)]
pub struct NewStudyTimeDaily {
    pub user_id: i64,
    pub stat_date: NaiveDate,
    pub chat_seconds: i32,
    pub review_seconds: i32,
    pub stage_seconds: i32,
    pub read_seconds: i32,
    pub dictionary_seconds: i32,
    pub translate_seconds: i32,
    pub total_seconds: i32,
    pub heartbeat_count: i32,
}

/// 用户活动状态 (存储在内存中)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivityState {
    pub last_activity_time: DateTime<Utc>,
    pub last_page_type: PageType,
}

/// 待持久化的学习时间增量
#[derive(Debug, Clone, Default)]
pub struct StudyTimeIncrement {
    pub chat_seconds: i32,
    pub review_seconds: i32,
    pub stage_seconds: i32,
    pub read_seconds: i32,
    pub dictionary_seconds: i32,
    pub translate_seconds: i32,
    pub heartbeat_count: i32,
}

impl StudyTimeIncrement {
    pub fn merge(&mut self, other: &Self) {
        self.chat_seconds += other.chat_seconds;
        self.review_seconds += other.review_seconds;
        self.stage_seconds += other.stage_seconds;
        self.read_seconds += other.read_seconds;
        self.dictionary_seconds += other.dictionary_seconds;
        self.translate_seconds += other.translate_seconds;
        self.heartbeat_count += other.heartbeat_count;
    }
    pub fn add_seconds(&mut self, page_type: PageType, seconds: i32) {
        match page_type {
            PageType::Chat => self.chat_seconds += seconds,
            PageType::Review => self.review_seconds += seconds,
            PageType::Stage => self.stage_seconds += seconds,
            PageType::Read => self.read_seconds += seconds,
            PageType::Dictionary => self.dictionary_seconds += seconds,
            PageType::Translate => self.translate_seconds += seconds,
        }
        self.heartbeat_count += 1;
    }

    pub fn total_seconds(&self) -> i32 {
        self.chat_seconds
            + self.review_seconds
            + self.stage_seconds
            + self.read_seconds
            + self.dictionary_seconds
            + self.translate_seconds
    }

    pub fn is_empty(&self) -> bool {
        self.total_seconds() == 0
    }
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

// ============================================================================
// Translations (saved translation results for favorites)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_translations)]
pub struct Translation {
    pub id: i64,
    pub user_id: i64,
    pub source_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub tone: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_translations)]
pub struct NewTranslation {
    pub user_id: i64,
    pub source_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub tone: String,
}

// ============================================================================
// Favorites (unified favorites system)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_favorites)]
pub struct Favorite {
    pub id: i64,
    pub user_id: i64,
    pub entity: String,
    pub record_id: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_favorites)]
pub struct NewFavorite {
    pub user_id: i64,
    pub entity: String,
    pub record_id: i64,
}

// ============================================================================
// Daily Tasks
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_daily_tasks)]
pub struct DailyTask {
    pub id: i64,
    pub user_id: i64,
    pub task_date: NaiveDate,
    pub task_type: String,
    pub task_title: String,
    pub task_description: Option<String>,
    pub target_value: i32,
    pub current_value: i32,
    pub status: String,
    pub display_order: i32,
    pub reference_id: Option<i64>,
    pub reference_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_daily_tasks)]
pub struct NewDailyTask {
    pub user_id: i64,
    pub task_date: NaiveDate,
    pub task_type: String,
    pub task_title: String,
    pub task_description: Option<String>,
    pub target_value: i32,
    pub current_value: Option<i32>,
    pub status: Option<String>,
    pub display_order: i32,
    pub reference_id: Option<i64>,
    pub reference_name: Option<String>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = learn_daily_tasks)]
pub struct UpdateDailyTask {
    pub current_value: Option<i32>,
    pub status: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Daily Insights (AI-generated)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_daily_insights)]
pub struct DailyInsight {
    pub id: i64,
    pub user_id: i64,
    pub insight_date: NaiveDate,
    pub insight_type: String,
    pub icon_type: String,
    pub content_zh: String,
    pub content_en: Option<String>,
    pub action_text: Option<String>,
    pub action_link: Option<String>,
    pub metadata: Option<Value>,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = learn_daily_insights)]
pub struct NewDailyInsight {
    pub user_id: i64,
    pub insight_date: NaiveDate,
    pub insight_type: String,
    pub icon_type: Option<String>,
    pub content_zh: String,
    pub content_en: Option<String>,
    pub action_text: Option<String>,
    pub action_link: Option<String>,
    pub metadata: Option<Value>,
    pub display_order: i32,
}

// ============================================================================
// Daily Generation Log
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = learn_daily_generation_log)]
pub struct DailyGenerationLog {
    pub id: i64,
    pub user_id: i64,
    pub generation_date: NaiveDate,
    pub tasks_generated: i32,
    pub insights_generated: i32,
    pub generation_source: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = learn_daily_generation_log)]
pub struct NewDailyGenerationLog {
    pub user_id: i64,
    pub generation_date: NaiveDate,
    pub tasks_generated: i32,
    pub insights_generated: i32,
    pub generation_source: Option<String>,
}
