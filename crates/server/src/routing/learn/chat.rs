use std::path::PathBuf;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::Utc;
use chrono::format;
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::oapi::extract::JsonBody;
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
pub async fn list_chats(req: &mut Request, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
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
        return Err(StatusError::bad_request().brief("title is required").into());
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
    pause_count: Option<i32>,
    hesitation_count: Option<i32>,
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
    let chat_id = req.try_query::<i64>("id")?;
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
        let chat_id_param = chat_id_param;
        move |conn| {
            let mut query = learn_chat_turns::table
                .filter(learn_chat_turns::user_id.eq(user_id))
                .into_boxed();

            if let Some(cid) = chat_id_param {
                query = query.filter(learn_chat_turns::chat_id.eq(cid));
            }

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
            let chat_id_param = chat_id_param;
            move |conn| {
                let mut query = learn_chat_turns::table
                    .filter(learn_chat_turns::user_id.eq(user_id))
                    .filter(learn_chat_turns::id.lt(fid))
                    .into_boxed();

                if let Some(cid) = chat_id_param {
                    query = query.filter(learn_chat_turns::chat_id.eq(cid));
                }

                query.count().get_result::<i64>(conn).map(|c| c > 0)
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
            let chat_id_param = chat_id_param;
            move |conn| {
                let mut query = learn_chat_turns::table
                    .filter(learn_chat_turns::user_id.eq(user_id))
                    .filter(learn_chat_turns::id.gt(lid))
                    .into_boxed();

                if let Some(cid) = chat_id_param {
                    query = query.filter(learn_chat_turns::chat_id.eq(cid));
                }

                query.count().get_result::<i64>(conn).map(|c| c > 0)
            }
        })
        .await
        .unwrap_or(false)
    } else {
        false
    };

    let response = PaginatedResponse {
        items: turns,
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

/// Get a single chat turn by ID with long-polling support
///
/// If the turn status is "processing", the server will block for up to 30 seconds,
/// polling periodically until the turn is completed or timeout is reached.
#[endpoint(tags("Chat"))]
pub async fn get_chat_turn(req: &mut Request, depot: &mut Depot) -> JsonResult<ChatTurn> {
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
    json_ok(turn)
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
    /// User's chat turn (status: completed)
    pub user_turn: ChatTurn,
    /// AI's chat turn (status: processing, will be updated async)
    pub ai_turn: ChatTurn,
    /// Grammar/word choice issues found in user input
    pub issues: Vec<TextIssue>,
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

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a friendly English conversation partner helping users practice their English speaking skills.

CRITICAL: You MUST respond with a valid JSON object. NO OTHER TEXT ALLOWED.

STEPS:
1. Detect language: If Chinese set use_lang=\"zh\", if English set use_lang=\"en\", if mixed set use_lang=\"mix\"

2. *** MUST ANALYZE ENGLISH FOR ERRORS ***
   When user writes in English, you MUST check for grammar/vocabulary errors.
   For EACH error found, add to 'issues' array:
   {\"type\":\"grammar\",\"original\":\"wrong text\",\"suggested\":\"correct text\",\"description_en\":\"explanation\",\"description_zh\":\"中文解释\",\"severity\":\"medium\",\"start_position\":0,\"end_position\":0}

   Then add a suggestion with the full corrected sentence:
   {\"type\":\"suggestion\",\"original\":\"full original\",\"suggested\":\"full corrected\",\"description_en\":\"Better way to say this\",\"description_zh\":\"更好的表达\",\"severity\":\"low\",\"start_position\":0,\"end_position\":0}

3. Generate reply_en and reply_zh as natural conversation. Do NOT mention errors here - errors go in 'issues' only.

EXAMPLE for \"I go to school yesterday\":
{
  \"use_lang\":\"en\",
  \"original_en\":\"I go to school yesterday\",
  \"original_zh\":\"我昨天去学校了\",
  \"reply_en\":\"That sounds nice! What did you do there?\",
  \"reply_zh\":\"听起来不错！你在那里做了什么？\",
  \"issues\":[
    {\"type\":\"grammar\",\"original\":\"go\",\"suggested\":\"went\",\"description_en\":\"Use past tense with yesterday\",\"description_zh\":\"yesterday要用过去式\",\"severity\":\"medium\",\"start_position\":2,\"end_position\":4},
    {\"type\":\"suggestion\",\"original\":\"I go to school yesterday\",\"suggested\":\"I went to school yesterday\",\"description_en\":\"Corrected sentence\",\"description_zh\":\"修正后的句子\",\"severity\":\"low\",\"start_position\":0,\"end_position\":0}
  ]
}

JSON format:
{\"use_lang\":\"\",\"original_en\":\"\",\"original_zh\":\"\",\"reply_en\":\"\",\"reply_zh\":\"\",\"issues\":[]}"#;

// ============================================================================
// Database helper functions
// ============================================================================

/// Get or create active session for user
async fn get_or_create_session(user_id: i64) -> Result<ChatSession, StatusError> {
    with_conn(move |conn| {
        // Try to find the most recent session for this user
        let existing = learn_chats::table
            .filter(learn_chats::user_id.eq(user_id))
            .order(learn_chats::created_at.desc())
            .first::<ChatSession>(conn)
            .optional()?;

        if let Some(session) = existing {
            return Ok(session);
        }

        // Create new session
        let new_session = NewChatSession {
            user_id,
            title: "Chat Session".to_string(),
            context_id: None,
            duration_ms: None,
            pause_count: None,
        };

        diesel::insert_into(learn_chats::table)
            .values(&new_session)
            .get_result::<ChatSession>(conn)
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to get/create session: {:?}", e);
        StatusError::internal_server_error().brief("database error")
    })
}

/// Get chat history for a session (last 20 messages)
async fn get_chat_leatest_turns(
    chat_id: i64,
    max_count: i64,
) -> Result<Vec<HistoryMessage>, StatusError> {
    with_conn(move |conn| {
        let messages = learn_chat_turns::table
            .filter(learn_chat_turns::chat_id.eq(chat_id))
            .order(learn_chat_turns::created_at.asc())
            .limit(max_count)
            .load::<DbChatMessage>(conn)?;

        Ok(messages.into_iter().map(Into::into).collect())
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to get history: {:?}", e);
        StatusError::internal_server_error().brief("database error")
    })
}

/// Parameters for saving a chat message
struct SaveMessageParams {
    user_id: i64,
    chat_id: i64,
    speaker: String,
    use_lang: String,
    content_en: String,
    content_zh: String,
    audio_path: Option<String>,
}

/// Save a single message to database and return the created turn
async fn save_message(params: SaveMessageParams, status: &str) -> Result<ChatTurn, StatusError> {
    let status = status.to_string();
    with_conn(move |conn| {
        diesel::insert_into(learn_chat_turns::table)
            .values(&NewChatTurn {
                user_id: params.user_id,
                chat_id: params.chat_id,
                speaker: params.speaker,
                use_lang: params.use_lang,
                content_en: params.content_en,
                content_zh: params.content_zh,
                audio_path: params.audio_path,
                duration_ms: None,
                words_per_minute: None,
                pause_count: None,
                hesitation_count: None,
                status,
            })
            .get_result::<ChatTurn>(conn)
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to save message: {:?}", e);
        StatusError::internal_server_error().brief("database error")
    })
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
pub async fn send_chat(req: &mut Request, depot: &mut Depot) -> JsonResult<ChatSendResponse> {
    let user_id = depot.user_id()?;

    // Read body with larger size limit for audio (50MB)
    let body_bytes = req
        .payload_with_max_size(50 * 1024 * 1024)
        .await
        .map_err(|e| {
            tracing::error!("send_chat: failed to read body: {:?}", e);
            StatusError::bad_request().brief("failed to read body")
        })?;

    // Parse JSON
    let input: ChatSendRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::error!("send_chat: parse_json error: {:?}", e);
        StatusError::bad_request().brief("invalid json")
    })?;

    // Get AI provider early - needed for both ASR and user input analysis
    let provider = create_provider_from_env()
        .ok_or_else(|| StatusError::internal_server_error().brief("AI provider not configured"))?;

    // Get or create session
    let session = get_or_create_session(user_id).await?;
    let chat_id = session.id;

    // Process based on input type - transcribe audio if needed
    let (user_text, user_audio_data) = match &input {
        ChatSendRequest::Audio { audio_base64 } => {
            // Decode audio
            let audio_data = BASE64
                .decode(audio_base64)
                .map_err(|_| StatusError::bad_request().brief("invalid base64 audio"))?;

            if audio_data.is_empty() {
                return Err(StatusError::bad_request()
                    .brief("audio data is empty")
                    .into());
            }

            // Transcribe audio to text using ASR
            let asr = provider.asr().ok_or_else(|| {
                StatusError::internal_server_error().brief("ASR service not available")
            })?;

            tracing::info!("Calling {} ASR API...", provider.name());
            let asr_result = asr
                .transcribe(audio_data.clone(), Some("auto"))
                .await
                .map_err(|e: AiProviderError| {
                    tracing::error!("{} ASR error: {:?}", provider.name(), e);
                    StatusError::internal_server_error().brief(e.to_string())
                })?;
            tracing::info!("{} ASR API completed: {}", provider.name(), asr_result.text);

            if asr_result.text.trim().is_empty() {
                return Err(StatusError::bad_request()
                    .brief("Could not transcribe audio - no speech detected")
                    .into());
            }

            (asr_result.text, Some(audio_data))
        }
        ChatSendRequest::Text { message } => {
            if message.trim().is_empty() {
                return Err(StatusError::bad_request()
                    .brief("message is required")
                    .into());
            }

            (message.clone(), None)
        }
    };

    println!("======user_text: {}", user_text);

    // Save user's audio file if provided
    let user_audio_path = if let Some(audio_data) = &user_audio_data {
        save_audio_file(user_id, audio_data, "user", "wav").await
    } else {
        None
    };

    // Get chat service for structured output
    let chat_service = provider
        .chat_service()
        .ok_or_else(|| StatusError::internal_server_error().brief("Chat service not available"))?;

    // Get conversation history (last 100 messages for context)
    let history_messages = get_chat_leatest_turns(chat_id, 100)
        .await
        .unwrap_or_default();
    let history: Vec<ChatMessage> = history_messages
        .into_iter()
        .filter(|m| m.role != "assistant" || !m.content.is_empty())
        .map(|m| ChatMessage {
            role: m.role,
            content: m.content,
        })
        .collect();

    tracing::info!(
        "Calling {} chat_structured API with {} history messages...",
        provider.name(),
        history.len()
    );
    let structured_response = chat_service
        .chat_structured(history, &user_text, DEFAULT_SYSTEM_PROMPT)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to get structured response: {}, using defaults", e);
            // Fallback: assume English, use original text
            StructuredChatResponse {
                use_lang: "en".to_string(),
                original_en: user_text.clone(),
                original_zh: String::new(),
                reply_en: "I'm sorry, I couldn't process your input.".to_string(),
                reply_zh: "抱歉，我无法处理您的输入。".to_string(),
                issues: vec![],
            }
        });
    tracing::info!(
        "{} chat_structured completed: use_lang={}, reply_len={}",
        provider.name(),
        structured_response.use_lang,
        structured_response.reply_en.len()
    );

    println!("======structured_response: {:#?}", structured_response);
    // Create user turn with enriched info (status: completed)
    let user_turn = save_message(
        SaveMessageParams {
            user_id,
            chat_id,
            speaker: "user".to_string(),
            use_lang: structured_response.use_lang.clone(),
            content_en: structured_response.original_en.clone(),
            content_zh: structured_response.original_zh.clone(),
            audio_path: user_audio_path,
        },
        "completed",
    )
    .await?;

    // Save chat issues if any issues were found in user input
    if !structured_response.issues.is_empty() {
        tracing::info!(
            "Saving {} chat issues for user turn {}",
            structured_response.issues.len(),
            user_turn.id
        );

        let user_turn_id = user_turn.id;
        let issues: Vec<NewChatIssue> = structured_response
            .issues
            .iter()
            .map(|issue| NewChatIssue {
                user_id,
                chat_id,
                chat_turn_id: user_turn_id,
                issue_type: issue.issue_type.clone(),
                start_position: issue.start_position,
                end_position: issue.end_position,
                original_text: Some(issue.original.clone()),
                suggested_text: Some(issue.suggested.clone()),
                description_en: Some(issue.description_en.clone()),
                description_zh: Some(issue.description_zh.clone()),
                severity: Some(issue.severity.clone()),
            })
            .collect();

        if let Err(e) = with_conn(move |conn| {
            diesel::insert_into(learn_chat_issues::table)
                .values(&issues)
                .execute(conn)
        })
        .await
        {
            tracing::error!("Failed to save chat issues: {:?}", e);
        } else {
            tracing::info!(
                "Saved {} issues successfully",
                structured_response.issues.len()
            );
        }
    }

    // Spawn background task for AI response generation
    tracing::info!(
        "Background: {} chat_structured API completed: reply_en_len={}",
        provider.name(),
        structured_response.reply_en.len()
    );

    // Generate TTS for AI response if needed
    let ai_audio_path = if let Some(tts) = provider.tts() {
        tracing::info!(
            "Generating TTS for AI response ({} chars)...",
            structured_response.reply_en.len()
        );
        // Use English voice for English response text
        match tts
            .synthesize(
                &structured_response.reply_en,
                Some("zh_female_vv_uranus_bigtts"),
                None,
            )
            .await
        {
            Ok(tts_response) => {
                tracing::info!(
                    "Background: TTS succeeded, saving {} bytes audio...",
                    tts_response.audio_data.len()
                );
                save_audio_file(user_id, &tts_response.audio_data, "ai", "mp3").await
            }
            Err(e) => {
                tracing::error!("Background: AI TTS failed: {}", e);
                None
            }
        }
    } else {
        tracing::warn!("Background: TTS service not available");
        None
    };

    let ai_turn = save_message(
        SaveMessageParams {
            user_id,
            chat_id,
            speaker: "assistant".to_string(),
            use_lang: "en".to_owned(),
            content_en: structured_response.reply_en.clone(),
            content_zh: structured_response.reply_zh.clone(),
            audio_path: ai_audio_path,
        },
        "completed",
    )
    .await?;

    json_ok(ChatSendResponse {
        user_turn,
        ai_turn,
        issues: structured_response.issues.clone(),
    })
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
