//! Zhipu (智谱) Provider - wraps outfox-zhipu crate
//!
//! Uses the outfox-zhipu crate for:
//! - Chat completions via GLM models
//! - ASR (Speech-to-Text) via GLM-ASR
//! - TTS (Text-to-Speech) via GLM-TTS

use std::sync::Arc;

use async_trait::async_trait;
use outfox_zhipu::Client as ZhipuSdkClient;
use outfox_zhipu::config::ZhipuConfig;
use outfox_zhipu::spec::asr::AudioInput;
use outfox_zhipu::spec::chat::{
    ChatMessage as ZhipuChatMessage, CreateChatCompletionRequestArgs, ResponseFormat,
};
use outfox_zhipu::spec::tts::CreateSpeechRequest;

use super::ai_provider::{
    AiProvider, AiProviderError, AsrResponse, AsrService, ChatMessage, ChatService,
    StructuredChatResponse, TextIssue, TtsResponse, TtsService,
};

const DEFAULT_CHAT_MODEL: &str = "glm-4-flash";

/// Zhipu client wrapper around outfox-zhipu crate
#[derive(Debug, Clone)]
pub struct ZhipuClient {
    client: ZhipuSdkClient,
    chat_model: String,
}

impl ZhipuClient {
    /// Create a new Zhipu client with API key
    pub fn new(api_key: String) -> Self {
        Self::with_models(api_key, None)
    }

    /// Create a new Zhipu client with custom model
    pub fn with_models(api_key: String, chat_model: Option<String>) -> Self {
        let config = ZhipuConfig::new().with_api_key(&api_key);

        Self {
            client: ZhipuSdkClient::with_config(config),
            chat_model: chat_model.unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string()),
        }
    }

    /// Create client from environment variables
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("ZHIPU_API_KEY")
            .or_else(|_| std::env::var("ZHIPUAI_API_KEY"))
            .ok()
            .filter(|s| !s.is_empty())?;
        let chat_model = std::env::var("ZHIPU_CHAT_MODEL").ok();

        Some(Self::with_models(api_key, chat_model))
    }

    /// Convert internal ChatMessage to Zhipu ChatMessage
    fn to_zhipu_message(msg: &ChatMessage) -> ZhipuChatMessage {
        match msg.role.as_str() {
            "system" => ZhipuChatMessage::system(&msg.content),
            "assistant" => ZhipuChatMessage::assistant(&msg.content),
            _ => ZhipuChatMessage::user(&msg.content),
        }
    }
}

#[async_trait]
impl AsrService for ZhipuClient {
    async fn transcribe(
        &self,
        audio_data: Vec<u8>,
        _language: Option<&str>,
    ) -> Result<AsrResponse, AiProviderError> {
        tracing::info!(
            "Zhipu ASR: transcribing {} bytes of audio",
            audio_data.len()
        );

        let audio = AudioInput::from_bytes(audio_data, "audio.wav");
        let request = outfox_zhipu::spec::asr::CreateTranscriptionRequest {
            audio: Some(audio),
            ..Default::default()
        };

        let response = self
            .client
            .asr()
            .transcribe(request)
            .await
            .map_err(|e| AiProviderError::Api(e.to_string()))?;

        tracing::info!("Zhipu ASR: transcribed text: {}", response.text);

        Ok(AsrResponse {
            text: response.text,
            confidence: None,
            words: None,
        })
    }

    fn supported_formats(&self) -> Vec<&'static str> {
        vec!["wav", "mp3", "m4a", "flac", "ogg"]
    }
}

#[async_trait]
impl TtsService for ZhipuClient {
    async fn synthesize(
        &self,
        text: &str,
        _voice: Option<&str>,
        speed: Option<f32>,
    ) -> Result<TtsResponse, AiProviderError> {
        tracing::info!("Zhipu TTS: synthesizing {} chars", text.len());

        let mut request = CreateSpeechRequest {
            input: text.to_string(),
            ..Default::default()
        };

        if let Some(s) = speed {
            request.speed = Some(s);
        }

        let response = self
            .client
            .tts()
            .create(request)
            .await
            .map_err(|e| AiProviderError::Api(e.to_string()))?;

        tracing::info!(
            "Zhipu TTS: generated {} bytes of audio",
            response.audio.len()
        );

        Ok(TtsResponse {
            audio_data: response.audio.to_vec(),
            format: "wav".to_string(),
            duration_ms: None,
        })
    }

