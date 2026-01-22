use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{AppResult, DepotExt};

// ============================================================================
// Write Practices API
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct CreateWritePracticeRequest {
    word_id: i64,
    practice_id: String,
    success_level: Option<i32>,
    notes: Option<String>,
}

#[handler]
pub async fn list_write_practices(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let word_id_param = req.query::<i64>("word_id");
    let practice_id_param = req.query::<String>("practice_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let practices: Vec<WritePractice> = with_conn(move |conn| {
        let mut query = learn_write_practices::table
            .filter(learn_write_practices::user_id.eq(user_id))
            .order(learn_write_practices::updated_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(wid) = word_id_param {
            query = query.filter(learn_write_practices::word_id.eq(wid));
        }
        if let Some(pid) = practice_id_param {
            query = query.filter(learn_write_practices::practice_id.eq(pid));
        }

        query.load::<WritePractice>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list write practices"))?;

    res.render(Json(practices));
    Ok(())
}

#[handler]
pub async fn create_write_practice(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateWritePracticeRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let new_practice = NewWritePractice {
        user_id,
        word_id: input.word_id,
        practice_id: input.practice_id,
        success_level: input.success_level,
        notes: input.notes,
    };

    let practice: WritePractice = with_conn(move |conn| {
        diesel::insert_into(learn_write_practices::table)
            .values(&new_practice)
            .get_result::<WritePractice>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create write practice"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(practice));
    Ok(())
}

// ============================================================================
// Reading Practices API
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct CreateReadPracticeRequest {
    sentence_id: Option<i64>,
    practice_id: String,
    user_audio_path: Option<String>,
    pronunciation_score: Option<i32>,
    fluency_score: Option<i32>,
    intonation_score: Option<i32>,
    overall_score: Option<i32>,
    detected_errors: Option<serde_json::Value>,
    ai_feedback_en: Option<String>,
    ai_feedback_zh: Option<String>,
    waveform_data: Option<serde_json::Value>,
}

#[handler]
pub async fn list_read_practices(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let sentence_id_param = req.query::<i64>("sentence_id");
    let practice_id_param = req.query::<String>("practice_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let practices: Vec<ReadPractice> = with_conn(move |conn| {
        let mut query = learn_read_practices::table
            .filter(learn_read_practices::user_id.eq(user_id))
            .order(learn_read_practices::created_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(sid) = sentence_id_param {
            query = query.filter(learn_read_practices::sentence_id.eq(sid));
        }
        if let Some(pid) = practice_id_param {
            query = query.filter(learn_read_practices::practice_id.eq(pid));
        }

        query.load::<ReadPractice>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list read practices"))?;

    res.render(Json(practices));
    Ok(())
}

#[handler]
pub async fn create_read_practice(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateReadPracticeRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let new_practice = NewReadPractice {
        user_id,
        sentence_id: input.sentence_id,
        practice_id: input.practice_id,
        user_audio_path: input.user_audio_path,
        pronunciation_score: input.pronunciation_score,
        fluency_score: input.fluency_score,
        intonation_score: input.intonation_score,
        overall_score: input.overall_score,
        detected_errors: input.detected_errors,
        ai_feedback_en: input.ai_feedback_en,
        ai_feedback_zh: input.ai_feedback_zh,
        waveform_data: input.waveform_data,
    };

    let practice: ReadPractice = with_conn(move |conn| {
        diesel::insert_into(learn_read_practices::table)
            .values(&new_practice)
            .get_result::<ReadPractice>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create read practice"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(practice));
    Ok(())
}
