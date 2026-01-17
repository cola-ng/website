use diesel::prelude::*;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[derive(Serialize, Debug)]
pub struct WordDictionaryView {
    pub link: WordDictionary,
    pub word: WordRef,
    pub dictionary: Dictionary,
}

#[handler]
pub async fn list_word_dictionaries(req: &mut Request) -> JsonResult<Vec<WordDictionaryView>> {
    let word_id = super::get_path_id(req, "word_id")?;

    let results: Vec<(WordDictionary, Word, Dictionary)> = with_conn(move |conn| {
        dict_word_dictionaries::table
            .inner_join(dict_words::table.on(dict_word_dictionaries::word_id.eq(dict_words::id)))
            .inner_join(
                dict_dictionaries::table
                    .on(dict_word_dictionaries::dictionary_id.eq(dict_dictionaries::id)),
            )
            .filter(dict_word_dictionaries::word_id.eq(word_id))
            .order(dict_dictionaries::priority_order.asc())
            .load(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch word dictionaries"))?;

    let views = results
        .into_iter()
        .map(|(link, word, dict)| WordDictionaryView {
            link,
            word: WordRef {
                id: word.id,
                word: word.word,
            },
            dictionary: dict,
        })
        .collect();

    json_ok(views)
}

#[handler]
pub async fn list_dictionary_words(req: &mut Request) -> JsonResult<Vec<WordDictionaryView>> {
    let dictionary_id = super::get_path_id(req, "dictionary_id")?;

    let results: Vec<(WordDictionary, Word, Dictionary)> = with_conn(move |conn| {
        dict_word_dictionaries::table
            .inner_join(dict_words::table.on(dict_word_dictionaries::word_id.eq(dict_words::id)))
            .inner_join(
                dict_dictionaries::table
                    .on(dict_word_dictionaries::dictionary_id.eq(dict_dictionaries::id)),
            )
            .filter(dict_word_dictionaries::dictionary_id.eq(dictionary_id))
            .order(dict_words::word.asc())
            .load(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch dictionary words"))?;

    let views = results
        .into_iter()
        .map(|(link, word, dict)| WordDictionaryView {
            link,
            word: WordRef {
                id: word.id,
                word: word.word,
            },
            dictionary: dict,
        })
        .collect();

    json_ok(views)
}

#[derive(Deserialize)]
pub struct CreateWordDictionaryRequest {
    pub word_id: i64,
    pub dictionary_id: i64,
}

#[handler]
pub async fn create_word_dictionary(req: &mut Request) -> JsonResult<WordDictionaryView> {
    let input: CreateWordDictionaryRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let created: WordDictionary = with_conn(move |conn| {
        diesel::insert_into(dict_word_dictionaries::table)
            .values(&NewWordDictionary {
                word_id: input.word_id,
                dictionary_id: input.dictionary_id,
            })
            .get_result::<WordDictionary>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create word dictionary association"))?;

    let results: Vec<(WordDictionary, Word, Dictionary)> =
        with_conn(move |conn| {
            dict_word_dictionaries::table
                .inner_join(dict_words::table.on(dict_word_dictionaries::word_id.eq(dict_words::id)))
                .inner_join(
                    dict_dictionaries::table
                        .on(dict_word_dictionaries::dictionary_id.eq(dict_dictionaries::id)),
                )
                .filter(dict_word_dictionaries::id.eq(created.id))
                .load(conn)
        })
        .await
        .map_err(|_| StatusError::internal_server_error().brief("failed to fetch word dictionary association"))?;

    let (link, word, dict) = results
        .into_iter()
        .next()
        .ok_or_else(|| StatusError::internal_server_error().brief("failed to fetch created association"))?;

    json_ok(WordDictionaryView {
        link,
        word: WordRef {
            id: word.id,
            word: word.word,
        },
        dictionary: dict,
    })
}

#[handler]
pub async fn delete_word_dictionary(req: &mut Request) -> JsonResult<serde_json::Value> {
    let word_id = super::get_path_id(req, "word_id")?;
    let dictionary_id = super::get_path_id(req, "dictionary_id")?;

    with_conn(move |conn| {
        diesel::delete(
            dict_word_dictionaries::table
                .filter(dict_word_dictionaries::word_id.eq(word_id))
                .filter(dict_word_dictionaries::dictionary_id.eq(dictionary_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete word dictionary association"))?;

    json_ok(serde_json::json!({ "message": "word dictionary association deleted" }))
}

#[handler]
pub async fn delete_word_dictionary_by_id(req: &mut Request) -> JsonResult<serde_json::Value> {
    let id = super::get_path_id(req, "id")?;

    with_conn(move |conn| {
        diesel::delete(dict_word_dictionaries::table.find(id)).execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete word dictionary association"))?;

    json_ok(serde_json::json!({ "message": "word dictionary association deleted" }))
}
