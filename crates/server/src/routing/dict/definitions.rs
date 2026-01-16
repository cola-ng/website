use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_definitions(req: &mut Request) -> JsonResult<Vec<DictWordDefinition>> {
    let word_id = super::get_path_id(req, "id")?;
    let definitions: Vec<DictWordDefinition> = with_conn(move |conn| {
        dict_word_definitions::table
            .filter(dict_word_definitions::word_id.eq(word_id))
            .order(dict_word_definitions::definition_order.asc())
            .load::<DictWordDefinition>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch definitions"))?;
    json_ok(definitions)
}

#[derive(Deserialize)]
pub struct CreateDefinitionRequest {
    pub definition_en: String,
    pub definition_zh: Option<String>,
    pub part_of_speech: Option<String>,
    pub definition_order: Option<i32>,
    pub register: Option<String>,
    pub region: Option<String>,
    pub context: Option<String>,
    pub usage_notes: Option<String>,
    pub is_primary: Option<bool>,
}

#[handler]
pub async fn create_definition(req: &mut Request) -> JsonResult<DictWordDefinition> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateDefinitionRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input.definition_en.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("definition_en is required")
            .into());
    }

    let created: DictWordDefinition = with_conn(move |conn| {
        diesel::insert_into(dict_word_definitions::table)
            .values(&NewDictWordDefinition {
                word_id,
                definition_en: input.definition_en.trim().to_string(),
                definition_zh: input.definition_zh,
                part_of_speech: input.part_of_speech,
                definition_order: input.definition_order,
                register: input.register,
                region: input.region,
                context: input.context,
                usage_notes: input.usage_notes,
                is_primary: input.is_primary,
            })
            .get_result::<DictWordDefinition>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create definition"))?;
    json_ok(created)
}

