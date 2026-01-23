//! AI Provider Abstraction Layer
//!
//! Defines traits for ASR, TTS, and Chat services that can be implemented
//! by different providers (BigModel, Doubao, etc.)

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Error type for AI provider operations
#[derive(Debug, thiserror::Error)]
pub enum AiProviderError {
    #[error("Request error: {0}")]
    Request(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Not supported: {0}")]
    NotSupported(String),
}

impl From<AiProviderError> for salvo::http::StatusError {
    fn from(e: AiProviderError) -> Self {
        salvo::http::StatusError::internal_server_error().brief(e.to_string())
    }
}

/// Chat message for LLM interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// ASR (Speech-to-Text) response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrResponse {
    pub text: String,
    /// Optional confidence score (0.0 - 1.0)
    pub confidence: Option<f32>,
    /// Optional word-level timings
    pub words: Option<Vec<WordTiming>>,
}

/// Word timing information from ASR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTiming {
    pub word: String,
    pub start_time: f64,
    pub end_time: f64,
    pub confidence: Option<f32>,
}

/// TTS (Text-to-Speech) response
#[derive(Debug, Clone)]
pub struct TtsResponse {
    pub audio_data: Vec<u8>,
    /// Audio format (wav, mp3, etc.)
    pub format: String,
    /// Optional duration in milliseconds
    pub duration_ms: Option<u64>,
}

/// Voice chat combined response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChatResponse {
    pub user_text: String,
    pub ai_text: String,
    pub ai_audio_base64: String,
}

/// Pronunciation analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PronunciationAnalysis {
    pub overall_score: f32,
    pub fluency_score: f32,
    pub pronunciation_score: f32,
    pub completeness_score: f32,
    pub word_scores: Vec<WordPronunciationScore>,
}

/// Word-level pronunciation score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordPronunciationScore {
    pub word: String,
    pub score: f32,
    pub error_type: Option<String>,
    pub suggestion: Option<String>,
}

/// ASR (Speech-to-Text) Service Trait
#[async_trait]
pub trait AsrService: Send + Sync {
    /// Transcribe audio to text
    ///
    /// # Arguments
    /// * `audio_data` - Raw audio bytes (WAV or MP3 format)
    /// * `language` - Optional language hint (e.g., "en", "zh", "auto")
    async fn transcribe(
        &self,
        audio_data: Vec<u8>,
        language: Option<&str>,
    ) -> Result<AsrResponse, AiProviderError>;

    /// Get supported audio formats
    fn supported_formats(&self) -> Vec<&'static str> {
        vec!["wav", "mp3"]
    }
}

/// TTS (Text-to-Speech) Service Trait
#[async_trait]
pub trait TtsService: Send + Sync {
    /// Synthesize text to audio
    ///
    /// # Arguments
    /// * `text` - Text to synthesize
    /// * `voice` - Optional voice ID
    /// * `speed` - Optional speed ratio (0.5 - 2.0)
    async fn synthesize(
        &self,
        text: &str,
        voice: Option<&str>,
        speed: Option<f32>,
    ) -> Result<TtsResponse, AiProviderError>;

    /// Get available voices
    fn available_voices(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}

/// Structured AI response for English teaching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredChatResponse {
    /// Language of user input: "en" | "zh" | "mix"
    pub use_lang: String,
    /// User text in English (original or translated)
    pub original_en: String,
    /// User text in Chinese (original or translated)
    pub original_zh: String,
    /// AI reply in English
    pub reply_en: String,
    /// AI reply in Chinese
    pub reply_zh: String,
    /// Grammar/word choice issues found
    pub issues: Vec<TextIssue>,
}

/// Text issue (grammar, word choice, or suggestion)
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Chat (LLM) Service Trait
#[async_trait]
pub trait ChatService: Send + Sync {
    /// Send chat completion request
    ///
    /// # Arguments
    /// * `messages` - Conversation history
    /// * `temperature` - Optional temperature (0.0 - 1.0)
    /// * `max_tokens` - Optional max tokens limit
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<String, AiProviderError>;

    /// Send chat completion with structured output (for English teaching)
    ///
    /// # Arguments
    /// * `messages` - Conversation history
    /// * `user_text` - The user's latest message text
    /// * `system_prompt` - System prompt for the AI
    async fn chat_structured(
        &self,
        messages: Vec<ChatMessage>,
        user_text: &str,
        system_prompt: &str,
    ) -> Result<StructuredChatResponse, AiProviderError> {
        // Default implementation falls back to regular chat
        let _ = user_text;
        let mut all_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        }];
        all_messages.extend(messages);

        let reply = self.chat(all_messages, Some(0.7), None).await?;

        Ok(StructuredChatResponse {
            use_lang: "en".to_string(),
            original_en: user_text.to_string(),
            original_zh: String::new(),
            reply_en: reply,
            reply_zh: String::new(),
            issues: vec![],
        })
    }
}

