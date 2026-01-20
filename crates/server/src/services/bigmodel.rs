//! BigModel API Service for ASR, TTS, and Chat
//!
//! Integrates with 智谱 BigModel APIs:
//! - GLM-ASR: Speech-to-Text (语音识别)
//! - GLM-TTS: Text-to-Speech (语音合成)
//! - GLM-4-Flash: Chat completion (对话生成)

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const BIGMODEL_API_BASE: &str = "https://open.bigmodel.cn/api/paas/v4";
const ASR_MODEL: &str = "glm-asr";
const TTS_MODEL: &str = "glm-tts";
const CHAT_MODEL: &str = "glm-4-flash";

#[derive(Debug, Clone)]
pub struct BigModelClient {
    api_key: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AsrResponse {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct ChatResponseChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatResponseChoice>,
}

#[derive(Debug, Serialize)]
struct TtsRequest {
    model: String,
    input: String,
    voice: String,
    speed: f32,
    volume: f32,
    response_format: String,
}

// ASR request using chat completions API with audio content
#[derive(Debug, Serialize)]
struct AsrChatRequest {
    model: String,
    messages: Vec<AsrChatMessage>,
}

#[derive(Debug, Serialize)]
struct AsrChatMessage {
    role: String,
    content: Vec<AsrContent>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum AsrContent {
    #[serde(rename = "input_audio")]
    InputAudio { input_audio: AudioData },
}

#[derive(Debug, Serialize)]
struct AudioData {
    data: String, // base64 encoded audio
    format: String,
}

#[derive(Debug, Deserialize)]
struct AsrApiResponse {
    choices: Vec<AsrApiChoice>,
}

#[derive(Debug, Deserialize)]
struct AsrApiChoice {
    message: AsrApiMessage,
}

#[derive(Debug, Deserialize)]
struct AsrApiMessage {
    content: String,
}

impl BigModelClient {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self { api_key, client }
    }

    /// Get API key from environment
    pub fn from_env() -> Option<Self> {
        std::env::var("BIGMODEL_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .map(Self::new)
    }

    /// Speech-to-Text using GLM-ASR via chat completions API
    /// Accepts audio bytes (WAV or MP3 format - BigModel only supports these two)
    pub async fn transcribe(&self, audio_data: Vec<u8>) -> Result<AsrResponse, BigModelError> {
        let url = format!("{}/chat/completions", BIGMODEL_API_BASE);

        // Encode audio as base64
        let audio_base64 = BASE64.encode(&audio_data);

        // Detect format from magic bytes (BigModel only supports wav and mp3)
        let format = if audio_data.starts_with(b"RIFF") {
            "wav"
        } else if audio_data.len() >= 3 && (
            // MP3 with ID3 tag
            audio_data.starts_with(b"ID3") ||
            // MP3 sync word (0xFF 0xFB, 0xFF 0xFA, 0xFF 0xF3, 0xFF 0xF2)
            (audio_data[0] == 0xFF && (audio_data[1] & 0xE0) == 0xE0)
        ) {
            "mp3"
        } else {
            // Default to wav - the API will return an error if format is wrong
            "wav"
        };

        let request = AsrChatRequest {
            model: ASR_MODEL.to_string(),
            messages: vec![AsrChatMessage {
                role: "user".to_string(),
                content: vec![AsrContent::InputAudio {
                    input_audio: AudioData {
                        data: audio_base64,
                        format: format.to_string(),
                    },
                }],
            }],
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e: reqwest::Error| BigModelError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BigModelError::Api(format!(
                "ASR API error {}: {}",
                status, body
            )));
        }

        let api_response: AsrApiResponse = response
            .json()
            .await
            .map_err(|e: reqwest::Error| BigModelError::Parse(e.to_string()))?;

        let text = api_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(AsrResponse { text })
    }

    /// Chat completion using GLM-4-Flash
    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
    ) -> Result<String, BigModelError> {
        let url = format!("{}/chat/completions", BIGMODEL_API_BASE);

        let request = ChatRequest {
            model: CHAT_MODEL.to_string(),
            messages,
            temperature: temperature.unwrap_or(0.7),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e: reqwest::Error| BigModelError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BigModelError::Api(format!(
                "Chat API error {}: {}",
                status, body
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e: reqwest::Error| BigModelError::Parse(e.to_string()))?;

        let reply = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(reply)
    }

    /// Text-to-Speech using GLM-TTS
    /// Returns audio bytes in WAV format
    pub async fn synthesize(
        &self,
        text: &str,
        voice: Option<&str>,
        speed: Option<f32>,
    ) -> Result<Vec<u8>, BigModelError> {
        let url = format!("{}/audio/speech", BIGMODEL_API_BASE);

        let request = TtsRequest {
            model: TTS_MODEL.to_string(),
            input: text.to_string(),
            voice: voice.unwrap_or("alloy").to_string(),
            speed: speed.unwrap_or(1.0),
            volume: 1.0,
            response_format: "wav".to_string(),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e: reqwest::Error| BigModelError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BigModelError::Api(format!(
                "TTS API error {}: {}",
                status, body
            )));
        }

        let audio_bytes = response
            .bytes()
            .await
            .map_err(|e: reqwest::Error| BigModelError::Request(e.to_string()))?;

        Ok(audio_bytes.to_vec())
    }

    /// Complete voice chat pipeline:
    /// 1. ASR: Convert user audio to text
    /// 2. Chat: Generate AI response
    /// 3. TTS: Convert AI response to audio
    pub async fn voice_chat(
        &self,
        audio_data: Vec<u8>,
        conversation_history: Vec<ChatMessage>,
        system_prompt: Option<String>,
    ) -> Result<VoiceChatResponse, BigModelError> {
        // Step 1: Transcribe user audio
        let asr_result = self.transcribe(audio_data).await?;
        let user_text = asr_result.text.clone();

        if user_text.trim().is_empty() {
            return Err(BigModelError::Api(
                "Could not transcribe audio - no speech detected".to_string(),
            ));
        }

        // Step 2: Build messages for chat
        let mut messages = Vec::new();

        // Add system prompt if provided
        if let Some(system) = system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system,
            });
        }

        // Add conversation history
        messages.extend(conversation_history);

        // Add user message
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_text.clone(),
        });

        // Generate AI response
        let ai_text = self.chat(messages, Some(0.7)).await?;

        // Step 3: Synthesize AI response to audio
        let ai_audio = self.synthesize(&ai_text, None, None).await?;
        let ai_audio_base64 = BASE64.encode(&ai_audio);

        Ok(VoiceChatResponse {
            user_text,
            ai_text,
            ai_audio_base64,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoiceChatResponse {
    pub user_text: String,
    pub ai_text: String,
    pub ai_audio_base64: String,
}

#[derive(Debug, thiserror::Error)]
pub enum BigModelError {
    #[error("Request error: {0}")]
    Request(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

impl From<BigModelError> for salvo::http::StatusError {
    fn from(e: BigModelError) -> Self {
        salvo::http::StatusError::internal_server_error().brief(e.to_string())
    }
}
