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
        /// Whether to generate audio response (default: true)
        #[serde(default = "default_true")]
        generate_audio: bool,
    },
}

fn default_true() -> bool {
    true
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

/// Response for voice/text chat
#[derive(Debug, Serialize, ToSchema)]
pub struct ChatResponse {
    /// Language of user input: "en" | "zh" | "mix"
    pub use_lang: String,
    /// User text in English (original or transcribed)
    pub user_text_en: String,
    /// User text in Chinese
    pub user_text_zh: String,
    /// AI's text response in English
    pub ai_text_en: String,
    /// AI's text response in Chinese
    pub ai_text_zh: String,
    /// Base64 encoded audio of user's message (if audio input or TTS generated)
    pub user_audio_base64: Option<String>,
    /// Base64 encoded audio of AI response
    pub ai_audio_base64: Option<String>,
    /// Grammar/word choice issues found in user's text
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
async fn get_session_history(chat_id: String) -> Result<Vec<HistoryMessage>, StatusError> {
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
    chat_id: String,
    speaker: String,
    use_lang: String,
    content_en: String,
    content_zh: String,
    audio_path: Option<String>,
}

/// Save a single message to database
async fn save_message(params: SaveMessageParams) -> Result<(), StatusError> {
    with_conn(move |conn| {
        diesel::insert_into(learn_chat_turns::table)
            .values(&NewChatMessage {
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
            })
            .execute(conn)?;
        Ok(())
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to save message: {:?}", e);
        StatusError::internal_server_error().brief("database error")
    })
}

/// Get the audio storage directory for a user
fn get_audio_dir(user_id: i64) -> PathBuf {
    let base = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "uploads".to_string());
    PathBuf::from(base)
        .join("chat_audio")
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
    let relative_path = format!("chat_audio/{}/{}", user_id, filename);
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

/// Send audio or text and receive AI response with audio
///
/// Accepts either audio or text input:
/// - Audio input: Transcribed to text using ASR, audio file is saved
/// - Text input: Optionally converted to audio using TTS
///
/// Uses structured output from AI (Doubao) to get bilingual response and grammar analysis
#[endpoint(tags("Chat"))]
pub async fn chat_send(req: &mut Request, depot: &mut Depot) -> JsonResult<ChatResponse> {
    let user_id = depot.user_id()?;

    // Read body with larger size limit for audio (50MB)
    let body_bytes = req
        .payload_with_max_size(50 * 1024 * 1024)
        .await
        .map_err(|e| {
            tracing::error!("chat_send: failed to read body: {:?}", e);
            StatusError::bad_request().brief("failed to read body")
        })?;

    // Parse JSON
    let input: ChatSendRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::error!("chat_send: parse_json error: {:?}", e);
        StatusError::bad_request().brief("invalid json")
    })?;

    // Get AI provider
    let provider = create_provider_from_env()
        .ok_or_else(|| StatusError::internal_server_error().brief("AI provider not configured"))?;

    // Get or create session
    let session = get_or_create_session(user_id).await?;
    let chat_id = format!("chat_{}", session.id);

    // Get history from database
    let history_messages = get_session_history(chat_id.clone()).await?;
    let history: Vec<ChatMessage> = history_messages
        .into_iter()
        .map(|m| ChatMessage {
            role: m.role,
            content: m.content,
        })
        .collect();

    // Process based on input type
    let (user_text, user_audio_data, generate_audio) = match &input {
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

            (
                asr_result.text,
                Some(audio_data),
                true, // Always generate audio for audio input
            )
        }
        ChatSendRequest::Text {
            message,
            generate_audio,
        } => {
            if message.trim().is_empty() {
                return Err(StatusError::bad_request()
                    .brief("message is required")
                    .into());
            }

            (message.clone(), None, *generate_audio)
        }
    };

    // Get chat service for structured output
    let chat_service = provider
        .chat_service()
        .ok_or_else(|| StatusError::internal_server_error().brief("Chat service not available"))?;

    // Generate AI response with structured output
    tracing::info!("Calling {} chat_structured API...", provider.name());
    let structured_response = chat_service
        .chat_structured(history, &user_text, DEFAULT_SYSTEM_PROMPT)
        .await
        .map_err(|e: AiProviderError| {
            tracing::error!("{} chat_structured error: {:?}", provider.name(), e);
            StatusError::internal_server_error().brief(e.to_string())
        })?;
    tracing::info!(
        "{} chat_structured API completed: use_lang={}, reply_en_len={}, issues={}",
        provider.name(),
        structured_response.use_lang,
        structured_response.reply_en.len(),
        structured_response.issues.len()
    );

    // Save user's audio file if provided
    let user_audio_path = if let Some(audio_data) = &user_audio_data {
        save_audio_file(user_id, audio_data, "user").await
    } else {
        None
    };

    // Generate TTS for user's text input if no audio was provided
    let (user_audio_base64, user_tts_audio_data) = if user_audio_data.is_some() {
        // User provided audio, encode it for response
        (user_audio_data.as_ref().map(|d| BASE64.encode(d)), None)
    } else if generate_audio {
        // Generate TTS for user's text
        if let Some(tts) = provider.tts() {
            tracing::info!("Generating TTS for user text...");
            match tts.synthesize(&user_text, None, None).await {
                Ok(tts_response) => {
                    let audio_base64 = BASE64.encode(&tts_response.audio_data);
                    (Some(audio_base64), Some(tts_response.audio_data))
                }
                Err(e) => {
                    tracing::warn!("User TTS failed: {}", e);
                    (None, None)
                }
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // Save user's TTS audio if generated
    let final_user_audio_path = if user_audio_path.is_some() {
        user_audio_path
    } else if let Some(tts_audio) = &user_tts_audio_data {
        save_audio_file(user_id, tts_audio, "user_tts").await
    } else {
        None
    };

    // Generate TTS for AI response
    let (ai_audio_base64, ai_audio_data) = if generate_audio {
        if let Some(tts) = provider.tts() {
            tracing::info!("Calling {} TTS API for AI response...", provider.name());
            match tts
                .synthesize(&structured_response.reply_en, None, None)
                .await
            {
                Ok(tts_response) => {
                    tracing::info!("{} TTS API completed successfully", provider.name());
                    (
                        Some(BASE64.encode(&tts_response.audio_data)),
                        Some(tts_response.audio_data),
                    )
                }
                Err(e) => {
                    tracing::warn!("AI TTS failed: {}", e);
                    (None, None)
                }
            }
        } else {
            tracing::warn!("TTS not available for provider {}", provider.name());
            (None, None)
        }
    } else {
        (None, None)
    };

    // Save AI's audio file if generated
    let ai_audio_path = if let Some(audio_data) = &ai_audio_data {
        save_audio_file(user_id, audio_data, "ai").await
    } else {
        None
    };

    // Save user message to database
    save_message(SaveMessageParams {
        user_id,
        chat_id: chat_id.clone(),
        speaker: "user".to_string(),
        use_lang: structured_response.use_lang.clone(),
        content_en: structured_response.original_en.clone(),
        content_zh: structured_response.original_zh.clone(),
        audio_path: final_user_audio_path,
    })
    .await?;

    // Save AI message to database
    save_message(SaveMessageParams {
        user_id,
        chat_id,
        speaker: "assistant".to_string(),
        use_lang: "en".to_string(),
        content_en: structured_response.reply_en.clone(),
        content_zh: structured_response.reply_zh.clone(),
        audio_path: ai_audio_path,
    })
    .await?;

    // Convert issues to response format
    let issues: Vec<TextIssue> = structured_response
        .issues
        .into_iter()
        .map(|issue| TextIssue {
            issue_type: issue.issue_type,
            original: issue.original,
            suggested: issue.suggested,
            description_en: issue.description_en,
            description_zh: issue.description_zh,
            severity: issue.severity,
            start_position: issue.start_position,
            end_position: issue.end_position,
        })
        .collect();

    json_ok(ChatResponse {
        use_lang: structured_response.use_lang,
        user_text_en: structured_response.original_en,
        user_text_zh: structured_response.original_zh,
        ai_text_en: structured_response.reply_en,
        ai_text_zh: structured_response.reply_zh,
        user_audio_base64,
        ai_audio_base64,
        issues,
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
    let chat_id = format!("chat_{}", session.id);
    let messages = get_session_history(chat_id).await?;

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
