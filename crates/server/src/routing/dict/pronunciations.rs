use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_pronunciations(req: &mut Request) -> JsonResult<Vec<WordPronunciation>> {
    let word_id = super::get_path_id(req, "id")?;
    let pronunciations: Vec<WordPronunciation> = with_conn(move |conn| {
        dict_word_pronunciations::table
            .filter(dict_word_pronunciations::word_id.eq(word_id))
            .order(dict_word_pronunciations::is_primary.desc())
            .load::<WordPronunciation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch pronunciations"))?;
    json_ok(pronunciations)
}

#[derive(Deserialize)]
pub struct CreatePronunciationRequest {
    pub definition_id: Option<i64>,
    pub ipa: String,
    pub audio_url: Option<String>,
    pub audio_path: Option<String>,
    pub dialect: Option<String>,
    pub gender: Option<String>,
    pub is_primary: Option<bool>,
}

#[handler]
pub async fn create_pronunciation(req: &mut Request) -> JsonResult<WordPronunciation> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreatePronunciationRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input.ipa.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("ipa is required")
            .into());
    }
    let created: WordPronunciation = with_conn(move |conn| {
        diesel::insert_into(dict_word_pronunciations::table)
            .values(&NewWordPronunciation {
                word_id,
                definition_id: input.definition_id,
                ipa: input.ipa.trim().to_string(),
                audio_url: input.audio_url,
                audio_path: input.audio_path,
                dialect: input.dialect,
                gender: input.gender,
                is_primary: input.is_primary,
            })
            .get_result::<WordPronunciation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create pronunciation"))?;
    json_ok(created)
}

#[handler]
pub async fn delete_pronunciation(req: &mut Request) -> JsonResult<()> {
    let pronunciation_id = super::get_path_id(req, "pronunciation_id")?;
    with_conn(move |conn| {
        diesel::delete(
            dict_word_pronunciations::table.filter(
                dict_word_pronunciations::id.eq(pronunciation_id)
            )
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete pronunciation"))?;
    json_ok(())
}
