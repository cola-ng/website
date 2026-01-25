//! Doubao (豆包) Provider - wraps outfox-doubao crate
//!
//! Uses the outfox-doubao crate for:
//! - TTS (Text-to-Speech) via WebSocket bidirectional API
//! - ASR (Speech-to-Text) via BigModel flash recognition API
//! - Chat completions via Ark API

use std::sync::Arc;

use async_trait::async_trait;
use outfox_doubao::Client as DoubaoSdkClient;
use outfox_doubao::config::DoubaoConfig;
use outfox_doubao::spec::chat::{
    ChatMessage as DoubaoChatMessage, CreateChatCompletionRequestArgs, ResponseFormat,
    ResponseFormatJsonSchema, ResponseFormatType,
};
use outfox_doubao::spec::tts::CreateSpeechRequestArgs;
use outfox_zhipu::spec::voice;
use serde_json::json;

use super::ai_provider::{
    AiProvider, AiProviderError, AsrResponse, AsrService, ChatMessage, ChatService,
    StructuredChatResponse, TextIssue, TtsResponse, TtsService, WordTiming,
};

const DEFAULT_CHAT_MODEL: &str = "doubao-1-5-pro-32k-250115";

/// Doubao client wrapper around outfox-doubao crate
#[derive(Debug, Clone)]
pub struct DoubaoClient {
    client: DoubaoSdkClient,
    chat_model: String,
}

impl DoubaoClient {
    /// Create a new Doubao client with the given credentials
    pub fn new(app_id: String, access_token: String, api_key: String) -> Self {
        Self::with_options(app_id, access_token, api_key, None, None, None)
    }

    /// Create a new Doubao client with options
    pub fn with_options(
        app_id: String,
        access_token: String,
        api_key: String,
        tts_resource_id: Option<String>,
        chat_model: Option<String>,
        voice_type: Option<String>,
    ) -> Self {
        let config = DoubaoConfig::new()
            .with_app_id(&app_id)
            .with_access_token(&access_token)
            .with_api_key(&api_key)
            .with_resource_id(&tts_resource_id.unwrap_or_else(|| "seed-tts-2.0".to_string()))
            .with_voice_type(voice_type.unwrap_or_else(|| "zh_female_vv_uranus_bigtts".to_string()));

        Self {
            client: DoubaoSdkClient::with_config(config),
            chat_model: chat_model.unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string()),
        }
    }

    /// Create client from environment variables
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
        let tts_resource_id = std::env::var("DOUBAO_RESOURCE_ID").ok();
        let voice_type = std::env::var("DOUBAO_VOICE_TYPE").ok();
        let chat_model = std::env::var("DOUBAO_CHAT_MODEL").ok();

        Some(Self::with_options(
            app_id,
            access_token,
            api_key,
            tts_resource_id,
            chat_model,
            voice_type,
        ))
    }

    /// Convert internal ChatMessage to Doubao ChatMessage
    fn to_doubao_message(msg: &ChatMessage) -> DoubaoChatMessage {
        match msg.role.as_str() {
            "system" => DoubaoChatMessage::system(msg.content.as_str()),
            "assistant" => DoubaoChatMessage::assistant(msg.content.as_str()),
            _ => DoubaoChatMessage::user(msg.content.as_str()),
        }
    }
}

#[async_trait]
impl AsrService for DoubaoClient {
    async fn transcribe(
        &self,
        audio_data: Vec<u8>,
        _language: Option<&str>,
    ) -> Result<AsrResponse, AiProviderError> {
        let user_id = uuid::Uuid::new_v4().to_string();

        tracing::info!(
            "Doubao ASR: transcribing {} bytes of audio",
            audio_data.len()
        );

        let response = self
            .client
            .asr()
            .recognition()
            .flash_bytes(&audio_data, &user_id)
            .await
            .map_err(|e| AiProviderError::Api(e.to_string()))?;

        let text = response.result.text.clone();
        println!("Doubao ASR transcribed text  1: {:#?}", response);
        println!("Doubao ASR text 2: {}", text);

        // Extract word timings from utterances
        let words = if !response.result.utterances.is_empty() {
            Some(
                response
                    .result
                    .utterances
                    .iter()
                    .flat_map(|u| {
                        u.words.iter().map(|w| WordTiming {
                            word: w.text.clone(),
                            start_time: w.start_time as f64 / 1000.0,
                            end_time: w.end_time as f64 / 1000.0,
                            confidence: Some(w.confidence),
                        })
                    })
                    .collect(),
            )
        } else {
            None
        };

        tracing::info!("Doubao ASR: transcribed text: {}", text);

        Ok(AsrResponse {
            text,
            confidence: None,
            words,
        })
    }

    fn supported_formats(&self) -> Vec<&'static str> {
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
        let speaker = voice.unwrap_or("zh_female_cancan_mars_bigtts").to_string();

        // Convert speed ratio to API format: speed 1.0 = 0, range [-50, 100]
        let speech_rate = speed
            .map(|s| ((s - 1.0) * 100.0) as i32)
            .unwrap_or(0)
            .clamp(-50, 100);

        tracing::info!(
            "Doubao TTS: synthesizing {} chars with voice={}, speed={}",
            text.len(),
            speaker,
            speech_rate
        );

        let request = CreateSpeechRequestArgs::default()
            .text(text)
            .speaker(speaker)
            .speech_rate(speech_rate)
            .sample_rate(24000u32)
            .build()
            .map_err(|e| AiProviderError::Config(e.to_string()))?;

        let response = self
            .client
            .tts()
            .speech()
            .create(request)
            .await
            .map_err(|e| AiProviderError::Api(e.to_string()))?;

