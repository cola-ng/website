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
        dict_relations::table
            .filter(dict_relations::word_id.eq(word_id))
            .load::<Relation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch relation"))?;
    json_ok(relation)
}

#[derive(Deserialize)]
pub struct CreateRelationRequest {
    pub relation_type: Option<String>,
    pub related_word_id: i64,
    pub semantic_field: Option<String>,
    pub relation_strength: Option<f32>,
}
#[handler]
pub async fn create_relation(req: &mut Request) -> JsonResult<Relation> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateRelationRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let created: Relation = with_conn(move |conn| {
        diesel::insert_into(dict_relations::table)
            .values(&NewRelation {
                word_id,
                relation_type: input.relation_type,
                related_word_id: input.related_word_id,
                semantic_field: input.semantic_field,
                relation_strength: input.relation_strength,
            })
            .get_result::<Relation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create relation"))?;

    json_ok(created)
}
