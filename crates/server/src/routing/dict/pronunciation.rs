use std::path::PathBuf;

use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::config::AppConfig;
use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{AppResult, JsonResult, json_ok};

#[handler]
pub async fn list_pronunciations(req: &mut Request) -> JsonResult<Vec<Pronunciation>> {
    let word_id = super::get_path_id(req, "id")?;
    let pronunciations: Vec<Pronunciation> = with_conn(move |conn| {
        dict_pronunciations::table
            .filter(dict_pronunciations::word_id.eq(word_id))
            .order(dict_pronunciations::is_primary.desc())
            .load::<Pronunciation>(conn)
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
pub async fn create_pronunciation(req: &mut Request) -> JsonResult<Pronunciation> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreatePronunciationRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input.ipa.trim().is_empty() {
        return Err(StatusError::bad_request().brief("ipa is required").into());
    }
    let created: Pronunciation = with_conn(move |conn| {
        diesel::insert_into(dict_pronunciations::table)
            .values(&NewPronunciation {
                word_id,
                definition_id: input.definition_id,
                ipa: input.ipa.trim().to_string(),
                audio_url: input.audio_url,
                audio_path: input.audio_path,
                dialect: input.dialect,
                gender: input.gender,
                is_primary: input.is_primary,
            })
            .get_result::<Pronunciation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create pronunciation"))?;
    json_ok(created)
}

#[derive(Deserialize)]
pub struct UpdatePronunciationRequest {
    pub definition_id: Option<i64>,
    pub ipa: Option<String>,
    pub audio_url: Option<String>,
    pub audio_path: Option<String>,
    pub dialect: Option<String>,
    pub gender: Option<String>,
    pub is_primary: Option<bool>,
}

#[handler]
pub async fn update_pronunciation(req: &mut Request) -> JsonResult<Pronunciation> {
    let pronunciation_id = super::get_path_id(req, "pronunciation_id")?;
    let input: UpdatePronunciationRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    // Validate IPA if provided
    if let Some(ref ipa) = input.ipa {
        if ipa.trim().is_empty() {
            return Err(StatusError::bad_request()
                .brief("ipa cannot be empty")
                .into());
        }
    }

    let changes = UpdatePronunciation {
        definition_id: input.definition_id,
        ipa: input.ipa.map(|i| i.trim().to_string()),
        audio_url: input.audio_url,
        audio_path: input.audio_path,
        dialect: input.dialect,
        gender: input.gender,
        is_primary: input.is_primary,
    };

    let updated: Pronunciation = with_conn(move |conn| {
        diesel::update(
            dict_pronunciations::table.filter(dict_pronunciations::id.eq(pronunciation_id)),
        )
        .set(changes)
        .get_result::<Pronunciation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update pronunciation"))?;

    json_ok(updated)
}

#[handler]
pub async fn delete_pronunciation(req: &mut Request) -> JsonResult<()> {
    let pronunciation_id = super::get_path_id(req, "pronunciation_id")?;
    with_conn(move |conn| {
        diesel::delete(
            dict_pronunciations::table.filter(dict_pronunciations::id.eq(pronunciation_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete pronunciation"))?;
    json_ok(())
}

/// Serve audio file for a pronunciation
///
/// Path: /dict/pronunciations/{id}/audio
#[handler]
pub async fn serve_pronunciation_audio(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let pronunciation_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing pronunciation id"))?;

    // Get the pronunciation from database to find audio_path
    let pronunciation: Pronunciation = with_conn(move |conn| {
        dict_pronunciations::table
            .find(pronunciation_id)
            .first::<Pronunciation>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("pronunciation not found"))?;

    // Check if audio_path exists
    let audio_path = pronunciation.audio_path.ok_or_else(|| {
        StatusError::not_found().brief("no audio available for this pronunciation")
    })?;

    // Build the full file path
    let file_path = PathBuf::from(&AppConfig::get().space_path).join(&audio_path);

    // Check if file exists
    if !file_path.exists() {
        tracing::warn!("Pronunciation audio file not found: {:?}", file_path);
        return Err(StatusError::not_found()
            .brief("audio file not found")
            .into());
    }

    res.send_file(file_path, req.headers()).await;

    Ok(())
}
