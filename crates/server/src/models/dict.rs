use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_words)]
pub struct DictWord {
    pub id: i64,
    pub word: String,
    pub word_lower: String,
    pub word_type: Option<String>,
    pub frequency_score: Option<i32>,
    pub difficulty_level: Option<i32>,
    pub syllable_count: Option<i32>,
    pub is_lemma: Option<bool>,
    pub lemma_id: Option<i64>,
    pub audio_url: Option<String>,
    pub audio_path: Option<String>,
    pub phonetic_transcription: Option<String>,
    pub ipa_text: Option<String>,
    pub word_count: Option<i32>,
    pub is_active: Option<bool>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_words)]
pub struct NewDictWord {
    pub word: String,
    pub word_lower: String,
    pub word_type: Option<String>,
    pub frequency_score: Option<i32>,
    pub difficulty_level: Option<i32>,
    pub syllable_count: Option<i32>,
    pub is_lemma: Option<bool>,
    pub lemma_id: Option<i64>,
    pub word_count: Option<i32>,
    pub is_active: Option<bool>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = dict_words)]
pub struct UpdateDictWord {
    pub word: Option<String>,
    pub word_lower: Option<String>,
    pub word_type: Option<String>,
    pub frequency_score: Option<i32>,
    pub difficulty_level: Option<i32>,
    pub syllable_count: Option<i32>,
    pub is_lemma: Option<bool>,
    pub lemma_id: Option<i64>,
    pub word_count: Option<i32>,
    pub is_active: Option<bool>,
    pub updated_by: Option<i64>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_definitions)]
pub struct DictWordDefinition {
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
pub struct NewDictWordDefinition {
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
pub struct DictWordExample {
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
pub struct NewDictWordExample {
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
pub struct DictWordSynonym {
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
pub struct NewDictWordSynonym {
    pub word_id: i64,
    pub synonym_word_id: i64,
    pub similarity_score: Option<f32>,
    pub context: Option<String>,
    pub nuance_notes: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_antonyms)]
pub struct DictWordAntonym {
    pub id: i64,
    pub word_id: i64,
    pub antonym_word_id: i64,
    pub antonym_type: Option<String>,
    pub context: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_antonyms)]
pub struct NewDictWordAntonym {
    pub word_id: i64,
    pub antonym_word_id: i64,
    pub antonym_type: Option<String>,
    pub context: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_forms)]
pub struct DictWordForm {
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
pub struct NewDictWordForm {
    pub word_id: i64,
    pub form_type: Option<String>,
    pub form: String,
    pub is_irregular: Option<bool>,
    pub notes: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_collocations)]
pub struct DictWordCollocation {
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
pub struct NewDictWordCollocation {
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
pub struct DictWordFamilyLink {
    pub id: i64,
    pub root_word_id: i64,
    pub related_word_id: i64,
    pub relationship_type: Option<String>,
    pub morpheme: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_family)]
pub struct NewDictWordFamilyLink {
    pub root_word_id: i64,
    pub related_word_id: i64,
    pub relationship_type: Option<String>,
    pub morpheme: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_phrases)]
pub struct DictPhrase {
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
pub struct NewDictPhrase {
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
pub struct DictPhraseWord {
    pub id: i64,
    pub phrase_id: i64,
    pub word_id: i64,
    pub word_position: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_phrase_words)]
pub struct NewDictPhraseWord {
    pub phrase_id: i64,
    pub word_id: i64,
    pub word_position: i32,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_categories)]
pub struct DictWordCategory {
    pub id: i64,
    pub word_id: i64,
    pub category_type: Option<String>,
    pub category_name: String,
    pub category_value: String,
    pub confidence_score: Option<f32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_categories)]
pub struct NewDictWordCategory {
    pub word_id: i64,
    pub category_type: Option<String>,
    pub category_name: String,
    pub category_value: String,
    pub confidence_score: Option<f32>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_related_topics)]
pub struct DictRelatedTopic {
    pub id: i64,
    pub word_id: i64,
    pub topic_name: String,
    pub topic_category: Option<String>,
    pub relevance_score: Option<f32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_related_topics)]
pub struct NewDictRelatedTopic {
    pub word_id: i64,
    pub topic_name: String,
    pub topic_category: Option<String>,
    pub relevance_score: Option<f32>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_etymology)]
pub struct DictWordEtymology {
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
pub struct NewDictWordEtymology {
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
pub struct DictWordUsageNote {
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
pub struct NewDictWordUsageNote {
    pub word_id: i64,
    pub note_type: Option<String>,
    pub note_en: String,
    pub note_zh: Option<String>,
    pub examples_en: Option<serde_json::Value>,
    pub examples_zh: Option<serde_json::Value>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_images)]
pub struct DictWordImage {
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
pub struct NewDictWordImage {
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
pub struct DictWordSynonymView {
    pub link: DictWordSynonym,
    pub synonym: WordRef,
}

#[derive(Serialize, Debug)]
pub struct DictWordAntonymView {
    pub link: DictWordAntonym,
    pub antonym: WordRef,
}

#[derive(Serialize, Debug)]
pub struct DictWordFamilyView {
    pub link: DictWordFamilyLink,
    pub related: WordRef,
}

#[derive(Serialize, Debug)]
pub struct WordQueryResponse {
    pub word: DictWord,
    pub definitions: Vec<DictWordDefinition>,
    pub examples: Vec<DictWordExample>,
    pub synonyms: Vec<DictWordSynonymView>,
    pub antonyms: Vec<DictWordAntonymView>,
    pub forms: Vec<DictWordForm>,
    pub collocations: Vec<DictWordCollocation>,
    pub word_family: Vec<DictWordFamilyView>,
    pub phrases: Vec<DictPhrase>,
    pub idioms: Vec<DictPhrase>,
    pub categories: Vec<DictWordCategory>,
    pub related_topics: Vec<DictRelatedTopic>,
    pub etymology: Vec<DictWordEtymology>,
    pub usage_notes: Vec<DictWordUsageNote>,
    pub images: Vec<DictWordImage>,
}
