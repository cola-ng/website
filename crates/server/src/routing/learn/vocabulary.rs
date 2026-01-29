use chrono::Utc;
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{AppResult, DepotExt, JsonResult, json_ok};

#[derive(Deserialize, ToSchema)]
pub struct CreateVocabularyRequest {
    word: String,
    word_zh: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct PaginatedVocabulary {
    pub items: Vec<UserVocabulary>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[endpoint(tags("Learn"))]
pub async fn list_vocabulary(req: &mut Request, depot: &mut Depot) -> JsonResult<PaginatedVocabulary> {
    let user_id = depot.user_id()?;
    let due_only = req.query::<bool>("due_only").unwrap_or(false);
    let page = req.query::<i64>("page").unwrap_or(1).max(1);
    let per_page = req.query::<i64>("per_page").unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let (items, total): (Vec<UserVocabulary>, i64) = with_conn(move |conn| {
        let mut query = learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .into_boxed();

        let mut count_query = learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .into_boxed();

        if due_only {
            let now = Utc::now();
            query = query.filter(
                learn_vocabularies::next_review_at
                    .is_null()
                    .or(learn_vocabularies::next_review_at.le(now)),
            );
            count_query = count_query.filter(
                learn_vocabularies::next_review_at
                    .is_null()
                    .or(learn_vocabularies::next_review_at.le(now)),
            );
        }

        let total: i64 = count_query.count().get_result(conn)?;

        let items = query
            .order(learn_vocabularies::first_seen_at.desc())
            .offset(offset)
            .limit(per_page)
            .load::<UserVocabulary>(conn)?;

        Ok((items, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list vocabulary"))?;

    json_ok(PaginatedVocabulary {
        items,
        total,
        page,
        per_page,
    })
}

#[handler]
pub async fn create_vocabulary(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateVocabularyRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.word.trim().is_empty() {
        return Err(StatusError::bad_request().brief("word is required").into());
    }

    let word = input.word.trim().to_lowercase();
    let word_zh = input.word_zh;

    let new_vocab = NewUserVocabulary {
        user_id,
        word: word.clone(),
        word_zh: word_zh.clone(),
    };

    // Use upsert pattern: if word already exists for this user, just return the existing one
    let vocab: UserVocabulary = with_conn(move |conn| {
        diesel::insert_into(learn_vocabularies::table)
            .values(&new_vocab)
            .on_conflict((learn_vocabularies::user_id, learn_vocabularies::word))
            .do_nothing()
            .execute(conn)?;

        // Fetch the vocabulary (either newly created or existing)
        learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .filter(learn_vocabularies::word.eq(&word))
            .first::<UserVocabulary>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create vocabulary"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(vocab));
    Ok(())
}

/// Toggle response
#[derive(Serialize, ToSchema)]
pub struct ToggleVocabularyResponse {
    pub word: String,
    pub added: bool,
}

/// Toggle a word in vocabulary (add if not exists, remove if exists)
#[endpoint(tags("Learn"))]
pub async fn toggle_vocabulary(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<ToggleVocabularyResponse> {
    let user_id = depot.user_id()?;
    let input: CreateVocabularyRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.word.trim().is_empty() {
        return Err(StatusError::bad_request().brief("word is required").into());
    }

    let word = input.word.trim().to_lowercase();

    let (added, _) = with_conn(move |conn| {
        // Check if word already exists
        let existing: Option<UserVocabulary> = learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .filter(learn_vocabularies::word.eq(&word))
            .first::<UserVocabulary>(conn)
            .optional()?;

        if existing.is_some() {
            // Word exists, remove it
            diesel::delete(
                learn_vocabularies::table
                    .filter(learn_vocabularies::user_id.eq(user_id))
                    .filter(learn_vocabularies::word.eq(&word)),
            )
            .execute(conn)?;
            Ok::<_, diesel::result::Error>((false, word))
        } else {
            // Word doesn't exist, add it
            let new_vocab = NewUserVocabulary {
                user_id,
                word: word.clone(),
                word_zh: None,
            };
            diesel::insert_into(learn_vocabularies::table)
                .values(&new_vocab)
                .execute(conn)?;
            Ok((true, word))
        }
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to toggle vocabulary: {:?}", e);
        StatusError::internal_server_error().brief("failed to toggle vocabulary")
    })?;

    json_ok(ToggleVocabularyResponse {
        word: input.word.trim().to_lowercase(),
        added,
    })
}
