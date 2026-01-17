use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_etymology(req: &mut Request) -> JsonResult<Vec<Etymology>> {
    let word_id = super::get_path_id(req, "id")?;
    let etymology: Vec<Etymology> = with_conn(move |conn| {
        dict_etymologies::table
            .filter(dict_etymologies::word_id.eq(word_id))
            .load::<Etymology>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch etymology"))?;
    json_ok(etymology)
}

#[derive(Deserialize)]
pub struct CreateEtymologyRequest {
    pub origin_language: Option<String>,
    pub origin_word: Option<String>,
    pub origin_meaning: Option<String>,
    pub etymology_en: Option<String>,
    pub etymology_zh: Option<String>,
    pub first_attested_year: Option<i32>,
    pub historical_forms: Option<serde_json::Value>,
    pub cognate_words: Option<serde_json::Value>,
}

#[handler]
pub async fn create_etymology(req: &mut Request) -> JsonResult<Etymology> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateEtymologyRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let created: Etymology = with_conn(move |conn| {
        diesel::insert_into(dict_etymologies::table)
            .values(&NewEtymology {
                word_id,
                origin_language: input.origin_language,
                origin_word: input.origin_word,
                origin_meaning: input.origin_meaning,
                etymology_en: input.etymology_en,
                etymology_zh: input.etymology_zh,
                first_attested_year: input.first_attested_year,
                historical_forms: input.historical_forms,
                cognate_words: input.cognate_words,
            })
            .get_result::<Etymology>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create etymology"))?;

    json_ok(created)
}
