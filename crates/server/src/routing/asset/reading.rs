use std::path::PathBuf;

use base64::prelude::{BASE64_STANDARD as BASE64, Engine};
use chrono::Utc;
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::AppResult;
use crate::config::AppConfig;
use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::asset::*;
use crate::models::learn::{NewReadPractice, NewUserLevel};
use crate::services::ai_provider::{AiProviderError, AsrResponse};
use crate::services::create_provider_from_env;
use crate::{json_ok, DepotExt, JsonResult};

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
// Sentence Audio
// ============================================================================

/// Serve audio file for a read sentence
///
/// This is a public endpoint (no auth required) since it serves reference audio.
/// Path: /asset/read/sentences/{id}/audio
#[handler]
pub async fn serve_sentence_audio(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let sentence_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing sentence id"))?;

    // Get the sentence from database to find audio_path
    let sentence: ReadSentence = with_conn(move |conn| {
        asset_read_sentences::table
            .find(sentence_id)
            .first::<ReadSentence>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("sentence not found"))?;

    // Check if audio_path exists
    let audio_path = sentence
        .audio_path
        .ok_or_else(|| StatusError::not_found().brief("no audio available for this sentence"))?;

    println!("audio_path: {:?}", audio_path);
    // Build the full file path
    let file_path = PathBuf::from(&AppConfig::get().space_path).join(&audio_path);
    println!("file_path: {:?}", file_path);

    // Check if file exists
    if !file_path.exists() {
        tracing::warn!("Sentence audio file not found: {:?}", file_path);
        return Err(StatusError::not_found()
            .brief("audio file not found")
            .into());
    }

    res.send_file(file_path, req.headers()).await;

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

// ============================================================================
// Recommended Sentences API (requires auth)
// ============================================================================

/// Response for recommended sentences
#[derive(Serialize, ToSchema)]
pub struct RecommendedSentencesResponse {
    pub items: Vec<ReadSentence>,
    pub user_level: i32,
    pub total_available: i64,
    pub all_completed: bool,
    /// If subject_id was provided, indicates if all sentences in that subject are completed
    pub subject_completed: bool,
    /// The subject_id that was filtered by (if any)
    pub subject_id: Option<i64>,
}

/// Get recommended sentences for reading practice
///
/// Returns sentences filtered by:
/// 1. Difficulty >= user_level - 1
/// 2. Excludes sentences where user scored >= 90
/// 3. Optionally filter by subject_id
#[endpoint(tags("Reading"))]
pub async fn get_recommended_sentences(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<RecommendedSentencesResponse> {
    let user_id = depot.user_id()?;
    let limit = req.query::<i64>("limit").unwrap_or(10).clamp(1, 20);
    let subject_id = req.query::<i64>("subject_id");

    let result = with_conn(move |conn| {
        // Get user's pronunciation level (default to 5)
        let user_level: i32 = learn_levels::table
            .filter(learn_levels::user_id.eq(user_id))
            .filter(learn_levels::skill_type.eq("pronunciation"))
            .select(learn_levels::level)
            .first::<i32>(conn)
            .unwrap_or(1);

        // Minimum difficulty threshold (user_level - 1, but at least 1)
        let min_difficulty = (user_level - 1).max(1) as i16;

        // Get sentence IDs where user scored >= 90
        let mastered_sentence_ids: Vec<i64> = learn_read_practices::table
            .filter(learn_read_practices::user_id.eq(user_id))
            .filter(learn_read_practices::overall_score.ge(90))
            .select(learn_read_practices::sentence_id)
            .distinct()
            .load::<Option<i64>>(conn)?
            .into_iter()
            .flatten()
            .collect();

        // Count total available sentences (meeting difficulty threshold, excluding mastered)
        let mut count_query = asset_read_sentences::table
            .filter(
                asset_read_sentences::difficulty
                    .is_null()
                    .or(asset_read_sentences::difficulty.ge(min_difficulty)),
            )
            .into_boxed();

        // Apply subject filter if provided
        if let Some(sid) = subject_id {
            count_query = count_query.filter(asset_read_sentences::subject_id.eq(sid));
        }

        if !mastered_sentence_ids.is_empty() {
            count_query = count_query.filter(
                asset_read_sentences::id.ne_all(mastered_sentence_ids.clone()),
            );
        }

        let total_available: i64 = count_query.count().get_result(conn)?;

        // Get recommended sentences
        let mut query = asset_read_sentences::table
            .filter(
                asset_read_sentences::difficulty
                    .is_null()
                    .or(asset_read_sentences::difficulty.ge(min_difficulty)),
            )
            .into_boxed();

        // Apply subject filter if provided
        if let Some(sid) = subject_id {
            query = query.filter(asset_read_sentences::subject_id.eq(sid));
        }

        if !mastered_sentence_ids.is_empty() {
            query = query.filter(
                asset_read_sentences::id.ne_all(mastered_sentence_ids.clone()),
            );
        }

        let items = query
            .order(asset_read_sentences::difficulty.asc().nulls_last())
            .limit(limit)
            .load::<ReadSentence>(conn)?;

        // Check if all sentences in the subject at current level are completed
        let subject_completed = if let Some(sid) = subject_id {
            // Get all sentence IDs in this subject at current difficulty level
            let subject_sentences: Vec<i64> = asset_read_sentences::table
                .filter(asset_read_sentences::subject_id.eq(sid))
                .filter(
                    asset_read_sentences::difficulty
                        .is_null()
                        .or(asset_read_sentences::difficulty.ge(min_difficulty)),
                )
                .select(asset_read_sentences::id)
                .load::<i64>(conn)?;

            // Check if all are mastered
            !subject_sentences.is_empty()
                && subject_sentences
                    .iter()
                    .all(|sid| mastered_sentence_ids.contains(sid))
        } else {
            false
        };

        Ok::<_, diesel::result::Error>((items, user_level, total_available, subject_completed, subject_id))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get recommended sentences"))?;

    let (items, user_level, total_available, subject_completed, subject_id) = result;
    let all_completed = items.is_empty() && total_available == 0;

    json_ok(RecommendedSentencesResponse {
        items,
        user_level,
        total_available,
        all_completed,
        subject_completed,
        subject_id,
    })
}

/// Request to submit reading practice result
#[derive(Deserialize, ToSchema)]
pub struct SubmitReadPracticeRequest {
    pub sentence_id: i64,
    pub practice_id: String,
    pub overall_score: i32,
    pub pronunciation_score: Option<i32>,
    pub fluency_score: Option<i32>,
    pub intonation_score: Option<i32>,
}

/// Response after submitting practice
#[derive(Serialize, ToSchema)]
pub struct SubmitReadPracticeResponse {
    pub success: bool,
    pub practice_id: i64,
    /// Whether user leveled up after this practice
    pub leveled_up: bool,
    /// User's new level (if leveled up)
    pub new_level: Option<i32>,
}

/// Submit reading practice result
#[endpoint(tags("Reading"))]
pub async fn submit_read_practice(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<SubmitReadPracticeResponse> {
    let user_id = depot.user_id()?;
    let input: SubmitReadPracticeRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let overall_score = input.overall_score;
    let sentence_id = input.sentence_id;

    let (practice_id, leveled_up, new_level) = with_conn(move |conn| {
        // Save the practice record
        let new_practice = NewReadPractice {
            user_id,
            sentence_id: Some(sentence_id),
            practice_id: input.practice_id,
            user_audio_path: None,
            pronunciation_score: input.pronunciation_score,
            fluency_score: input.fluency_score,
            intonation_score: input.intonation_score,
            overall_score: Some(overall_score),
            detected_errors: None,
            ai_feedback_en: None,
            ai_feedback_zh: None,
            waveform_data: None,
        };

        let practice_id = diesel::insert_into(learn_read_practices::table)
            .values(&new_practice)
            .returning(learn_read_practices::id)
            .get_result::<i64>(conn)?;

        // Check if we should level up (only if score > 90)
        let mut leveled_up = false;
        let mut new_level: Option<i32> = None;

        if overall_score > 90 {
            // Get user's current pronunciation level
            let user_level: i32 = learn_levels::table
                .filter(learn_levels::user_id.eq(user_id))
                .filter(learn_levels::skill_type.eq("pronunciation"))
                .select(learn_levels::level)
                .first::<i32>(conn)
                .unwrap_or(1);

            // Get the difficulty of all sentences at the current user level
            // Find sentences where user has NOT scored >= 90
            // i.e., sentences at current difficulty that are not mastered yet
            let current_level_difficulty = user_level as i16;

            // Get all sentence IDs at current difficulty level
            let sentences_at_level: Vec<i64> = asset_read_sentences::table
                .filter(asset_read_sentences::difficulty.eq(current_level_difficulty))
                .select(asset_read_sentences::id)
                .load::<i64>(conn)?;

            if !sentences_at_level.is_empty() {
                // Get sentence IDs where user scored >= 90
                let mastered_ids: Vec<i64> = learn_read_practices::table
                    .filter(learn_read_practices::user_id.eq(user_id))
                    .filter(learn_read_practices::sentence_id.is_not_null())
                    .filter(learn_read_practices::overall_score.ge(90))
                    .select(learn_read_practices::sentence_id)
                    .distinct()
                    .load::<Option<i64>>(conn)?
                    .into_iter()
                    .flatten()
                    .collect();

                // Check if all sentences at current level are mastered
                let all_mastered = sentences_at_level
                    .iter()
                    .all(|sid| mastered_ids.contains(sid));

                if all_mastered && user_level < 10 {
                    // Level up!
                    let next_level = user_level + 1;
                    let now = Utc::now();

                    diesel::insert_into(learn_levels::table)
                        .values(&NewUserLevel {
                            user_id,
                            skill_type: "pronunciation".to_string(),
                            level: next_level,
                        })
                        .on_conflict((learn_levels::user_id, learn_levels::skill_type))
                        .do_update()
                        .set((
                            learn_levels::level.eq(next_level),
                            learn_levels::updated_at.eq(now),
                        ))
                        .execute(conn)?;

                    leveled_up = true;
                    new_level = Some(next_level);
                    tracing::info!(
                        "User {} leveled up pronunciation from {} to {}",
                        user_id,
                        user_level,
                        next_level
                    );
                }
            }
        }

        Ok::<_, diesel::result::Error>((practice_id, leveled_up, new_level))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to save practice"))?;

    json_ok(SubmitReadPracticeResponse {
        success: true,
        practice_id,
        leveled_up,
        new_level,
    })
}

/// Request for resetting skill level
#[derive(Deserialize, ToSchema)]
pub struct ResetLevelRequest {
    /// The skill type to reset (e.g., "pronunciation", "speaking", "listening")
    pub skill_type: String,
}

/// Response for reset level
#[derive(Serialize, ToSchema)]
pub struct ResetLevelResponse {
    pub success: bool,
    pub new_level: i32,
    pub skill_type: String,
}

/// Reset user's skill level to 1
/// This also clears practice records for the corresponding skill
#[endpoint(tags("Reading"))]
pub async fn reset_pronunciation_level(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<ResetLevelResponse> {
    let user_id = depot.user_id()?;
    let input: ResetLevelRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let skill_type = input.skill_type;
    let new_level = 1;
    let now = Utc::now();

    let skill_type_clone = skill_type.clone();
    with_conn(move |conn| {
        // Upsert user level
        diesel::insert_into(learn_levels::table)
            .values(&NewUserLevel {
                user_id,
                skill_type: skill_type_clone.clone(),
                level: new_level,
            })
            .on_conflict((learn_levels::user_id, learn_levels::skill_type))
            .do_update()
            .set((
                learn_levels::level.eq(new_level),
                learn_levels::updated_at.eq(now),
            ))
            .execute(conn)?;

        // Delete all practice records with score >= 90 to "reset" progress
        // Only for pronunciation skill type (reading practices)
        if skill_type_clone == "pronunciation" {
            diesel::delete(
                learn_read_practices::table
                    .filter(learn_read_practices::user_id.eq(user_id))
                    .filter(learn_read_practices::overall_score.ge(90)),
            )
            .execute(conn)?;
        }

        Ok::<_, diesel::result::Error>(())
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset level"))?;

    json_ok(ResetLevelResponse {
        success: true,
        new_level,
        skill_type,
    })
}

/// Request for advancing skill level
#[derive(Deserialize, ToSchema)]
pub struct AdvanceLevelRequest {
    /// The skill type to advance (e.g., "pronunciation", "speaking", "listening")
    pub skill_type: String,
}

/// Advance user's skill level by 1 (max 10)
/// Allows user to manually skip to harder content
#[endpoint(tags("Reading"))]
pub async fn advance_pronunciation_level(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<ResetLevelResponse> {
    let user_id = depot.user_id()?;
    let input: AdvanceLevelRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let skill_type = input.skill_type;
    let now = Utc::now();

    let skill_type_clone = skill_type.clone();
    let new_level = with_conn(move |conn| {
        // Get current level
        let current_level: i32 = learn_levels::table
            .filter(learn_levels::user_id.eq(user_id))
            .filter(learn_levels::skill_type.eq(&skill_type_clone))
            .select(learn_levels::level)
            .first::<i32>(conn)
            .unwrap_or(1);

        // Cap at level 10
        let next_level = (current_level + 1).min(10);

        // Upsert user level
        diesel::insert_into(learn_levels::table)
            .values(&NewUserLevel {
                user_id,
                skill_type: skill_type_clone.clone(),
                level: next_level,
            })
            .on_conflict((learn_levels::user_id, learn_levels::skill_type))
            .do_update()
            .set((
                learn_levels::level.eq(next_level),
                learn_levels::updated_at.eq(now),
            ))
            .execute(conn)?;

        Ok::<_, diesel::result::Error>(next_level)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to advance level"))?;

    json_ok(ResetLevelResponse {
        success: true,
        new_level,
        skill_type,
    })
}
