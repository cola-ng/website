use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_words)]
pub struct Word {
    pub id: i64,
    pub word: String,
    pub word_lower: String,
    pub word_type: Option<String>,
    pub language: Option<String>,
    pub frequency_score: Option<i32>,
    pub difficulty_level: Option<i32>,
    pub syllable_count: Option<i32>,
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
    pub frequency_score: Option<i32>,
    pub difficulty_level: Option<i32>,
    pub syllable_count: Option<i32>,
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
    pub frequency_score: Option<i32>,
    pub difficulty_level: Option<i32>,
    pub syllable_count: Option<i32>,
    pub is_lemma: Option<bool>,
    pub word_count: Option<i32>,
    pub is_active: Option<bool>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_definitions)]
pub struct WordDefinition {
    pub id: i64,
    pub word_id: i64,
    pub definition_en: String,
    pub definition_zh: Option<String>,
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
#[diesel(table_name = dict_word_definitions)]
pub struct NewWordDefinition {
    pub word_id: i64,
    pub definition_en: String,
    pub definition_zh: Option<String>,
    pub part_of_speech: Option<String>,
    pub definition_order: Option<i32>,
    pub register: Option<String>,
    pub region: Option<String>,
    pub context: Option<String>,
    pub usage_notes: Option<String>,
    pub is_primary: Option<bool>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_examples)]
pub struct WordExample {
    pub id: i64,
    pub word_id: i64,
    pub definition_id: Option<i64>,
    pub sentence_en: String,
    pub sentence_zh: Option<String>,
    pub source: Option<String>,
    pub author: Option<String>,
    pub example_order: Option<i32>,
    pub difficulty_level: Option<i32>,
    pub is_common: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_examples)]
pub struct NewWordExample {
    pub word_id: i64,
    pub definition_id: Option<i64>,
    pub sentence_en: String,
    pub sentence_zh: Option<String>,
    pub source: Option<String>,
    pub author: Option<String>,
    pub example_order: Option<i32>,
    pub difficulty_level: Option<i32>,
    pub is_common: Option<bool>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_synonyms)]
pub struct WordSynonym {
    pub id: i64,
    pub word_id: i64,
    pub synonym_word_id: i64,
    pub similarity_score: Option<f32>,
    pub context: Option<String>,
    pub nuance_notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_synonyms)]
pub struct NewWordSynonym {
    pub word_id: i64,
    pub synonym_word_id: i64,
    pub similarity_score: Option<f32>,
    pub context: Option<String>,
    pub nuance_notes: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_antonyms)]
pub struct WordAntonym {
    pub id: i64,
    pub word_id: i64,
    pub antonym_word_id: i64,
    pub antonym_type: Option<String>,
    pub context: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_antonyms)]
pub struct NewWordAntonym {
    pub word_id: i64,
    pub antonym_word_id: i64,
    pub antonym_type: Option<String>,
    pub context: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_forms)]
pub struct WordForm {
    pub id: i64,
    pub word_id: i64,
    pub form_type: Option<String>,
    pub form: String,
    pub is_irregular: Option<bool>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_forms)]
pub struct NewWordForm {
    pub word_id: i64,
    pub form_type: Option<String>,
    pub form: String,
    pub is_irregular: Option<bool>,
    pub notes: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_collocations)]
pub struct WordCollocation {
    pub id: i64,
    pub word_id: i64,
    pub collocation_type: Option<String>,
    pub collocated_word_id: Option<i64>,
    pub phrase: String,
    pub phrase_en: String,
    pub phrase_zh: Option<String>,
    pub frequency_score: Option<i32>,
    pub register: Option<String>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
    pub is_common: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_collocations)]
pub struct NewWordCollocation {
    pub word_id: i64,
    pub collocation_type: Option<String>,
    pub collocated_word_id: Option<i64>,
    pub phrase: String,
    pub phrase_en: String,
    pub phrase_zh: Option<String>,
    pub frequency_score: Option<i32>,
    pub register: Option<String>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
    pub is_common: Option<bool>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_family)]
pub struct WordFamilyLink {
    pub id: i64,
    pub root_word_id: i64,
    pub related_word_id: i64,
    pub relationship_type: Option<String>,
    pub morpheme: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_family)]
pub struct NewWordFamilyLink {
    pub root_word_id: i64,
    pub related_word_id: i64,
    pub relationship_type: Option<String>,
    pub morpheme: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_phrases)]
pub struct Phrase {
    pub id: i64,
    pub phrase: String,
    pub phrase_lower: String,
    pub phrase_type: Option<String>,
    pub meaning_en: String,
    pub meaning_zh: Option<String>,
    pub origin: Option<String>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
    pub difficulty_level: Option<i32>,
    pub frequency_score: Option<i32>,
    pub is_active: Option<bool>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_phrases)]
