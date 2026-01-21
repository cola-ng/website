//! Chat API Routes
//!
//! Provides chat functionality using configurable AI providers (BigModel, Doubao):
//! - POST /api/chat/send: Send audio and receive AI response with audio
//! - POST /api/chat/text-send: Send text and receive AI response with optional audio
//! - POST /api/chat/tts: Text to speech only
//! - POST /api/chat/clear: Clear chat history
//! - GET /api/chat/history: Get chat history

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use diesel::prelude::*;
use salvo::oapi::extract::JsonBody;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::{learn_chat_messages, learn_chat_sessions};
use crate::db::with_conn;
use crate::models::{
    ChatMessage as DbChatMessage, ChatSession, HistoryMessage, NewChatMessage, NewChatSession,
};
use crate::services::{create_provider_from_env, AiProviderError, ChatMessage};
use crate::{hoops, AppResult, DepotExt, JsonResult, OkResponse, json_ok};

pub fn router() -> Router {
    Router::with_path("chat")
        .hoop(hoops::require_auth)
        .push(Router::with_path("send").post(chat_send))
        .push(Router::with_path("text-send").post(text_chat_send))
        .push(Router::with_path("tts").post(text_to_speech))
        .push(Router::with_path("clear").post(clear_session))
        .push(Router::with_path("history").get(get_history))
}

/// Request for voice chat - audio input
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChatSendRequest {
    /// Base64 encoded audio data (WAV format)
    pub audio_base64: String,
    /// Optional custom system prompt
    pub system_prompt: Option<String>,
}

/// Request for text chat - text input with TTS response
#[derive(Debug, Deserialize, ToSchema)]
pub struct TextChatRequest {
    /// User's text message
    pub message: String,
    /// Optional custom system prompt
    pub system_prompt: Option<String>,
    /// Whether to generate audio response
    #[serde(default = "default_true")]
    pub generate_audio: bool,
}

