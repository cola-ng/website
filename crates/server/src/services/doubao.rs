//! Doubao (豆包) Volcanic Engine AI Provider Implementation
//!
//! Integrates with Bytedance/Volcanic Engine APIs:
//! - OpenSpeech ASR: Speech-to-Text (语音识别)
//! - OpenSpeech TTS: Text-to-Speech (语音合成) - WebSocket Bidirectional API
//! - Ark Chat: Chat completion (对话生成)

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;

use super::ai_provider::{
    AiProvider, AiProviderError, AsrResponse, AsrService, ChatMessage, ChatService,
    PronunciationAnalysis, PronunciationService, StructuredChatResponse, TextIssue, TtsResponse,
    TtsService, WordPronunciationScore, WordTiming,
};

const DOUBAO_SPEECH_API_BASE: &str = "https://openspeech.bytedance.com/api/v1";

// ============================================================================
// WebSocket TTS Protocol Constants (Bidirectional TTS API)
// ============================================================================

const TTS_WS_URL: &str = "wss://openspeech.bytedance.com/api/v3/tts/bidirection";

// Protocol header bytes
const PROTOCOL_VERSION: u8 = 0x11; // v1, 4-byte header
const MSG_TYPE_FULL_CLIENT: u8 = 0x14; // Full-client request with event
const MSG_TYPE_AUDIO_ONLY: u8 = 0xB4; // Audio-only response with event
const SERIALIZATION_JSON: u8 = 0x10; // JSON serialization
const NO_COMPRESSION: u8 = 0x00;
const RESERVED: u8 = 0x00;

// Event numbers
const EVENT_START_CONNECTION: i32 = 1;
const EVENT_CONNECTION_STARTED: i32 = 50;
const EVENT_START_SESSION: i32 = 100;
const EVENT_SESSION_STARTED: i32 = 150;
const EVENT_TASK_REQUEST: i32 = 200;
const EVENT_TTS_RESPONSE: i32 = 352;
const EVENT_SESSION_FINISHED: i32 = 152;
const EVENT_FINISH_SESSION: i32 = 102;
const EVENT_FINISH_CONNECTION: i32 = 2;
/// 大模型录音文件极速版识别API endpoint
const DOUBAO_ZHIPU_ASR_URL: &str =
    "https://openspeech.bytedance.com/api/v3/auc/bigmodel/recognize/flash";
const DOUBAO_CHAT_API_BASE: &str = "https://ark.cn-beijing.volces.com/api/v3";
const DEFAULT_CHAT_MODEL: &str = "doubao-seed-1-8-251228";

/// Doubao API Client
#[derive(Debug, Clone)]
pub struct DoubaoClient {
    app_id: String,
    access_token: String,
    api_key: String,           // For WebSocket TTS authorization
    chat_model: String,
    tts_resource_id: String,   // TTS resource ID (e.g., "seed-tts-2.0")
    client: reqwest::Client,
}

// Internal request/response types for Doubao API

#[derive(Debug, Serialize)]
struct SpeechAppInfo {
    appid: String,
    token: String,
}

#[derive(Debug, Serialize)]
struct SpeechUserInfo {
    uid: String,
}

// ============================================================================
// BigModel ASR (大模型录音文件极速版识别API) request/response types
// ============================================================================

#[derive(Debug, Serialize)]
struct BigModelAsrUser {
    uid: String,
}

#[derive(Debug, Serialize)]
struct BigModelAsrAudio {
    /// Base64 encoded audio data
    data: String,
}

#[derive(Debug, Serialize)]
struct BigModelAsrRequest {
    model_name: String,
}

#[derive(Debug, Serialize)]
struct BigModelAsrApiRequest {
    user: BigModelAsrUser,
    audio: BigModelAsrAudio,
    request: BigModelAsrRequest,
}

