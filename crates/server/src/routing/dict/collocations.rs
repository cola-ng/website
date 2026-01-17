use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_collocations(req: &mut Request) -> JsonResult<Vec<WordCollocation>> {
    let word_id = super::get_path_id(req, "id")?;
    let collocations: Vec<WordCollocation> = with_conn(move |conn| {
        dict_word_collocations::table
            .filter(dict_word_collocations::word_id.eq(word_id))
            .load::<WordCollocation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch collocations"))?;
    json_ok(collocations)
}

#[derive(Deserialize)]
pub struct CreateCollocationRequest {
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

#[handler]
pub async fn create_collocation(req: &mut Request) -> JsonResult<WordCollocation> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateCollocationRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.phrase.trim().is_empty() || input.phrase_en.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("phrase and phrase_en are required")
            .into());
    }

    let created: WordCollocation = with_conn(move |conn| {
        diesel::insert_into(dict_word_collocations::table)
            .values(&NewWordCollocation {
                word_id,
                collocation_type: input.collocation_type,
                collocated_word_id: input.collocated_word_id,
                phrase: input.phrase.trim().to_string(),
                phrase_en: input.phrase_en.trim().to_string(),
                phrase_zh: input.phrase_zh,
                frequency_score: input.frequency_score,
                register: input.register,
                example_en: input.example_en,
                example_zh: input.example_zh,
                is_common: input.is_common,
            })
            .get_result::<WordCollocation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create collocation"))?;

    json_ok(created)
}

