use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_words(req: &mut Request) -> JsonResult<Vec<DictWord>> {
    let search_term = req
        .query::<String>("search")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let min_diff = req.query::<i32>("min_difficulty");
    let max_diff = req.query::<i32>("max_difficulty");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);
    let offset = req.query::<i64>("offset").unwrap_or(0).max(0);

    let words: Vec<DictWord> = with_conn(move |conn| {
        let mut query = dict_words::table
            .order(dict_words::word.asc())
            .limit(limit)
            .offset(offset)
            .into_boxed();

        if let Some(term) = search_term {
            query = query.filter(dict_words::word.ilike(format!("%{}%", term)));
        }

        if let Some(min_d) = min_diff {
            query = query.filter(dict_words::difficulty_level.ge(min_d));
        }

        if let Some(max_d) = max_diff {
            query = query.filter(dict_words::difficulty_level.le(max_d));
        }

        query.load::<DictWord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch words"))?;

    json_ok(words)
}

#[derive(Deserialize)]
pub struct CreateWordRequest {
    pub word: String,
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
}

#[handler]
pub async fn create_word(req: &mut Request) -> JsonResult<DictWord> {
    let input: CreateWordRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let word = input.word.trim().to_string();
    if word.is_empty() {
        return Err(StatusError::bad_request().brief("word is required").into());
    }
    let word_lower = word.to_lowercase();

    let created_word: DictWord = with_conn(move |conn| {
        diesel::insert_into(dict_words::table)
            .values(&NewDictWord {
                word,
                word_lower,
                word_type: input.word_type,
                frequency_score: input.frequency_score,
                difficulty_level: input.difficulty_level,
                syllable_count: input.syllable_count,
                is_lemma: input.is_lemma,
                lemma_id: input.lemma_id,
                audio_url: input.audio_url,
                audio_path: input.audio_path,
                phonetic_transcription: input.phonetic_transcription,
                ipa_text: input.ipa_text,
                word_count: input.word_count,
                is_active: input.is_active,
                created_by: None,
                updated_by: None,
            })
            .get_result::<DictWord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create word"))?;

    json_ok(created_word)
}

#[handler]
pub async fn update_word(req: &mut Request) -> JsonResult<DictWord> {
    let id = super::get_path_id(req, "id")?;
    let input: UpdateDictWord = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let mut changes = input;
    if let Some(w) = changes.word.as_ref() {
        let trimmed = w.trim().to_string();
        if trimmed.is_empty() {
            return Err(StatusError::bad_request().brief("word is required").into());
        }
        changes.word = Some(trimmed.clone());
        changes.word_lower = Some(trimmed.to_lowercase());
    }

    let updated: DictWord = with_conn(move |conn| {
        diesel::update(dict_words::table.filter(dict_words::id.eq(id)))
            .set(changes)
            .get_result::<DictWord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update word"))?;

    json_ok(updated)
}

#[handler]
pub async fn delete_word(req: &mut Request) -> JsonResult<serde_json::Value> {
    let id = super::get_path_id(req, "id")?;
    let _ = with_conn(move |conn| {
        diesel::delete(dict_words::table.filter(dict_words::id.eq(id))).execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete word"))?;
    json_ok(serde_json::json!({ "deleted": true }))
}

#[handler]
pub async fn lookup_word(req: &mut Request) -> JsonResult<WordQueryResponse> {
    let word = req
        .param::<String>("word")
        .ok_or_else(|| StatusError::bad_request().brief("missing word parameter"))?;
    let word_lower_value = word.trim().to_lowercase();
    if word_lower_value.is_empty() {
        return Err(StatusError::bad_request()
            .brief("missing word parameter")
            .into());
    }

    let result: WordQueryResponse = with_conn(move |conn| {
        let word_record = dict_words::table
            .filter(dict_words::word_lower.eq(&word_lower_value))
            .first::<DictWord>(conn)
            .optional()?
            .ok_or_else(|| diesel::result::Error::NotFound)?;

        let word_id = word_record.id;

        let definitions = dict_word_definitions::table
            .filter(dict_word_definitions::word_id.eq(word_id))
            .order(dict_word_definitions::definition_order.asc())
            .load::<DictWordDefinition>(conn)?;

        let examples = dict_word_examples::table
            .filter(dict_word_examples::word_id.eq(word_id))
            .order(dict_word_examples::example_order.asc())
            .load::<DictWordExample>(conn)?;

        let synonym_rows: Vec<(DictWordSynonym, DictWord)> = dict_word_synonyms::table
            .inner_join(
                dict_words::table.on(dict_word_synonyms::synonym_word_id.eq(dict_words::id)),
            )
            .filter(dict_word_synonyms::word_id.eq(word_id))
            .load(conn)?;
        let synonyms = synonym_rows
            .into_iter()
            .map(|(link, w)| DictWordSynonymView {
                link,
                synonym: WordRef {
                    id: w.id,
                    word: w.word,
                },
            })
            .collect();

        let antonym_rows: Vec<(DictWordAntonym, DictWord)> = dict_word_antonyms::table
            .inner_join(
                dict_words::table.on(dict_word_antonyms::antonym_word_id.eq(dict_words::id)),
            )
            .filter(dict_word_antonyms::word_id.eq(word_id))
            .load(conn)?;
        let antonyms = antonym_rows
            .into_iter()
            .map(|(link, w)| DictWordAntonymView {
                link,
                antonym: WordRef {
                    id: w.id,
                    word: w.word,
                },
            })
            .collect();

        let forms = dict_word_forms::table
            .filter(dict_word_forms::word_id.eq(word_id))
            .load::<DictWordForm>(conn)?;

        let collocations = dict_word_collocations::table
            .filter(dict_word_collocations::word_id.eq(word_id))
            .load::<DictWordCollocation>(conn)?;

        let family_out: Vec<(DictWordFamilyLink, DictWord)> = dict_word_family::table
            .inner_join(dict_words::table.on(dict_word_family::related_word_id.eq(dict_words::id)))
            .filter(dict_word_family::root_word_id.eq(word_id))
            .load(conn)?;
        let family_in: Vec<(DictWordFamilyLink, DictWord)> = dict_word_family::table
            .inner_join(dict_words::table.on(dict_word_family::root_word_id.eq(dict_words::id)))
            .filter(dict_word_family::related_word_id.eq(word_id))
            .load(conn)?;

        let mut word_family: Vec<DictWordFamilyView> = Vec::new();
        word_family.extend(family_out.into_iter().map(|(link, w)| DictWordFamilyView {
            link,
            related: WordRef {
                id: w.id,
                word: w.word,
            },
        }));
        word_family.extend(family_in.into_iter().map(|(link, w)| DictWordFamilyView {
            link,
            related: WordRef {
                id: w.id,
                word: w.word,
            },
        }));

        let phrases: Vec<DictPhrase> = dict_phrases::table
            .inner_join(
                dict_phrase_words::table.on(dict_phrase_words::phrase_id.eq(dict_phrases::id)),
            )
            .filter(dict_phrase_words::word_id.eq(word_id))
            .filter(
                dict_phrases::phrase_type
                    .is_null()
                    .or(dict_phrases::phrase_type.ne(Some("idiom"))),
            )
            .select(dict_phrases::all_columns)
            .distinct()
            .load(conn)?;

        let idioms: Vec<DictPhrase> = dict_phrases::table
            .inner_join(
                dict_phrase_words::table.on(dict_phrase_words::phrase_id.eq(dict_phrases::id)),
            )
            .filter(dict_phrase_words::word_id.eq(word_id))
            .filter(dict_phrases::phrase_type.eq(Some("idiom")))
            .select(dict_phrases::all_columns)
            .distinct()
            .load(conn)?;

        let categories = dict_word_categories::table
            .filter(dict_word_categories::word_id.eq(word_id))
            .load::<DictWordCategory>(conn)?;

        let related_topics = dict_related_topics::table
            .filter(dict_related_topics::word_id.eq(word_id))
            .load::<DictRelatedTopic>(conn)?;

        let etymology = dict_word_etymology::table
            .filter(dict_word_etymology::word_id.eq(word_id))
            .load::<DictWordEtymology>(conn)?;

        let usage_notes = dict_word_usage_notes::table
            .filter(dict_word_usage_notes::word_id.eq(word_id))
            .load::<DictWordUsageNote>(conn)?;

        let images = dict_word_images::table
            .filter(dict_word_images::word_id.eq(word_id))
            .load::<DictWordImage>(conn)?;

        Ok(WordQueryResponse {
            word: word_record,
            definitions,
            examples,
            synonyms,
            antonyms,
            forms,
            collocations,
            word_family,
            phrases,
            idioms,
            categories,
            related_topics,
            etymology,
            usage_notes,
            images,
        })
    })
    .await
    .map_err(|_| StatusError::not_found().brief("word not found"))?;

    json_ok(result)
}
