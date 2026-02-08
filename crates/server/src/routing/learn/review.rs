use chrono::{Duration, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{DepotExt, JsonResult, json_ok};

/// Review types supported by the system
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewType {
    Pronunciation,
    Meaning,
    Spelling,
    Recognition,
    Phrase,
}

/// A single item to review
#[derive(Debug, Serialize, ToSchema)]
pub struct ReviewItem {
    pub id: i64,
    pub word: String,
    pub word_zh: Option<String>,
    pub phonetic: Option<String>,
    pub example: Option<String>,
    pub example_zh: Option<String>,
    pub review_type: String,
    pub mastery_level: i32,
    pub source: String, // "vocabulary" or "issue_word"
    pub source_id: i64,
}

/// Response for list review items
#[derive(Serialize, ToSchema)]
pub struct ReviewItemsResponse {
    pub items: Vec<ReviewItem>,
    pub total: i64,
}

/// Review statistics by type
#[derive(Serialize, ToSchema)]
pub struct ReviewStats {
    pub pronunciation: i64,
    pub meaning: i64,
    pub spelling: i64,
    pub recognition: i64,
    pub phrase: i64,
    pub total: i64,
}

/// Request to submit review result
#[derive(Deserialize, ToSchema)]
pub struct SubmitReviewRequest {
    pub source: String, // "vocabulary" or "issue_word"
    pub source_id: i64,
    pub review_type: String,
    pub result: String,     // "correct", "partial", "incorrect"
    pub score: Option<i32>, // 0-100 for pronunciation
    pub time_spent_ms: i64,
}

/// Response after submitting review
#[derive(Serialize, ToSchema)]
pub struct SubmitReviewResponse {
    pub success: bool,
    pub new_mastery_level: Option<i32>,
    pub next_review_at: Option<String>,
}

/// Get review items by type
#[endpoint(tags("Learn"))]
pub async fn get_review_items(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<ReviewItemsResponse> {
    let user_id = depot.user_id()?;
    let review_type = req.query::<String>("type").unwrap_or_default();
    let limit = req.query::<i64>("limit").unwrap_or(20).clamp(1, 100);

    let now = Utc::now();

    let items: Vec<ReviewItem> = with_conn(move |conn| {
        let mut result = Vec::new();

        match review_type.as_str() {
            "pronunciation" => {
                // Get issue words with pronunciation issues
                let issue_words: Vec<IssueWord> = learn_issue_words::table
                    .filter(learn_issue_words::user_id.eq(user_id))
                    .filter(learn_issue_words::issue_type.eq("pronunciation"))
                    .filter(
                        learn_issue_words::next_review_at
                            .is_null()
                            .or(learn_issue_words::next_review_at.le(now)),
                    )
                    .order(learn_issue_words::next_review_at.asc().nulls_first())
                    .limit(limit)
                    .load::<IssueWord>(conn)?;

                for iw in issue_words {
                    result.push(ReviewItem {
                        id: iw.id,
                        word: iw.word.clone(),
                        word_zh: iw.description_zh.clone(),
                        phonetic: None,
                        example: iw.context.clone(),
                        example_zh: None,
                        review_type: "pronunciation".to_string(),
                        mastery_level: iw.difficulty.unwrap_or(3),
                        source: "issue_word".to_string(),
                        source_id: iw.id,
                    });
                }
            }
            "meaning" | "spelling" | "recognition" => {
                // Get vocabulary words due for review
                let vocabs: Vec<UserVocabulary> = learn_vocabularies::table
                    .filter(learn_vocabularies::user_id.eq(user_id))
                    .filter(
                        learn_vocabularies::next_review_at
                            .is_null()
                            .or(learn_vocabularies::next_review_at.le(now)),
                    )
                    .filter(learn_vocabularies::mastery_level.lt(4)) // Not mastered
                    .order(learn_vocabularies::next_review_at.asc().nulls_first())
                    .limit(limit)
                    .load::<UserVocabulary>(conn)?;

                for v in vocabs {
                    result.push(ReviewItem {
                        id: v.id,
                        word: v.word.clone(),
                        word_zh: v.word_zh.clone(),
                        phonetic: None,
                        example: None,
                        example_zh: None,
                        review_type: review_type.clone(),
                        mastery_level: v.mastery_level.unwrap_or(0),
                        source: "vocabulary".to_string(),
                        source_id: v.id,
                    });
                }
            }
            "phrase" => {
                // Get issue words with usage issues (phrases)
                let issue_words: Vec<IssueWord> = learn_issue_words::table
                    .filter(learn_issue_words::user_id.eq(user_id))
                    .filter(learn_issue_words::issue_type.eq("usage"))
                    .filter(
                        learn_issue_words::next_review_at
                            .is_null()
                            .or(learn_issue_words::next_review_at.le(now)),
                    )
                    .order(learn_issue_words::next_review_at.asc().nulls_first())
                    .limit(limit)
                    .load::<IssueWord>(conn)?;

                for iw in issue_words {
                    result.push(ReviewItem {
                        id: iw.id,
                        word: iw.word.clone(),
                        word_zh: iw.description_zh.clone(),
                        phonetic: None,
                        example: iw.context.clone(),
                        example_zh: None,
                        review_type: "phrase".to_string(),
                        mastery_level: iw.difficulty.unwrap_or(3),
                        source: "issue_word".to_string(),
                        source_id: iw.id,
                    });
                }
            }
            "smart" | _ => {
                // Smart mode: mix items from all types based on priority
                let items_per_type = (limit / 5).max(2);

                // 1. Pronunciation issues (highest priority)
                let pronunciation_issues: Vec<IssueWord> = learn_issue_words::table
                    .filter(learn_issue_words::user_id.eq(user_id))
                    .filter(learn_issue_words::issue_type.eq("pronunciation"))
                    .filter(
                        learn_issue_words::next_review_at
                            .is_null()
                            .or(learn_issue_words::next_review_at.le(now)),
                    )
                    .order(learn_issue_words::next_review_at.asc().nulls_first())
                    .limit(items_per_type)
                    .load::<IssueWord>(conn)?;

                for iw in pronunciation_issues {
                    result.push(ReviewItem {
                        id: iw.id,
                        word: iw.word.clone(),
                        word_zh: iw.description_zh.clone(),
                        phonetic: None,
                        example: iw.context.clone(),
                        example_zh: None,
                        review_type: "pronunciation".to_string(),
                        mastery_level: iw.difficulty.unwrap_or(3),
                        source: "issue_word".to_string(),
                        source_id: iw.id,
                    });
                }

                // 2. Phrase (usage) issues
                let phrase_issues: Vec<IssueWord> = learn_issue_words::table
                    .filter(learn_issue_words::user_id.eq(user_id))
                    .filter(learn_issue_words::issue_type.eq("usage"))
                    .filter(
                        learn_issue_words::next_review_at
                            .is_null()
                            .or(learn_issue_words::next_review_at.le(now)),
                    )
                    .order(learn_issue_words::next_review_at.asc().nulls_first())
                    .limit(items_per_type)
                    .load::<IssueWord>(conn)?;

                for iw in phrase_issues {
                    result.push(ReviewItem {
                        id: iw.id,
                        word: iw.word.clone(),
                        word_zh: iw.description_zh.clone(),
                        phonetic: None,
                        example: iw.context.clone(),
                        example_zh: None,
                        review_type: "phrase".to_string(),
                        mastery_level: iw.difficulty.unwrap_or(3),
                        source: "issue_word".to_string(),
                        source_id: iw.id,
                    });
                }

                // 3. Vocabulary items - distribute across meaning, spelling, recognition
                let remaining_slots = (limit as usize).saturating_sub(result.len());
                let vocab_items: Vec<UserVocabulary> = learn_vocabularies::table
                    .filter(learn_vocabularies::user_id.eq(user_id))
                    .filter(
                        learn_vocabularies::next_review_at
                            .is_null()
                            .or(learn_vocabularies::next_review_at.le(now)),
                    )
                    .filter(learn_vocabularies::mastery_level.lt(4))
                    .order(learn_vocabularies::next_review_at.asc().nulls_first())
                    .limit(remaining_slots as i64)
                    .load::<UserVocabulary>(conn)?;

                // Rotate through review types for vocabulary
                let vocab_types = ["meaning", "spelling", "recognition"];
                for (i, v) in vocab_items.iter().enumerate() {
                    let review_type = vocab_types[i % vocab_types.len()];
                    result.push(ReviewItem {
                        id: v.id,
                        word: v.word.clone(),
                        word_zh: v.word_zh.clone(),
                        phonetic: None,
                        example: None,
                        example_zh: None,
                        review_type: review_type.to_string(),
                        mastery_level: v.mastery_level.unwrap_or(0),
                        source: "vocabulary".to_string(),
                        source_id: v.id,
                    });
                }
            }
        }

        Ok::<_, diesel::result::Error>(result)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get review items"))?;

    let total = items.len() as i64;

    json_ok(ReviewItemsResponse { items, total })
}

/// Get review statistics by type
#[endpoint(tags("Learn"))]
pub async fn get_review_stats(depot: &mut Depot) -> JsonResult<ReviewStats> {
    let user_id = depot.user_id()?;
    let now = Utc::now();

    let stats = with_conn(move |conn| {
        // Count pronunciation issues
        let pronunciation: i64 = learn_issue_words::table
            .filter(learn_issue_words::user_id.eq(user_id))
            .filter(learn_issue_words::issue_type.eq("pronunciation"))
            .filter(
                learn_issue_words::next_review_at
                    .is_null()
                    .or(learn_issue_words::next_review_at.le(now)),
            )
            .count()
            .get_result(conn)?;

        // Count phrase (usage) issues
        let phrase: i64 = learn_issue_words::table
            .filter(learn_issue_words::user_id.eq(user_id))
            .filter(learn_issue_words::issue_type.eq("usage"))
            .filter(
                learn_issue_words::next_review_at
                    .is_null()
                    .or(learn_issue_words::next_review_at.le(now)),
            )
            .count()
            .get_result(conn)?;

        // Count vocabulary due for review (not mastered)
        let vocab_due: i64 = learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .filter(
                learn_vocabularies::next_review_at
                    .is_null()
                    .or(learn_vocabularies::next_review_at.le(now)),
            )
            .filter(learn_vocabularies::mastery_level.lt(4))
            .count()
            .get_result(conn)?;

        // Distribute vocab count across meaning, spelling, recognition
        let per_type = vocab_due / 3;
        let remainder = vocab_due % 3;

        Ok::<_, diesel::result::Error>(ReviewStats {
            pronunciation,
            meaning: per_type + if remainder > 0 { 1 } else { 0 },
            spelling: per_type + if remainder > 1 { 1 } else { 0 },
            recognition: per_type,
            phrase,
            total: pronunciation + vocab_due + phrase,
        })
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get review stats"))?;

    json_ok(stats)
}

/// Submit review result
#[endpoint(tags("Learn"))]
pub async fn submit_review(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<SubmitReviewResponse> {
    let user_id = depot.user_id()?;
    let input: SubmitReviewRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let now = Utc::now();

    // Calculate next review interval based on result (simple spaced repetition)
    let interval_days = match input.result.as_str() {
        "correct" => {
            if let Some(score) = input.score {
                if score >= 90 {
                    7
                } else if score >= 80 {
                    4
                } else {
                    2
                }
            } else {
                4
            }
        }
        "partial" => 1,
        "incorrect" | _ => 0, // Review again soon (same day)
    };

    let next_review_at = if interval_days > 0 {
        Some(now + Duration::days(interval_days))
    } else {
        Some(now + Duration::hours(4)) // Review in 4 hours for incorrect
    };

    let mastery_delta = match input.result.as_str() {
        "correct" => 1,
        "partial" => 0,
        "incorrect" | _ => -1,
    };

    let response = with_conn(move |conn| {
        match input.source.as_str() {
            "vocabulary" => {
                // Update vocabulary
                let vocab: Option<UserVocabulary> = learn_vocabularies::table
                    .filter(learn_vocabularies::id.eq(input.source_id))
                    .filter(learn_vocabularies::user_id.eq(user_id))
                    .first::<UserVocabulary>(conn)
                    .optional()?;

                if let Some(v) = vocab {
                    let current_mastery = v.mastery_level.unwrap_or(0);
                    let new_mastery = (current_mastery + mastery_delta).clamp(0, 5);
                    let practice_count = v.practice_count.unwrap_or(0) + 1;
                    let correct_count = v.correct_count.unwrap_or(0)
                        + if input.result == "correct" { 1 } else { 0 };

                    diesel::update(learn_vocabularies::table.find(input.source_id))
                        .set((
                            learn_vocabularies::mastery_level.eq(new_mastery),
                            learn_vocabularies::practice_count.eq(practice_count),
                            learn_vocabularies::correct_count.eq(correct_count),
                            learn_vocabularies::last_practiced_at.eq(now),
                            learn_vocabularies::next_review_at.eq(next_review_at),
                        ))
                        .execute(conn)?;

                    Ok::<_, diesel::result::Error>(SubmitReviewResponse {
                        success: true,
                        new_mastery_level: Some(new_mastery),
                        next_review_at: next_review_at.map(|t| t.to_rfc3339()),
                    })
                } else {
                    Ok(SubmitReviewResponse {
                        success: false,
                        new_mastery_level: None,
                        next_review_at: None,
                    })
                }
            }
            "issue_word" => {
                // Update issue word
                let issue: Option<IssueWord> = learn_issue_words::table
                    .filter(learn_issue_words::id.eq(input.source_id))
                    .filter(learn_issue_words::user_id.eq(user_id))
                    .first::<IssueWord>(conn)
                    .optional()?;

                if let Some(iw) = issue {
                    let current_difficulty = iw.difficulty.unwrap_or(3);
                    let new_difficulty = (current_difficulty - mastery_delta).clamp(1, 5);
                    let pick_count = iw.pick_count + 1;

                    diesel::update(learn_issue_words::table.find(input.source_id))
                        .set((
                            learn_issue_words::difficulty.eq(new_difficulty),
                            learn_issue_words::pick_count.eq(pick_count),
                            learn_issue_words::last_picked_at.eq(now),
                            learn_issue_words::next_review_at.eq(next_review_at),
                            learn_issue_words::review_interval_days.eq(interval_days as i32),
                        ))
                        .execute(conn)?;

                    Ok(SubmitReviewResponse {
                        success: true,
                        new_mastery_level: Some(5 - new_difficulty), // Invert for consistency
                        next_review_at: next_review_at.map(|t| t.to_rfc3339()),
                    })
                } else {
                    Ok(SubmitReviewResponse {
                        success: false,
                        new_mastery_level: None,
                        next_review_at: None,
                    })
                }
            }
            _ => Ok(SubmitReviewResponse {
                success: false,
                new_mastery_level: None,
                next_review_at: None,
            }),
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to submit review"))?;

    json_ok(response)
}
