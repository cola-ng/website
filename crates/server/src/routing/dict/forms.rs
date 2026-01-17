use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_forms(req: &mut Request) -> JsonResult<Vec<WordForm>> {
    let word_id = super::get_path_id(req, "id")?;
    let forms: Vec<WordForm> = with_conn(move |conn| {
        dict_word_forms::table
            .filter(dict_word_forms::word_id.eq(word_id))
            .load::<WordForm>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch forms"))?;
    json_ok(forms)
}

#[derive(Deserialize)]
pub struct CreateFormRequest {
    pub form_type: Option<String>,
    pub form: String,
    pub is_irregular: Option<bool>,
    pub notes: Option<String>,
}

#[handler]
pub async fn create_form(req: &mut Request) -> JsonResult<WordForm> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateFormRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input.form.trim().is_empty() {
        return Err(StatusError::bad_request().brief("form is required").into());
    }
    let created: WordForm = with_conn(move |conn| {
        diesel::insert_into(dict_word_forms::table)
            .values(&NewWordForm {
                word_id,
                form_type: input.form_type,
                form: input.form.trim().to_string(),
                is_irregular: input.is_irregular,
                notes: input.notes,
            })
            .get_result::<WordForm>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create form"))?;
    json_ok(created)
}

