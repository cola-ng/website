use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_examples(req: &mut Request) -> JsonResult<Vec<DictWordExample>> {
    let word_id = super::get_path_id(req, "id")?;
    let examples: Vec<DictWordExample> = with_conn(move |conn| {
        dict_word_examples::table
            .filter(dict_word_examples::word_id.eq(word_id))
            .order(dict_word_examples::example_order.asc())
            .load::<DictWordExample>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch examples"))?;
    json_ok(examples)
}

#[derive(Deserialize)]
pub struct CreateExampleRequest {
    pub definition_id: Option<i64>,
    pub sentence_en: String,
    pub sentence_zh: Option<String>,
    pub source: Option<String>,
    pub author: Option<String>,
    pub example_order: Option<i32>,
    pub difficulty_level: Option<i32>,
    pub is_common: Option<bool>,
}

#[handler]
pub async fn create_example(req: &mut Request) -> JsonResult<DictWordExample> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateExampleRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input.sentence_en.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("sentence_en is required")
            .into());
    }
    let created: DictWordExample = with_conn(move |conn| {
        diesel::insert_into(dict_word_examples::table)
            .values(&NewDictWordExample {
                word_id,
                definition_id: input.definition_id,
                sentence_en: input.sentence_en.trim().to_string(),
                sentence_zh: input.sentence_zh,
                source: input.source,
                author: input.author,
                example_order: input.example_order,
                difficulty_level: input.difficulty_level,
                is_common: input.is_common,
            })
            .get_result::<DictWordExample>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create example"))?;
    json_ok(created)
}
