use chrono::Utc;
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{AppResult, DepotExt};

#[derive(Deserialize, ToSchema)]
pub struct CreateVocabularyRequest {
    word: String,
    word_zh: Option<String>,
}

#[handler]
pub async fn list_vocabulary(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let due_only = req.query::<bool>("due_only").unwrap_or(false);
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let vocab: Vec<UserVocabulary> = with_conn(move |conn| {
        let mut query = learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .order(learn_vocabularies::first_seen_at.desc())
            .limit(limit)
            .into_boxed();

        if due_only {
            let now = Utc::now();
            query = query.filter(
                learn_vocabularies::next_review_at
                    .is_null()
                    .or(learn_vocabularies::next_review_at.le(now)),
            );
        }

        query.load::<UserVocabulary>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list vocabulary"))?;

    res.render(Json(vocab));
    Ok(())
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

    let new_vocab = NewUserVocabulary {
        user_id,
        word: input.word,
        word_zh: input.word_zh,
    };

    let vocab: UserVocabulary = with_conn(move |conn| {
        diesel::insert_into(learn_vocabularies::table)
            .values(&new_vocab)
            .get_result::<UserVocabulary>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create vocabulary"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(vocab));
    Ok(())
}
