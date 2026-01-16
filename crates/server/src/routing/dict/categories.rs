use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_categories(req: &mut Request) -> JsonResult<Vec<DictWordCategory>> {
    let word_id = super::get_path_id(req, "id")?;
    let categories: Vec<DictWordCategory> = with_conn(move |conn| {
        dict_word_categories::table
            .filter(dict_word_categories::word_id.eq(word_id))
            .load::<DictWordCategory>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch categories"))?;
    json_ok(categories)
}

#[derive(Deserialize)]
pub struct CreateCategoryRequest {
    pub category_type: Option<String>,
    pub category_name: String,
    pub category_value: String,
    pub confidence_score: Option<f32>,
}

#[handler]
pub async fn create_category(req: &mut Request) -> JsonResult<DictWordCategory> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateCategoryRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input.category_name.trim().is_empty() || input.category_value.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("category_name and category_value are required")
            .into());
    }
    let created: DictWordCategory = with_conn(move |conn| {
        diesel::insert_into(dict_word_categories::table)
            .values(&NewDictWordCategory {
                word_id,
                category_type: input.category_type,
                category_name: input.category_name.trim().to_string(),
                category_value: input.category_value.trim().to_string(),
                confidence_score: input.confidence_score,
            })
            .get_result::<DictWordCategory>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create category"))?;
    json_ok(created)
}

