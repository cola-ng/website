use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_dictionaries(req: &mut Request) -> JsonResult<Vec<DictDictionary>> {
    let is_active = req.query::<bool>("is_active");
    let is_official = req.query::<bool>("is_official");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);
    let offset = req.query::<i64>("offset").unwrap_or(0).max(0);

    let dictionaries: Vec<DictDictionary> = with_conn(move |conn| {
        let mut query = dict_dictionaries::table
            .order(dict_dictionaries::priority_order.asc())
            .then_order_by(dict_dictionaries::name.asc())
            .limit(limit)
            .offset(offset)
            .into_boxed();

        if let Some(active) = is_active {
            query = query.filter(dict_dictionaries::is_active.eq(active));
        }

        if let Some(official) = is_official {
            query = query.filter(dict_dictionaries::is_official.eq(official));
        }

        query.load::<DictDictionary>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch dictionaries"))?;

    json_ok(dictionaries)
}

#[derive(Deserialize)]
pub struct CreateDictionaryRequest {
    pub name: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub license_type: Option<String>,
    pub license_url: Option<String>,
    pub source_url: Option<String>,
    pub total_entries: Option<i64>,
    pub is_active: Option<bool>,
    pub is_official: Option<bool>,
    pub priority_order: Option<i32>,
}

#[handler]
pub async fn create_dictionary(req: &mut Request) -> JsonResult<DictDictionary> {
    let input: CreateDictionaryRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(StatusError::bad_request().brief("name is required").into());
    }

    let created: DictDictionary = with_conn(move |conn| {
        diesel::insert_into(dict_dictionaries::table)
            .values(&NewDictDictionary {
                name,
                description_en: input.description_en,
                description_zh: input.description_zh,
                version: input.version,
                publisher: input.publisher,
                license_type: input.license_type,
                license_url: input.license_url,
                source_url: input.source_url,
                total_entries: input.total_entries,
                is_active: input.is_active,
                is_official: input.is_official,
                priority_order: input.priority_order,
                created_by: None,
                updated_by: None,
            })
            .get_result::<DictDictionary>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create dictionary"))?;

    json_ok(created)
}

#[derive(Deserialize)]
pub struct UpdateDictionaryRequest {
    pub name: Option<String>,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub license_type: Option<String>,
    pub license_url: Option<String>,
    pub source_url: Option<String>,
    pub total_entries: Option<i64>,
    pub is_active: Option<bool>,
    pub is_official: Option<bool>,
    pub priority_order: Option<i32>,
}

#[handler]
pub async fn update_dictionary(req: &mut Request) -> JsonResult<DictDictionary> {
    let id = super::get_path_id(req, "id")?;
    let input: UpdateDictionaryRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let updated: DictDictionary = with_conn(move |conn| {
        diesel::update(dict_dictionaries::table.find(id))
            .set(&UpdateDictDictionary {
                name: input.name.map(|n| n.trim().to_string()),
                description_en: input.description_en,
                description_zh: input.description_zh,
                version: input.version,
                publisher: input.publisher,
                license_type: input.license_type,
                license_url: input.license_url,
                source_url: input.source_url,
                total_entries: input.total_entries,
                is_active: input.is_active,
                is_official: input.is_official,
                priority_order: input.priority_order,
                updated_by: None,
            })
            .get_result::<DictDictionary>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update dictionary"))?;

    json_ok(updated)
}

#[handler]
pub async fn get_dictionary(req: &mut Request) -> JsonResult<DictDictionary> {
    let id = super::get_path_id(req, "id")?;

    let dictionary: DictDictionary = with_conn(move |conn| {
        dict_dictionaries::table.find(id).first::<DictDictionary>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("dictionary not found"))?;

    json_ok(dictionary)
}

#[handler]
pub async fn delete_dictionary(req: &mut Request) -> JsonResult<serde_json::Value> {
    let id = super::get_path_id(req, "id")?;

    with_conn(move |conn| {
        diesel::delete(dict_dictionaries::table.find(id)).execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete dictionary"))?;

    json_ok(serde_json::json!({ "message": "dictionary deleted" }))
}
