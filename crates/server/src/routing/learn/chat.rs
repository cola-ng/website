use std::path::PathBuf;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{Utc, format};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::{Chat, ChatIssue, ChatTurn, NewChat, NewChatIssue, NewChatTurn};
use crate::services::{
    AiProviderError, ChatMessage, StructuredChatResponse, TextIssue, create_provider_from_env,
};
use crate::{AppResult, DepotExt, JsonResult, OkResponse, json_ok};

mod streaming;
pub(super) use streaming::send_stream;

// Type aliases for backward compatibility
type ChatSession = Chat;
type NewChatSession = NewChat;
type DbChatMessage = ChatTurn;

/// History message for API response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

impl From<ChatTurn> for HistoryMessage {
    fn from(msg: ChatTurn) -> Self {
        HistoryMessage {
            role: msg.speaker,
            content: msg.content_en,
        }
    }
}

/// Chat with context icon for list response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatWithIcon {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub context_id: Option<i64>,
    pub icon_emoji: Option<String>,
    pub duration_ms: Option<i32>,
    pub issues_count: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateChatRequest {
    title: String,
    context_id: Option<i64>,
    duration_ms: Option<i32>,
    issues_count: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateChatRequest {
    title: Option<String>,
}

#[handler]
pub async fn list_chats(req: &mut Request, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let chats: Vec<ChatWithIcon> = with_conn(move |conn| {
        learn_chats::table
            .left_join(
                asset_contexts::table.on(learn_chats::context_id.eq(asset_contexts::id.nullable())),
            )
            .filter(learn_chats::user_id.eq(user_id))
            .order(learn_chats::created_at.desc())
            .limit(limit)
            .select((
                learn_chats::id,
                learn_chats::user_id,
                learn_chats::title,
                learn_chats::context_id,
                asset_contexts::icon_emoji.nullable(),
                learn_chats::duration_ms,
                learn_chats::issues_count,
                learn_chats::created_at,
            ))
            .load::<(
                i64,
                i64,
                String,
                Option<i64>,
                Option<String>,
                Option<i32>,
                Option<i32>,
                chrono::DateTime<chrono::Utc>,
            )>(conn)
            .map(|rows| {
                rows.into_iter()
                    .map(
                        |(
                            id,
                            user_id,
                            title,
                            context_id,
                            icon_emoji,
                            duration_ms,
                            issues_count,
                            created_at,
                        )| {
                            ChatWithIcon {
                                id,
                                user_id,
                                title,
                                context_id,
                                icon_emoji,
                                duration_ms,
                                issues_count,
                                created_at,
                            }
                        },
                    )
                    .collect()
            })
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
        return Err(StatusError::bad_request().brief("title is required").into());
    }

    let new_chat = NewChat {
        user_id,
        title: input.title,
        context_id: input.context_id,
        duration_ms: input.duration_ms,
        issues_count: input.issues_count,
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

/// Reset a chat - delete all turns and issues for this chat
#[handler]
pub async fn reset_chat(req: &mut Request, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let chat_id = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing chat id"))?;

    // Verify ownership and delete all turns and issues for this chat
    with_conn(move |conn| {
        // First verify ownership
        let _existing = learn_chats::table
            .filter(learn_chats::id.eq(chat_id))
            .filter(learn_chats::user_id.eq(user_id))
            .first::<Chat>(conn)?;

        // Delete all issues for this chat
        diesel::delete(
            learn_chat_issues::table
                .filter(learn_chat_issues::chat_id.eq(chat_id))
                .filter(learn_chat_issues::user_id.eq(user_id)),
        )
        .execute(conn)?;

        // Delete all turns for this chat
        diesel::delete(
            learn_chat_turns::table
                .filter(learn_chat_turns::chat_id.eq(chat_id))
                .filter(learn_chat_turns::user_id.eq(user_id)),
        )
        .execute(conn)?;

        Ok::<_, diesel::result::Error>(())
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to reset chat: {:?}", e);
        StatusError::internal_server_error().brief("failed to reset chat")
    })?;

    res.render(Json(serde_json::json!({ "ok": true })));
    Ok(())
}

// ============================================================================
// Chat Turns API
// ============================================================================

/// Paginated response for list endpoints using cursor-based pagination
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T> {
    /// The items in this page
    pub items: Vec<T>,
    /// Total count of items matching the query
    pub total: i64,
    /// Number of items requested
    pub limit: i64,
    /// Whether there are more items before this page
    pub has_prev: bool,
    /// Whether there are more items after this page
    pub has_next: bool,
    /// ID of the first item in this page (use as `before_id` to load previous page)
    pub first_id: Option<i64>,
    /// ID of the last item in this page (use as `after_id` to load next page)
    pub last_id: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateChatTurnRequest {
    chat_id: i64,
    speaker: String,
    use_lang: String,
    content_en: String,
    content_zh: String,
    audio_path: Option<String>,
    duration_ms: Option<i32>,
    words_per_minute: Option<f32>,
    issues_count: Option<i32>,
    hesitation_count: Option<i32>,
}

/// Chat turn with embedded issues - response structure for turn APIs
#[derive(Debug, Serialize, ToSchema)]
pub struct ChatTurnWithIssues {
    pub id: i64,
    pub user_id: i64,
    pub chat_id: i64,
    pub speaker: String,
    pub use_lang: String,
    pub content_en: String,
    pub content_zh: String,
    pub audio_path: Option<String>,
    pub duration_ms: Option<i32>,
    pub words_per_minute: Option<f32>,
    pub issues_count: i32,
    pub hesitation_count: Option<i32>,
    pub status: String,
    pub error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Associated issues for this turn (only present if issues_count > 0)
    pub issues: Vec<ChatIssue>,
}

impl ChatTurnWithIssues {
    /// Create from ChatTurn with empty issues
    pub fn from_turn(turn: ChatTurn) -> Self {
        Self {
            id: turn.id,
            user_id: turn.user_id,
            chat_id: turn.chat_id,
            speaker: turn.speaker,
            use_lang: turn.use_lang,
            content_en: turn.content_en,
            content_zh: turn.content_zh,
            audio_path: turn.audio_path,
            duration_ms: turn.duration_ms,
            words_per_minute: turn.words_per_minute,
            issues_count: turn.issues_count,
            hesitation_count: turn.hesitation_count,
            status: turn.status,
            error: turn.error,
            created_at: turn.created_at,
            issues: vec![],
        }
    }

    /// Create from ChatTurn with issues
    pub fn from_turn_with_issues(turn: ChatTurn, issues: Vec<ChatIssue>) -> Self {
        Self {
            id: turn.id,
            user_id: turn.user_id,
            chat_id: turn.chat_id,
            speaker: turn.speaker,
            use_lang: turn.use_lang,
            content_en: turn.content_en,
            content_zh: turn.content_zh,
            audio_path: turn.audio_path,
            duration_ms: turn.duration_ms,
            words_per_minute: turn.words_per_minute,
            issues_count: turn.issues_count,
            hesitation_count: turn.hesitation_count,
            status: turn.status,
            error: turn.error,
            created_at: turn.created_at,
            issues,
        }
    }
}

/// List chat turns with cursor-based pagination
///
/// Query parameters:
/// - `chat_id`: Filter by chat ID (required for /chats/{id}/turns, optional for /chats/turns)
/// - `limit`: Max items to return (default 50, max 500)
/// - `after_id`: Load items after this ID (for loading older messages)
/// - `before_id`: Load items before this ID (for loading newer messages)
/// - `from_latest`: If true, load the latest messages first (default false)
#[handler]
pub async fn list_turns(req: &mut Request, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let chat_id = req.try_param::<i64>("id")?;
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 500);
    let after_id = req.query::<i64>("after_id");
    let before_id = req.query::<i64>("before_id");
    let from_latest = req.query::<bool>("from_latest").unwrap_or(false);

    // Get total count for this query
    let total: i64 = with_conn({
        move |conn| {
            let mut query = learn_chat_turns::table
                .filter(learn_chat_turns::user_id.eq(user_id))
                .into_boxed();

            query = query.filter(learn_chat_turns::chat_id.eq(chat_id));
            query.count().get_result::<i64>(conn)
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to count chat turns"))?;

    // Get turns with cursor-based pagination
    // When from_latest is true, order by DESC to get newest first, then reverse for display
    // When from_latest is false, order by ASC (oldest first) for normal pagination
    let mut turns: Vec<ChatTurn> = with_conn({
        move |conn| {
            let mut query = learn_chat_turns::table
                .filter(learn_chat_turns::user_id.eq(user_id))
                .filter(learn_chat_turns::chat_id.eq(chat_id))
                .into_boxed();

            // Cursor-based filtering
            if let Some(aid) = after_id {
                // Load items with ID > after_id (older messages in chronological order)
                query = query.filter(learn_chat_turns::id.gt(aid));
            }
            if let Some(bid) = before_id {
                // Load items with ID < before_id (newer messages)
                query = query.filter(learn_chat_turns::id.lt(bid));
            }

            if from_latest {
                // Load latest messages first (for initial load)
                query
                    .order(learn_chat_turns::created_at.desc())
                    .limit(limit)
                    .load::<ChatTurn>(conn)
            } else {
                // Normal chronological order (for loading older messages)
                query
                    .order(learn_chat_turns::created_at.asc())
                    .limit(limit)
                    .load::<ChatTurn>(conn)
            }
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list chat turns"))?;

    // When from_latest, reverse to get chronological order for display
    if from_latest {
        turns.reverse();
    }

    // Determine if there are more items
    let first_id = turns.first().map(|t| t.id);
    let last_id = turns.last().map(|t| t.id);

    // Check if there are items before the first item
    let has_prev: bool = if let Some(fid) = first_id {
        with_conn({
            move |conn| {
                learn_chat_turns::table
                    .filter(learn_chat_turns::user_id.eq(user_id))
                    .filter(learn_chat_turns::id.lt(fid))
                    .filter(learn_chat_turns::chat_id.eq(chat_id))
                    .count()
                    .get_result::<i64>(conn)
                    .map(|c| c > 0)
            }
        })
        .await
        .unwrap_or(false)
    } else {
        false
    };

    // Check if there are items after the last item
    let has_next: bool = if let Some(lid) = last_id {
        with_conn({
            move |conn| {
                learn_chat_turns::table
                    .filter(learn_chat_turns::user_id.eq(user_id))
                    .filter(learn_chat_turns::id.gt(lid))
                    .filter(learn_chat_turns::chat_id.eq(chat_id))
                    .count()
                    .get_result::<i64>(conn)
                    .map(|c| c > 0)
            }
        })
        .await
        .unwrap_or(false)
    } else {
        false
    };

    // Fetch issues for turns that have issues_count > 0
    let turn_ids_with_issues: Vec<i64> = turns
        .iter()
        .filter(|t| t.issues_count > 0)
        .map(|t| t.id)
        .collect();

    let issues_by_turn: std::collections::HashMap<i64, Vec<ChatIssue>> =
        if turn_ids_with_issues.is_empty() {
            std::collections::HashMap::new()
        } else {
            let all_issues: Vec<ChatIssue> = with_conn({
                let turn_ids = turn_ids_with_issues.clone();
                move |conn| {
                    learn_chat_issues::table
                        .filter(learn_chat_issues::user_id.eq(user_id))
                        .filter(learn_chat_issues::chat_turn_id.eq_any(turn_ids))
                        .load::<ChatIssue>(conn)
                }
            })
            .await
            .unwrap_or_default();

            let mut map: std::collections::HashMap<i64, Vec<ChatIssue>> =
                std::collections::HashMap::new();
            for issue in all_issues {
                map.entry(issue.chat_turn_id).or_default().push(issue);
            }
            map
        };
    println!("issues_by_turn: {:?}", issues_by_turn);

    // Convert turns to ChatTurnWithIssues
    let items: Vec<ChatTurnWithIssues> = turns
        .into_iter()
        .map(|turn| {
            let issues = issues_by_turn.get(&turn.id).cloned().unwrap_or_default();
            ChatTurnWithIssues::from_turn_with_issues(turn, issues)
        })
        .collect();

    let response = PaginatedResponse {
        items,
        total,
        limit,
        has_prev,
        has_next,
        first_id,
        last_id,
    };

    res.render(Json(response));
    Ok(())
}

/// Delete a single chat turn by ID
///
/// Also deletes any associated issues for this turn.
#[handler]
pub async fn delete_turn(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let turn_id = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing turn id"))?;

    // Delete the turn and its associated issues
    with_conn(move |conn| {
        // First verify ownership
        let _existing = learn_chat_turns::table
            .filter(learn_chat_turns::id.eq(turn_id))
            .filter(learn_chat_turns::user_id.eq(user_id))
            .first::<ChatTurn>(conn)?;

        // Delete associated issues first
        diesel::delete(
            learn_chat_issues::table
                .filter(learn_chat_issues::chat_turn_id.eq(turn_id))
                .filter(learn_chat_issues::user_id.eq(user_id)),
        )
        .execute(conn)?;

        // Delete the turn
        diesel::delete(
            learn_chat_turns::table
                .filter(learn_chat_turns::id.eq(turn_id))
                .filter(learn_chat_turns::user_id.eq(user_id)),
        )
        .execute(conn)?;

        Ok::<_, diesel::result::Error>(())
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to delete chat turn: {:?}", e);
        StatusError::internal_server_error().brief("failed to delete chat turn")
    })?;

    res.render(Json(serde_json::json!({ "ok": true })));
    Ok(())
}

/// Get the current persisted snapshot of an owned chat turn.
#[endpoint(tags("Chat"))]
pub async fn get_turn(req: &mut Request, depot: &mut Depot) -> JsonResult<ChatTurnWithIssues> {
    let user_id = depot.user_id()?;
    let turn_id = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing turn id"))?;

    // Fetch the turn from database
    let turn: ChatTurn = with_conn(move |conn| {
        learn_chat_turns::table
            .filter(learn_chat_turns::id.eq(turn_id))
            .filter(learn_chat_turns::user_id.eq(user_id))
            .first::<ChatTurn>(conn)
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to get chat turn: {:?}", e);
        StatusError::not_found().brief("chat turn not found")
    })?;

    // Fetch issues if this turn has any
    let issues: Vec<ChatIssue> = if turn.issues_count > 0 {
        with_conn(move |conn| {
            learn_chat_issues::table
                .filter(learn_chat_issues::chat_turn_id.eq(turn_id))
                .filter(learn_chat_issues::user_id.eq(user_id))
                .load::<ChatIssue>(conn)
        })
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    json_ok(ChatTurnWithIssues::from_turn_with_issues(turn, issues))
}

/// Request for chat - supports both audio and text input
#[derive(Debug, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatSendRequest {
    /// Audio input - will be transcribed to text
    Audio {
        /// Base64 encoded audio data (WAV format)
        audio_base64: String,
    },
    /// Text input - will be converted to audio
    Text {
        /// User's text message
        message: String,
    },
}

/// Request for TTS only
#[derive(Debug, Deserialize, ToSchema)]
pub struct TtsRequest {
    /// Text to synthesize
    pub text: String,
    /// Voice option
    pub voice: Option<String>,
    /// Speed (0.5 - 2.0)
    pub speed: Option<f32>,
}

/// Response for voice/text chat - returns two chat turns
#[derive(Debug, Serialize, ToSchema)]
pub struct ChatSendResponse {
    /// User's chat turn with embedded issues (status: completed)
    pub user_turn: ChatTurnWithIssues,
    /// AI's chat turn (status: processing, will be updated async)
    pub ai_turn: ChatTurnWithIssues,
}

/// Response for TTS only
#[derive(Debug, Serialize, ToSchema)]
pub struct TtsResponse {
    /// Base64 encoded audio
    pub audio_base64: String,
}

/// Response for history
#[derive(Debug, Serialize, ToSchema)]
pub struct HistoryResponse {
    /// Chat messages history
    pub messages: Vec<HistoryMessage>,
}

// ============================================================================
// Database helper functions
// ============================================================================

/// Get chat history for a session (last 20 messages)
async fn get_chat_latest_turns(
    chat_id: i64,
    user_id: i64,
    before: i64,
    max_count: i64,
) -> Result<Vec<ChatMessage>, StatusError> {
    with_conn(move |conn| {
        let mut turns = learn_chat_turns::table
            .filter(learn_chat_turns::chat_id.eq(chat_id))
            .filter(learn_chat_turns::user_id.eq(user_id))
            .filter(learn_chat_turns::id.lt(before))
            .filter(learn_chat_turns::status.eq("completed"))
            .order(learn_chat_turns::id.desc())
            .limit(max_count)
            .load::<ChatTurn>(conn)?;
        turns.reverse();
        let mut messages: Vec<HistoryMessage> = turns.into_iter().map(Into::into).collect();
        while messages.len() > 1 && messages.iter().map(|m| m.content.len()).sum::<usize>() > 24000
        {
            messages.remove(0);
        }
        Ok(messages
            .into_iter()
            .map(|m| ChatMessage {
                role: m.role,
                content: m.content,
            })
            .collect())
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("无法读取对话历史"))
}

/// Get the audio storage directory for a user
fn get_audio_dir(user_id: i64) -> PathBuf {
    PathBuf::from(&AppConfig::get().space_path)
        .join("learn/audios")
        .join(user_id.to_string())
}

/// Save audio data to file and return the relative path
async fn save_audio_file(
    user_id: i64,
    audio_data: &[u8],
    prefix: &str,
    format: &str,
) -> Option<String> {
    let audio_dir: PathBuf = get_audio_dir(user_id);

    // Create directory if it doesn't exist
    if let Err(e) = tokio::fs::create_dir_all(&audio_dir).await {
        tracing::error!("Failed to create audio directory: {:?}", e);
        return None;
    }

    // Generate unique filename with timestamp
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let filename = format!("{}_{}.{}", prefix, timestamp, format);
    let file_path = audio_dir.join(&filename);

    // Write audio data to file
    if let Err(e) = tokio::fs::write(&file_path, audio_data).await {
        tracing::error!("Failed to save audio file: {:?}", e);
        return None;
    }

    // Return relative path for database storage
    let relative_path = format!("learn/audios/{}/{}", user_id, filename);
    tracing::info!("Saved audio file: {}", relative_path);
    Some(relative_path)
}

/// Clear session (delete all messages for this user's active chat)
async fn clear_user_session(user_id: i64) -> Result<(), StatusError> {
    with_conn(move |conn| {
        // Delete all chat turns for this user
        diesel::delete(learn_chat_turns::table.filter(learn_chat_turns::user_id.eq(user_id)))
            .execute(conn)?;

        // Delete all chat sessions for this user
        diesel::delete(learn_chats::table.filter(learn_chats::user_id.eq(user_id)))
            .execute(conn)?;

        Ok(())
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to clear session: {:?}", e);
        StatusError::internal_server_error().brief("database error")
    })
}

// ============================================================================
// API Handlers
// ============================================================================

/// Update AI turn with response content
async fn update_ai_turn(
    turn_id: i64,
    audio_path: Option<String>,
    status: String,
    error: Option<String>,
) -> Result<(), StatusError> {
    with_conn(move |conn| {
        diesel::update(learn_chat_turns::table.find(turn_id))
            .set((
                learn_chat_turns::audio_path.eq(audio_path),
                learn_chat_turns::status.eq(status),
                learn_chat_turns::error.eq(error),
            ))
            .execute(conn)?;
        Ok(())
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to update AI turn: {:?}", e);
        StatusError::internal_server_error().brief("database error")
    })
}

/// Send audio or text chat message
///
/// Returns two chat turns immediately:
/// - User turn (status: completed) - enriched with language detection and translation
/// - AI turn (status: processing) - will be updated async
///
/// Accepts either audio or text input:
/// - Audio input: Transcribed to text using ASR, audio file is saved
/// - Text input: Optionally converted to audio using TTS
#[endpoint(tags("Chat"))]
pub async fn send_chat(
    id: PathParam<i64>,
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<ChatSendResponse> {
    let user = depot.user_id()?;
    let chat = id.into_inner();
    let (input, provider) = streaming::prepare(req, user, chat).await?;
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    json_ok(streaming::conversation(user, chat, input, provider, &tx).await?)
}

/// Convert text to speech only
#[endpoint(tags("Chat"))]
pub async fn text_to_speech(
    input: JsonBody<TtsRequest>,
    depot: &mut Depot,
) -> JsonResult<TtsResponse> {
    let _user_id = depot.user_id()?;

    if input.text.trim().is_empty() {
        return Err(StatusError::bad_request().brief("text is required").into());
    }

    // Get AI provider
    let provider = create_provider_from_env()
        .ok_or_else(|| StatusError::internal_server_error().brief("AI provider not configured"))?;

    // Get TTS service
    let tts = provider
        .tts()
        .ok_or_else(|| StatusError::internal_server_error().brief("TTS service not available"))?;

    // Generate audio
    tracing::info!("Calling {} TTS API...", provider.name());
    let tts_response = tts
        .synthesize(&input.text, input.voice.as_deref(), input.speed)
        .await
        .map_err(|e: AiProviderError| {
            tracing::error!("{} TTS error: {:?}", provider.name(), e);
            StatusError::internal_server_error().brief(e.to_string())
        })?;
    tracing::info!("{} TTS API completed successfully", provider.name());

    let audio_base64 = BASE64.encode(&tts_response.audio_data);

    json_ok(TtsResponse { audio_base64 })
}

// ============================================================================
// Chat Issues API
// ============================================================================

/// List issues for a specific chat
///
/// Path: /learn/chats/{id}/issues
#[handler]
pub async fn list_chat_issues(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let chat_id = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing chat id"))?;

    // Verify the chat belongs to this user
    let chat_exists: bool = with_conn(move |conn| {
        learn_chats::table
            .filter(learn_chats::id.eq(chat_id))
            .filter(learn_chats::user_id.eq(user_id))
            .count()
            .get_result::<i64>(conn)
            .map(|c| c > 0)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("database error"))?;

    if !chat_exists {
        return Err(StatusError::not_found().brief("chat not found").into());
    }

    // Get all issues for this chat
    let issues: Vec<ChatIssue> = with_conn(move |conn| {
        learn_chat_issues::table
            .filter(learn_chat_issues::chat_id.eq(chat_id))
            .filter(learn_chat_issues::user_id.eq(user_id))
            .order(learn_chat_issues::created_at.asc())
            .load::<ChatIssue>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list issues"))?;

    res.render(Json(issues));
    Ok(())
}

/// List issues for a specific chat turn
///
/// Path: /learn/chats/turns/{id}/issues
#[handler]
pub async fn list_turn_issues(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let turn_id = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing turn id"))?;

    // Verify the turn belongs to this user
    let turn_exists: bool = with_conn(move |conn| {
        learn_chat_turns::table
            .filter(learn_chat_turns::id.eq(turn_id))
            .filter(learn_chat_turns::user_id.eq(user_id))
            .count()
            .get_result::<i64>(conn)
            .map(|c| c > 0)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("database error"))?;

    if !turn_exists {
        return Err(StatusError::not_found().brief("chat turn not found").into());
    }

    // Get issues for this turn
    let issues: Vec<ChatIssue> = with_conn(move |conn| {
        learn_chat_issues::table
            .filter(learn_chat_issues::chat_turn_id.eq(turn_id))
            .filter(learn_chat_issues::user_id.eq(user_id))
            .order(learn_chat_issues::created_at.asc())
            .load::<ChatIssue>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list issues"))?;

    res.render(Json(issues));
    Ok(())
}

// ============================================================================
// Audio File Serving API
// ============================================================================

/// Serve audio file for a chat turn
///
/// Path: /learn/chats/turns/{id}/audio
#[handler]
pub async fn serve_turn_audio(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let turn_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing turn id"))?;

    // Get the turn from database to find audio_path
    let turn: ChatTurn = with_conn(move |conn| {
        learn_chat_turns::table
            .find(turn_id)
            .filter(learn_chat_turns::user_id.eq(user_id))
            .first::<ChatTurn>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("turn not found"))?;

    // Check if audio_path exists
    let audio_path = turn
        .audio_path
        .ok_or_else(|| StatusError::not_found().brief("no audio available for this turn"))?;

    // Build the full file path
    let file_path = PathBuf::from(&AppConfig::get().space_path).join(&audio_path);

    // Check if file exists
    if !file_path.exists() {
        tracing::warn!("Turn audio file not found: {:?}", file_path);
        return Err(StatusError::not_found()
            .brief("audio file not found")
            .into());
    }

    res.send_file(file_path, req.headers()).await;

    Ok(())
}

/// Serve audio file for a user
///
/// Users can only access their own audio files.
/// Path: /learn/audios/{user_id}/{filename}
#[handler]
pub async fn serve_audio(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let auth_user_id = depot.user_id()?;

    // Get user_id from path parameter
    let path_user_id = req
        .param::<i64>("user_id")
        .ok_or_else(|| StatusError::bad_request().brief("missing user_id"))?;

    // Verify the authenticated user can only access their own files
    if auth_user_id != path_user_id {
        return Err(StatusError::forbidden()
            .brief("cannot access other user's audio files")
            .into());
    }

    // Get filename from path parameter
    let filename = req
        .param::<String>("filename")
        .ok_or_else(|| StatusError::bad_request().brief("missing filename"))?;

    // Sanitize filename to prevent directory traversal
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(StatusError::bad_request().brief("invalid filename").into());
    }

    // Build the file path
    let file_path = PathBuf::from(&AppConfig::get().space_path)
        .join("learn/audios")
        .join(path_user_id.to_string())
        .join(&filename);

    // Check if file exists
    if !file_path.exists() {
        tracing::warn!("Audio file not found: {:?}", file_path);
        return Err(StatusError::not_found()
            .brief("audio file not found")
            .into());
    }

    // Read the file
    let audio_data = tokio::fs::read(&file_path).await.map_err(|e| {
        tracing::error!("Failed to read audio file {:?}: {:?}", file_path, e);
        StatusError::internal_server_error().brief("failed to read audio file")
    })?;

    // Determine content type based on file extension
    let content_type = if filename.ends_with(".wav") {
        "audio/wav"
    } else if filename.ends_with(".mp3") {
        "audio/mpeg"
    } else {
        "application/octet-stream"
    };

    // Set response headers and body
    res.headers_mut().insert(
        salvo::http::header::CONTENT_TYPE,
        content_type.parse().unwrap(),
    );
    res.headers_mut().insert(
        salvo::http::header::CONTENT_LENGTH,
        audio_data.len().to_string().parse().unwrap(),
    );
    res.headers_mut().insert(
        salvo::http::header::CACHE_CONTROL,
        "private, max-age=3600".parse().unwrap(),
    );

    res.write_body(audio_data).ok();
    Ok(())
}
