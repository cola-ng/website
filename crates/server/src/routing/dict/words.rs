use diesel::prelude::*;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[derive(Serialize)]
pub struct WordListResponse {
    pub words: Vec<Word>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[handler]
pub async fn list_words(req: &mut Request) -> JsonResult<WordListResponse> {
    let search_term = req
        .query::<String>("search")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let min_diff = req.query::<i16>("min_difficulty");
    let max_diff = req.query::<i16>("max_difficulty");
    let page = req.query::<i64>("page").unwrap_or(1).max(1);
    let per_page = req.query::<i64>("per_page").unwrap_or(20).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let search_clone = search_term.clone();
    let min_diff_clone = min_diff;
    let max_diff_clone = max_diff;

    let (words, total) = with_conn(move |conn| {
        // Build base query for filtering
        let mut base_query = dict_words::table.into_boxed();

        if let Some(ref term) = search_term {
            base_query = base_query.filter(dict_words::word.ilike(format!("%{}%", term)));
        }

        if let Some(min_d) = min_diff {
            base_query = base_query.filter(dict_words::difficulty.ge(min_d));
        }

        if let Some(max_d) = max_diff {
            base_query = base_query.filter(dict_words::difficulty.le(max_d));
        }

        // Get total count with same filters
        let mut count_query = dict_words::table.into_boxed();

        if let Some(ref term) = search_clone {
            count_query = count_query.filter(dict_words::word.ilike(format!("%{}%", term)));
        }

        if let Some(min_d) = min_diff_clone {
            count_query = count_query.filter(dict_words::difficulty.ge(min_d));
        }

        if let Some(max_d) = max_diff_clone {
            count_query = count_query.filter(dict_words::difficulty.le(max_d));
        }

        let total: i64 = count_query.count().get_result(conn)?;

        // Get paginated results
        let words: Vec<Word> = base_query
            .order(dict_words::word.asc())
            .limit(per_page)
            .offset(offset)
            .load::<Word>(conn)?;

        Ok::<_, diesel::result::Error>((words, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch words"))?;

    json_ok(WordListResponse {
        words,
        total,
        page,
        per_page,
    })
}

#[derive(Deserialize)]
pub struct CreateWordRequest {
    pub word: String,
    pub word_type: Option<String>,
    pub language: Option<String>,
    pub frequency: Option<i16>,
    pub difficulty: Option<i16>,
    pub syllable_count: Option<i16>,
    pub is_lemma: Option<bool>,
    pub word_count: Option<i32>,
    pub is_active: Option<bool>,
    pub memory: Option<String>,
    pub notes: Option<String>,
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
                frequency: input.frequency,
                difficulty: input.difficulty,
                syllable_count: input.syllable_count,
                is_lemma: input.is_lemma,
                word_count: input.word_count,
                is_active: input.is_active,
                memory: input.memory,
                notes: input.notes,
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
