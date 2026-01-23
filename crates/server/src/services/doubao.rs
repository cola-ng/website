//! Doubao (豆包) Volcanic Engine AI Provider Implementation
//!
//! Integrates with Bytedance/Volcanic Engine APIs:
//! - OpenSpeech ASR: Speech-to-Text (语音识别)
//! - OpenSpeech TTS: Text-to-Speech (语音合成)
//! - Ark Chat: Chat completion (对话生成)

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use super::ai_provider::{
    AiProvider, AiProviderError, AsrResponse, AsrService, ChatMessage, ChatService,
    PronunciationAnalysis, PronunciationService, StructuredChatResponse, TextIssue, TtsResponse,
    TtsService, WordPronunciationScore, WordTiming,
};

const DOUBAO_SPEECH_API_BASE: &str = "https://openspeech.bytedance.com/api/v1";
const DOUBAO_CHAT_API_BASE: &str = "https://ark.cn-beijing.volces.com/api/v3";
const DEFAULT_CHAT_MODEL: &str = "doubao-seed-1-8-251228";

/// Doubao API Client
#[derive(Debug, Clone)]
pub struct DoubaoClient {
    app_id: String,
    access_token: String,
    chat_api_key: String,
    chat_model: String,
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

#[derive(Debug, Serialize)]
struct AsrAudioInfo {
    format: String,
    rate: u32,
    language: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct AsrApiRequest {
    app: SpeechAppInfo,
    user: SpeechUserInfo,
    audio: AsrAudioInfo,
}

#[derive(Debug, Serialize)]
struct TtsAudioConfig {
    voice_type: String,
    encoding: String,
    speed_ratio: f32,
    volume_ratio: f32,
    pitch_ratio: f32,
    sample_rate: u32,
}

#[derive(Debug, Serialize)]
struct TtsRequestInfo {
    text: String,
    operation: String,
}

#[derive(Debug, Serialize)]
struct TtsApiRequest {
    app: SpeechAppInfo,
    user: SpeechUserInfo,
    audio: TtsAudioConfig,
    request: TtsRequestInfo,
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
    pub fn new(app_id: String, access_token: String, chat_api_key: String) -> Self {
        Self::with_model(app_id, access_token, chat_api_key, None)
    }

    pub fn with_model(
        app_id: String,
        access_token: String,
        chat_api_key: String,
        chat_model: Option<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            app_id,
            access_token,
            chat_api_key,
            chat_model: chat_model.unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string()),
            client,
        }
    }

    /// Get client from environment variables
    pub fn from_env() -> Option<Self> {
        let app_id = std::env::var("DOUBAO_APP_ID").ok().filter(|s| !s.is_empty())?;
        let access_token = std::env::var("DOUBAO_ACCESS_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())?;
        let chat_api_key = std::env::var("DOUBAO_CHAT_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())?;
        let chat_model = std::env::var("DOUBAO_CHAT_MODEL").ok();

        Some(Self::with_model(app_id, access_token, chat_api_key, chat_model))
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
}

#[async_trait]
impl AsrService for DoubaoClient {
    async fn transcribe(
        &self,
        audio_data: Vec<u8>,
        language: Option<&str>,
    ) -> Result<AsrResponse, AiProviderError> {
        let url = format!("{}/asr", DOUBAO_SPEECH_API_BASE);

        let audio_base64 = BASE64.encode(&audio_data);
        let format = Self::detect_format(&audio_data);
        let sample_rate = Self::detect_sample_rate(&audio_data);

        let request = AsrApiRequest {
            app: self.speech_app_info(),
            user: Self::speech_user_info(),
            audio: AsrAudioInfo {
                format: format.to_string(),
                rate: sample_rate,
                language: language.unwrap_or("auto").to_string(),
                data: audio_base64,
            },
        };

        tracing::info!("Doubao ASR: sending request to {}", url);

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
            tracing::error!("Doubao ASR error {}: {}", status, body);
            return Err(AiProviderError::Api(format!(
                "ASR API error {}: {}",
                status, body
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AiProviderError::Parse(e.to_string()))?;

        // Parse Doubao response format
        let text = result["result"]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let confidence = result["result"]["confidence"].as_f64().map(|c| c as f32);

        let words = result["result"]["words"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|w| {
                    Some(WordTiming {
                        word: w["word"].as_str()?.to_string(),
                        start_time: w["start_time"].as_f64()?,
                        end_time: w["end_time"].as_f64()?,
                        confidence: w["confidence"].as_f64().map(|c| c as f32),
                    })
                })
                .collect()
        });

        tracing::info!("Doubao ASR: transcribed text: {}", text);

        Ok(AsrResponse {
            text,
            confidence,
            words,
        })
    }

    fn supported_formats(&self) -> Vec<&'static str> {
        vec!["wav", "mp3", "pcm"]
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
        let url = format!("{}/tts", DOUBAO_SPEECH_API_BASE);

        let request = TtsApiRequest {
            app: self.speech_app_info(),
            user: Self::speech_user_info(),
            audio: TtsAudioConfig {
                voice_type: voice.unwrap_or("zh_female_shuangkuaisisi_moon_bigtts").to_string(),
                encoding: "wav".to_string(),
                speed_ratio: speed.unwrap_or(1.0),
                volume_ratio: 1.0,
                pitch_ratio: 1.0,
                sample_rate: 24000,
            },
            request: TtsRequestInfo {
                text: text.to_string(),
                operation: "submit".to_string(),
            },
        };

        tracing::info!("Doubao TTS: synthesizing {} chars", text.len());

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
            tracing::error!("Doubao TTS error {}: {}", status, body);
            return Err(AiProviderError::Api(format!(
                "TTS API error {}: {}",
                status, body
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AiProviderError::Parse(e.to_string()))?;

        let audio_base64 = result["data"]
            .as_str()
            .ok_or_else(|| AiProviderError::Parse("Missing audio data in response".to_string()))?;

        let audio_data = BASE64
            .decode(audio_base64)
            .map_err(|e| AiProviderError::Parse(format!("Failed to decode audio: {}", e)))?;

        let duration_ms = result["duration"].as_u64();

        tracing::info!("Doubao TTS: generated {} bytes", audio_data.len());

        Ok(TtsResponse {
            audio_data,
            format: "wav".to_string(),
            duration_ms,
        })
    }

    fn available_voices(&self) -> Vec<&'static str> {
        vec![
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

        tracing::info!(
            "Doubao Chat: sending request with {} messages",
            request.messages.len()
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.chat_api_key))
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
            .header("Authorization", format!("Bearer {}", self.chat_api_key))
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
        let structured: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| AiProviderError::Parse(format!("Failed to parse structured response: {}", e)))?;

        let use_lang = structured["use_lang"]
            .as_str()
            .unwrap_or("en")
            .to_string();
        let original_en = structured["original_en"]
            .as_str()
            .unwrap_or(user_text)
            .to_string();
        let original_zh = structured["original_zh"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let reply_en = structured["reply_en"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let reply_zh = structured["reply_zh"]
            .as_str()
            .unwrap_or("")
            .to_string();

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

        let overall_score = result["result"]["overall_score"]
            .as_f64()
            .unwrap_or(0.0) as f32;
        let fluency_score = result["result"]["fluency_score"]
            .as_f64()
            .unwrap_or(0.0) as f32;
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

        tracing::info!(
            "Doubao Pronunciation: overall score {}",
            overall_score
        );

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