        tracing::info!(
            "Doubao TTS: generated {} bytes of audio",
            response.bytes.len()
        );

        Ok(TtsResponse {
            audio_data: response.bytes.to_vec(),
            format: "mp3".to_string(),
            duration_ms: None,
        })
    }

    fn available_voices(&self) -> Vec<&'static str> {
        vec![
            "zh_female_cancan_mars_bigtts",
            "zh_female_shuangkuaisisi_moon_bigtts",
            "zh_male_aojiaobazong_moon_bigtts",
            "zh_female_tianmeixiaoyuan_moon_bigtts",
            "zh_male_wennuanahu_moon_bigtts",
            "zh_female_vv_uranus_bigtts",
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
        let doubao_messages: Vec<DoubaoChatMessage> =
            messages.iter().map(Self::to_doubao_message).collect();

        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder
            .model(&self.chat_model)
            .messages(doubao_messages);

        if let Some(temp) = temperature {
            request_builder.temperature(temp);
        }
        if let Some(max) = max_tokens {
            request_builder.max_tokens(max as i32);
        }

        let request = request_builder
            .build()
            .map_err(|e| AiProviderError::Config(e.to_string()))?;

        tracing::info!(
            "Doubao Chat: sending request with {} messages to model {}",
            messages.len(),
            self.chat_model
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
            .and_then(|c| c.message.content.as_ref())
            .map(|c| match c {
                outfox_doubao::spec::chat::MessageContent::Text(s) => s.clone(),
                outfox_doubao::spec::chat::MessageContent::Parts(parts) => parts
                    .iter()
                    .filter_map(|p| p.text.clone())
                    .collect::<Vec<_>>()
                    .join(""),
            })
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
        // Build enhanced system prompt that requests JSON output
        let enhanced_system_prompt = format!(
            "{}\n\n\
            IMPORTANT: You must respond with a JSON object containing:\n\
            1. A natural conversational reply to the user\n\
            2. Grammar/vocabulary analysis of the user's last message\n\n\
            Response JSON format:\n\
            {{\n\
              \"use_lang\": \"<en|zh|mix>\",\n\
              \"original_en\": \"<the last user message or translation in English>\",\n\
              \"original_zh\": \"<the last user message or translation in Chinese>\",\n\
              \"reply_en\": \"<your reply in English>\",\n\
              \"reply_zh\": \"<your reply in Chinese>\",\n\
              \"issues\": [\n\
                {{\n\
                  \"type\": \"grammar|word_choice|suggestion\",\n\
                  \"original\": \"<problematic text>\",\n\
                  \"suggested\": \"<corrected text>\",\n\
                  \"description_en\": \"<explanation in English>\",\n\
                  \"description_zh\": \"<explanation in Chinese>\",\n\
                  \"severity\": \"low|medium|high\"\n\
                }}\n\
              ]\n\
            }}",
            system_prompt
        );

        let mut all_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: enhanced_system_prompt,
        }];
        all_messages.extend(messages);
        all_messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_text.to_string(),
        });

        println!("Doubao Chat Structured: total messages: {:#?}", all_messages);

        // Convert to Doubao messages
        let doubao_messages: Vec<DoubaoChatMessage> =
            all_messages.iter().map(Self::to_doubao_message).collect();

        // JSON Schema for structured output
        let response_schema = json!({
            "type": "object",
            "properties": {
                "use_lang": {
                    "type": "string",
                    "description": "The language of the original text, either 'en' for English, 'zh' for Chinese, or 'mix' for mixed. ONLY contains issues if this value is 'en'."
                },
                "original_en": {
                    "type": "string",
                    "description": "The last user message or translation in English. If the user wrote in Chinese or mixed language, translate it to English here."
                },
                "original_zh": {
                    "type": "string",
                    "description": "The last user message or translation in Chinese. If the user wrote in English or mixed language, translate it to Chinese here."
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
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.chat_model)
            .messages(doubao_messages)
            .temperature(0.7f32)
            .max_tokens(2000i32)
            .response_format(ResponseFormat {
                format_type: ResponseFormatType::JsonObject,
                json_schema: Some(ResponseFormatJsonSchema {
                    name: "english_teacher_response".to_owned(),
                    strict: Some(true),
                    schema: response_schema,
                    description: Some(
                        "Structured response for English learning with corrections".to_owned(),
                    ),
                }),
            })
            .build()
            .map_err(|e| AiProviderError::Config(e.to_string()))?;

        tracing::info!(
            "Doubao Chat Structured: sending request for user text: {}",
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
            .and_then(|c| c.message.content.as_ref())
            .map(|c| match c {
                outfox_doubao::spec::chat::MessageContent::Text(s) => s.clone(),
                outfox_doubao::spec::chat::MessageContent::Parts(parts) => parts
                    .iter()
                    .filter_map(|p| p.text.clone())
                    .collect::<Vec<_>>()
                    .join(""),
            })
            .unwrap_or_default();

        tracing::debug!("Doubao Chat Structured response: {}", content);

        // Parse JSON response
        let json_str = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        // Try to parse as JSON, with fallback for plain text responses
        let structured: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "Failed to parse structured response as JSON: {}, content: {}",
                    e,
                    content
                );
                // Fallback: treat the response as a plain text reply
                return Ok(StructuredChatResponse {
                    use_lang: "en".to_string(),
                    original_en: user_text.to_string(),
                    original_zh: String::new(),
                    reply_en: content.clone(),
                    reply_zh: String::new(),
                    issues: vec![],
                });
            }
        };

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
            "Doubao Chat Structured: reply_en={} chars, issues={}",
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
}
