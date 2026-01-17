use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::schema::*;

// ============================================================================
// Shared content models (no user_id)
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = asset_scenes)]
pub struct Scene {
    pub id: i64,
    pub name_en: String,
    pub name_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub icon_emoji: Option<String>,
    pub difficulty: Option<String>,
    pub category: Option<String>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_scenes)]
pub struct NewScene {
    pub name_en: String,
    pub name_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub icon_emoji: Option<String>,
    pub difficulty: Option<String>,
    pub category: Option<String>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = asset_dialogues)]
#[diesel(belongs_to(Scene))]
pub struct Dialogue {
    pub id: i64,
    pub scene_id: i64,
    pub title_en: String,
    pub title_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub total_turns: Option<i32>,
    pub estimated_duration_seconds: Option<i32>,
    pub difficulty: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_dialogues)]
pub struct NewDialogue {
    pub scene_id: i64,
    pub title_en: String,
    pub title_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub total_turns: Option<i32>,
    pub estimated_duration_seconds: Option<i32>,
    pub difficulty: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = asset_dialogue_turns)]
pub struct DialogueTurn {
    pub id: i64,
    pub dialogue_id: i64,
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
#[diesel(table_name = asset_dialogue_turns)]
pub struct NewDialogueTurn {
    pub dialogue_id: i64,
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

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = asset_classic_sources)]
pub struct ClassicDialogueSource {
    pub id: i64,
    pub source_type: String,
    pub title: String,
    pub year: Option<i32>,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub thumbnail_url: Option<String>,
    pub imdb_id: Option<String>,
    pub difficulty: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_classic_sources)]
pub struct NewClassicDialogueSource {
    pub source_type: String,
    pub title: String,
    pub year: Option<i32>,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub thumbnail_url: Option<String>,
    pub imdb_id: Option<String>,
    pub difficulty: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = asset_classic_clips)]
#[diesel(belongs_to(ClassicDialogueSource, foreign_key = source_id))]
pub struct ClassicDialogueClip {
    pub id: i64,
    pub source_id: i64,
    pub clip_title_en: String,
    pub clip_title_zh: String,
    pub start_time_seconds: Option<i32>,
    pub end_time_seconds: Option<i32>,
    pub video_url: Option<String>,
    pub transcript_en: String,
    pub transcript_zh: String,
    pub key_vocabulary: Option<Value>,
    pub cultural_notes: Option<String>,
    pub grammar_points: Option<Value>,
    pub difficulty_vocab: Option<i32>,
    pub difficulty_speed: Option<i32>,
    pub difficulty_slang: Option<i32>,
    pub popularity_score: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_classic_clips)]
pub struct NewClassicDialogueClip {
    pub source_id: i64,
    pub clip_title_en: String,
    pub clip_title_zh: String,
    pub start_time_seconds: Option<i32>,
    pub end_time_seconds: Option<i32>,
    pub video_url: Option<String>,
    pub transcript_en: String,
    pub transcript_zh: String,
    pub key_vocabulary: Option<Value>,
    pub cultural_notes: Option<String>,
    pub grammar_points: Option<Value>,
    pub difficulty_vocab: Option<i32>,
    pub difficulty_speed: Option<i32>,
    pub difficulty_slang: Option<i32>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = asset_read_exercises)]
pub struct ReadingExercise {
    pub id: i64,
    pub title_en: String,
    pub title_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub difficulty: Option<String>,
    pub exercise_type: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_read_exercises)]
pub struct NewReadingExercise {
    pub title_en: String,
    pub title_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub difficulty: Option<String>,
    pub exercise_type: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = asset_read_sentences)]
#[diesel(belongs_to(ReadingExercise, foreign_key = exercise_id))]
pub struct ReadingSentence {
    pub id: i64,
    pub exercise_id: i64,
    pub sentence_order: i32,
    pub content_en: String,
    pub content_zh: String,
    pub phonetic_transcription: Option<String>,
    pub native_audio_path: Option<String>,
    pub focus_sounds: Option<Value>,
    pub common_mistakes: Option<Value>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_read_sentences)]
pub struct NewReadingSentence {
    pub exercise_id: i64,
    pub sentence_order: i32,
    pub content_en: String,
    pub content_zh: String,
    pub phonetic_transcription: Option<String>,
    pub native_audio_path: Option<String>,
    pub focus_sounds: Option<Value>,
    pub common_mistakes: Option<Value>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = asset_phrases)]
pub struct KeyPhrase {
    pub id: i64,
    pub phrase_en: String,
    pub phrase_zh: String,
    pub phonetic_transcription: Option<String>,
    pub usage_context: Option<String>,
    pub example_sentence_en: Option<String>,
    pub example_sentence_zh: Option<String>,
    pub category: Option<String>,
    pub formality_level: Option<String>,
    pub frequency: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = asset_phrases)]
pub struct NewKeyPhrase {
    pub phrase_en: String,
    pub phrase_zh: String,
    pub phonetic_transcription: Option<String>,
    pub usage_context: Option<String>,
    pub example_sentence_en: Option<String>,
    pub example_sentence_zh: Option<String>,
    pub category: Option<String>,
    pub formality_level: Option<String>,
}
