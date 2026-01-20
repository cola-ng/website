//! Voice Chat API Routes
//!
//! Provides voice-based conversation functionality using BigModel APIs:
//! - POST /api/voice-chat/send: Send audio and receive AI response with audio

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use salvo::http::StatusCode;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::services::bigmodel::{BigModelClient, BigModelError, ChatMessage};
use crate::{hoops, AppResult, DepotExt};

pub fn router() -> Router {
    Router::with_path("voice-chat")
        .hoop(hoops::require_auth)
        .push(Router::with_path("send").post(voice_chat_send))
        .push(Router::with_path("text-send").post(text_chat_send))
        .push(Router::with_path("tts").post(text_to_speech))
}

/// Request for voice chat - audio input
#[derive(Debug, Deserialize)]
pub struct VoiceChatRequest {
    /// Base64 encoded audio data (WAV format)
    pub audio_base64: String,
    /// Conversation history for context
    #[serde(default)]
    pub history: Vec<HistoryMessage>,
    /// Optional custom system prompt
    pub system_prompt: Option<String>,
}

/// Request for text chat - text input with TTS response
#[derive(Debug, Deserialize)]
pub struct TextChatRequest {
    /// User's text message
    pub message: String,
    /// Conversation history for context
    #[serde(default)]
    pub history: Vec<HistoryMessage>,
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
#[derive(Debug, Deserialize)]
pub struct TtsRequest {
    /// Text to synthesize
    pub text: String,
    /// Voice option
    pub voice: Option<String>,
    /// Speed (0.5 - 2.0)
    pub speed: Option<f32>,
}

/// History message format from frontend
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HistoryMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

impl From<HistoryMessage> for ChatMessage {
    fn from(msg: HistoryMessage) -> Self {
        ChatMessage {
            role: msg.role,
            content: msg.content,
        }
    }
}

/// Response for voice/text chat
#[derive(Debug, Serialize)]
pub struct VoiceChatResponse {
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

#[derive(Debug, Serialize)]
pub struct Correction {
    pub original: String,
    pub corrected: String,
    pub explanation: String,
}

/// Response for TTS only
#[derive(Debug, Serialize)]
pub struct TtsResponse {
    /// Base64 encoded audio
    pub audio_base64: String,
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

/// POST /api/voice-chat/send
/// Send audio and receive AI response with audio
#[handler]
pub async fn voice_chat_send(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let _user_id = depot.user_id()?;

    let input: VoiceChatRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    // Decode audio
    let audio_data = BASE64
        .decode(&input.audio_base64)
        .map_err(|_| StatusError::bad_request().brief("invalid base64 audio"))?;

    if audio_data.is_empty() {
        return Err(StatusError::bad_request()
            .brief("audio data is empty")
            .into());
    }

    // Get BigModel client
    let client = BigModelClient::from_env()
        .ok_or_else(|| StatusError::internal_server_error().brief("BigModel API key not configured"))?;

    // Convert history
    let history: Vec<ChatMessage> = input.history.into_iter().map(Into::into).collect();

    // Use custom or default system prompt
    let system_prompt = input
        .system_prompt
        .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());

    // Call voice chat pipeline
    let result = client
        .voice_chat(audio_data, history, Some(system_prompt))
        .await
        .map_err(|e: BigModelError| StatusError::internal_server_error().brief(e.to_string()))?;

    // Analyze user_text for corrections
    let corrections = analyze_corrections(&result.user_text);

    res.status_code(StatusCode::OK);
    res.render(Json(VoiceChatResponse {
        user_text: Some(result.user_text),
        ai_text: result.ai_text,
        ai_text_zh: None, // Could add translation later
        ai_audio_base64: Some(result.ai_audio_base64),
        corrections,
    }));
    Ok(())
}

/// POST /api/voice-chat/text-send
/// Send text and receive AI response with optional audio
#[handler]
pub async fn text_chat_send(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let _user_id = depot.user_id()?;

    let input: TextChatRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.message.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("message is required")
            .into());
    }

    // Get BigModel client
    let client = BigModelClient::from_env()
        .ok_or_else(|| StatusError::internal_server_error().brief("BigModel API key not configured"))?;

    // Build messages
    let mut messages: Vec<ChatMessage> = Vec::new();

    // Add system prompt
    let system_prompt = input
        .system_prompt
        .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());
    messages.push(ChatMessage {
        role: "system".to_string(),
        content: system_prompt,
    });

    // Add history
    messages.extend(input.history.into_iter().map(Into::into));

    // Add user message
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: input.message.clone(),
    });

    // Generate AI response
    let ai_text: String = client
        .chat(messages, Some(0.7))
        .await
        .map_err(|e: BigModelError| StatusError::internal_server_error().brief(e.to_string()))?;

    // Generate audio if requested
    let ai_audio_base64 = if input.generate_audio {
        match client.synthesize(&ai_text, None, None).await {
            Ok(audio_bytes) => Some(BASE64.encode(&audio_bytes)),
            Err(e) => {
                tracing::warn!("TTS failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Analyze for corrections
    let corrections = analyze_corrections(&input.message);

    res.status_code(StatusCode::OK);
    res.render(Json(VoiceChatResponse {
        user_text: Some(input.message),
        ai_text,
        ai_text_zh: None,
        ai_audio_base64,
        corrections,
    }));
    Ok(())
}

/// POST /api/voice-chat/tts
/// Convert text to speech only
#[handler]
pub async fn text_to_speech(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let _user_id = depot.user_id()?;

    let input: TtsRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.text.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("text is required")
            .into());
    }

    // Get BigModel client
    let client = BigModelClient::from_env()
        .ok_or_else(|| StatusError::internal_server_error().brief("BigModel API key not configured"))?;

    // Generate audio
    let audio_bytes: Vec<u8> = client
        .synthesize(&input.text, input.voice.as_deref(), input.speed)
        .await
        .map_err(|e: BigModelError| StatusError::internal_server_error().brief(e.to_string()))?;

    let audio_base64 = BASE64.encode(&audio_bytes);

    res.status_code(StatusCode::OK);
    res.render(Json(TtsResponse { audio_base64 }));
    Ok(())
}

/// Simple grammar/vocabulary analysis for corrections
/// This is a basic implementation - could be enhanced with AI analysis
fn analyze_corrections(text: &str) -> Vec<Correction> {
    let mut corrections = Vec::new();
    let lower = text.to_lowercase();

    // Common mistakes patterns
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
