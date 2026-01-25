use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::schema::*;

// ============================================================================
// Taxonomy models
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = taxon_domains)]
pub struct TaxonDomain {
    pub id: i64,
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = taxon_domains)]
pub struct NewTaxonDomain {
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
}

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = taxon_categories)]
pub struct TaxonCategory {
    pub id: i64,
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub domain_id: i64,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = taxon_categories)]
pub struct NewTaxonCategory {
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub domain_id: i64,
    pub parent_id: Option<i64>,
}

// ============================================================================
// Context models
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = asset_contexts)]
pub struct Context {
    pub id: i64,
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub icon_emoji: Option<String>,
    pub display_order: Option<i32>,
    pub difficulty: Option<i16>,
    pub user_id: Option<i64>,
    pub prompt: Option<String>,
    pub is_active: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_contexts)]
pub struct NewContext {
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub icon_emoji: Option<String>,
    pub display_order: Option<i32>,
    pub difficulty: Option<i16>,
    pub is_active: Option<bool>,
}

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = asset_context_categories)]
pub struct ContextCategory {
    pub id: i64,
    pub context_id: i64,
    pub category_id: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_context_categories)]
pub struct NewContextCategory {
    pub context_id: i64,
    pub category_id: i64,
}

// ============================================================================
// Stage models (learning stages)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = asset_stages)]
pub struct Stage {
    pub id: i64,
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub icon_emoji: Option<String>,
    pub display_order: Option<i32>,
    pub difficulty: Option<i16>,
    pub is_active: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_stages)]
pub struct NewStage {
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub icon_emoji: Option<String>,
    pub display_order: Option<i32>,
    pub difficulty: Option<i16>,
    pub is_active: Option<bool>,
}

// ============================================================================
// Script models (dialogues)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = asset_scripts)]
#[diesel(belongs_to(Stage))]
pub struct Script {
    pub id: i64,
    pub stage_id: i64,
    pub code: String,
    pub title_en: String,
    pub title_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub total_turns: Option<i32>,
    pub estimated_duration_seconds: Option<i32>,
    pub difficulty: Option<i16>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_scripts)]
pub struct NewScript {
    pub stage_id: i64,
    pub code: String,
    pub title_en: String,
    pub title_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub total_turns: Option<i32>,
    pub estimated_duration_seconds: Option<i32>,
    pub difficulty: Option<i16>,
}

// ============================================================================
// Script turn models (dialogue lines)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = asset_script_turns)]
#[diesel(belongs_to(Script))]
pub struct ScriptTurn {
    pub id: i64,
    pub script_id: i64,
    pub turn_number: i32,
    pub speaker_role: String,
    pub speaker_name: Option<String>,
    pub content_en: String,
    pub content_zh: String,
    pub audio_path: Option<String>,
    pub phonetic_transcription: Option<String>,
    pub asset_phrases: Option<Value>,
    pub notes: Option<String>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_script_turns)]
pub struct NewScriptTurn {
    pub script_id: i64,
    pub turn_number: i32,
    pub speaker_role: String,
    pub speaker_name: Option<String>,
    pub content_en: String,
    pub content_zh: String,
    pub audio_path: Option<String>,
    pub phonetic_transcription: Option<String>,
    pub asset_phrases: Option<Value>,
    pub notes: Option<String>,
}

// ============================================================================
// Reading subject models
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = asset_read_subjects)]
pub struct ReadSubject {
    pub id: i64,
    pub code: String,
    pub title_en: String,
    pub title_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub difficulty: Option<i16>,
    pub subject_type: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_read_subjects)]
pub struct NewReadSubject {
    pub code: String,
    pub title_en: String,
    pub title_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub difficulty: Option<i16>,
    pub subject_type: Option<String>,
}

// ============================================================================
// Reading sentence models
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = asset_read_sentences)]
#[diesel(belongs_to(ReadSubject, foreign_key = subject_id))]
pub struct ReadSentence {
    pub id: i64,
    pub subject_id: i64,
    pub sentence_order: i32,
    pub content_en: String,
    pub content_zh: String,
    pub phonetic_transcription: Option<String>,
    pub native_audio_path: Option<String>,
    pub difficulty: Option<i16>,
    pub focus_sounds: Option<Value>,
    pub common_mistakes: Option<Value>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_read_sentences)]
pub struct NewReadSentence {
    pub subject_id: i64,
    pub sentence_order: i32,
    pub content_en: String,
    pub content_zh: String,
    pub phonetic_transcription: Option<String>,
    pub native_audio_path: Option<String>,
    pub difficulty: Option<i16>,
    pub focus_sounds: Option<Value>,
    pub common_mistakes: Option<Value>,
}

// ============================================================================
// Word sentence junction model
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = asset_word_sentences)]
pub struct WordSentence {
    pub id: i64,
    pub word_id: i64,
    pub sentence_id: i64,
    pub sentence_order: i32,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_word_sentences)]
pub struct NewWordSentence {
    pub word_id: i64,
    pub sentence_id: i64,
    pub sentence_order: i32,
}