#[derive(Debug, Serialize)]
struct ChatApiRequest {
    model: String,
    messages: Vec<ChatMessageInternal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessageInternal {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatApiResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageInternal,
}

#[derive(Debug, Serialize)]
struct PronunciationAudioInfo {
    data: String,
    format: String,
    language: String,
}

#[derive(Debug, Serialize)]
struct PronunciationScoreCoefficients {
    fluency: f32,
    pronunciation: f32,
    completeness: f32,
}

#[derive(Debug, Serialize)]
struct PronunciationRequestInfo {
    reference_text: String,
    score_coefficients: PronunciationScoreCoefficients,
}

#[derive(Debug, Serialize)]
struct PronunciationApiRequest {
    app: SpeechAppInfo,
    user: SpeechUserInfo,
    audio: PronunciationAudioInfo,
    request: PronunciationRequestInfo,
}

impl DoubaoClient {
    pub fn new(
        app_id: String,
        access_token: String,
        api_key: String,
    ) -> Self {
        Self::with_options(app_id, access_token, api_key, None, None)
    }

    pub fn with_options(
        app_id: String,
        access_token: String,
        api_key: String,
        chat_model: Option<String>,
        tts_resource_id: Option<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            app_id,
            access_token,
            api_key,
            chat_model: chat_model.unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string()),
            tts_resource_id: tts_resource_id.unwrap_or_else(|| "seed-tts-2.0".to_string()),
            client,
        }
    }

    /// Get client from environment variables
    pub fn from_env() -> Option<Self> {
        let app_id = std::env::var("DOUBAO_APP_ID")
            .ok()
            .filter(|s| !s.is_empty())?;
        let access_token = std::env::var("DOUBAO_ACCESS_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())?;
        let api_key = std::env::var("DOUBAO_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())?;
        let chat_model = std::env::var("DOUBAO_CHAT_MODEL").ok();
        let tts_resource_id = std::env::var("DOUBAO_RESOURCE_ID").ok();

        Some(Self::with_options(
            app_id,
            access_token,
            api_key,
            chat_model,
            tts_resource_id,
        ))
    }

    /// Create speech API app info
    fn speech_app_info(&self) -> SpeechAppInfo {
        SpeechAppInfo {
            appid: self.app_id.clone(),
            token: self.access_token.clone(),
        }
    }

    /// Create speech API user info
    fn speech_user_info() -> SpeechUserInfo {
        SpeechUserInfo {
            uid: "user_001".to_string(),
        }
    }

    /// Detect audio format from magic bytes
    fn detect_format(audio_data: &[u8]) -> &'static str {
        if audio_data.starts_with(b"RIFF") {
            "wav"
        } else if audio_data.len() >= 3
            && (audio_data.starts_with(b"ID3")
                || (audio_data[0] == 0xFF && (audio_data[1] & 0xE0) == 0xE0))
        {
            "mp3"
        } else {
            "wav"
        }
    }

    /// Detect sample rate from WAV header
    fn detect_sample_rate(audio_data: &[u8]) -> u32 {
        // WAV format: sample rate is at bytes 24-27 (little endian)
        if audio_data.starts_with(b"RIFF") && audio_data.len() > 27 {
            let rate = u32::from_le_bytes([
                audio_data[24],
                audio_data[25],
                audio_data[26],
                audio_data[27],
            ]);
            if rate > 0 && rate <= 48000 {
                return rate;
            }
        }
        // Default to 16kHz for speech
        16000
    }

    // ========================================================================
    // WebSocket TTS Implementation (Bidirectional TTS API)
    // ========================================================================

    /// Perform TTS using WebSocket bidirectional API
    async fn perform_tts_websocket(
        &self,
        speaker: &str,
        speech_rate: i32,
        text: &str,
    ) -> Result<Vec<u8>, AiProviderError> {
        use tokio_tungstenite::connect_async;

        // Generate unique IDs
        let connect_id = uuid::Uuid::new_v4().to_string();
        let session_id = uuid::Uuid::new_v4().to_string();
        let user_id = uuid::Uuid::new_v4().to_string();

        // Create Authorization header (format: "Bearer;{api_key}")
        let authorization = format!("Bearer;{}", self.api_key);

        // Build WebSocket request with custom headers
        let request = tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(TTS_WS_URL)
            .header("Host", "openspeech.bytedance.com")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .header("Authorization", &authorization)
            .header("X-Api-App-Key", &self.app_id)
            .header("X-Api-Access-Key", &self.access_token)
            .header("X-Api-Resource-Id", &self.tts_resource_id)
            .header("X-Api-Connect-Id", &connect_id)
            .body(())
            .map_err(|e| AiProviderError::Request(format!("Failed to build request: {}", e)))?;

        // Connect to WebSocket
        let (ws_stream, response) = connect_async(request)
            .await
            .map_err(|e| AiProviderError::Request(format!("WebSocket connection failed: {}", e)))?;

        // Log the X-Tt-Logid for debugging
        if let Some(logid) = response.headers().get("X-Tt-Logid") {
            tracing::debug!("Doubao TTS X-Tt-Logid: {:?}", logid);
        }

        let (mut write, mut read) = ws_stream.split();

        // 1. Send StartConnection
        let start_conn_frame = Self::build_event_frame(EVENT_START_CONNECTION, None, json!({}));
        write
            .send(WsMessage::Binary(start_conn_frame.into()))
            .await
            .map_err(|e| AiProviderError::Request(format!("Failed to send StartConnection: {}", e)))?;

        // Wait for ConnectionStarted
        Self::wait_for_event(&mut read, EVENT_CONNECTION_STARTED).await?;
        tracing::debug!("Doubao TTS: ConnectionStarted received");

        // 2. Send StartSession
        let start_session_payload = json!({
            "user": { "uid": user_id },
            "event": EVENT_START_SESSION,
            "namespace": "BidirectionalTTS",
            "req_params": {
                "speaker": speaker,
                "audio_params": {
                    "format": "mp3",
                    "sample_rate": 24000,
                    "speech_rate": speech_rate,
                    "enable_timestamp": true
                },
                "additions": json!({ "disable_markdown_filter": false }).to_string()
            }
        });
        let start_session_frame =
            Self::build_event_frame(EVENT_START_SESSION, Some(&session_id), start_session_payload);
        write
            .send(WsMessage::Binary(start_session_frame.into()))
            .await
            .map_err(|e| AiProviderError::Request(format!("Failed to send StartSession: {}", e)))?;

        // Wait for SessionStarted
        Self::wait_for_event(&mut read, EVENT_SESSION_STARTED).await?;
        tracing::debug!("Doubao TTS: SessionStarted received");

        // 3. Send TaskRequest with text
        let task_payload = json!({
            "user": { "uid": user_id },
            "event": EVENT_TASK_REQUEST,
            "namespace": "BidirectionalTTS",
            "req_params": {
                "speaker": speaker,
                "audio_params": {
                    "format": "mp3",
                    "sample_rate": 24000,
                    "speech_rate": speech_rate,
                    "enable_timestamp": true
                },
                "text": text,
                "additions": json!({ "disable_markdown_filter": false }).to_string()
            }
        });
        let task_frame = Self::build_event_frame(EVENT_TASK_REQUEST, Some(&session_id), task_payload);
        write
            .send(WsMessage::Binary(task_frame.into()))
            .await
            .map_err(|e| AiProviderError::Request(format!("Failed to send TaskRequest: {}", e)))?;

        // 4. Receive audio data
        let mut audio_data = Vec::new();
        loop {
            match read.next().await {
                Some(Ok(WsMessage::Binary(data))) => {
                    if data.len() < 4 {
                        continue;
                    }

                    let event = Self::parse_event(&data)?;

                    match event {
                        EVENT_TTS_RESPONSE => {
                            if let Some(audio) = Self::extract_audio_from_frame(&data) {
                                audio_data.extend_from_slice(&audio);
                            }
                        }
                        EVENT_SESSION_FINISHED => {
                            tracing::debug!("Doubao TTS: SessionFinished received");
                            break;
                        }
                        _ => {
                            tracing::debug!("Doubao TTS: received event {}", event);
                        }
                    }
                }
                Some(Ok(WsMessage::Text(txt))) => {
                    tracing::warn!("Doubao TTS: unexpected text message: {}", txt);
                }
                Some(Err(e)) => {
                    tracing::error!("Doubao TTS: WebSocket error: {}", e);
                    break;
                }
                None => {
                    tracing::debug!("Doubao TTS: WebSocket stream ended");
                    break;
                }
                _ => {
                    break;
                }
            }
        }

        // 5. Send FinishSession
        let finish_session_frame =
            Self::build_event_frame(EVENT_FINISH_SESSION, Some(&session_id), json!({}));
        let _ = write.send(WsMessage::Binary(finish_session_frame.into())).await;

        // 6. Send FinishConnection
        let finish_conn_frame = Self::build_event_frame(EVENT_FINISH_CONNECTION, None, json!({}));
        let _ = write.send(WsMessage::Binary(finish_conn_frame.into())).await;

        if audio_data.is_empty() {
            return Err(AiProviderError::Api("No audio data received".to_string()));
        }

        Ok(audio_data)
    }

    /// Build a WebSocket event frame
    fn build_event_frame(event: i32, session_id: Option<&str>, payload: serde_json::Value) -> Vec<u8> {
        let mut frame = Vec::new();

        // Header (4 bytes)
        frame.push(PROTOCOL_VERSION);
        frame.push(MSG_TYPE_FULL_CLIENT);
        frame.push(SERIALIZATION_JSON | NO_COMPRESSION);
        frame.push(RESERVED);

        // Event number (4 bytes, big-endian)
        frame.extend_from_slice(&event.to_be_bytes());

        // Session ID (if provided)
        if let Some(sid) = session_id {
            let sid_bytes = sid.as_bytes();
            frame.extend_from_slice(&(sid_bytes.len() as u32).to_be_bytes());
            frame.extend_from_slice(sid_bytes);
        }

        // Payload
        let payload_str = payload.to_string();
        let payload_bytes = payload_str.as_bytes();
        frame.extend_from_slice(&(payload_bytes.len() as u32).to_be_bytes());
        frame.extend_from_slice(payload_bytes);

        frame
    }

    /// Wait for a specific event from the WebSocket stream
    async fn wait_for_event<S>(read: &mut S, expected_event: i32) -> Result<(), AiProviderError>
    where
        S: StreamExt<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        while let Some(result) = read.next().await {
            match result {
                Ok(WsMessage::Binary(data)) => {
                    if data.len() >= 8 {
                        let event = i32::from_be_bytes([data[4], data[5], data[6], data[7]]);
                        if event == expected_event {
                            return Ok(());
                        }
                    }
                }
                Err(e) => {
                    return Err(AiProviderError::Request(format!(
                        "WebSocket error while waiting for event: {}",
                        e
                    )));
                }
                _ => {}
            }
        }
        Err(AiProviderError::Api(format!(
            "Expected event {} not received",
            expected_event
        )))
    }

    /// Parse event number from a WebSocket frame
    fn parse_event(data: &[u8]) -> Result<i32, AiProviderError> {
        if data.len() < 8 {
            return Err(AiProviderError::Parse("Frame too short".to_string()));
        }
        Ok(i32::from_be_bytes([data[4], data[5], data[6], data[7]]))
    }

    /// Extract audio data from a TTS response frame
    fn extract_audio_from_frame(data: &[u8]) -> Option<Vec<u8>> {
        if data.len() < 4 {
            return None;
        }

        let msg_type = data[1];

        // Audio-only response (0xB4)
        if msg_type == MSG_TYPE_AUDIO_ONLY {
            // Header (4 bytes) + Event (4 bytes) + Session ID length (4 bytes)
            if data.len() < 12 {
                return None;
            }

            let session_id_len = u32::from_be_bytes([data[8], data[9], data[10], data[11]]) as usize;
            let audio_offset = 12 + session_id_len + 4; // +4 for payload size

            if data.len() > audio_offset {
                return Some(data[audio_offset..].to_vec());
            }
        }

        None
    }
}

