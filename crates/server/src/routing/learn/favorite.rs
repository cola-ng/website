use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{DepotExt, JsonResult, json_ok};

#[derive(Deserialize, ToSchema)]
pub struct CreateFavoriteRequest {
    /// Entity type (table name in singular form, e.g., "word", "sentence")
    pub entity: String,
    /// Record ID of the favorited item
    pub record_id: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct CheckFavoritesRequest {
    /// Entity type to check
    pub entity: String,
    /// List of record IDs to check
    pub record_ids: Vec<i64>,
}

#[derive(Deserialize)]
pub struct ListFavoritesQuery {
    /// Filter by entity type (optional)
    pub entity: Option<String>,
    /// Limit number of results (default 50)
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
}

/// Favorite with entity details
#[derive(Debug, Serialize, ToSchema)]
pub struct FavoriteWithDetails {
    pub id: i64,
    pub user_id: i64,
    pub entity: String,
    pub record_id: i64,
    pub created_at: DateTime<Utc>,
    /// Word string (only present when entity is "word")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word: Option<String>,
    /// Chinese definition (only present when entity is "word")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_zh: Option<String>,
    /// Pronunciation ID for audio playback (only present when entity is "word" and audio exists)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronunciation_id: Option<i64>,
    /// English sentence (only present when entity is "sentence")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentence_en: Option<String>,
    /// Chinese sentence (only present when entity is "sentence")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentence_zh: Option<String>,
    /// Audio URL for sentence playback (only present when entity is "sentence" and audio exists)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_url: Option<String>,
}

/// List favorites for the authenticated user with entity details
#[handler]
pub async fn list_favorites(req: &mut Request, depot: &mut Depot) -> JsonResult<Vec<FavoriteWithDetails>> {
    let user_id = depot.user_id()?;

    let entity_filter = req.query::<String>("entity");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);
    let offset = req.query::<i64>("offset").unwrap_or(0);

