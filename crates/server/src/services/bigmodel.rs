//! BigModel (智谱) AI Provider Implementation
//!
//! Integrates with 智谱 BigModel APIs:
//! - GLM-ASR: Speech-to-Text (语音识别)
//! - GLM-TTS: Text-to-Speech (语音合成)
//! - GLM-4-Flash: Chat completion (对话生成)

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use super::ai_provider::{
    AiProvider, AiProviderError, AsrResponse, AsrService, ChatMessage, ChatService, TtsResponse,
    TtsService,
};

const BIGMODEL_API_BASE: &str = "https://open.bigmodel.cn/api/paas/v4";
const DEFAULT_ASR_MODEL: &str = "glm-asr";
const DEFAULT_TTS_MODEL: &str = "glm-tts";
const DEFAULT_CHAT_MODEL: &str = "glm-4-flash";

/// BigModel API Client
#[derive(Debug, Clone)]
pub struct BigModelClient {
    api_key: String,
    client: reqwest::Client,
    asr_model: String,
    tts_model: String,
    chat_model: String,
}

// Internal request/response types for BigModel API
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessageInternal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessageInternal {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponseChoice {
    message: ChatMessageInternal,
}

#[derive(Debug, Deserialize)]
struct ChatResponseBody {
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
    data: String,
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
        Self::with_models(api_key, None, None, None)
    }

    pub fn with_models(
        api_key: String,
        asr_model: Option<String>,
        tts_model: Option<String>,
        chat_model: Option<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            client,
            asr_model: asr_model.unwrap_or_else(|| DEFAULT_ASR_MODEL.to_string()),
            tts_model: tts_model.unwrap_or_else(|| DEFAULT_TTS_MODEL.to_string()),
            chat_model: chat_model.unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string()),
        }
    }

    /// Get API key from environment
    pub fn from_env() -> Option<Self> {
        std::env::var("BIGMODEL_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .map(|api_key| {
                Self::with_models(
                    api_key,
                    std::env::var("BIGMODEL_ASR_MODEL").ok(),
                    std::env::var("BIGMODEL_TTS_MODEL").ok(),
                    std::env::var("BIGMODEL_CHAT_MODEL").ok(),
                )
            })
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
}

#[async_trait]
impl AsrService for BigModelClient {
    async fn transcribe(
        &self,
        audio_data: Vec<u8>,
        _language: Option<&str>,
    ) -> Result<AsrResponse, AiProviderError> {
        let url = format!("{}/chat/completions", BIGMODEL_API_BASE);

        let audio_base64 = BASE64.encode(&audio_data);
        let format = Self::detect_format(&audio_data);

        let request = AsrChatRequest {
            model: self.asr_model.clone(),
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

        tracing::info!("BigModel ASR: sending request to {}", url);

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
            tracing::error!("BigModel ASR error {}: {}", status, body);
            return Err(AiProviderError::Api(format!(
                "ASR API error {}: {}",
                status, body
            )));
        }

        let api_response: AsrApiResponse = response
            .json()
            .await
            .map_err(|e| AiProviderError::Parse(e.to_string()))?;

        let text = api_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        tracing::info!("BigModel ASR: transcribed text: {}", text);

        Ok(AsrResponse {
            text,
            confidence: None,
            words: None,
        })
    }

    fn supported_formats(&self) -> Vec<&'static str> {
        vec!["wav", "mp3"]
    }
}

#[async_trait]
impl TtsService for BigModelClient {
    async fn synthesize(
        &self,
        text: &str,
        voice: Option<&str>,
        speed: Option<f32>,
    ) -> Result<TtsResponse, AiProviderError> {
        let url = format!("{}/audio/speech", BIGMODEL_API_BASE);

        let request = TtsRequest {
            model: self.tts_model.clone(),
            input: text.to_string(),
            voice: voice.unwrap_or("alloy").to_string(),
            speed: speed.unwrap_or(1.0),
            volume: 1.0,
            response_format: "wav".to_string(),
        };

        tracing::info!("BigModel TTS: synthesizing {} chars", text.len());

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
            tracing::error!("BigModel TTS error {}: {}", status, body);
            return Err(AiProviderError::Api(format!(
                "TTS API error {}: {}",
                status, body
            )));
        }

        let audio_bytes = response
            .bytes()
            .await
            .map_err(|e| AiProviderError::Request(e.to_string()))?;

        tracing::info!("BigModel TTS: generated {} bytes", audio_bytes.len());

        Ok(TtsResponse {
            audio_data: audio_bytes.to_vec(),
            format: "wav".to_string(),
            duration_ms: None,
        })
    }

    fn available_voices(&self) -> Vec<&'static str> {
        vec!["alloy", "echo", "fable", "onyx", "nova", "shimmer"]
    }
}

#[async_trait]
impl ChatService for BigModelClient {
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<String, AiProviderError> {
        let url = format!("{}/chat/completions", BIGMODEL_API_BASE);

        let request = ChatRequest {
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
        };

        tracing::info!("BigModel Chat: sending request with {} messages", request.messages.len());

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
            tracing::error!("BigModel Chat error {}: {}", status, body);
            return Err(AiProviderError::Api(format!(
                "Chat API error {}: {}",
                status, body
            )));
        }

        let chat_response: ChatResponseBody = response
            .json()
            .await
            .map_err(|e| AiProviderError::Parse(e.to_string()))?;

        let reply = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        tracing::info!("BigModel Chat: received {} chars response", reply.len());

        Ok(reply)
    }
}

#[async_trait]
impl AiProvider for BigModelClient {
    fn name(&self) -> &'static str {
        "bigmodel"
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
}

/// Legacy error type alias
pub type BigModelError = AiProviderError;