pub struct NewPhrase {
    pub phrase: String,
    pub phrase_lower: String,
    pub phrase_type: Option<String>,
    pub meaning_en: String,
    pub meaning_zh: Option<String>,
    pub origin: Option<String>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
    pub difficulty_level: Option<i32>,
    pub frequency_score: Option<i32>,
    pub is_active: Option<bool>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_phrase_words)]
pub struct PhraseWord {
    pub id: i64,
    pub phrase_id: i64,
    pub word_id: i64,
    pub word_position: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_phrase_words)]
pub struct NewPhraseWord {
    pub phrase_id: i64,
    pub word_id: i64,
    pub word_position: i32,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_categories)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_categories)]
pub struct NewCategory {
    pub name: String,
    pub parent_id: Option<i64>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_categories)]
pub struct WordCategory {
    pub id: i64,
    pub word_id: i64,
    pub category_id: i64,
    pub confidence: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_categories)]
pub struct NewWordCategory {
    pub word_id: i64,
    pub category_id: i64,
    pub confidence: Option<i16>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_etymology)]
pub struct WordEtymology {
    pub id: i64,
    pub word_id: i64,
    pub origin_language: Option<String>,
    pub origin_word: Option<String>,
    pub origin_meaning: Option<String>,
    pub etymology_en: Option<String>,
    pub etymology_zh: Option<String>,
    pub first_attested_year: Option<i32>,
    pub historical_forms: Option<serde_json::Value>,
    pub cognate_words: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_etymology)]
pub struct NewWordEtymology {
    pub word_id: i64,
    pub origin_language: Option<String>,
    pub origin_word: Option<String>,
    pub origin_meaning: Option<String>,
    pub etymology_en: Option<String>,
    pub etymology_zh: Option<String>,
    pub first_attested_year: Option<i32>,
    pub historical_forms: Option<serde_json::Value>,
    pub cognate_words: Option<serde_json::Value>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_usage_notes)]
pub struct WordUsageNote {
    pub id: i64,
    pub word_id: i64,
    pub note_type: Option<String>,
    pub note_en: String,
    pub note_zh: Option<String>,
    pub examples_en: Option<serde_json::Value>,
    pub examples_zh: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_usage_notes)]
pub struct NewWordUsageNote {
    pub word_id: i64,
    pub note_type: Option<String>,
    pub note_en: String,
    pub note_zh: Option<String>,
    pub examples_en: Option<serde_json::Value>,
    pub examples_zh: Option<serde_json::Value>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_images)]
pub struct WordImage {
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
#[diesel(table_name = dict_word_images)]
pub struct NewWordImage {
    pub word_id: i64,
    pub image_url: Option<String>,
    pub image_path: Option<String>,
    pub image_type: Option<String>,
    pub alt_text_en: Option<String>,
    pub alt_text_zh: Option<String>,
    pub is_primary: Option<bool>,
    pub created_by: Option<i64>,
}

#[derive(Serialize, Debug, Clone)]
pub struct WordRef {
    pub id: i64,
    pub word: String,
}

#[derive(Serialize, Debug)]
pub struct WordSynonymView {
    pub link: WordSynonym,
    pub synonym: WordRef,
}

#[derive(Serialize, Debug)]
pub struct WordAntonymView {
    pub link: WordAntonym,
    pub antonym: WordRef,
}

#[derive(Serialize, Debug)]
pub struct WordFamilyView {
    pub link: WordFamilyLink,
    pub related: WordRef,
}

#[derive(Serialize, Debug)]
pub struct WordQueryResponse {
    pub word: Word,
    pub definitions: Vec<WordDefinition>,
    pub examples: Vec<WordExample>,
    pub synonyms: Vec<WordSynonymView>,
    pub antonyms: Vec<WordAntonymView>,
    pub forms: Vec<WordForm>,
    pub collocations: Vec<WordCollocation>,
    pub word_family: Vec<WordFamilyView>,
    pub phrases: Vec<Phrase>,
    pub idioms: Vec<Phrase>,
    pub categories: Vec<WordCategory>,
    pub related_topics: Vec<DictRelatedTopic>,
    pub etymology: Vec<WordEtymology>,
    pub usage_notes: Vec<WordUsageNote>,
    pub images: Vec<WordImage>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_dictionaries)]
pub struct DictDictionary {
    pub id: i64,
    pub name: String,
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
pub struct NewDictDictionary {
    pub name: String,
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
pub struct UpdateDictDictionary {
    pub name: Option<String>,
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

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_dictionaries)]
pub struct WordDictionary {
    pub id: i64,
    pub word_id: i64,
    pub dictionary_id: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_dictionaries)]
pub struct NewWordDictionary {
    pub word_id: i64,
    pub dictionary_id: i64,
}
