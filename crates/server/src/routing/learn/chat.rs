use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{AppResult, DepotExt};

#[derive(Deserialize, ToSchema)]
pub struct CreateChatRequest {
    title: String,
    context_id: Option<i64>,
    duration_ms: Option<i32>,
    pause_count: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateChatRequest {
    title: Option<String>,
}

#[handler]
pub async fn list_chats(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let chats: Vec<Chat> = with_conn(move |conn| {
        learn_chats::table
            .filter(learn_chats::user_id.eq(user_id))
            .order(learn_chats::created_at.desc())
            .limit(limit)
            .load::<Chat>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list chats"))?;

    res.render(Json(chats));
    Ok(())
}

#[handler]
pub async fn create_chat(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateChatRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.title.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("title is required")
            .into());
    }

    let new_chat = NewChat {
        user_id,
        title: input.title,
        context_id: input.context_id,
        duration_ms: input.duration_ms,
        pause_count: input.pause_count,
    };

    let chat: Chat = with_conn(move |conn| {
        diesel::insert_into(learn_chats::table)
            .values(&new_chat)
            .get_result::<Chat>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create chat"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(chat));
    Ok(())
}

#[handler]
pub async fn update_chat(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let chat_id = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing chat id"))?;

    let input: UpdateChatRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if let Some(ref title) = input.title {
        if title.trim().is_empty() {
            return Err(StatusError::bad_request()
                .brief("title cannot be empty")
                .into());
        }
    }

    let chat: Chat = with_conn(move |conn| {
        // First verify ownership
        let existing = learn_chats::table
            .filter(learn_chats::id.eq(chat_id))
            .filter(learn_chats::user_id.eq(user_id))
            .first::<Chat>(conn)?;

        // Update if title provided
        if let Some(title) = input.title {
            diesel::update(learn_chats::table.find(existing.id))
                .set(learn_chats::title.eq(title))
                .get_result::<Chat>(conn)
        } else {
            Ok(existing)
        }
    })
    .await?;

    res.render(Json(chat));
    Ok(())
}

// ============================================================================
// Chat Turns API
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct CreateChatTurnRequest {
    chat_id: String,
    speaker: String,
    use_lang: String,
    content_en: String,
    content_zh: String,
    audio_path: Option<String>,
    duration_ms: Option<i32>,
    words_per_minute: Option<f32>,
    pause_count: Option<i32>,
    hesitation_count: Option<i32>,
}

#[handler]
pub async fn list_chat_turns(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let chat_id_param = req.query::<String>("chat_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let turns: Vec<ChatTurn> = with_conn(move |conn| {
        let mut query = learn_chat_turns::table
            .filter(learn_chat_turns::user_id.eq(user_id))
            .order(learn_chat_turns::created_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(cid) = chat_id_param {
            query = query.filter(learn_chat_turns::chat_id.eq(cid));
        }

        query.load::<ChatTurn>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list chat turns"))?;

    res.render(Json(turns));
    Ok(())
}
// ============================================================================
// Chat Annotations API
// ============================================================================

#[handler]
pub async fn list_chat_annotations(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let chat_id_param = req.query::<i64>("chat_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let annotations: Vec<ChatAnnotation> = with_conn(move |conn| {
        let mut query = learn_chat_annotations::table
            .filter(learn_chat_annotations::user_id.eq(user_id))
            .order(learn_chat_annotations::created_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(cid) = chat_id_param {
            query = query.filter(learn_chat_annotations::chat_id.eq(cid));
        }

        query.load::<ChatAnnotation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list annotations"))?;

    res.render(Json(annotations));
    Ok(())
}
