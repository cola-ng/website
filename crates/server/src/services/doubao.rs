//! Doubao (豆包) Provider - wraps outfox-doubao crate for ASR/TTS
//!
//! Uses the outfox-doubao crate for:
//! - TTS (Text-to-Speech) via WebSocket bidirectional API
//! - ASR (Speech-to-Text) via BigModel flash recognition API

use async_trait::async_trait;
use std::sync::Arc;

use outfox_doubao::{
    config::DoubaoConfig,
    spec::tts::CreateSpeechRequestArgs,
    Client as DoubaoSdkClient,
};

use super::ai_provider::{
    AiProvider, AiProviderError, AsrResponse, AsrService, ChatService, TtsResponse, TtsService,
    WordTiming,
};

/// Doubao client wrapper around outfox-doubao crate
#[derive(Debug, Clone)]
pub struct DoubaoClient {
    client: DoubaoSdkClient,
}

impl DoubaoClient {
    /// Create a new Doubao client with the given credentials
    pub fn new(app_id: String, access_token: String, api_key: String) -> Self {
        Self::with_options(app_id, access_token, api_key, None)
    }

    /// Create a new Doubao client with options
    pub fn with_options(
        app_id: String,
        access_token: String,
        api_key: String,
        tts_resource_id: Option<String>,
    ) -> Self {
        let config = DoubaoConfig::new()
            .with_app_id(&app_id)
            .with_access_token(&access_token)
            .with_api_key(&api_key)
            .with_resource_id(&tts_resource_id.unwrap_or_else(|| "seed-tts-2.0".to_string()));

        Self {
            client: DoubaoSdkClient::with_config(config),
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

        Some(Self::with_options(
            app_id,
            access_token,
            api_key,
            tts_resource_id,
        ))
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
        let speaker = voice
            .unwrap_or("zh_female_cancan_mars_bigtts")
            .to_string();

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
            "en_female_amanda_moon_bigtts",
            "en_male_adam_moon_bigtts",
        ]
    }
}

// Note: Doubao Chat is not implemented in outfox-doubao crate
// Use ZhipuClient for Chat service instead

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
        // Chat not available in outfox-doubao crate
        None
    }
}

/// Legacy error type alias
pub type DoubaoError = AiProviderError;
