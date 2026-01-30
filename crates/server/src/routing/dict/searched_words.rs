use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::Text;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{DepotExt, JsonResult, json_ok};

// Define a SQL function for case-insensitive comparison
diesel::sql_function!(fn lower(x: Text) -> Text);

#[derive(Deserialize)]
pub struct CreateSearchedWordRequest {
    pub word: String,
    /// Optional timestamp for when the word was searched.
    /// If not provided, current time will be used.
    pub searched_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct BatchCreateSearchedWordsRequest {
    pub words: Vec<CreateSearchedWordRequest>,
}

/// Create or update a searched word record.
/// If the word already exists for this user, updates the searched_at time.
/// Requires authentication.
#[handler]
pub async fn create_searched_word(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<SearchedWord> {
    let user_id = depot.user_id()?;

    let input: CreateSearchedWordRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let word = input.word.trim().to_string();
    if word.is_empty() {
        return Err(StatusError::bad_request().brief("word is required").into());
    }

    let searched_at = input.searched_at.unwrap_or_else(Utc::now);
    let word_lower = word.to_lowercase();

    // Upsert: update if exists, insert if not
    let result: SearchedWord = with_conn(move |conn| {
        // First try to find existing record
        let existing = dict_searched_words::table
            .filter(dict_searched_words::user_id.eq(user_id))
            .filter(lower(dict_searched_words::word).eq(&word_lower))
            .first::<SearchedWord>(conn)
            .optional()?;

        if let Some(existing_record) = existing {
            // Update the searched_at time
            diesel::update(dict_searched_words::table.filter(dict_searched_words::id.eq(existing_record.id)))
                .set(dict_searched_words::searched_at.eq(Some(searched_at)))
                .get_result::<SearchedWord>(conn)
        } else {
            // Insert new record
            diesel::insert_into(dict_searched_words::table)
                .values(&NewSearchedWord { user_id, word })
                .get_result::<SearchedWord>(conn)
                .map(|mut record| {
                    // Update searched_at if provided (since NewSearchedWord doesn't include it)
                    record.searched_at = Some(searched_at);
                    record
                })
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to save searched word"))?;

    json_ok(result)
}

/// Batch create/update searched words.
/// Used when syncing local history after login.
/// Requires authentication.
#[handler]
pub async fn batch_create_searched_words(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<Vec<SearchedWord>> {
    let user_id = depot.user_id()?;

    let input: BatchCreateSearchedWordsRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.words.is_empty() {
        return json_ok(vec![]);
    }

    let results: Vec<SearchedWord> = with_conn(move |conn| {
        let mut saved = Vec::new();

        for item in input.words {
            let word = item.word.trim().to_string();
            if word.is_empty() {
                continue;
            }

            let searched_at = item.searched_at.unwrap_or_else(Utc::now);
            let word_lower = word.to_lowercase();

            // Check if exists
            let existing = dict_searched_words::table
                .filter(dict_searched_words::user_id.eq(user_id))
                .filter(lower(dict_searched_words::word).eq(&word_lower))
                .first::<SearchedWord>(conn)
                .optional()?;

            let record = if let Some(existing_record) = existing {
                // Update if the new timestamp is newer
                if existing_record.searched_at.map_or(true, |t| searched_at > t) {
                    diesel::update(
                        dict_searched_words::table.filter(dict_searched_words::id.eq(existing_record.id)),
                    )
                    .set(dict_searched_words::searched_at.eq(Some(searched_at)))
                    .get_result::<SearchedWord>(conn)?
                } else {
                    existing_record
                }
            } else {
                // Insert new
                let new_record: SearchedWord = diesel::insert_into(dict_searched_words::table)
                    .values(&NewSearchedWord {
                        user_id,
                        word: word.clone(),
                    })
                    .get_result(conn)?;

                // Update searched_at
                diesel::update(
                    dict_searched_words::table.filter(dict_searched_words::id.eq(new_record.id)),
                )
                .set(dict_searched_words::searched_at.eq(Some(searched_at)))
                .get_result::<SearchedWord>(conn)?
            };

            saved.push(record);
        }

        Ok(saved)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to save searched words"))?;

    json_ok(results)
}

/// List searched words for the authenticated user.
/// Results are unique by word (no duplicates) and sorted by most recent first.
/// If user is not authenticated, returns empty list.
#[handler]
pub async fn list_searched_words(req: &mut Request, depot: &mut Depot) -> JsonResult<Vec<SearchedWord>> {
    let limit = req.query::<i64>("limit").unwrap_or(20).clamp(1, 100);

    // Get user_id from depot, return empty if not authenticated
    let user_id = match depot.get::<i64>("user_id").copied() {
        Ok(id) => id,
        Err(_) => return json_ok(vec![]),
    };

    let words: Vec<SearchedWord> = with_conn(move |conn| {
        // Get distinct words ordered by most recent searched_at
        dict_searched_words::table
            .filter(dict_searched_words::user_id.eq(user_id))
            .order(dict_searched_words::searched_at.desc())
            .limit(limit)
            .load::<SearchedWord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch searched words"))?;

    json_ok(words)
}

/// Delete a single searched word by ID.
/// Requires authentication.
#[handler]
pub async fn delete_searched_word(req: &mut Request, depot: &mut Depot) -> JsonResult<serde_json::Value> {
    let user_id = depot.user_id()?;

    let id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id parameter"))?;

    let deleted = with_conn(move |conn| {
        diesel::delete(
            dict_searched_words::table
                .filter(dict_searched_words::id.eq(id))
                .filter(dict_searched_words::user_id.eq(user_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete searched word"))?;

    json_ok(serde_json::json!({ "deleted": deleted > 0 }))
}

/// Clear all searched words for the authenticated user.
/// Requires authentication.
#[handler]
pub async fn clear_searched_words(depot: &mut Depot) -> JsonResult<serde_json::Value> {
    let user_id = depot.user_id()?;

    let deleted = with_conn(move |conn| {
        diesel::delete(dict_searched_words::table.filter(dict_searched_words::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to clear searched words"))?;

    json_ok(serde_json::json!({ "cleared": true, "count": deleted }))
}
