use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_words)]
pub struct Word {
    pub id: i64,
    pub word: String,
    pub word_lower: String,
    pub word_type: Option<String>,
    pub language: Option<String>,
    pub frequency: Option<i16>,
    pub difficulty: Option<i16>,
    pub syllable_count: Option<i16>,
    pub is_lemma: Option<bool>,
    pub word_count: Option<i32>,
    pub is_active: Option<bool>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_words)]
pub struct NewWord {
    pub word: String,
    pub word_lower: String,
    pub word_type: Option<String>,
    pub language: Option<String>,
    pub frequency: Option<i16>,
    pub difficulty: Option<i16>,
    pub syllable_count: Option<i16>,
    pub is_lemma: Option<bool>,
    pub word_count: Option<i32>,
    pub is_active: Option<bool>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = dict_words)]
pub struct UpdateWord {
    pub word: Option<String>,
    pub word_lower: Option<String>,
    pub word_type: Option<String>,
    pub language: Option<String>,
    pub frequency: Option<i16>,
    pub difficulty: Option<i16>,
    pub syllable_count: Option<i16>,
    pub is_lemma: Option<bool>,
    pub word_count: Option<i32>,
    pub is_active: Option<bool>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
}

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_definitions)]
pub struct Definition {
    pub id: i64,
    pub word_id: i64,
    pub language: String,
    pub definition: String,
    pub part_of_speech: Option<String>,
    pub definition_order: Option<i32>,
    pub register: Option<String>,
    pub region: Option<String>,
    pub context: Option<String>,
    pub usage_notes: Option<String>,
    pub is_primary: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_definitions)]
pub struct NewDefinition {
    pub word_id: i64,
    pub language: String,
    pub definition: String,
    pub part_of_speech: Option<String>,
    pub definition_order: Option<i32>,
    pub register: Option<String>,
    pub region: Option<String>,
    pub context: Option<String>,
    pub usage_notes: Option<String>,
    pub is_primary: Option<bool>,
}
#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_sentences)]
pub struct Sentence {
    pub id: i64,
    pub language: String,
    pub sentence: String,
    pub source: Option<String>,
    pub author: Option<String>,
    pub difficulty: Option<i32>,
    pub is_common: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_sentences)]
pub struct NewSentence {
    pub language: String,
    pub sentence: String,
    pub source: Option<String>,
    pub author: Option<String>,
    pub difficulty: Option<i32>,
    pub is_common: Option<bool>,
}

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_word_sentences)]
pub struct WordSentence {
    pub id: i64,
    pub word_id: i64,
    pub definition_id: Option<i64>,
    pub sentence_id: i64,
    pub priority_order: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_sentences)]
pub struct NewWordSentence {
    pub word_id: i64,
    pub definition_id: Option<i64>,
    pub sentence_id: i64,
    pub priority_order: Option<i32>,
}

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_pronunciations)]
pub struct Pronunciation {
    pub id: i64,
    pub word_id: i64,
    pub definition_id: Option<i64>,
    pub ipa: String,
    pub audio_url: Option<String>,
    pub audio_path: Option<String>,
    pub dialect: Option<String>,
    pub gender: Option<String>,
    pub is_primary: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_pronunciations)]
pub struct NewPronunciation {
    pub word_id: i64,
    pub definition_id: Option<i64>,
    pub ipa: String,
    pub audio_url: Option<String>,
    pub audio_path: Option<String>,
    pub dialect: Option<String>,
    pub gender: Option<String>,
    pub is_primary: Option<bool>,
}

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_forms)]
pub struct Form {
    pub id: i64,
    pub word_id: i64,
    pub form_type: Option<String>,
    pub form: String,
    pub is_irregular: Option<bool>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_forms)]
pub struct NewForm {
    pub word_id: i64,
    pub form_type: Option<String>,
    pub form: String,
    pub is_irregular: Option<bool>,
    pub notes: Option<String>,
}

// Note: dict_categories table does not exist - category features disabled

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_word_categories)]
pub struct WordCategory {
    pub id: i64,
    pub word_id: i64,
    pub category_id: i64,
    pub confidence: i16,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_categories)]
pub struct NewWordCategory {
    pub word_id: i64,
    pub category_id: i64,
    pub confidence: Option<i16>,
}

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_images)]
pub struct Image {
    pub id: i64,
    pub word_id: i64,
    pub image_url: Option<String>,
    pub image_path: Option<String>,
    pub image_type: Option<String>,
    pub alt_text_en: Option<String>,
    pub alt_text_zh: Option<String>,
    pub is_primary: Option<bool>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_images)]
