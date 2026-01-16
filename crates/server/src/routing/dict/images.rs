use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_images(req: &mut Request) -> JsonResult<Vec<DictWordImage>> {
    let word_id = super::get_path_id(req, "id")?;
    let images: Vec<DictWordImage> = with_conn(move |conn| {
        dict_word_images::table
            .filter(dict_word_images::word_id.eq(word_id))
            .load::<DictWordImage>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch images"))?;
    json_ok(images)
}

#[derive(Deserialize)]
pub struct CreateImageRequest {
    pub image_url: Option<String>,
    pub image_path: Option<String>,
    pub image_type: Option<String>,
    pub alt_text_en: Option<String>,
    pub alt_text_zh: Option<String>,
    pub is_primary: Option<bool>,
}

#[handler]
pub async fn create_image(req: &mut Request) -> JsonResult<DictWordImage> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateImageRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input
        .image_url
        .as_ref()
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
        && input
            .image_path
            .as_ref()
            .map(|v| v.trim().is_empty())
            .unwrap_or(true)
    {
        return Err(StatusError::bad_request()
            .brief("image_url or image_path is required")
            .into());
    }

    let created: DictWordImage = with_conn(move |conn| {
        diesel::insert_into(dict_word_images::table)
            .values(&NewDictWordImage {
                word_id,
                image_url: input
                    .image_url
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty()),
                image_path: input
                    .image_path
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty()),
                image_type: input.image_type,
                alt_text_en: input.alt_text_en,
                alt_text_zh: input.alt_text_zh,
                is_primary: input.is_primary,
                created_by: None,
            })
            .get_result::<DictWordImage>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create image"))?;
    json_ok(created)
}
