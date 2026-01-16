use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_phrases(req: &mut Request) -> JsonResult<Vec<DictPhrase>> {
    let word_id = super::get_path_id(req, "id")?;
    let phrases: Vec<DictPhrase> = with_conn(move |conn| {
        dict_phrases::table
            .inner_join(dict_phrase_words::table.on(dict_phrase_words::phrase_id.eq(dict_phrases::id)))
            .filter(dict_phrase_words::word_id.eq(word_id))
            .filter(
                dict_phrases::phrase_type
                    .is_null()
                    .or(dict_phrases::phrase_type.ne(Some("idiom"))),
            )
            .select(dict_phrases::all_columns)
            .distinct()
            .load(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch phrases"))?;
    json_ok(phrases)
}

#[handler]
pub async fn list_idioms(req: &mut Request) -> JsonResult<Vec<DictPhrase>> {
    let word_id = super::get_path_id(req, "id")?;
    let idioms: Vec<DictPhrase> = with_conn(move |conn| {
        dict_phrases::table
            .inner_join(dict_phrase_words::table.on(dict_phrase_words::phrase_id.eq(dict_phrases::id)))
            .filter(dict_phrase_words::word_id.eq(word_id))
            .filter(dict_phrases::phrase_type.eq(Some("idiom")))
            .select(dict_phrases::all_columns)
            .distinct()
            .load(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch idioms"))?;
    json_ok(idioms)
}

#[derive(Deserialize)]
pub struct CreatePhraseRequest {
    pub phrase: String,
    pub phrase_type: Option<String>,
    pub meaning_en: String,
    pub meaning_zh: Option<String>,
    pub origin: Option<String>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
    pub difficulty_level: Option<i32>,
    pub frequency_score: Option<i32>,
    pub is_active: Option<bool>,
    pub word_position: Option<i32>,
}

async fn create_phrase_impl(
    word_id: i64,
    force_type: Option<&'static str>,
    input: CreatePhraseRequest,
) -> Result<DictPhrase, StatusError> {
    if input.phrase.trim().is_empty() || input.meaning_en.trim().is_empty() {
        return Err(StatusError::bad_request().brief("phrase and meaning_en are required"));
    }
    let phrase = input.phrase.trim().to_string();
    let phrase_lower = phrase.to_lowercase();
    let phrase_type = force_type
        .map(|v| v.to_string())
        .or(input.phrase_type.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()));
    let word_position = input.word_position.unwrap_or(1);

    with_conn(move |conn| {
        conn.transaction::<DictPhrase, diesel::result::Error, _>(|conn| {
            let created: DictPhrase = diesel::insert_into(dict_phrases::table)
                .values(&NewDictPhrase {
                    phrase,
                    phrase_lower,
                    phrase_type,
                    meaning_en: input.meaning_en.trim().to_string(),
                    meaning_zh: input.meaning_zh,
                    origin: input.origin,
                    example_en: input.example_en,
                    example_zh: input.example_zh,
                    difficulty_level: input.difficulty_level,
                    frequency_score: input.frequency_score,
                    is_active: input.is_active,
                    created_by: None,
                    updated_by: None,
                })
                .get_result(conn)?;

            let _link: DictPhraseWord = diesel::insert_into(dict_phrase_words::table)
                .values(&NewDictPhraseWord {
                    phrase_id: created.id,
                    word_id,
                    word_position,
                })
                .get_result(conn)?;

            Ok(created)
        })
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create phrase"))
}

#[handler]
pub async fn create_phrase(req: &mut Request) -> JsonResult<DictPhrase> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreatePhraseRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    let created = create_phrase_impl(word_id, None, input).await?;
    json_ok(created)
}

#[handler]
pub async fn create_idiom(req: &mut Request) -> JsonResult<DictPhrase> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreatePhraseRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    let created = create_phrase_impl(word_id, Some("idiom"), input).await?;
    json_ok(created)
}
