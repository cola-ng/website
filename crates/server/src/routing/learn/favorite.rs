use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::Pronunciation;
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

/// Favorite word with details
#[derive(Debug, Serialize, ToSchema)]
pub struct FavoriteWord {
    pub id: i64,
    pub user_id: i64,
    pub record_id: i64,
    pub created_at: DateTime<Utc>,
    pub word: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_zh: Option<String>,
    pub pronunciations: Vec<Pronunciation>,
}

/// Favorite sentence with details
#[derive(Debug, Serialize, ToSchema)]
pub struct FavoriteSentence {
    pub id: i64,
    pub user_id: i64,
    pub record_id: i64,
    pub created_at: DateTime<Utc>,
    pub sentence_en: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentence_zh: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_url: Option<String>,
}

/// Favorite translation with details
#[derive(Debug, Serialize, ToSchema)]
pub struct FavoriteTranslation {
    pub id: i64,
    pub user_id: i64,
    pub record_id: i64,
    pub created_at: DateTime<Utc>,
    pub source_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub tone: String,
}

/// Favorite with entity details (discriminated union)
#[derive(Debug, Serialize, ToSchema)]
#[serde(tag = "entity", rename_all = "snake_case")]
pub enum FavoriteWithDetails {
    Word(FavoriteWord),
    Sentence(FavoriteSentence),
    Translation(FavoriteTranslation),
}