    let favorites: Vec<FavoriteWithDetails> = with_conn(move |conn| {
        let mut query = learn_favorites::table
            .filter(learn_favorites::user_id.eq(user_id))
            .into_boxed();

        if let Some(ref entity) = entity_filter {
            query = query.filter(learn_favorites::entity.eq(entity));
        }

        let favorites: Vec<Favorite> = query
            .order(learn_favorites::created_at.desc())
            .limit(limit)
            .offset(offset)
            .load::<Favorite>(conn)?;

        // Fetch entity details based on entity type
        let mut result: Vec<FavoriteWithDetails> = Vec::with_capacity(favorites.len());

        if entity_filter.as_deref() == Some("word") && !favorites.is_empty() {
            // Fetch word details
            let word_ids: Vec<i64> = favorites.iter().map(|f| f.record_id).collect();

            let words: Vec<(i64, String)> = dict_words::table
                .filter(dict_words::id.eq_any(&word_ids))
                .select((dict_words::id, dict_words::word))
                .load::<(i64, String)>(conn)?;

            let word_map: std::collections::HashMap<i64, String> =
                words.into_iter().collect();

            // Fetch Chinese definitions (first one for each word)
            let definitions: Vec<(i64, String)> = dict_definitions::table
                .filter(dict_definitions::word_id.eq_any(&word_ids))
                .filter(dict_definitions::language.eq("zh"))
                .select((dict_definitions::word_id, dict_definitions::definition))
                .load::<(i64, String)>(conn)?;

            let def_map: std::collections::HashMap<i64, String> =
                definitions.into_iter().collect();

            // Fetch pronunciation IDs (primary or first one with audio)
            let pronunciations: Vec<(i64, i64)> = dict_pronunciations::table
                .filter(dict_pronunciations::word_id.eq_any(&word_ids))
                .filter(dict_pronunciations::audio_path.is_not_null())
                .select((dict_pronunciations::word_id, dict_pronunciations::id))
                .order(dict_pronunciations::is_primary.desc())
                .load::<(i64, i64)>(conn)?;

            // Keep only the first pronunciation per word
            let mut pron_map: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
            for (word_id, pron_id) in pronunciations {
                pron_map.entry(word_id).or_insert(pron_id);
            }

            for fav in favorites {
                result.push(FavoriteWithDetails {
                    id: fav.id,
                    user_id: fav.user_id,
                    entity: fav.entity,
                    record_id: fav.record_id,
                    created_at: fav.created_at,
                    word: word_map.get(&fav.record_id).cloned(),
                    word_zh: def_map.get(&fav.record_id).cloned(),
                    pronunciation_id: pron_map.get(&fav.record_id).copied(),
                    sentence_en: None,
                    sentence_zh: None,
                    audio_url: None,
                });
            }
        } else if entity_filter.as_deref() == Some("sentence") && !favorites.is_empty() {
            // Fetch sentence details from learn_chat_turns
            let turn_ids: Vec<i64> = favorites.iter().map(|f| f.record_id).collect();

            let turns: Vec<(i64, String, String, Option<String>)> = learn_chat_turns::table
                .filter(learn_chat_turns::id.eq_any(&turn_ids))
                .select((
                    learn_chat_turns::id,
                    learn_chat_turns::content_en,
                    learn_chat_turns::content_zh,
                    learn_chat_turns::audio_path,
                ))
                .load::<(i64, String, String, Option<String>)>(conn)?;

            let turn_map: std::collections::HashMap<i64, (String, String, Option<String>)> =
                turns.into_iter().map(|(id, en, zh, audio)| (id, (en, zh, audio))).collect();

            for fav in favorites {
                let (sentence_en, sentence_zh, audio_url) = turn_map
                    .get(&fav.record_id)
                    .map(|(en, zh, audio)| {
                        // Convert audio_path to API URL using turn ID endpoint
                        let url = audio.as_ref().map(|_| format!("/api/learn/chats/turns/{}/audio", fav.record_id));
                        (Some(en.clone()), Some(zh.clone()), url)
                    })
                    .unwrap_or((None, None, None));

                result.push(FavoriteWithDetails {
                    id: fav.id,
                    user_id: fav.user_id,
                    entity: fav.entity,
                    record_id: fav.record_id,
                    created_at: fav.created_at,
                    word: None,
                    word_zh: None,
                    pronunciation_id: None,
                    sentence_en,
                    sentence_zh,
                    audio_url,
                });
            }
        } else {
            // No specific entity filter or empty results
            for fav in favorites {
                result.push(FavoriteWithDetails {
                    id: fav.id,
                    user_id: fav.user_id,
                    entity: fav.entity,
                    record_id: fav.record_id,
                    created_at: fav.created_at,
                    word: None,
                    word_zh: None,
                    pronunciation_id: None,
                    sentence_en: None,
                    sentence_zh: None,
                    audio_url: None,
                });
            }
        }

        Ok(result)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch favorites"))?;

    json_ok(favorites)
}

/// Add a favorite
#[handler]
pub async fn create_favorite(req: &mut Request, depot: &mut Depot) -> JsonResult<Favorite> {
    let user_id = depot.user_id()?;

    let input: CreateFavoriteRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let entity = input.entity.trim().to_lowercase();
    if entity.is_empty() {
        return Err(StatusError::bad_request().brief("entity is required").into());
    }

    let record_id = input.record_id;

    // Check if already favorited, return existing if so
    let favorite: Favorite = with_conn(move |conn| {
        let existing = learn_favorites::table
            .filter(learn_favorites::user_id.eq(user_id))
            .filter(learn_favorites::entity.eq(&entity))
            .filter(learn_favorites::record_id.eq(record_id))
            .first::<Favorite>(conn)
            .optional()?;

        if let Some(fav) = existing {
            Ok(fav)
        } else {
            diesel::insert_into(learn_favorites::table)
                .values(&NewFavorite {
                    user_id,
                    entity,
                    record_id,
                })
                .get_result::<Favorite>(conn)
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create favorite"))?;

    json_ok(favorite)
}

/// Remove a favorite by entity and record_id
#[handler]
pub async fn delete_favorite(req: &mut Request, depot: &mut Depot) -> JsonResult<serde_json::Value> {
    let user_id = depot.user_id()?;

    let entity = req
        .query::<String>("entity")
        .ok_or_else(|| StatusError::bad_request().brief("entity query parameter required"))?;
    let record_id = req
        .query::<i64>("record_id")
        .ok_or_else(|| StatusError::bad_request().brief("record_id query parameter required"))?;

    let deleted = with_conn(move |conn| {
        diesel::delete(
            learn_favorites::table
                .filter(learn_favorites::user_id.eq(user_id))
                .filter(learn_favorites::entity.eq(&entity))
                .filter(learn_favorites::record_id.eq(record_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete favorite"))?;

    json_ok(serde_json::json!({ "deleted": deleted > 0 }))
}

/// Toggle a favorite (add if not exists, remove if exists)
#[handler]
pub async fn toggle_favorite(req: &mut Request, depot: &mut Depot) -> JsonResult<serde_json::Value> {
    let user_id = depot.user_id()?;

    let input: CreateFavoriteRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let entity = input.entity.trim().to_lowercase();
    if entity.is_empty() {
        return Err(StatusError::bad_request().brief("entity is required").into());
    }

    let record_id = input.record_id;

    let (is_favorited, favorite_id): (bool, Option<i64>) = with_conn(move |conn| {
        let existing = learn_favorites::table
            .filter(learn_favorites::user_id.eq(user_id))
            .filter(learn_favorites::entity.eq(&entity))
            .filter(learn_favorites::record_id.eq(record_id))
            .first::<Favorite>(conn)
            .optional()?;

        if let Some(fav) = existing {
            // Remove favorite
            diesel::delete(learn_favorites::table.filter(learn_favorites::id.eq(fav.id)))
                .execute(conn)?;
            Ok((false, None))
        } else {
            // Add favorite
            let new_fav: Favorite = diesel::insert_into(learn_favorites::table)
                .values(&NewFavorite {
                    user_id,
                    entity,
                    record_id,
                })
                .get_result(conn)?;
            Ok((true, Some(new_fav.id)))
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to toggle favorite"))?;

    json_ok(serde_json::json!({
        "is_favorited": is_favorited,
        "favorite_id": favorite_id
    }))
}

/// Check if items are favorited (batch check)
#[handler]
pub async fn check_favorites(req: &mut Request, depot: &mut Depot) -> JsonResult<Vec<i64>> {
    let user_id = depot.user_id()?;

    let input: CheckFavoritesRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let entity = input.entity.trim().to_lowercase();
    let record_ids = input.record_ids;

    if record_ids.is_empty() {
        return json_ok(vec![]);
    }

    let favorited_ids: Vec<i64> = with_conn(move |conn| {
        learn_favorites::table
            .filter(learn_favorites::user_id.eq(user_id))
            .filter(learn_favorites::entity.eq(&entity))
            .filter(learn_favorites::record_id.eq_any(&record_ids))
            .select(learn_favorites::record_id)
            .load::<i64>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to check favorites"))?;

    json_ok(favorited_ids)
}
