use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_usage_notes(req: &mut Request) -> JsonResult<Vec<DictWordUsageNote>> {
    let word_id = super::get_path_id(req, "id")?;
    let usage_notes: Vec<DictWordUsageNote> = with_conn(move |conn| {
        dict_word_usage_notes::table
            .filter(dict_word_usage_notes::word_id.eq(word_id))
            .load::<DictWordUsageNote>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch usage notes"))?;
    json_ok(usage_notes)
}

#[derive(Deserialize)]
pub struct CreateUsageNoteRequest {
    pub note_type: Option<String>,
    pub note_en: String,
    pub note_zh: Option<String>,
    pub examples_en: Option<serde_json::Value>,
    pub examples_zh: Option<serde_json::Value>,
}

#[handler]
pub async fn create_usage_note(req: &mut Request) -> JsonResult<DictWordUsageNote> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateUsageNoteRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input.note_en.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("note_en is required")
            .into());
    }
    let created: DictWordUsageNote = with_conn(move |conn| {
        diesel::insert_into(dict_word_usage_notes::table)
            .values(&NewDictWordUsageNote {
                word_id,
                note_type: input.note_type,
                note_en: input.note_en.trim().to_string(),
                note_zh: input.note_zh,
                examples_en: input.examples_en,
                examples_zh: input.examples_zh,
            })
            .get_result::<DictWordUsageNote>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create usage note"))?;
    json_ok(created)
}