/// List favorites for the authenticated user with entity details
#[handler]
pub async fn list_favorites(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<Vec<FavoriteWithDetails>> {
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

            let word_map: std::collections::HashMap<i64, String> = words.into_iter().collect();

            // Fetch Chinese definitions (first one for each word)
            let definitions: Vec<(i64, String)> = dict_definitions::table
                .filter(dict_definitions::word_id.eq_any(&word_ids))
                .filter(dict_definitions::language.eq("zh"))
                .select((dict_definitions::word_id, dict_definitions::definition))
                .load::<(i64, String)>(conn)?;

            let def_map: std::collections::HashMap<i64, String> = definitions.into_iter().collect();

            // Fetch all pronunciations with audio for these words
            let pronunciations: Vec<Pronunciation> = dict_pronunciations::table
                .filter(dict_pronunciations::word_id.eq_any(&word_ids))
                .filter(dict_pronunciations::audio_path.is_not_null())
                .order((
                    dict_pronunciations::word_id,
                    dict_pronunciations::is_primary.desc(),
                ))
                .load::<Pronunciation>(conn)?;

            // Group pronunciations by word_id
            let mut pron_map: std::collections::HashMap<i64, Vec<Pronunciation>> =
                std::collections::HashMap::new();
            for pron in pronunciations {
                pron_map.entry(pron.word_id).or_default().push(pron);
            }

            for fav in favorites {
                if let Some(word) = word_map.get(&fav.record_id).cloned() {
                    result.push(FavoriteWithDetails::Word(FavoriteWord {
                        id: fav.id,
                        user_id: fav.user_id,
                        record_id: fav.record_id,
                        created_at: fav.created_at,
                        word,
                        word_zh: def_map.get(&fav.record_id).cloned(),
                        pronunciations: pron_map.remove(&fav.record_id).unwrap_or_default(),
                    }));
                }
            }
        } else if entity_filter.as_deref() == Some("translation") && !favorites.is_empty() {
            // Fetch translation details
            let translation_ids: Vec<i64> = favorites.iter().map(|f| f.record_id).collect();

            let translations: Vec<Translation> = learn_translations::table
                .filter(learn_translations::id.eq_any(&translation_ids))
                .load::<Translation>(conn)?;

            let translation_map: std::collections::HashMap<i64, Translation> =
                translations.into_iter().map(|t| (t.id, t)).collect();

            for fav in favorites {
                if let Some(t) = translation_map.get(&fav.record_id) {
                    result.push(FavoriteWithDetails::Translation(FavoriteTranslation {
                        id: fav.id,
                        user_id: fav.user_id,
                        record_id: fav.record_id,
                        created_at: fav.created_at,
                        source_text: t.source_text.clone(),
                        translated_text: t.translated_text.clone(),
                        source_lang: t.source_lang.clone(),
                        target_lang: t.target_lang.clone(),
                        tone: t.tone.clone(),
                    }));
                }
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

            let turn_map: std::collections::HashMap<i64, (String, String, Option<String>)> = turns
                .into_iter()
                .map(|(id, en, zh, audio)| (id, (en, zh, audio)))
                .collect();

            for fav in favorites {
                if let Some((sentence_en, sentence_zh, audio_path)) = turn_map.get(&fav.record_id) {
                    let audio_url = audio_path
                        .as_ref()
                        .map(|_| format!("/api/learn/chats/turns/{}/audio", fav.record_id));
                    result.push(FavoriteWithDetails::Sentence(FavoriteSentence {
                        id: fav.id,
                        user_id: fav.user_id,
                        record_id: fav.record_id,
                        created_at: fav.created_at,
                        sentence_en: sentence_en.clone(),
                        sentence_zh: Some(sentence_zh.clone()),
                        audio_url,
                    }));
                }
            }
        } else if !favorites.is_empty() {
            // No specific entity filter - need to fetch details for all types
            let word_favs: Vec<&Favorite> =
                favorites.iter().filter(|f| f.entity == "word").collect();
            let sentence_favs: Vec<&Favorite> = favorites
                .iter()
                .filter(|f| f.entity == "sentence")
                .collect();
            let translation_favs: Vec<&Favorite> = favorites
                .iter()
                .filter(|f| f.entity == "translation")
                .collect();

            // Fetch word details
            let mut word_map: std::collections::HashMap<i64, String> =
                std::collections::HashMap::new();
            let mut def_map: std::collections::HashMap<i64, String> =
                std::collections::HashMap::new();
            let mut pron_map: std::collections::HashMap<i64, Vec<Pronunciation>> =
                std::collections::HashMap::new();

            if !word_favs.is_empty() {
                let word_ids: Vec<i64> = word_favs.iter().map(|f| f.record_id).collect();

                let words: Vec<(i64, String)> = dict_words::table
                    .filter(dict_words::id.eq_any(&word_ids))
                    .select((dict_words::id, dict_words::word))
                    .load::<(i64, String)>(conn)?;
                word_map = words.into_iter().collect();

                let definitions: Vec<(i64, String)> = dict_definitions::table
                    .filter(dict_definitions::word_id.eq_any(&word_ids))
                    .filter(dict_definitions::language.eq("zh"))
                    .select((dict_definitions::word_id, dict_definitions::definition))
                    .load::<(i64, String)>(conn)?;
                def_map = definitions.into_iter().collect();

                let pronunciations: Vec<Pronunciation> = dict_pronunciations::table
                    .filter(dict_pronunciations::word_id.eq_any(&word_ids))
                    .filter(dict_pronunciations::audio_path.is_not_null())
                    .order((
                        dict_pronunciations::word_id,
                        dict_pronunciations::is_primary.desc(),
                    ))
                    .load::<Pronunciation>(conn)?;

                for pron in pronunciations {
                    pron_map.entry(pron.word_id).or_default().push(pron);
                }
            }

            // Fetch translation details
            let mut trans_map: std::collections::HashMap<i64, Translation> =
                std::collections::HashMap::new();

            if !translation_favs.is_empty() {
                let translation_ids: Vec<i64> =
                    translation_favs.iter().map(|f| f.record_id).collect();

                let translations: Vec<Translation> = learn_translations::table
                    .filter(learn_translations::id.eq_any(&translation_ids))
                    .load::<Translation>(conn)?;

                trans_map = translations.into_iter().map(|t| (t.id, t)).collect();
            }

            // Fetch sentence details
            let mut turn_map: std::collections::HashMap<i64, (String, String, Option<String>)> =
                std::collections::HashMap::new();

            if !sentence_favs.is_empty() {
                let turn_ids: Vec<i64> = sentence_favs.iter().map(|f| f.record_id).collect();

                let turns: Vec<(i64, String, String, Option<String>)> = learn_chat_turns::table
                    .filter(learn_chat_turns::id.eq_any(&turn_ids))
                    .select((
                        learn_chat_turns::id,
                        learn_chat_turns::content_en,
                        learn_chat_turns::content_zh,
                        learn_chat_turns::audio_path,
                    ))
                    .load::<(i64, String, String, Option<String>)>(conn)?;

                turn_map = turns
                    .into_iter()
                    .map(|(id, en, zh, audio)| (id, (en, zh, audio)))
                    .collect();
            }

            // Build results maintaining original order
            for fav in favorites {
                match fav.entity.as_str() {
                    "word" => {
                        if let Some(word) = word_map.get(&fav.record_id).cloned() {
                            result.push(FavoriteWithDetails::Word(FavoriteWord {
                                id: fav.id,
                                user_id: fav.user_id,
                                record_id: fav.record_id,
                                created_at: fav.created_at,
                                word,
                                word_zh: def_map.get(&fav.record_id).cloned(),
                                pronunciations: pron_map.remove(&fav.record_id).unwrap_or_default(),
                            }));
                        }
                    }
                    "sentence" => {
                        if let Some((sentence_en, sentence_zh, audio_path)) =
                            turn_map.get(&fav.record_id)
                        {
                            let audio_url = audio_path
                                .as_ref()
                                .map(|_| format!("/api/learn/chats/turns/{}/audio", fav.record_id));
                            result.push(FavoriteWithDetails::Sentence(FavoriteSentence {
                                id: fav.id,
                                user_id: fav.user_id,
                                record_id: fav.record_id,
                                created_at: fav.created_at,
                                sentence_en: sentence_en.clone(),
                                sentence_zh: Some(sentence_zh.clone()),
                                audio_url,
                            }));
                        }
                    }
                    "translation" => {
                        if let Some(t) = trans_map.get(&fav.record_id) {
                            result.push(FavoriteWithDetails::Translation(FavoriteTranslation {
                                id: fav.id,
                                user_id: fav.user_id,
                                record_id: fav.record_id,
                                created_at: fav.created_at,
                                source_text: t.source_text.clone(),
                                translated_text: t.translated_text.clone(),
                                source_lang: t.source_lang.clone(),
                                target_lang: t.target_lang.clone(),
                                tone: t.tone.clone(),
                            }));
                        }
                    }
                    _ => {} // Skip unknown entity types
                }
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
        return Err(StatusError::bad_request()
            .brief("entity is required")
            .into());
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
pub async fn delete_favorite(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<serde_json::Value> {
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
pub async fn toggle_favorite(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<serde_json::Value> {
    let user_id = depot.user_id()?;

    let input: CreateFavoriteRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let entity = input.entity.trim().to_lowercase();
    if entity.is_empty() {
        return Err(StatusError::bad_request()
            .brief("entity is required")
            .into());
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
