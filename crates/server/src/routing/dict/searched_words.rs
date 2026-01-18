use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[derive(Deserialize)]
pub struct CreateSearchedWordRequest {
    pub word: String,
}

#[handler]
pub async fn create_searched_word(req: &mut Request) -> JsonResult<SearchedWord> {
    let input: CreateSearchedWordRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let word = input.word.trim().to_string();
    if word.is_empty() {
        return Err(StatusError::bad_request().brief("word is required").into());
    }

    let user_id = 1; // TODO: Get from authenticated user

    let created: SearchedWord = with_conn(move |conn| {
        diesel::insert_into(dict_searched_words::table)
            .values(&NewSearchedWord {
                user_id,
                word,
            })
            .get_result::<SearchedWord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create searched word"))?;

    json_ok(created)
}

#[handler]
pub async fn list_searched_words(req: &mut Request) -> JsonResult<Vec<SearchedWord>> {
    let limit = req.query::<i64>("limit").unwrap_or(10).clamp(1, 100);
    let user_id = 1; // TODO: Get from authenticated user

    let words: Vec<SearchedWord> = with_conn(move |conn| {
        dict_searched_words::table
            .filter(dict_searched_words::user_id.eq(user_id))
            .order(dict_searched_words::searched_at.desc())
            .limit(limit)
            .load::<SearchedWord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch searched words"))?;

    json_ok(words)
}

#[handler]
pub async fn clear_searched_words(req: &mut Request) -> JsonResult<serde_json::Value> {
    let user_id = 1; // TODO: Get from authenticated user

    let _ = with_conn(move |conn| {
        diesel::delete(dict_searched_words::table.filter(dict_searched_words::user_id.eq(user_id))).execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to clear searched words"))?;

    json_ok(serde_json::json!({ "cleared": true }))
}