    fn available_voices(&self) -> Vec<&'static str> {
        vec![
            "tongtong", "chuichui", "xiaochen", "jam", "kazi", "douji", "luodo",
        ]
    }
}

#[async_trait]
impl ChatService for ZhipuClient {
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<String, AiProviderError> {
        let zhipu_messages: Vec<ZhipuChatMessage> =
            messages.iter().map(Self::to_zhipu_message).collect();

        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder
            .model(&self.chat_model)
            .messages(zhipu_messages);

        if let Some(temp) = temperature {
            request_builder.temperature(temp);
        }
        if let Some(max) = max_tokens {
            request_builder.max_tokens(max);
        }

        let request = request_builder
            .build()
            .map_err(|e| AiProviderError::Config(e.to_string()))?;

        tracing::info!(
            "Zhipu Chat: sending request with {} messages",
            messages.len()
        );

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| AiProviderError::Api(e.to_string()))?;

        let reply = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        tracing::info!("Zhipu Chat: received {} chars response", reply.len());

        Ok(reply)
    }

    async fn chat_structured(
        &self,
        messages: Vec<ChatMessage>,
        user_text: &str,
        system_prompt: &str,
    ) -> Result<StructuredChatResponse, AiProviderError> {
        let mut all_messages = vec![ChatMessage {
            role: "system".to_owned(),
            content: system_prompt.to_owned(),
        }];
        all_messages.extend(messages);

        // Convert to Zhipu messages
        let zhipu_messages: Vec<ZhipuChatMessage> =
            all_messages.iter().map(Self::to_zhipu_message).collect();

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.chat_model)
            .messages(zhipu_messages)
            .temperature(0.7f32)
            .max_tokens(2000u32)
            .response_format(ResponseFormat::json_object())
            .build()
            .map_err(|e| AiProviderError::Config(e.to_string()))?;

        tracing::info!(
            "Zhipu Chat Structured: sending request for user text: {}",
            user_text
        );

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| AiProviderError::Api(e.to_string()))?;

        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        tracing::debug!("Zhipu Chat Structured response: {}", content);

        // Parse JSON response
        let json_str = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        println!("Parsed JSON string: {}", json_str);

        let structured: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "Failed to parse structured response as JSON: {}, content: {}",
                    e,
                    content
                );
                // Fallback: detect if user_text is Chinese or English
                let is_chinese = user_text
                    .chars()
                    .any(|c| c >= '\u{4e00}' && c <= '\u{9fff}');
                return Ok(StructuredChatResponse {
                    use_lang: if is_chinese { "zh" } else { "en" }.to_string(),
                    original_en: if is_chinese {
                        String::new()
                    } else {
                        user_text.to_string()
                    },
                    original_zh: if is_chinese {
                        user_text.to_string()
                    } else {
                        String::new()
                    },
                    reply_en: content.clone(),
                    reply_zh: String::new(),
                    issues: vec![],
                });
            }
        };

        let use_lang = structured["use_lang"].as_str().unwrap_or("en").to_string();

        // Detect if user_text is Chinese for fallback handling
        let is_chinese = user_text
            .chars()
            .any(|c| c >= '\u{4e00}' && c <= '\u{9fff}');

        let original_en = structured["original_en"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                if is_chinese {
                    String::new()
                } else {
                    user_text.to_string()
                }
            });
        let original_zh = structured["original_zh"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                if is_chinese {
                    user_text.to_string()
                } else {
                    String::new()
                }
            });
        let reply_en = structured["reply_en"].as_str().unwrap_or("").to_string();
        let reply_zh = structured["reply_zh"].as_str().unwrap_or("").to_string();

        tracing::debug!(
            "Zhipu parsed: use_lang={}, original_en={}, original_zh={}, reply_en_len={}, reply_zh_len={}",
            use_lang,
            original_en,
            original_zh,
            reply_en.len(),
            reply_zh.len()
        );

        let issues: Vec<TextIssue> =
            serde_json::from_value(structured["issues"].clone()).unwrap_or_default();

        tracing::info!(
            "Zhipu Chat Structured: reply_en={} chars, issues={}",
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
impl AiProvider for ZhipuClient {
    fn name(&self) -> &'static str {
        "zhipu"
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
