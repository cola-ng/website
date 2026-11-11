//! Translation favorite endpoints: save translation + toggle favorite

use horme::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{DepotExt, JsonResult, json_ok};

#[derive(Debug, Deserialize, ToSchema)]
pub struct ToggleTranslationFavoriteRequest {
    /// Original text
    pub source_text: String,
    /// Translated text
    pub translated_text: String,
    /// Source language (e.g. "en", "zh")
    pub source_lang: String,
    /// Target language (e.g. "en", "zh")
    pub target_lang: String,
    /// Translation tone (e.g. "original", "casual", "business")
    #[serde(default = "default_tone")]
    pub tone: String,
}

fn default_tone() -> String {
    "original".to_string()
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ToggleTranslationFavoriteResponse {
    pub is_favorited: bool,
    pub translation_id: Option<i64>,
    pub favorite_id: Option<i64>,
}

/// Toggle translation favorite: upsert translation record, then toggle favorite
#[endpoint(tags("Dictionary"))]
pub async fn toggle_translation_favorite(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<ToggleTranslationFavoriteResponse> {
    let user_id = depot.user_id()?;

    let input: ToggleTranslationFavoriteRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.source_text.trim().is_empty() || input.translated_text.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("source_text and translated_text are required")
            .into());
    }

    let source_text = input.source_text.trim().to_string();
    let translated_text = input.translated_text.trim().to_string();
    let source_lang = input.source_lang.trim().to_lowercase();
    let target_lang = input.target_lang.trim().to_lowercase();
    let tone = input.tone.trim().to_lowercase();

    let result: ToggleTranslationFavoriteResponse = with_conn!(move |conn| {
        // Upsert translation record
        let translation = learn_translations::table
            .filter(learn_translations::user_id.eq(user_id))
            .filter(learn_translations::source_text.eq(&source_text))
            .filter(learn_translations::target_lang.eq(&target_lang))
            .filter(learn_translations::tone.eq(&tone))
            .first::<Translation>(conn)
            .await
            .optional()?;

        let translation = if let Some(t) = translation {
            t
        } else {
            diesel::insert_into(learn_translations::table)
                .values(&NewTranslation {
                    user_id,
                    source_text,
                    translated_text,
                    source_lang,
                    target_lang,
                    tone,
                })
                .get_result::<Translation>(conn)
                .await?
        };

        let translation_id = translation.id;

        // Toggle favorite
        let existing_fav = learn_favorites::table
            .filter(learn_favorites::user_id.eq(user_id))
            .filter(learn_favorites::entity.eq("translation"))
            .filter(learn_favorites::record_id.eq(translation_id))
            .first::<Favorite>(conn)
            .await
            .optional()?;

        if let Some(fav) = existing_fav {
            // Remove favorite
            diesel::delete(learn_favorites::table.filter(learn_favorites::id.eq(fav.id)))
                .execute(conn)
                .await?;
            Ok(ToggleTranslationFavoriteResponse {
                is_favorited: false,
                translation_id: Some(translation_id),
                favorite_id: None,
            })
        } else {
            // Add favorite
            let new_fav: Favorite = diesel::insert_into(learn_favorites::table)
                .values(&NewFavorite {
                    user_id,
                    entity: "translation".to_string(),
                    record_id: translation_id,
                })
                .get_result(conn)
                .await?;
            Ok(ToggleTranslationFavoriteResponse {
                is_favorited: true,
                translation_id: Some(translation_id),
                favorite_id: Some(new_fav.id),
            })
        }
    })
    .await
    .map_err(|_| {
        StatusError::internal_server_error().brief("failed to toggle translation favorite")
    })?;

    json_ok(result)
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CheckTranslationFavoriteQuery {
    pub source_text: String,
    pub target_lang: String,
    #[serde(default = "default_tone")]
    pub tone: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CheckTranslationFavoriteResponse {
    pub is_favorited: bool,
}

/// Check if a translation is favorited
#[endpoint(tags("Dictionary"))]
pub async fn check_translation_favorite(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<CheckTranslationFavoriteResponse> {
    let user_id = depot.user_id()?;

    let source_text = req
        .query::<String>("source_text")
        .ok_or_else(|| StatusError::bad_request().brief("source_text query parameter required"))?;
    let target_lang = req
        .query::<String>("target_lang")
        .ok_or_else(|| StatusError::bad_request().brief("target_lang query parameter required"))?;
    let tone = req
        .query::<String>("tone")
        .unwrap_or_else(|| "original".to_string());

    let is_favorited: bool = with_conn!(move |conn| {
        // Find translation record
        let translation = learn_translations::table
            .filter(learn_translations::user_id.eq(user_id))
            .filter(learn_translations::source_text.eq(&source_text))
            .filter(learn_translations::target_lang.eq(&target_lang))
            .filter(learn_translations::tone.eq(&tone))
            .first::<Translation>(conn)
            .await
            .optional()?;

        if let Some(t) = translation {
            // Check if favorited
            let fav = learn_favorites::table
                .filter(learn_favorites::user_id.eq(user_id))
                .filter(learn_favorites::entity.eq("translation"))
                .filter(learn_favorites::record_id.eq(t.id))
                .first::<Favorite>(conn)
                .await
                .optional()?;
            Ok(fav.is_some())
        } else {
            Ok(false)
        }
    })
    .await
    .map_err(|_| {
        StatusError::internal_server_error().brief("failed to check translation favorite")
    })?;

    json_ok(CheckTranslationFavoriteResponse { is_favorited })
}
