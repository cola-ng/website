use std::path::PathBuf;
use std::time::Duration;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use chrono::Utc;
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::oapi::extract::JsonBody;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::services::{AiProviderError, ChatMessage, UserInputAnalysis, create_provider_from_env};
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
#[handler]
pub async fn list_chat_turns(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let chat_id_param = req.query::<i64>("chat_id");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 500);
    let after_id = req.query::<i64>("after_id");
    let before_id = req.query::<i64>("before_id");

    // Get total count for this query
    let total: i64 = with_conn({
        let chat_id_param = chat_id_param;
        move |conn| {
            let mut query = learn_chat_turns::table
                .filter(learn_chat_turns::user_id.eq(user_id))
                .into_boxed();

            if let Some(cid) = chat_id_param {
                query = query.filter(learn_chat_turns::chat_id.eq(cid));
            }

            query.count().get_result::<i64>(conn)
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to count chat turns"))?;

    // Get turns with cursor-based pagination
    // Order by created_at ASC (oldest first) so messages display correctly in chat
    let turns: Vec<ChatTurn> = with_conn({
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

            query
                .order(learn_chat_turns::created_at.asc())
                .limit(limit)
                .load::<ChatTurn>(conn)
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list chat turns"))?;

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

    // Long-polling settings
    const MAX_WAIT_SECS: u64 = 30;
    const POLL_INTERVAL_MS: u64 = 500;

    let start_time = std::time::Instant::now();
    let max_wait = Duration::from_secs(MAX_WAIT_SECS);
    let poll_interval = Duration::from_millis(POLL_INTERVAL_MS);

    loop {
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

        // If not processing, return immediately
        if turn.status != "processing" {
            return json_ok(turn);
        }

        // Check if we've exceeded the max wait time
        if start_time.elapsed() >= max_wait {
            // Return the current state (still processing)
            return json_ok(turn);
        }

        // Wait before next poll
        tokio::time::sleep(poll_interval).await;
    }
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

/// Text issue (grammar, word choice, or suggestion)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TextIssue {
    /// Type of issue: grammar | word_choice | suggestion
    #[serde(rename = "type")]
    pub issue_type: String,
    /// Original problematic text
    pub original: String,
    /// Suggested correction
    pub suggested: String,
    /// Explanation in English
    pub description_en: String,
    /// Explanation in Chinese
    pub description_zh: String,
    /// Severity: low | medium | high
    pub severity: String,
    /// Start position in text (optional)
    #[serde(default)]
    pub start_position: Option<i32>,
    /// End position in text (optional)
    #[serde(default)]
    pub end_position: Option<i32>,
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

Guidelines:
1. Respond naturally in English, as if having a casual conversation
2. If the user makes grammar or vocabulary mistakes, gently correct them
3. Keep responses concise (2-3 sentences) to encourage back-and-forth dialogue
4. Be encouraging and supportive
5. If the user speaks in Chinese, respond in English but acknowledge what they said
6. Occasionally ask follow-up questions to keep the conversation going

Remember: Your goal is to help users improve their spoken English through natural conversation practice."#;

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
async fn get_session_history(chat_id: i64) -> Result<Vec<HistoryMessage>, StatusError> {
    with_conn(move |conn| {
        let messages = learn_chat_turns::table
            .filter(learn_chat_turns::chat_id.eq(chat_id))
            .order(learn_chat_turns::created_at.asc())
            .limit(20)
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
async fn save_audio_file(user_id: i64, audio_data: &[u8], prefix: &str) -> Option<String> {
    let audio_dir = get_audio_dir(user_id);

    // Create directory if it doesn't exist
    if let Err(e) = tokio::fs::create_dir_all(&audio_dir).await {
        tracing::error!("Failed to create audio directory: {:?}", e);
        return None;
    }

    // Generate unique filename with timestamp
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let filename = format!("{}_{}.wav", prefix, timestamp);
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
    content_en: String,
    content_zh: String,
    audio_path: Option<String>,
    status: String,
    error: Option<String>,
) -> Result<(), StatusError> {
    with_conn(move |conn| {
        diesel::update(learn_chat_turns::table.find(turn_id))
            .set((
                learn_chat_turns::content_en.eq(content_en),
                learn_chat_turns::content_zh.eq(content_zh),
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
    let provider = create_provider_from_env().ok_or_else(|| {
        StatusError::internal_server_error().brief("AI provider not configured")
    })?;

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

    // Save user's audio file if provided
    let user_audio_path = if let Some(audio_data) = &user_audio_data {
        save_audio_file(user_id, audio_data, "user").await
    } else {
        None
    };

    // Call AI service to analyze user input (language detection, translation)
    let chat_service = provider.chat_service().ok_or_else(|| {
        StatusError::internal_server_error().brief("Chat service not available")
    })?;

    tracing::info!("Calling {} analyze_user_input API...", provider.name());
    let user_analysis = chat_service
        .analyze_user_input(&user_text)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to analyze user input: {}, using defaults", e);
            // Fallback: assume English, use original text
            UserInputAnalysis {
                use_lang: "en".to_string(),
                content_en: user_text.clone(),
                content_zh: String::new(),
            }
        });
    tracing::info!(
        "{} analyze_user_input completed: use_lang={}",
        provider.name(),
        user_analysis.use_lang
    );

    println!("======user_audio_path: {:?}", user_audio_path);
    // Create user turn with enriched info (status: completed)
    let user_turn = save_message(
        SaveMessageParams {
            user_id,
            chat_id,
            speaker: "user".to_string(),
            use_lang: user_analysis.use_lang,
            content_en: user_analysis.content_en.clone(),
            content_zh: user_analysis.content_zh,
            audio_path: user_audio_path,
        },
        "completed",
    )
    .await?;

    // Create AI turn placeholder (status: processing)
    let ai_turn = save_message(
        SaveMessageParams {
            user_id,
            chat_id,
            speaker: "assistant".to_string(),
            use_lang: "en".to_string(),
            content_en: String::new(),
            content_zh: String::new(),
            audio_path: None,
        },
        "processing",
    )
    .await?;

    // Spawn background task for AI response generation
    let ai_turn_id = ai_turn.id;
    let user_content_en = user_analysis.content_en;
    tokio::spawn(async move {
        // Get AI provider
        let provider = match create_provider_from_env() {
            Some(p) => p,
            None => {
                tracing::error!("AI provider not configured for background task");
                let _ = update_ai_turn(
                    ai_turn_id,
                    String::new(),
                    String::new(),
                    None,
                    "error".to_string(),
                    Some("AI provider not configured".to_string()),
                )
                .await;
                return;
            }
        };

        // Get history from database
        let history_messages = match get_session_history(chat_id).await {
            Ok(msgs) => msgs,
            Err(e) => {
                tracing::error!("Failed to get history: {:?}", e);
                let _ = update_ai_turn(
                    ai_turn_id,
                    String::new(),
                    String::new(),
                    None,
                    "error".to_string(),
                    Some("Failed to get chat history".to_string()),
                )
                .await;
                return;
            }
        };

        let history: Vec<ChatMessage> = history_messages
            .into_iter()
            .filter(|m| m.role != "assistant" || !m.content.is_empty()) // Skip empty AI turns
            .map(|m| ChatMessage {
                role: m.role,
                content: m.content,
            })
            .collect();

        // Get chat service for structured output
        let chat_service = match provider.chat_service() {
            Some(s) => s,
            None => {
                tracing::error!("Chat service not available");
                let _ = update_ai_turn(
                    ai_turn_id,
                    String::new(),
                    String::new(),
                    None,
                    "error".to_string(),
                    Some("Chat service not available".to_string()),
                )
                .await;
                return;
            }
        };

        // Generate AI response with structured output
        tracing::info!(
            "Background: Calling {} chat_structured API...",
            provider.name()
        );
        let structured_response = match chat_service
            .chat_structured(history, &user_content_en, DEFAULT_SYSTEM_PROMPT)
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("{} chat_structured error: {:?}", provider.name(), e);
                let _ = update_ai_turn(
                    ai_turn_id,
                    String::new(),
                    String::new(),
                    None,
                    "error".to_string(),
                    Some(format!("AI error: {}", e)),
                )
                .await;
                return;
            }
        };

        tracing::info!(
            "Background: {} chat_structured API completed: reply_en_len={}",
            provider.name(),
            structured_response.reply_en.len()
        );

        // Generate TTS for AI response if needed
        let ai_audio_path = if let Some(tts) = provider.tts() {
            tracing::info!(
                "Background: Generating TTS for AI response ({} chars)...",
                structured_response.reply_en.len()
            );
            // Use English voice for English response text
            match tts
                .synthesize(
                    &structured_response.reply_en,
                    Some("en_female_amanda_moon_bigtts"),
                    None,
                )
                .await
            {
                Ok(tts_response) => {
                    tracing::info!(
                        "Background: TTS succeeded, saving {} bytes audio...",
                        tts_response.audio_data.len()
                    );
                    save_audio_file(user_id, &tts_response.audio_data, "ai").await
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

        // Update AI turn with response
        tracing::info!(
            "Background: Updating AI turn {} with audio_path={:?}",
            ai_turn_id,
            ai_audio_path
        );
        if let Err(e) = update_ai_turn(
            ai_turn_id,
            structured_response.reply_en,
            structured_response.reply_zh,
            ai_audio_path.clone(),
            "completed".to_string(),
            None,
        )
        .await
        {
            tracing::error!("Failed to update AI turn: {:?}", e);
        } else {
            tracing::info!("Background: AI turn {} updated successfully", ai_turn_id);
        }
    });

    json_ok(ChatSendResponse { user_turn, ai_turn })
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

/// Clear chat history
#[endpoint(tags("Chat"))]
pub async fn clear_session(depot: &mut Depot) -> JsonResult<OkResponse> {
    let user_id = depot.user_id()?;
    clear_user_session(user_id).await?;
    json_ok(OkResponse::default())
}

/// Get chat history
#[endpoint(tags("Chat"))]
pub async fn get_history(depot: &mut Depot) -> JsonResult<HistoryResponse> {
    let user_id = depot.user_id()?;

    let session = get_or_create_session(user_id).await?;
    let messages = get_session_history(session.id).await?;

    json_ok(HistoryResponse { messages })
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

// ============================================================================
// Audio File Serving API
// ============================================================================

/// Serve audio file for a user
///
/// Users can only access their own audio files.
/// Path: /learn/audios/{user_id}/{filename}
#[handler]
pub async fn serve_audio(req: &mut Request, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
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
        return Err(StatusError::bad_request()
            .brief("invalid filename")
            .into());
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
    res.headers_mut()
        .insert(salvo::http::header::CONTENT_TYPE, content_type.parse().unwrap());
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
