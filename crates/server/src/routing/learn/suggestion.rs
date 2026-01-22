use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{AppResult, DepotExt};

#[derive(Deserialize, ToSchema)]
pub struct CreateSuggestionRequest {
    suggestion_type: Option<String>,
    suggested_text: String,
    was_accepted: Option<bool>,
}

#[handler]
pub async fn list_suggestions(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let suggestion_type_param = req.query::<String>("type");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let suggestions: Vec<Suggestion> = with_conn(move |conn| {
        let mut query = learn_suggestions::table
            .filter(learn_suggestions::user_id.eq(user_id))
            .order(learn_suggestions::created_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(st) = suggestion_type_param {
            query = query.filter(learn_suggestions::suggestion_type.eq(st));
        }

        query.load::<Suggestion>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list suggestions"))?;

    res.render(Json(suggestions));
    Ok(())
}

#[handler]
pub async fn create_suggestion(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateSuggestionRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.suggested_text.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("suggested_text is required")
            .into());
    }

    let new_suggestion = NewSuggestion {
        user_id,
        suggestion_type: input.suggestion_type,
        suggested_text: input.suggested_text,
        was_accepted: input.was_accepted,
    };

    let suggestion: Suggestion = with_conn(move |conn| {
        diesel::insert_into(learn_suggestions::table)
            .values(&new_suggestion)
            .get_result::<Suggestion>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create suggestion"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(suggestion));
    Ok(())
}