/// Pronunciation Assessment Service Trait (optional capability)
#[async_trait]
pub trait PronunciationService: Send + Sync {
    /// Analyze pronunciation quality
    ///
    /// # Arguments
    /// * `audio_data` - Raw audio bytes
    /// * `reference_text` - Expected text
    /// * `language` - Language code
    async fn analyze_pronunciation(
        &self,
        audio_data: Vec<u8>,
        reference_text: &str,
        language: &str,
    ) -> Result<PronunciationAnalysis, AiProviderError>;
}

/// Combined AI Provider with all capabilities
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Provider name (e.g., "bigmodel", "doubao")
    fn name(&self) -> &'static str;

    /// Get ASR service
    fn asr(&self) -> Option<Arc<dyn AsrService>>;

    /// Get TTS service
    fn tts(&self) -> Option<Arc<dyn TtsService>>;

    /// Get Chat service
    fn chat_service(&self) -> Option<Arc<dyn ChatService>>;

    /// Get Pronunciation service (optional)
    fn pronunciation(&self) -> Option<Arc<dyn PronunciationService>> {
        None
    }

    /// Convenience method: Complete voice chat pipeline
    async fn voice_chat(
        &self,
        audio_data: Vec<u8>,
        conversation_history: Vec<ChatMessage>,
        system_prompt: Option<String>,
    ) -> Result<VoiceChatResponse, AiProviderError> {
        let asr = self.asr().ok_or_else(|| {
            AiProviderError::NotSupported("ASR not available for this provider".to_string())
        })?;
        let chat = self.chat_service().ok_or_else(|| {
            AiProviderError::NotSupported("Chat not available for this provider".to_string())
        })?;
        let tts = self.tts().ok_or_else(|| {
            AiProviderError::NotSupported("TTS not available for this provider".to_string())
        })?;

        // Step 1: Transcribe user audio
        let asr_result = asr.transcribe(audio_data, Some("auto")).await?;
        let user_text = asr_result.text.clone();

        if user_text.trim().is_empty() {
            return Err(AiProviderError::Api(
                "Could not transcribe audio - no speech detected".to_string(),
            ));
        }

        // Step 2: Build messages for chat
        let mut messages = Vec::new();

        if let Some(system) = system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system,
            });
        }

        messages.extend(conversation_history);
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_text.clone(),
        });

        // Generate AI response
        let ai_text = chat.chat(messages, Some(0.7), None).await?;

        // Step 3: Synthesize AI response to audio
        let tts_result = tts.synthesize(&ai_text, None, None).await?;
        let ai_audio_base64 = BASE64.encode(&tts_result.audio_data);

        Ok(VoiceChatResponse {
            user_text,
            ai_text,
            ai_audio_base64,
        })
    }
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider")]
pub enum ProviderConfig {
    #[serde(rename = "bigmodel")]
    BigModel {
        api_key: String,
        #[serde(default)]
        asr_model: Option<String>,
        #[serde(default)]
        tts_model: Option<String>,
        #[serde(default)]
        chat_model: Option<String>,
    },
    #[serde(rename = "doubao")]
    Doubao {
        app_id: String,
        access_token: String,
        chat_api_key: String,
        #[serde(default)]
        chat_model: Option<String>,
    },
}

impl ProviderConfig {
    /// Load from environment variables
    pub fn from_env() -> Option<Self> {
        // Try BigModel first
        if let Ok(api_key) = std::env::var("BIGMODEL_API_KEY") {
            if !api_key.is_empty() {
                return Some(ProviderConfig::BigModel {
                    api_key,
                    asr_model: std::env::var("BIGMODEL_ASR_MODEL").ok(),
                    tts_model: std::env::var("BIGMODEL_TTS_MODEL").ok(),
                    chat_model: std::env::var("BIGMODEL_CHAT_MODEL").ok(),
                });
            }
        }

        // Try Doubao
        if let (Ok(app_id), Ok(access_token), Ok(chat_api_key)) = (
            std::env::var("DOUBAO_APP_ID"),
            std::env::var("DOUBAO_ACCESS_TOKEN"),
            std::env::var("DOUBAO_CHAT_API_KEY"),
        ) {
            if !app_id.is_empty() && !access_token.is_empty() && !chat_api_key.is_empty() {
                return Some(ProviderConfig::Doubao {
                    app_id,
                    access_token,
                    chat_api_key,
                    chat_model: std::env::var("DOUBAO_CHAT_MODEL").ok(),
                });
            }
        }

        None
    }
}