fn default_true() -> bool {
    true
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
    /// Transcribed user text (for voice input)
    pub user_text: Option<String>,
    /// AI's text response
    pub ai_text: String,
    /// AI's text response in Chinese (if available)
    pub ai_text_zh: Option<String>,
    /// Base64 encoded audio of AI response
    pub ai_audio_base64: Option<String>,
    /// Any corrections or suggestions
    pub corrections: Vec<Correction>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Correction {
    /// Original text with error
    pub original: String,
    /// Corrected text
    pub corrected: String,
    /// Explanation of the correction
    pub explanation: String,
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
        // Try to find active session
        let existing = learn_chat_sessions::table
            .filter(learn_chat_sessions::user_id.eq(user_id))
            .filter(learn_chat_sessions::is_active.eq(true))
            .first::<ChatSession>(conn)
            .optional()?;

        if let Some(session) = existing {
            return Ok(session);
        }

        // Create new session
        let new_session = NewChatSession {
            user_id,
            title: None,
            system_prompt: None,
        };

        diesel::insert_into(learn_chat_sessions::table)
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
async fn get_session_history(session_id: i64) -> Result<Vec<HistoryMessage>, StatusError> {
    with_conn(move |conn| {
        let messages = learn_chat_messages::table
            .filter(learn_chat_messages::session_id.eq(session_id))
            .order(learn_chat_messages::created_at.asc())
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

/// Save messages to database
async fn save_messages(
    session_id: i64,
    user_id: i64,
    user_message: &str,
    ai_message: &str,
) -> Result<(), StatusError> {
    let user_msg = user_message.to_string();
    let ai_msg = ai_message.to_string();

    with_conn(move |conn| {
        // Insert user message
        diesel::insert_into(learn_chat_messages::table)
            .values(&NewChatMessage {
                session_id,
                user_id,
                role: "user".to_string(),
                content: user_msg,
                audio_base64: None,
            })
            .execute(conn)?;

        // Insert assistant message
        diesel::insert_into(learn_chat_messages::table)
            .values(&NewChatMessage {
                session_id,
                user_id,
                role: "assistant".to_string(),
                content: ai_msg,
                audio_base64: None,
            })
            .execute(conn)?;

        // Update session message count and timestamp
        diesel::update(learn_chat_sessions::table.find(session_id))
            .set((
                learn_chat_sessions::message_count
                    .eq(learn_chat_sessions::message_count + 2),
                learn_chat_sessions::updated_at.eq(Utc::now()),
            ))
            .execute(conn)?;

        Ok(())
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to save messages: {:?}", e);
        StatusError::internal_server_error().brief("database error")
    })
}

/// Clear session (mark as inactive and create new one)
async fn clear_user_session(user_id: i64) -> Result<(), StatusError> {
    with_conn(move |conn| {
        diesel::update(
            learn_chat_sessions::table
                .filter(learn_chat_sessions::user_id.eq(user_id))
                .filter(learn_chat_sessions::is_active.eq(true)),
        )
        .set((
            learn_chat_sessions::is_active.eq(false),
            learn_chat_sessions::updated_at.eq(Utc::now()),
        ))
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

/// Send audio and receive AI response with audio
#[endpoint(tags("Chat"))]
pub async fn chat_send(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<ChatResponse> {
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

    // Decode audio
    let audio_data = BASE64
        .decode(&input.audio_base64)
        .map_err(|_| StatusError::bad_request().brief("invalid base64 audio"))?;

    if audio_data.is_empty() {
        return Err(StatusError::bad_request()
            .brief("audio data is empty")
            .into());
    }

    // Get AI provider
    let provider = create_provider_from_env().ok_or_else(|| {
        StatusError::internal_server_error().brief("AI provider not configured")
    })?;

    // Get or create session
    let session = get_or_create_session(user_id).await?;

    // Get history from database
    let history_messages = get_session_history(session.id).await?;
    let history: Vec<ChatMessage> = history_messages
        .into_iter()
        .map(|m| ChatMessage {
            role: m.role,
            content: m.content,
        })
        .collect();

    // Use custom or default system prompt
    let system_prompt = input
        .system_prompt
        .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());

    // Call voice chat pipeline
    tracing::info!("Calling {} voice_chat API...", provider.name());
    let result = provider
        .voice_chat(audio_data, history, Some(system_prompt))
        .await
        .map_err(|e: AiProviderError| {
            tracing::error!("{} voice_chat error: {:?}", provider.name(), e);
            StatusError::internal_server_error().brief(e.to_string())
        })?;
    tracing::info!("{} voice_chat API completed successfully", provider.name());

    // Save to database
    save_messages(session.id, user_id, &result.user_text, &result.ai_text).await?;

    // Analyze user_text for corrections
    let corrections = analyze_corrections(&result.user_text);

    json_ok(ChatResponse {
        user_text: Some(result.user_text),
        ai_text: result.ai_text,
        ai_text_zh: None,
        ai_audio_base64: Some(result.ai_audio_base64),
        corrections,
    })
}

/// Send text and receive AI response with optional audio
#[endpoint(tags("Chat"))]
pub async fn text_chat_send(
    input: JsonBody<TextChatRequest>,
    depot: &mut Depot,
) -> JsonResult<ChatResponse> {
    let user_id = depot.user_id()?;

    if input.message.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("message is required")
            .into());
    }

    // Get AI provider
    let provider = create_provider_from_env().ok_or_else(|| {
        StatusError::internal_server_error().brief("AI provider not configured")
    })?;

    // Get or create session
    let session = get_or_create_session(user_id).await?;

    // Build messages
    let mut messages: Vec<ChatMessage> = Vec::new();

    // Add system prompt
    let system_prompt = input
        .system_prompt
        .clone()
        .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());
    messages.push(ChatMessage {
        role: "system".to_string(),
        content: system_prompt,
    });

    // Add history from database
    let history_messages = get_session_history(session.id).await?;
    messages.extend(history_messages.into_iter().map(|m| ChatMessage {
        role: m.role,
        content: m.content,
    }));

    // Add user message
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: input.message.clone(),
    });

    // Generate AI response
    let chat_service = provider.chat_service().ok_or_else(|| {
        StatusError::internal_server_error().brief("Chat service not available")
    })?;
    tracing::info!("Calling {} chat API...", provider.name());
    let ai_text: String = chat_service.chat(messages, Some(0.7), None).await.map_err(|e: AiProviderError| {
        tracing::error!("{} chat error: {:?}", provider.name(), e);
        StatusError::internal_server_error().brief(e.to_string())
    })?;
    tracing::info!("{} chat API completed successfully", provider.name());

    // Save to database
    save_messages(session.id, user_id, &input.message, &ai_text).await?;

    // Generate audio if requested
    let ai_audio_base64 = if input.generate_audio {
        if let Some(tts) = provider.tts() {
            tracing::info!("Calling {} TTS API...", provider.name());
            match tts.synthesize(&ai_text, None, None).await {
                Ok(tts_response) => {
                    tracing::info!("{} TTS API completed successfully", provider.name());
                    Some(BASE64.encode(&tts_response.audio_data))
                }
                Err(e) => {
                    tracing::warn!("TTS failed: {}", e);
                    None
                }
            }
        } else {
            tracing::warn!("TTS not available for provider {}", provider.name());
            None
        }
    } else {
        None
    };

    // Analyze for corrections
    let corrections = analyze_corrections(&input.message);

    json_ok(ChatResponse {
        user_text: Some(input.message.clone()),
        ai_text,
        ai_text_zh: None,
        ai_audio_base64,
        corrections,
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
        return Err(StatusError::bad_request()
            .brief("text is required")
            .into());
    }

    // Get AI provider
    let provider = create_provider_from_env().ok_or_else(|| {
        StatusError::internal_server_error().brief("AI provider not configured")
    })?;

    // Get TTS service
    let tts = provider.tts().ok_or_else(|| {
        StatusError::internal_server_error().brief("TTS service not available")
    })?;

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

/// Simple grammar/vocabulary analysis for corrections
fn analyze_corrections(text: &str) -> Vec<Correction> {
    let mut corrections = Vec::new();
    let lower = text.to_lowercase();

    if lower.contains("i has ") {
        corrections.push(Correction {
            original: "I has".to_string(),
            corrected: "I have".to_string(),
            explanation: "Use 'have' with 'I', not 'has'. 'Has' is for third person singular (he/she/it).".to_string(),
        });
    }

    if lower.contains("he go ") || lower.contains("she go ") {
        corrections.push(Correction {
            original: "he/she go".to_string(),
            corrected: "he/she goes".to_string(),
            explanation: "Use 'goes' for third person singular in present tense.".to_string(),
        });
    }

    if lower.contains("yesterday i go ") {
        corrections.push(Correction {
            original: "yesterday I go".to_string(),
            corrected: "yesterday I went".to_string(),
            explanation: "Use past tense 'went' when talking about yesterday.".to_string(),
        });
    }

    if lower.contains("more better") {
        corrections.push(Correction {
            original: "more better".to_string(),
            corrected: "better".to_string(),
            explanation: "'Better' is already comparative. Don't use 'more' with it.".to_string(),
        });
    }

    if lower.contains("i am agree") {
        corrections.push(Correction {
            original: "I am agree".to_string(),
            corrected: "I agree".to_string(),
            explanation: "'Agree' is a verb, not an adjective. Say 'I agree' directly.".to_string(),
        });
    }

    corrections
}
