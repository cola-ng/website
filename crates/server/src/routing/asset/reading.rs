use base64::prelude::{Engine, BASE64_STANDARD as BASE64};
use diesel::prelude::*;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::AppResult;
use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::asset::*;
use crate::services::ai_provider::{AiProviderError, AsrResponse};
use crate::services::create_provider_from_env;

#[derive(Serialize, ToSchema)]
pub struct PaginatedSubjects {
    pub items: Vec<ReadSubject>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Serialize, ToSchema)]
pub struct PaginatedSentences {
    pub items: Vec<ReadSentence>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[handler]
pub async fn list_read_subjects(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let difficulty = req.query::<i16>("difficulty");
    let subject_type = req.query::<String>("type");
    let page = req.query::<i64>("page").unwrap_or(1).max(1);
    let per_page = req.query::<i64>("per_page").unwrap_or(20).clamp(1, 100);

    let offset = (page - 1) * per_page;

    let (subjects, total): (Vec<ReadSubject>, i64) = with_conn(move |conn| {
        let mut query = asset_read_subjects::table.into_boxed();
        let mut count_query = asset_read_subjects::table.into_boxed();

        if let Some(diff) = difficulty {
            query = query.filter(asset_read_subjects::difficulty.eq(diff));
            count_query = count_query.filter(asset_read_subjects::difficulty.eq(diff));
        }
        if let Some(ref st) = subject_type {
            query = query.filter(asset_read_subjects::subject_type.eq(st));
            count_query = count_query.filter(asset_read_subjects::subject_type.eq(st.clone()));
        }

        let total: i64 = count_query.count().get_result(conn)?;

        let items = query
            .order(asset_read_subjects::id.asc())
            .offset(offset)
            .limit(per_page)
            .load::<ReadSubject>(conn)?;

        Ok((items, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list subjects"))?;

    res.render(Json(PaginatedSubjects {
        items: subjects,
        total,
        page,
        per_page,
    }));
    Ok(())
}

#[handler]
pub async fn get_read_sentences(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let subject_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let page = req.query::<i64>("page").unwrap_or(1).max(1);
    let per_page = req.query::<i64>("per_page").unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let (sentences, total): (Vec<ReadSentence>, i64) = with_conn(move |conn| {
        let total: i64 = asset_read_sentences::table
            .filter(asset_read_sentences::subject_id.eq(subject_id))
            .count()
            .get_result(conn)?;

        let items = asset_read_sentences::table
            .filter(asset_read_sentences::subject_id.eq(subject_id))
            .order(asset_read_sentences::sentence_order.asc())
            .offset(offset)
            .limit(per_page)
            .load::<ReadSentence>(conn)?;

        Ok((items, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list sentences"))?;

    res.render(Json(PaginatedSentences {
        items: sentences,
        total,
        page,
        per_page,
    }));
    Ok(())
}

#[handler]
pub async fn list_read_sentences(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let subject_id = req.query::<i64>("subject");
    let page = req.query::<i64>("page").unwrap_or(1).max(1);
    let per_page = req.query::<i64>("per_page").unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let (sentences, total): (Vec<ReadSentence>, i64) = with_conn(move |conn| {
        let mut query = asset_read_sentences::table.into_boxed();
        let mut count_query = asset_read_sentences::table.into_boxed();

        if let Some(sid) = subject_id {
            query = query.filter(asset_read_sentences::subject_id.eq(sid));
            count_query = count_query.filter(asset_read_sentences::subject_id.eq(sid));
        }

        let total: i64 = count_query.count().get_result(conn)?;

        let items = query
            .order(asset_read_sentences::sentence_order.asc())
            .offset(offset)
            .limit(per_page)
            .load::<ReadSentence>(conn)?;

        Ok((items, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list sentences"))?;

    res.render(Json(PaginatedSentences {
        items: sentences,
        total,
        page,
        per_page,
    }));
    Ok(())
}

// ============================================================================
// Pronunciation Evaluation
// ============================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct EvaluateRequest {
    /// Base64 encoded audio data
    pub audio_base64: String,
    /// Reference text to compare against
    pub reference_text: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EvaluateResponse {
    /// Transcribed text from audio
    pub transcribed_text: String,
    /// Overall pronunciation score (0-100)
    pub overall_score: i32,
    /// Pronunciation accuracy score (0-100)
    pub pronunciation_score: i32,
    /// Fluency score (0-100)
    pub fluency_score: i32,
    /// Intonation score (0-100)
    pub intonation_score: i32,
    /// Feedback messages
    pub feedback: Vec<FeedbackItem>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeedbackItem {
    /// Type: "good" | "warning"
    #[serde(rename = "type")]
    pub item_type: String,
    /// Feedback message
    pub message: String,
}

#[handler]
pub async fn evaluate_pronunciation(req: &mut Request, res: &mut Response) -> AppResult<()> {
    // Parse request body
    let body_bytes = req.payload().await.map_err(|e| {
        tracing::error!("evaluate_pronunciation: read_body error: {:?}", e);
        StatusError::bad_request().brief("failed to read request body")
    })?;

    let input: EvaluateRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::error!("evaluate_pronunciation: parse_json error: {:?}", e);
        StatusError::bad_request().brief("invalid json")
    })?;

    // Decode audio
    let audio_data = BASE64
        .decode(&input.audio_base64)
        .map_err(|_| StatusError::bad_request().brief("invalid base64 audio"))?;

    if audio_data.is_empty() {
        return Err(StatusError::bad_request()
            .brief("audio data is empty")
            .into());
    }

    // Get AI provider
    let provider = create_provider_from_env()
        .ok_or_else(|| StatusError::internal_server_error().brief("AI provider not configured"))?;

    // Get ASR service
    let asr = provider.asr().ok_or_else(|| {
        StatusError::internal_server_error().brief("ASR service not available")
    })?;

    // Transcribe audio
    tracing::info!("Evaluating pronunciation using {} ASR...", provider.name());
    let asr_result: AsrResponse = asr
        .transcribe(audio_data, Some("en"))
        .await
        .map_err(|e: AiProviderError| {
            tracing::error!("{} ASR error: {:?}", provider.name(), e);
            StatusError::internal_server_error().brief(e.to_string())
        })?;

    let transcribed_text = asr_result.text.trim().to_string();
    tracing::info!("ASR result: {}", transcribed_text);

    if transcribed_text.is_empty() {
        return Err(StatusError::bad_request()
            .brief("Could not transcribe audio - no speech detected")
            .into());
    }

    // Calculate scores by comparing transcribed text with reference
    let (overall_score, pronunciation_score, fluency_score, intonation_score, feedback) =
        calculate_pronunciation_score(&transcribed_text, &input.reference_text);

    res.render(Json(EvaluateResponse {
        transcribed_text,
        overall_score,
        pronunciation_score,
        fluency_score,
        intonation_score,
        feedback,
    }));
    Ok(())
}

/// Calculate pronunciation score by comparing transcribed text with reference
fn calculate_pronunciation_score(
    transcribed: &str,
    reference: &str,
) -> (i32, i32, i32, i32, Vec<FeedbackItem>) {
    // Normalize texts for comparison
    let transcribed_lower = transcribed.to_lowercase();
    let transcribed_words: Vec<&str> = transcribed_lower
        .split_whitespace()
        .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|s| !s.is_empty())
        .collect();

    let reference_lower = reference.to_lowercase();
    let reference_words: Vec<&str> = reference_lower
        .split_whitespace()
        .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|s| !s.is_empty())
        .collect();

    if reference_words.is_empty() {
        return (0, 0, 0, 0, vec![]);
    }

    // Calculate word match score
    let mut matched_words = 0;
    let mut transcribed_set: std::collections::HashSet<&str> =
        transcribed_words.iter().copied().collect();

    for ref_word in &reference_words {
        if transcribed_set.remove(*ref_word) {
            matched_words += 1;
        }
    }

    // Base pronunciation score from word matching
    let match_ratio = matched_words as f32 / reference_words.len() as f32;
    let pronunciation_score = (match_ratio * 100.0).round() as i32;

    // Fluency score - penalize for extra or missing words
    let word_count_diff = (transcribed_words.len() as i32 - reference_words.len() as i32).abs();
    let fluency_penalty = (word_count_diff as f32 / reference_words.len() as f32 * 30.0).min(30.0);
    let fluency_score = (100.0 - fluency_penalty).max(0.0) as i32;

    // Intonation score - simplified, based on completeness
    let intonation_score = if match_ratio > 0.9 {
        85 + (match_ratio * 15.0) as i32
    } else if match_ratio > 0.7 {
        70 + (match_ratio * 20.0) as i32
    } else {
        (match_ratio * 80.0) as i32
    };

    // Overall score
    let overall_score = (pronunciation_score as f32 * 0.5
        + fluency_score as f32 * 0.3
        + intonation_score as f32 * 0.2)
        .round() as i32;

    // Generate feedback
    let mut feedback = Vec::new();

    if pronunciation_score >= 90 {
        feedback.push(FeedbackItem {
            item_type: "good".to_string(),
            message: "发音清晰准确！".to_string(),
        });
    } else if pronunciation_score >= 70 {
        feedback.push(FeedbackItem {
            item_type: "good".to_string(),
            message: "发音基本准确，继续保持！".to_string(),
        });
    }

    if fluency_score >= 90 {
        feedback.push(FeedbackItem {
            item_type: "good".to_string(),
            message: "语速流畅自然！".to_string(),
        });
    } else if fluency_score < 70 {
        feedback.push(FeedbackItem {
            item_type: "warning".to_string(),
            message: "注意控制语速，保持自然流畅".to_string(),
        });
    }

    if pronunciation_score < 70 {
        feedback.push(FeedbackItem {
            item_type: "warning".to_string(),
            message: "注意单词发音的准确性".to_string(),
        });
    }

    if matched_words < reference_words.len() {
        let missing_count = reference_words.len() - matched_words;
        feedback.push(FeedbackItem {
            item_type: "warning".to_string(),
            message: format!("有{}个单词发音不够清晰", missing_count),
        });
    }

    (
        overall_score.clamp(0, 100),
        pronunciation_score.clamp(0, 100),
        fluency_score.clamp(0, 100),
        intonation_score.clamp(0, 100),
        feedback,
    )
}
