use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;

// ============================================================================
// Chat (conversation sessions)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_chats)]
pub struct ChatSession {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub duration_ms: Option<i32>,
    pub pause_count: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = learn_chats)]
pub struct NewChatSession {
    pub user_id: i64,
    pub title: String,
    pub duration_ms: Option<i32>,
    pub pause_count: Option<i32>,
}

// ============================================================================
// Chat Turns (conversation messages)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = learn_chat_turns)]
pub struct ChatMessage {
    pub id: i64,
    pub user_id: i64,
    pub chat_id: String,
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

#[derive(Insertable)]
#[diesel(table_name = learn_chat_turns)]
pub struct NewChatMessage {
    pub user_id: i64,
    pub chat_id: String,
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

// ============================================================================
// Helper types for API
// ============================================================================

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct HistoryMessage {
    /// Message role (user, assistant)
    pub role: String,
    /// Message content in English
    pub content: String,
    /// Message content in Chinese
    pub content_zh: Option<String>,
}

impl From<ChatMessage> for HistoryMessage {
    fn from(msg: ChatMessage) -> Self {
        HistoryMessage {
            role: msg.speaker,
            content: msg.content_en,
            content_zh: Some(msg.content_zh),
        }
    }
}
