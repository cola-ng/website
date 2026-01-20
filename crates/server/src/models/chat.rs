use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;

// ============================================================================
// Chat Session
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = learn_chat_sessions)]
pub struct ChatSession {
    pub id: i64,
    pub user_id: i64,
    pub title: Option<String>,
    pub system_prompt: Option<String>,
    pub is_active: bool,
    pub message_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = learn_chat_sessions)]
pub struct NewChatSession {
    pub user_id: i64,
    pub title: Option<String>,
    pub system_prompt: Option<String>,
}

#[derive(AsChangeset)]
#[diesel(table_name = learn_chat_sessions)]
pub struct UpdateChatSession {
    pub title: Option<String>,
    pub system_prompt: Option<String>,
    pub is_active: Option<bool>,
    pub message_count: Option<i32>,
    pub updated_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Chat Message
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = learn_chat_messages)]
pub struct ChatMessage {
    pub id: i64,
    pub session_id: i64,
    pub user_id: i64,
    pub role: String,
    pub content: String,
    pub audio_base64: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = learn_chat_messages)]
pub struct NewChatMessage {
    pub session_id: i64,
    pub user_id: i64,
    pub role: String,
    pub content: String,
    pub audio_base64: Option<String>,
}

// ============================================================================
// Helper types for API
// ============================================================================

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

impl From<ChatMessage> for HistoryMessage {
    fn from(msg: ChatMessage) -> Self {
        HistoryMessage {
            role: msg.role,
            content: msg.content,
        }
    }
}
