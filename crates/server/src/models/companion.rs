use chrono::{DateTime,Utc};
use diesel::prelude::*;
use serde::Serialize;
use crate::db::schema::learn_person_memories;
#[derive(Debug, Clone, Queryable, Identifiable, Serialize)]
#[diesel(table_name = learn_person_memories)]
pub struct PersonMemory {
    pub id: i64,
    pub user_id: i64,
    pub memory_type: String,
    pub subject: String,
    pub summary: String,
    pub confidence: f32,
    pub evidence_count: i32,
    pub positive_feedback: i32,
    pub negative_feedback: i32,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
