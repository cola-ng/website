use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_relation(req: &mut Request) -> JsonResult<Vec<Relation>> {
    let word_id = super::get_path_id(req, "id")?;
    let relation: Vec<Relation> = with_conn(move |conn| {
        dict_word_relations::table
            .filter(dict_word_relations::word_id.eq(word_id))
            .load::<Relation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch relation"))?;
    json_ok(relation)
}

#[derive(Deserialize)]
pub struct CreateRelationRequest {
    pub origin_language: Option<String>,
    pub origin_word: Option<String>,
    pub origin_meaning: Option<String>,
    pub relation_en: Option<String>,
    pub relation_zh: Option<String>,
    pub first_attested_year: Option<i32>,
    pub historical_forms: Option<serde_json::Value>,
    pub cognate_words: Option<serde_json::Value>,
}

#[handler]
pub async fn create_relation(req: &mut Request) -> JsonResult<Relation> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateRelationRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let created: Relation = with_conn(move |conn| {
        diesel::insert_into(dict_word_relations::table)
            .values(&NewRelation {
                word_id,
                origin_language: input.origin_language,
                origin_word: input.origin_word,
                origin_meaning: input.origin_meaning,
                relation_en: input.relation_en,
                relation_zh: input.relation_zh,
                first_attested_year: input.first_attested_year,
                historical_forms: input.historical_forms,
                cognate_words: input.cognate_words,
            })
            .get_result::<Relation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create relation"))?;

    json_ok(created)
}