/// 大模型录音文件极速版识别API (BigModel ASR)
/// Documentation: https://www.volcengine.com/docs/6561/1631584
#[async_trait]
impl AsrService for DoubaoClient {
    async fn transcribe(
        &self,
        audio_data: Vec<u8>,
        _language: Option<&str>,
    ) -> Result<AsrResponse, AiProviderError> {
        let audio_base64 = BASE64.encode(&audio_data);

        let request = BigModelAsrApiRequest {
            user: BigModelAsrUser {
                uid: self.app_id.clone(),
            },
            audio: BigModelAsrAudio { data: audio_base64 },
            request: BigModelAsrRequest {
                model_name: "bigmodel".to_string(),
            },
        };

        // Generate UUID for request ID
        let request_id = uuid::Uuid::new_v4().to_string();

        tracing::info!(
            "Doubao ASR: sending request to {}, audio_size={}",
            DOUBAO_ZHIPU_ASR_URL,
            audio_data.len()
        );

        let response = self
            .client
            .post(DOUBAO_ZHIPU_ASR_URL)
            .header("Content-Type", "application/json")
            .header("X-Api-App-Key", &self.app_id)
            .header("X-Api-Access-Key", &self.access_token)
            .header("X-Api-Resource-Id", "volc.bigasr.auc_turbo")
            .header("X-Api-Request-Id", &request_id)
            .header("X-Api-Sequence", "-1")
            .json(&request)
            .send()
            .await
            .map_err(|e| AiProviderError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Doubao BigModel ASR error {}: {}", status, body);
            return Err(AiProviderError::Api(format!(
                "Doubao ASR API error {}: {}",
                status, body
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AiProviderError::Parse(e.to_string()))?;

        tracing::debug!("Doubao ASR response: {}", result);

        // Parse Doubao ASR response format
        // Response: { "result": { "text": "...", "utterances": [...] }, "audio_info": { "duration": ... } }
        let text = result["result"]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Extract word timings from utterances
        let words = result["result"]["utterances"]
            .as_array()
            .map(|utterances| {
                utterances
                    .iter()
                    .flat_map(|u| {
                        u["words"].as_array().map(|words| {
                            words
                                .iter()
                                .filter_map(|w| {
                                    Some(WordTiming {
                                        word: w["text"].as_str()?.to_string(),
                                        // Convert milliseconds to seconds
                                        start_time: w["start_time"].as_f64()? / 1000.0,
                                        end_time: w["end_time"].as_f64()? / 1000.0,
                                        confidence: w["confidence"].as_f64().map(|c| c as f32),
                                    })
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                    .flatten()
                    .collect::<Vec<_>>()
            });

        tracing::info!("Doubao BigModel ASR: transcribed text: {}", text);

        Ok(AsrResponse {
            text,
            confidence: None, // BigModel ASR doesn't provide overall confidence
            words,
        })
    }

    fn supported_formats(&self) -> Vec<&'static str> {
        // BigModel ASR supports WAV, MP3, OGG OPUS
        vec!["wav", "mp3", "ogg"]
    }
}

#[async_trait]
impl TtsService for DoubaoClient {
    async fn synthesize(
        &self,
        text: &str,
        voice: Option<&str>,
        speed: Option<f32>,
    ) -> Result<TtsResponse, AiProviderError> {
        let speaker = voice
            .unwrap_or("zh_female_cancan_mars_bigtts")
            .to_string();

        // Convert speed ratio to API format: speed 1.0 = 0, range [-50, 100]
        let speech_rate = speed
            .map(|s| ((s - 1.0) * 100.0) as i32)
            .unwrap_or(0)
            .clamp(-50, 100);

        tracing::info!(
            "Doubao TTS WebSocket: synthesizing {} chars with voice={}, speed={}",
            text.len(),
            speaker,
            speech_rate
        );

        let audio_data = self
            .perform_tts_websocket(&speaker, speech_rate, text)
            .await?;

        tracing::info!("Doubao TTS: generated {} bytes MP3 audio", audio_data.len());

        Ok(TtsResponse {
            audio_data,
            format: "mp3".to_string(),
            duration_ms: None, // Could calculate from MP3 frames if needed
        })
    }

    fn available_voices(&self) -> Vec<&'static str> {
        vec![
            "zh_female_cancan_mars_bigtts",
            "zh_female_shuangkuaisisi_moon_bigtts",
            "zh_male_aojiaobazong_moon_bigtts",
            "zh_female_tianmeixiaoyuan_moon_bigtts",
            "zh_male_wennuanahu_moon_bigtts",
            "en_female_amanda_moon_bigtts",
            "en_male_adam_moon_bigtts",
        ]
    }
}

#[async_trait]
impl ChatService for DoubaoClient {
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<String, AiProviderError> {
        let url = format!("{}/chat/completions", DOUBAO_CHAT_API_BASE);

        let request = ChatApiRequest {
            model: self.chat_model.clone(),
            messages: messages
                .into_iter()
                .map(|m| ChatMessageInternal {
                    role: m.role,
                    content: m.content,
                })
                .collect(),
            temperature,
            max_tokens,
            stream: false,
        };

        println!(
            "Doubao Chat: sending request with {} messages",
            request.messages.len()
        );
        tracing::info!(
            "Doubao Chat: sending request with {} messages",
            request.messages.len()
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AiProviderError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Doubao Chat error {}: {}", status, body);
            return Err(AiProviderError::Api(format!(
                "Chat API error {}: {}",
                status, body
            )));
        }

        let chat_response: ChatApiResponse = response
            .json()
            .await
            .map_err(|e| AiProviderError::Parse(e.to_string()))?;

        let reply = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        tracing::info!("Doubao Chat: received {} chars response", reply.len());

        Ok(reply)
    }

    async fn chat_structured(
        &self,
        messages: Vec<ChatMessage>,
        user_text: &str,
        system_prompt: &str,
    ) -> Result<StructuredChatResponse, AiProviderError> {
        let url = format!("{}/chat/completions", DOUBAO_CHAT_API_BASE);

        println!(
            "Doubao Chat Structured: preparing request for user text: {}",
            user_text
        );
        // Build messages with system prompt
        let mut all_messages: Vec<serde_json::Value> = vec![serde_json::json!({
            "role": "system",
            "content": format!(
                "{}\n\nIMPORTANT: You must respond with a JSON object containing:\n\
                1. A natural conversational reply to the user\n\
                2. Grammar/vocabulary analysis of the user's last message",
                system_prompt
            )
        })];

        // Add conversation history
        for msg in messages {
            all_messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content,
            }));
        }

        // JSON Schema for structured output (matching dora-english-teacher format)
        let response_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "use_lang": {
                    "type": "string",
                    "description": "The language of the original text, either 'en' for English, 'zh' for Chinese, or 'mix' for mixed."
                },
                "original_en": {
                    "type": "string",
                    "description": "The original user text in English. If the user wrote in Chinese or mixed language, translate it to English here."
                },
                "original_zh": {
                    "type": "string",
                    "description": "The original user text in Chinese. If the user wrote in English or mixed language, translate it to Chinese here."
                },
                "reply_en": {
                    "type": "string",
                    "description": "Your natural conversational response to the user in English. Keep it concise and encouraging."
                },
                "reply_zh": {
                    "type": "string",
                    "description": "Translation of your reply_en into Chinese."
                },
                "issues": {
                    "type": "array",
                    "description": "Grammar, word choice, or phrasing issues found in the user's last message. Empty array if no issues.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "type": {
                                "type": "string",
                                "enum": ["grammar", "word_choice", "suggestion"],
                                "description": "Type of issue"
                            },
                            "original": {
                                "type": "string",
                                "description": "The problematic text from user's message"
                            },
                            "suggested": {
                                "type": "string",
                                "description": "The corrected or better alternative"
                            },
                            "description_en": {
                                "type": "string",
                                "description": "Explanation of the issue using simple English"
                            },
                            "description_zh": {
                                "type": "string",
                                "description": "Explanation of the issue using simple Chinese"
                            },
                            "severity": {
                                "type": "string",
                                "enum": ["low", "medium", "high"],
                                "description": "Severity level of the issue"
                            },
                            "start_position": {
                                "type": ["integer", "null"],
                                "description": "0-based character offset where issue starts (null if unknown)"
                            },
                            "end_position": {
                                "type": ["integer", "null"],
                                "description": "0-based character offset where issue ends, exclusive (null if unknown)"
                            }
                        },
                        "required": ["type", "original", "suggested", "description_en", "description_zh", "severity"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["use_lang", "original_en", "original_zh", "reply_en", "reply_zh", "issues"],
            "additionalProperties": false
        });

        let payload = serde_json::json!({
            "model": self.chat_model,
            "messages": all_messages,
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "english_teacher_response",
                    "strict": true,
                    "schema": response_schema
                }
            },
            "temperature": 0.7,
            "max_tokens": 2000
        });

        tracing::info!(
            "Doubao Chat Structured: sending request for user text: {}",
            user_text
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiProviderError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Doubao Chat Structured error {}: {}", status, body);
            return Err(AiProviderError::Api(format!(
                "Chat API error {}: {}",
                status, body
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AiProviderError::Parse(e.to_string()))?;

        // Extract content from Chat Completions response
        let content = result["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| AiProviderError::Parse("No content in response".to_string()))?;

        tracing::debug!("Doubao Chat Structured response: {}", content);

        // Parse structured JSON response
        let structured: serde_json::Value = serde_json::from_str(content).map_err(|e| {
            AiProviderError::Parse(format!("Failed to parse structured response: {}", e))
        })?;

        let use_lang = structured["use_lang"].as_str().unwrap_or("en").to_string();
        let original_en = structured["original_en"]
            .as_str()
            .unwrap_or(user_text)
            .to_string();
        let original_zh = structured["original_zh"].as_str().unwrap_or("").to_string();
        let reply_en = structured["reply_en"].as_str().unwrap_or("").to_string();
        let reply_zh = structured["reply_zh"].as_str().unwrap_or("").to_string();

        let issues: Vec<TextIssue> =
            serde_json::from_value(structured["issues"].clone()).unwrap_or_default();

        tracing::info!(
            "Doubao Chat Structured: reply_en={}, issues={}",
            reply_en.len(),
            issues.len()
        );

        Ok(StructuredChatResponse {
            use_lang,
            original_en,
            original_zh,
            reply_en,
            reply_zh,
            issues,
        })
    }
}

#[async_trait]
impl PronunciationService for DoubaoClient {
    async fn analyze_pronunciation(
        &self,
        audio_data: Vec<u8>,
        reference_text: &str,
        language: &str,
    ) -> Result<PronunciationAnalysis, AiProviderError> {
        let url = format!("{}/pronunciation_assessment", DOUBAO_SPEECH_API_BASE);

        let audio_base64 = BASE64.encode(&audio_data);

        let request = PronunciationApiRequest {
            app: self.speech_app_info(),
            user: Self::speech_user_info(),
            audio: PronunciationAudioInfo {
                data: audio_base64,
                format: "wav".to_string(),
                language: language.to_string(),
            },
            request: PronunciationRequestInfo {
                reference_text: reference_text.to_string(),
                score_coefficients: PronunciationScoreCoefficients {
                    fluency: 1.0,
                    pronunciation: 1.0,
                    completeness: 1.0,
                },
            },
        };

        tracing::info!("Doubao Pronunciation: analyzing audio");

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AiProviderError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Doubao Pronunciation error {}: {}", status, body);
            return Err(AiProviderError::Api(format!(
                "Pronunciation API error {}: {}",
                status, body
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AiProviderError::Parse(e.to_string()))?;

        let overall_score = result["result"]["overall_score"].as_f64().unwrap_or(0.0) as f32;
        let fluency_score = result["result"]["fluency_score"].as_f64().unwrap_or(0.0) as f32;
        let pronunciation_score = result["result"]["pronunciation_score"]
            .as_f64()
            .unwrap_or(0.0) as f32;
        let completeness_score = result["result"]["completeness_score"]
            .as_f64()
            .unwrap_or(0.0) as f32;

        let word_scores = result["result"]["words"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|w| {
                        Some(WordPronunciationScore {
                            word: w["word"].as_str()?.to_string(),
                            score: w["score"].as_f64()? as f32,
                            error_type: w["error_type"].as_str().map(String::from),
                            suggestion: w["suggestion"].as_str().map(String::from),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        tracing::info!("Doubao Pronunciation: overall score {}", overall_score);

        Ok(PronunciationAnalysis {
            overall_score,
            fluency_score,
            pronunciation_score,
            completeness_score,
            word_scores,
        })
    }
}

#[async_trait]
impl AiProvider for DoubaoClient {
    fn name(&self) -> &'static str {
        "doubao"
    }

    fn asr(&self) -> Option<Arc<dyn AsrService>> {
        Some(Arc::new(self.clone()))
    }

    fn tts(&self) -> Option<Arc<dyn TtsService>> {
        Some(Arc::new(self.clone()))
    }

    fn chat_service(&self) -> Option<Arc<dyn ChatService>> {
        Some(Arc::new(self.clone()))
    }

    fn pronunciation(&self) -> Option<Arc<dyn PronunciationService>> {
        Some(Arc::new(self.clone()))
    }
}

/// Legacy error type alias
pub type DoubaoError = AiProviderError;
