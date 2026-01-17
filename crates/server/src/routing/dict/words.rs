use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_words(req: &mut Request) -> JsonResult<Vec<Word>> {
    let search_term = req
        .query::<String>("search")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let min_diff = req.query::<i32>("min_difficulty");
    let max_diff = req.query::<i32>("max_difficulty");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);
    let offset = req.query::<i64>("offset").unwrap_or(0).max(0);

    let words: Vec<Word> = with_conn(move |conn| {
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

        query.load::<Word>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch words"))?;

    json_ok(words)
}

#[derive(Deserialize)]
pub struct CreateWordRequest {
    pub word: String,
    pub word_type: Option<String>,
    pub language: Option<String>,
    pub frequency_score: Option<i32>,
    pub difficulty_level: Option<i32>,
    pub syllable_count: Option<i32>,
    pub is_lemma: Option<bool>,
    pub word_count: Option<i32>,
    pub is_active: Option<bool>,
}

#[handler]
pub async fn create_word(req: &mut Request) -> JsonResult<Word> {
    let input: CreateWordRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let word = input.word.trim().to_string();
    if word.is_empty() {
        return Err(StatusError::bad_request().brief("word is required").into());
    }
    let word_lower = word.to_lowercase();

    let created_word: Word = with_conn(move |conn| {
        diesel::insert_into(dict_words::table)
            .values(&NewWord {
                word,
                word_lower,
                word_type: input.word_type,
                language: input.language,
                frequency_score: input.frequency_score,
                difficulty_level: input.difficulty_level,
                syllable_count: input.syllable_count,
                is_lemma: input.is_lemma,
                word_count: input.word_count,
                is_active: input.is_active,
                created_by: None,
                updated_by: None,
            })
            .get_result::<Word>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create word"))?;

    json_ok(created_word)
}

#[handler]
pub async fn update_word(req: &mut Request) -> JsonResult<Word> {
    let id = super::get_path_id(req, "id")?;
    let input: UpdateWord = req
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

    let updated: Word = with_conn(move |conn| {
        diesel::update(dict_words::table.filter(dict_words::id.eq(id)))
            .set(changes)
            .get_result::<Word>(conn)
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
