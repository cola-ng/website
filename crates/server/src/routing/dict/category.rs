use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_categories(req: &mut Request) -> JsonResult<Vec<WordCategory>> {
    let word_id = super::get_path_id(req, "id")?;
    let categories: Vec<WordCategory> = with_conn(move |conn| {
        dict_word_categories::table
            .filter(dict_word_categories::word_id.eq(word_id))
            .load::<WordCategory>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch categories"))?;
    json_ok(categories)
}

#[derive(Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub parent_id: Option<i64>,
}
#[handler]
pub async fn create_category(req: &mut Request) -> JsonResult<Category> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateCategoryRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input.name.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("category_name and category_value are required")
            .into());
    }
    let created = with_conn(move |conn| {
        diesel::insert_into(dict_categories::table)
            .values(&NewCategory {
                name: input.name.trim().to_string(),
                parent_id: input.parent_id,
            })
            .get_result::<Category>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create category"))?;
    json_ok(created)
}
