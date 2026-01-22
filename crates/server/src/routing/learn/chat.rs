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
    .await
    .map_err(|e| match e {
        diesel::result::Error::NotFound => {
            StatusError::not_found().brief("chat not found")
        }
        _ => StatusError::internal_server_error().brief("failed to update chat"),
    })?;

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

#[handler]
pub async fn create_chat_turn(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateChatTurnRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let new_turn = NewChatTurn {
        user_id,
        chat_id: input.chat_id,
        speaker: input.speaker,
        use_lang: input.use_lang,
        content_en: input.content_en,
        content_zh: input.content_zh,
        audio_path: input.audio_path,
        duration_ms: input.duration_ms,
        words_per_minute: input.words_per_minute,
        pause_count: input.pause_count,
        hesitation_count: input.hesitation_count,
    };

    let turn: ChatTurn = with_conn(move |conn| {
        diesel::insert_into(learn_chat_turns::table)
            .values(&new_turn)
            .get_result::<ChatTurn>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create chat turn"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(turn));
    Ok(())
}

// ============================================================================
// Chat Annotations API
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct CreateChatAnnotationRequest {
    chat_id: i64,
    chat_turn_id: i64,
    annotation_type: String,
    start_position: Option<i32>,
    end_position: Option<i32>,
    original_text: Option<String>,
    suggested_text: Option<String>,
    description_en: Option<String>,
    description_zh: Option<String>,
    severity: Option<String>,
}

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

#[handler]
pub async fn create_chat_annotation(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateChatAnnotationRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let new_annotation = NewChatAnnotation {
        user_id,
        chat_id: input.chat_id,
        chat_turn_id: input.chat_turn_id,
        annotation_type: input.annotation_type,
        start_position: input.start_position,
        end_position: input.end_position,
        original_text: input.original_text,
        suggested_text: input.suggested_text,
        description_en: input.description_en,
        description_zh: input.description_zh,
        severity: input.severity,
    };

    let annotation: ChatAnnotation = with_conn(move |conn| {
        diesel::insert_into(learn_chat_annotations::table)
            .values(&new_annotation)
            .get_result::<ChatAnnotation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create annotation"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(annotation));
    Ok(())
}
