use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_sentences(req: &mut Request) -> JsonResult<Vec<WordSentence>> {
    let word_id = super::get_path_id(req, "id")?;
    let examples: Vec<WordSentence> = with_conn(move |conn| {
        dict_word_sentences::table
            .filter(dict_word_sentences::word_id.eq(word_id))
            .order(dict_word_sentences::priority_order.asc())
            .load::<WordSentence>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch examples"))?;
    json_ok(examples)
}

#[derive(Deserialize)]
pub struct CreateSentenceRequest {
    pub language: String,
    pub definition_id: Option<i64>,
    pub sentence: String,
    pub source: Option<String>,
    pub author: Option<String>,
    pub priority_order: Option<i32>,
    pub difficulty: Option<i32>,
    pub is_common: Option<bool>,
}

#[handler]
pub async fn create_sentence(req: &mut Request) -> JsonResult<Sentence> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateSentenceRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input.sentence.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("sentence is required")
            .into());
    }
    let created = with_conn(move |conn| {
        let created = diesel::insert_into(dict_sentences::table)
            .values(&NewSentence {
                language: input.language.trim().to_string(),
                sentence: input.sentence,
                source: input.source,
                author: input.author,
                difficulty: input.difficulty,
                is_common: input.is_common,
            })
            .get_result::<Sentence>(conn)?;
        diesel::insert_into(dict_word_sentences::table)
            .values(&NewWordSentence {
                word_id,
                definition_id: input.definition_id,
                sentence_id: created.id,
                priority_order: input.priority_order,
            })
            .get_result::<WordSentence>(conn)?;
        Ok(created)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create example"))?;
    json_ok(created)
}
