use serde_json::{Value, json};

use crate::services::ai_provider::TtsResponse;
use crate::services::minimax::setting;

#[derive(Clone)]
pub struct VoiceStudioClient {
    http: reqwest::Client,
    endpoint: String,
    api_key: Option<String>,
    model: String,
}

impl VoiceStudioClient {
    pub fn from_env() -> Result<Option<Self>, String> {
        let Some(base) = setting("VOICESTUDIO_URL") else {
            return Ok(None);
        };
        let url = url::Url::parse(&base).map_err(|_| "VOICESTUDIO_URL is invalid")?;
        let loopback = url
            .host_str()
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1" | "127.1"));
        if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
            return Err("VoiceStudio remote connections require HTTPS".into());
        }
        let endpoint = format!(
            "{}/v1/audio/speech",
            base.trim_end_matches('/')
                .trim_end_matches("/v1/audio/speech")
        );
        Ok(Some(Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .build()
                .map_err(|_| "could not initialize VoiceStudio client")?,
            endpoint,
            api_key: setting("VOICESTUDIO_API_KEY"),
            model: setting("VOICESTUDIO_MODEL").unwrap_or_else(|| "omnivoice".into()),
        }))
    }

    pub async fn synthesize(&self, text: &str, voice: &str) -> Result<TtsResponse, String> {
        if text.is_empty() || text.chars().count() > 4096 || voice.is_empty() {
            return Err("VoiceStudio text and voice are required".into());
        }
        let request = self.http.post(&self.endpoint).json(&json!({
            "model": self.model,
            "input": text,
            "voice": voice,
            "response_format": "wav",
            "speed": 0.9,
        }));
        let request = if let Some(key) = &self.api_key {
            request.bearer_auth(key)
        } else {
            request
        };
        let response = request
            .send()
            .await
            .map_err(|_| "VoiceStudio request failed")?;
        if !response.status().is_success() {
            return Err(format!(
                "VoiceStudio returned HTTP {}; check the authorized voice and installed model",
                response.status().as_u16()
            ));
        }
        let mut bytes = Vec::new();
        let mut response = response;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "VoiceStudio audio response was interrupted")?
        {
            if bytes.len() + chunk.len() > 32 * 1024 * 1024 {
                return Err("VoiceStudio audio exceeds the 32 MiB limit".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.len() < 44 || &bytes[..4] != b"RIFF" {
            return Err("VoiceStudio did not return a WAV file".into());
        }
        Ok(TtsResponse {
            audio_data: bytes,
            format: "wav".into(),
            duration_ms: None,
        })
    }
}

pub fn runtime_status() -> Value {
    match VoiceStudioClient::from_env() {
        Ok(Some(_)) => json!({"configured":true,"endpoint":"OpenAI-compatible /v1/audio/speech"}),
        Ok(None) => json!({"configured":false}),
        Err(error) => json!({"configured":false,"message":error}),
    }
}
