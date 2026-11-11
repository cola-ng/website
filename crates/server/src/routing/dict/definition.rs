use horme::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_definitions(req: &mut Request) -> JsonResult<Vec<Definition>> {
    let word_id = super::get_path_id(req, "id")?;
    let definitions: Vec<Definition> = with_conn!(move |conn| {
        dict_definitions::table
            .filter(dict_definitions::word_id.eq(word_id))
            .order(dict_definitions::definition_order.asc())
            .load::<Definition>(conn)
            .await
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch definitions"))?;
    json_ok(definitions)
}

#[derive(Deserialize)]
pub struct CreateDefinitionRequest {
    pub language: String,
    pub definition: String,
    pub part_of_speech: Option<String>,
    pub definition_order: Option<i32>,
    pub register: Option<String>,
    pub region: Option<String>,
    pub context: Option<String>,
    pub usage_notes: Option<String>,
    pub is_primary: Option<bool>,
}

#[handler]
pub async fn create_definition(req: &mut Request) -> JsonResult<Definition> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateDefinitionRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input.definition.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("definition is required")
            .into());
    }

    let created: Definition = with_conn!(move |conn| {
        diesel::insert_into(dict_definitions::table)
            .values(&NewDefinition {
                word_id,
                language: input.language,
                definition: input.definition.trim().to_string(),
                part_of_speech: input.part_of_speech,
                definition_order: input.definition_order,
                register: input.register,
                region: input.region,
                context: input.context,
                usage_notes: input.usage_notes,
                is_primary: input.is_primary,
            })
            .get_result::<Definition>(conn)
            .await
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create definition"))?;
    json_ok(created)
}

#[derive(Deserialize)]
pub struct UpdateDefinitionRequest {
    pub language: Option<String>,
    pub definition: Option<String>,
    pub part_of_speech: Option<String>,
    pub definition_order: Option<i32>,
    pub register: Option<String>,
    pub region: Option<String>,
    pub context: Option<String>,
    pub usage_notes: Option<String>,
    pub is_primary: Option<bool>,
}

#[handler]
pub async fn update_definition(req: &mut Request) -> JsonResult<Definition> {
    let definition_id = super::get_path_id(req, "definition_id")?;
    let input: UpdateDefinitionRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    // Validate definition if provided
    if let Some(ref def) = input.definition {
        if def.trim().is_empty() {
            return Err(StatusError::bad_request()
                .brief("definition cannot be empty")
                .into());
        }
    }

    let changes = UpdateDefinition {
        language: input.language,
        definition: input.definition.map(|d| d.trim().to_string()),
        part_of_speech: input.part_of_speech,
        definition_order: input.definition_order,
        register: input.register,
        region: input.region,
        context: input.context,
        usage_notes: input.usage_notes,
        is_primary: input.is_primary,
    };

    let updated: Definition = with_conn!(move |conn| {
        diesel::update(dict_definitions::table.filter(dict_definitions::id.eq(definition_id)))
            .set(changes)
            .get_result::<Definition>(conn)
            .await
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update definition"))?;

    json_ok(updated)
}

#[handler]
pub async fn delete_definition(req: &mut Request) -> JsonResult<serde_json::Value> {
    let definition_id = super::get_path_id(req, "definition_id")?;
    let _ = with_conn!(move |conn| {
        diesel::delete(dict_definitions::table.filter(dict_definitions::id.eq(definition_id)))
            .execute(conn)
            .await
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete definition"))?;
    json_ok(serde_json::json!({ "deleted": true }))
}
