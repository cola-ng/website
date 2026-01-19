use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;

// ============================================================================
// Achievement Definitions (global, read-only for users)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = achievement_definitions)]
pub struct AchievementDefinition {
    pub id: i64,
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub icon: Option<String>,
    pub category: String,
    pub rarity: String,
    pub xp_reward: i32,
    pub requirement_type: String,
    pub requirement_value: i32,
    pub requirement_field: Option<String>,
    pub is_hidden: Option<bool>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Rank Definitions (global, read-only for users)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = rank_definitions)]
pub struct RankDefinition {
    pub id: i64,
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub min_xp: i32,
    pub level: i32,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// User Profile (user-specific)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = user_profiles)]
pub struct UserProfile {
    pub id: i64,
    pub user_id: i64,
    pub total_xp: i32,
    pub current_rank_id: Option<i64>,
    pub current_streak_days: i32,
    pub longest_streak_days: i32,
    pub last_activity_date: Option<NaiveDate>,
    pub total_study_minutes: i32,
    pub total_words_mastered: i32,
    pub total_conversations: i32,
    pub total_sessions: i32,
    pub joined_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = user_profiles)]
pub struct NewUserProfile {
    pub user_id: i64,
}

#[derive(AsChangeset, Default)]
#[diesel(table_name = user_profiles)]
pub struct UpdateUserProfile {
    pub total_xp: Option<i32>,
    pub current_rank_id: Option<i64>,
    pub current_streak_days: Option<i32>,
    pub longest_streak_days: Option<i32>,
    pub last_activity_date: Option<NaiveDate>,
    pub total_study_minutes: Option<i32>,
    pub total_words_mastered: Option<i32>,
    pub total_conversations: Option<i32>,
    pub total_sessions: Option<i32>,
    pub updated_at: Option<DateTime<Utc>>,
}

// ============================================================================
// User Achievements (user-specific)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = user_achievements)]
pub struct UserAchievementRecord {
    pub id: i64,
    pub user_id: i64,
    pub achievement_id: i64,
    pub progress: i32,
    pub is_completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub notified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = user_achievements)]
pub struct NewUserAchievement {
    pub user_id: i64,
    pub achievement_id: i64,
    pub progress: i32,
    pub is_completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(AsChangeset)]
#[diesel(table_name = user_achievements)]
pub struct UpdateUserAchievement {
    pub progress: Option<i32>,
    pub is_completed: Option<bool>,
    pub completed_at: Option<DateTime<Utc>>,
    pub notified_at: Option<DateTime<Utc>>,
}

// ============================================================================
// User XP History (user-specific)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = user_xp_history)]
pub struct UserXpHistory {
    pub id: i64,
    pub user_id: i64,
    pub xp_amount: i32,
    pub source_type: String,
    pub source_id: Option<i64>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = user_xp_history)]
pub struct NewUserXpHistory {
    pub user_id: i64,
    pub xp_amount: i32,
    pub source_type: String,
    pub source_id: Option<i64>,
    pub description: Option<String>,
}

// ============================================================================
// API Response Types
// ============================================================================

/// Achievement with user progress combined
#[derive(Serialize, Debug)]
pub struct AchievementWithProgress {
    pub id: i64,
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub icon: Option<String>,
    pub category: String,
    pub rarity: String,
    pub xp_reward: i32,
    pub requirement_value: i32,
    pub progress: i32,
    pub is_completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

/// User profile summary for dropdown/header
#[derive(Serialize, Debug)]
pub struct UserProfileSummary {
    pub total_xp: i32,
    pub current_streak_days: i32,
    pub rank: Option<RankInfo>,
    pub next_rank: Option<RankInfo>,
    pub xp_to_next_rank: i32,
    pub recent_achievements: Vec<AchievementBadge>,
    pub total_achievements: i32,
    pub completed_achievements: i32,
}

/// Simplified rank info for API responses
#[derive(Serialize, Debug, Clone)]
pub struct RankInfo {
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub level: i32,
    pub min_xp: i32,
}

impl From<RankDefinition> for RankInfo {
    fn from(r: RankDefinition) -> Self {
        RankInfo {
            code: r.code,
            name_en: r.name_en,
            name_zh: r.name_zh,
            icon: r.icon,
            color: r.color,
            level: r.level,
            min_xp: r.min_xp,
        }
    }
}

/// Simplified achievement badge for display
#[derive(Serialize, Debug)]
pub struct AchievementBadge {
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub icon: Option<String>,
    pub rarity: String,
    pub completed_at: Option<DateTime<Utc>>,
}
