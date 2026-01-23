//! AI Provider Abstraction Layer
//!
//! Defines traits for ASR, TTS, and Chat services that can be implemented
//! by different providers (BigModel, Doubao, etc.)

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
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

/// User input analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInputAnalysis {
    /// Language of user input: "en" | "zh" | "mix"
    pub use_lang: String,
    /// User text in English (original or translated)
    pub content_en: String,
    /// User text in Chinese (original or translated)
    pub content_zh: String,
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
    /// * `messages` - Chat history
    /// * `temperature` - Optional temperature (0.0 - 1.0)
    /// * `max_tokens` - Optional max tokens limit
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<String, AiProviderError>;

    /// Analyze user input: detect language and translate
    ///
    /// # Arguments
    /// * `user_text` - The user's message text to analyze
    async fn analyze_user_input(
        &self,
        user_text: &str,
    ) -> Result<UserInputAnalysis, AiProviderError> {
        // Default implementation: use LLM to detect language and translate
        let prompt = format!(
            r#"Analyze the following user input and respond in JSON format only:
{{
  "use_lang": "<en|zh|mix>",
  "content_en": "<text in English, translate if needed>",
  "content_zh": "<text in Chinese, translate if needed>"
}}

User input: {}"#,
            user_text
        );

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let response = self.chat(messages, Some(0.3), Some(500)).await?;

        // Try to parse JSON from response
        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        serde_json::from_str(json_str).map_err(|e| {
            tracing::warn!("Failed to parse user input analysis: {}, response: {}", e, response);
            // Fallback: assume English
            AiProviderError::Parse(format!("Failed to parse analysis: {}", e))
        })
    }

    /// Send chat completion with structured output (for English teaching)
    ///
    /// # Arguments
    /// * `messages` - Chat history
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
    /// Try to load Doubao configuration from environment variables
    fn try_doubao() -> Option<Self> {
        let app_id = std::env::var("DOUBAO_APP_ID").ok().filter(|s| !s.is_empty())?;
        let access_token = std::env::var("DOUBAO_ACCESS_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())?;
        let chat_api_key = std::env::var("DOUBAO_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())?;

        Some(ProviderConfig::Doubao {
            app_id,
            access_token,
            chat_api_key,
            chat_model: std::env::var("DOUBAO_CHAT_MODEL").ok(),
        })
    }

    /// Try to load BigModel configuration from environment variables
    fn try_bigmodel() -> Option<Self> {
        let api_key = std::env::var("BIGMODEL_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())?;

        Some(ProviderConfig::BigModel {
            api_key,
            asr_model: std::env::var("BIGMODEL_ASR_MODEL").ok(),
            tts_model: std::env::var("BIGMODEL_TTS_MODEL").ok(),
            chat_model: std::env::var("BIGMODEL_CHAT_MODEL").ok(),
        })
    }

    /// Load from environment variables
    ///
    /// Uses `AI_PROVIDER_DEFAULT` env var to determine which provider to use.
    /// Valid values: "doubao", "bigmodel"
    /// If not set or invalid, tries Doubao first, then BigModel.
    pub fn from_env() -> Option<Self> {
        let default_provider = std::env::var("AI_PROVIDER_DEFAULT")
            .ok()
            .map(|s| s.to_lowercase());

        match default_provider.as_deref() {
            Some("bigmodel") => {
                tracing::info!("AI_PROVIDER_DEFAULT=bigmodel, trying BigModel first");
                Self::try_bigmodel().or_else(Self::try_doubao)
            }
            Some("doubao") => {
                tracing::info!("AI_PROVIDER_DEFAULT=doubao, trying Doubao first");
                Self::try_doubao().or_else(Self::try_bigmodel)
            }
            Some(other) => {
                tracing::warn!(
                    "Unknown AI_PROVIDER_DEFAULT='{}', using default order (doubao first)",
                    other
                );
                Self::try_doubao().or_else(Self::try_bigmodel)
            }
            None => {
                // Default: try Doubao first, then BigModel
                Self::try_doubao().or_else(Self::try_bigmodel)
            }
        }
    }
}