pub struct NewImage {
    pub word_id: i64,
    pub image_url: Option<String>,
    pub image_path: Option<String>,
    pub image_type: Option<String>,
    pub alt_text_en: Option<String>,
    pub alt_text_zh: Option<String>,
    pub is_primary: Option<bool>,
    pub created_by: Option<i64>,
}

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_relations)]
pub struct Relation {
    pub id: i64,
    pub word_id: i64,
    pub relation_type: Option<String>,
    pub related_word_id: i64,
    pub semantic_field: Option<String>,
    pub relation_strength: Option<i16>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_relations)]
pub struct NewRelation {
    pub word_id: i64,
    pub relation_type: Option<String>,
    pub related_word_id: i64,
    pub semantic_field: Option<String>,
    pub relation_strength: Option<i16>,
}

#[derive(Serialize, ToSchema, Debug)]
pub struct WordQueryResponse {
    pub word: Word,
    pub definitions: Vec<Definition>,
    pub sentences: Vec<Sentence>,
    pub pronunciations: Vec<Pronunciation>,
    pub dictionaries: Vec<Dictionary>,
    pub relations: Vec<Relation>,
    pub etymologies: Vec<Etymology>,
    pub forms: Vec<Form>,
    pub images: Vec<Image>,
}

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_dictionaries)]
pub struct Dictionary {
    pub id: i64,
    pub name_en: String,
    pub name_zh: String,
    pub short_en: String,
    pub short_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub license_type: Option<String>,
    pub license_url: Option<String>,
    pub source_url: Option<String>,
    pub total_entries: Option<i64>,
    pub is_active: Option<bool>,
    pub is_official: Option<bool>,
    pub priority_order: Option<i32>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_dictionaries)]
pub struct NewDictionary {
    pub name_en: String,
    pub name_zh: String,
    pub short_en: String,
    pub short_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub license_type: Option<String>,
    pub license_url: Option<String>,
    pub source_url: Option<String>,
    pub total_entries: Option<i64>,
    pub is_active: Option<bool>,
    pub is_official: Option<bool>,
    pub priority_order: Option<i32>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = dict_dictionaries)]
pub struct UpdateDictionary {
    pub name_en: Option<String>,
    pub name_zh: Option<String>,
    pub short_en: Option<String>,
    pub short_zh: Option<String>,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub license_type: Option<String>,
    pub license_url: Option<String>,
    pub source_url: Option<String>,
    pub total_entries: Option<i64>,
    pub is_active: Option<bool>,
    pub is_official: Option<bool>,
    pub priority_order: Option<i32>,
    pub updated_by: Option<i64>,
}

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_word_dictionaries)]
pub struct WordDictionary {
    pub id: i64,
    pub word_id: i64,
    pub dictionary_id: i64,
    pub definition_id: Option<i64>,
    pub priority_order: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_dictionaries)]
pub struct NewWorldDictionary {
    pub word_id: i64,
    pub dictionary_id: i64,
}

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_etymologies)]
pub struct Etymology {
    pub id: i64,
    pub origin_language: Option<String>,
    pub origin_word: Option<String>,
    pub origin_meaning: Option<String>,
    pub language: String,
    pub etymology: String,
    pub first_attested_year: Option<i32>,
    pub historical_forms: Option<serde_json::Value>,
    pub cognate_words: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_word_etymologies)]
pub struct WordEtymology {
    pub id: i64,
    pub word_id: i64,
    pub etymology_id: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_etymologies)]
pub struct NewEtymology {
    pub origin_language: Option<String>,
    pub origin_word: Option<String>,
    pub origin_meaning: Option<String>,
    pub language: String,
    pub etymology: String,
    pub first_attested_year: Option<i32>,
    pub historical_forms: Option<serde_json::Value>,
    pub cognate_words: Option<serde_json::Value>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_etymologies)]
pub struct NewWordEtymology {
    pub word_id: i64,
    pub etymology_id: i64,
}

// ============================================================================
// Searched Words
// ============================================================================

#[derive(Queryable, Identifiable, Serialize, ToSchema, Debug, Clone)]
#[diesel(table_name = dict_searched_words)]
pub struct SearchedWord {
    pub id: i64,
    pub user_id: i64,
    pub word_id: Option<i64>,
    pub word: String,
    pub searched_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_searched_words)]
pub struct NewSearchedWord {
    pub user_id: i64,
    pub word: String,
}
