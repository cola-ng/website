use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_words)]
pub struct DictWord {
    pub id: i64,
    pub word: String,
    pub phonetic_us: Option<String>,
    pub phonetic_uk: Option<String>,
    pub audio_us: Option<String>,
    pub audio_uk: Option<String>,
    pub difficulty_level: Option<i32>,
    pub word_frequency_rank: Option<i32>,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_words)]
pub struct NewDictWord {
    pub word: String,
    pub phonetic_us: Option<String>,
    pub phonetic_uk: Option<String>,
    pub audio_us: Option<String>,
    pub audio_uk: Option<String>,
    pub difficulty_level: Option<i32>,
    pub word_frequency_rank: Option<i32>,
    pub is_primary: bool,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_definitions)]
pub struct DictWordDefinition {
    pub id: i64,
    pub word_id: i64,
    pub definition_en: String,
    pub definition_zh: Option<String>,
    pub part_of_speech: Option<String>,
    pub definition_order: i32,
    pub register: Option<String>,
    pub region: Option<String>,
    pub context: Option<String>,
    pub usage_notes: Option<String>,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_definitions)]
pub struct NewDictWordDefinition {
    pub word_id: i64,
    pub definition_en: String,
    pub definition_zh: Option<String>,
    pub part_of_speech: Option<String>,
    pub definition_order: i32,
    pub register: Option<String>,
    pub region: Option<String>,
    pub context: Option<String>,
    pub usage_notes: Option<String>,
    pub is_primary: bool,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_examples)]
pub struct DictWordExample {
    pub id: i64,
    pub word_id: i64,
    pub example_en: String,
    pub example_zh: Option<String>,
    pub example_order: i32,
    pub source: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_examples)]
pub struct NewDictWordExample {
    pub word_id: i64,
    pub example_en: String,
    pub example_zh: Option<String>,
    pub example_order: i32,
    pub source: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_synonyms)]
pub struct DictSynonym {
    pub id: i64,
    pub word_id: i64,
    pub synonym_word: String,
    pub similarity_score: Option<f32>,
    pub context: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_synonyms)]
pub struct NewDictSynonym {
    pub word_id: i64,
    pub synonym_word: String,
    pub similarity_score: Option<f32>,
    pub context: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_antonyms)]
pub struct DictAntonym {
    pub id: i64,
    pub word_id: i64,
    pub antonym_word: String,
    pub context: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_antonyms)]
pub struct NewDictAntonym {
    pub word_id: i64,
    pub antonym_word: String,
    pub context: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_forms)]
pub struct DictWordForm {
    pub id: i64,
    pub word_id: i64,
    pub form_type: String,
    pub form_value: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_forms)]
pub struct NewDictWordForm {
    pub word_id: i64,
    pub form_type: String,
    pub form_value: String,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_collocations)]
pub struct DictCollocation {
    pub id: i64,
    pub word_id: i64,
    pub collocation_text: String,
    pub collocation_zh: Option<String>,
    pub collocation_type: Option<String>,
    pub frequency: Option<i32>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_collocations)]
pub struct NewDictCollocation {
    pub word_id: i64,
    pub collocation_text: String,
    pub collocation_zh: Option<String>,
    pub collocation_type: Option<String>,
    pub frequency: Option<i32>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_families)]
pub struct DictWordFamily {
    pub id: i64,
    pub word_id: i64,
    pub family_type: String,
    pub related_word: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_families)]
pub struct NewDictWordFamily {
    pub word_id: i64,
    pub family_type: String,
    pub related_word: String,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_phrases)]
pub struct DictPhrase {
    pub id: i64,
    pub word_id: i64,
    pub phrase_en: String,
    pub phrase_zh: Option<String>,
    pub phrase_type: Option<String>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_phrases)]
pub struct NewDictPhrase {
    pub word_id: i64,
    pub phrase_en: String,
    pub phrase_zh: Option<String>,
    pub phrase_type: Option<String>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_idioms)]
pub struct DictIdiom {
    pub id: i64,
    pub word_id: i64,
    pub idiom_en: String,
    pub idiom_zh: Option<String>,
    pub meaning_en: Option<String>,
    pub meaning_zh: Option<String>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_idioms)]
pub struct NewDictIdiom {
    pub word_id: i64,
    pub idiom_en: String,
    pub idiom_zh: Option<String>,
    pub meaning_en: Option<String>,
    pub meaning_zh: Option<String>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_tags)]
pub struct DictWordTag {
    pub id: i64,
    pub word_id: i64,
    pub tag_name: String,
    pub tag_category: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_tags)]
pub struct NewDictWordTag {
    pub word_id: i64,
    pub tag_name: String,
    pub tag_category: Option<String>,
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
#[diesel(table_name = dict_etymology)]
pub struct DictEtymology {
    pub id: i64,
    pub word_id: i64,
    pub origin: Option<String>,
    pub origin_language: Option<String>,
    pub historical_forms: Option<String>,
    pub first_known_use: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_etymology)]
pub struct NewDictEtymology {
    pub word_id: i64,
    pub origin: Option<String>,
    pub origin_language: Option<String>,
    pub historical_forms: Option<String>,
    pub first_known_use: Option<String>,
    pub notes: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_usage_notes)]
pub struct DictUsageNote {
    pub id: i64,
    pub word_id: i64,
    pub note_type: String,
    pub note_content_en: Option<String>,
    pub note_content_zh: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_usage_notes)]
pub struct NewDictUsageNote {
    pub word_id: i64,
    pub note_type: String,
    pub note_content_en: Option<String>,
    pub note_content_zh: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_images)]
pub struct DictWordImage {
    pub id: i64,
    pub word_id: i64,
    pub image_url: String,
    pub image_description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_images)]
pub struct NewDictWordImage {
    pub word_id: i64,
    pub image_url: String,
    pub image_description: Option<String>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_word_videos)]
pub struct DictWordVideo {
    pub id: i64,
    pub word_id: i64,
    pub video_url: String,
    pub video_title: Option<String>,
    pub video_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_word_videos)]
pub struct NewDictWordVideo {
    pub word_id: i64,
    pub video_url: String,
    pub video_title: Option<String>,
    pub video_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<i32>,
}

#[derive(Queryable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = dict_common_errors)]
pub struct DictCommonError {
    pub id: i64,
    pub word_id: i64,
    pub incorrect_form: String,
    pub explanation_en: Option<String>,
    pub explanation_zh: Option<String>,
    pub error_type: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = dict_common_errors)]
pub struct NewDictCommonError {
    pub word_id: i64,
    pub incorrect_form: String,
    pub explanation_en: Option<String>,
    pub explanation_zh: Option<String>,
    pub error_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WordQueryResponse {
    pub word: DictWord,
    pub definitions: Vec<DictWordDefinition>,
    pub examples: Vec<DictWordExample>,
    pub synonyms: Vec<DictSynonym>,
    pub antonyms: Vec<DictAntonym>,
    pub forms: Vec<DictWordForm>,
    pub collocations: Vec<DictCollocation>,
    pub word_families: Vec<DictWordFamily>,
    pub phrases: Vec<DictPhrase>,
    pub idioms: Vec<DictIdiom>,
    pub tags: Vec<DictWordTag>,
    pub related_topics: Vec<DictRelatedTopic>,
    pub etymology: Vec<DictEtymology>,
    pub usage_notes: Vec<DictUsageNote>,
    pub images: Vec<DictWordImage>,
    pub videos: Vec<DictWordVideo>,
    pub common_errors: Vec<DictCommonError>,
}
