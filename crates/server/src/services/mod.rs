//! AI Services module
//!
//! This module provides AI services using outfox crates:
//! - outfox-doubao for ASR, TTS, and Chat (Bytedance Doubao)
//! - outfox-zhipu for ASR, TTS, and Chat (Zhipu AI GLM models)
//!
//! Both providers now offer full ASR/TTS/Chat capabilities.
//! You can use either provider independently, or combine them.

pub mod activity;
pub mod ai_provider;
pub mod chat_stream;
pub mod doubao;
pub mod study_time;
pub mod zhipu;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

pub use ai_provider::{
    AiProvider, AiProviderError, AsrService, ChatMessage, ChatService, ProviderConfig,
    StructuredChatResponse, TextIssue, TtsService,
};
use async_trait::async_trait;
use diesel::prelude::*;
pub use doubao::DoubaoClient;
use serde_json::Value;
pub use zhipu::ZhipuClient;

use crate::db::schema::user_ai_providers;
use crate::db::with_conn;
use crate::models::ai_provider::{DoubaoSettings, UserAiProvider, ZhipuSettings};

/// Combined AI Provider that mixes services from different providers.
///
/// Use this when you want to use Doubao for ASR/TTS (better Chinese voice quality)
/// and Zhipu for Chat (GLM models).
#[derive(Clone)]
pub struct CombinedProvider {
    doubao: DoubaoClient,
    zhipu: ZhipuClient,
}

impl CombinedProvider {
    /// Create a new combined provider with both clients
    pub fn new(doubao: DoubaoClient, zhipu: ZhipuClient) -> Self {
        Self { doubao, zhipu }
    }

    /// Create from environment variables
    pub fn from_env() -> Option<Self> {
        let doubao = DoubaoClient::from_env()?;
        let zhipu = ZhipuClient::from_env()?;
        Some(Self::new(doubao, zhipu))
    }
}

#[async_trait]
impl AiProvider for CombinedProvider {
    fn name(&self) -> &'static str {
        "combined"
    }

    fn asr(&self) -> Option<Arc<dyn AsrService>> {
        // Use Doubao ASR for better Chinese speech recognition
        self.doubao.asr()
    }

    fn tts(&self) -> Option<Arc<dyn TtsService>> {
        // Use Doubao TTS for better Chinese voice quality
        self.doubao.tts()
    }

    fn chat_service(&self) -> Option<Arc<dyn ChatService>> {
        // Use Zhipu Chat (GLM models)
        self.zhipu.chat_service()
    }
}

/// Create an AI provider from configuration
///
/// - Doubao config: Creates Doubao provider for ASR/TTS, adds Zhipu for Chat if available
/// - Zhipu config: Creates Zhipu provider with full ASR/TTS/Chat capabilities
pub fn create_provider(config: &ProviderConfig) -> Arc<dyn AiProvider> {
    static CLIENTS: OnceLock<Mutex<HashMap<String, Arc<dyn AiProvider>>>> = OnceLock::new();
    let key = crate::auth::hash_code(
        &serde_json::to_string(config).expect("provider configuration serializes"),
    );
    let clients = CLIENTS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(provider) = clients
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&key)
        .cloned()
    {
        return provider;
    }
    let provider = build_provider(config);
    let mut clients = clients.lock().unwrap_or_else(|e| e.into_inner());
    if clients.len() >= 128 {
        clients.clear();
    }
    clients.insert(key, provider.clone());
    provider
}

fn build_provider(config: &ProviderConfig) -> Arc<dyn AiProvider> {
    tracing::debug!("Creating configured AI provider");

    match config {
        ProviderConfig::Doubao {
            app_id,
            access_token,
            api_key,
            chat_model,
            tts_resource_id,
            voice_type,
        } => {
            // Doubao now provides full ASR/TTS/Chat capabilities
            let doubao = DoubaoClient::with_options(
                app_id.clone(),
                access_token.clone(),
                api_key.clone(),
                tts_resource_id.clone(),
                chat_model.clone(),
                voice_type.clone(),
            );
            tracing::info!("Created Doubao provider with full ASR/TTS/Chat capabilities");
            Arc::new(doubao)
        }
        ProviderConfig::Zhipu {
            api_key,
            asr_model: _,
            tts_model: _,
            chat_model,
        } => {
            // Zhipu now provides full ASR/TTS/Chat capabilities
            let zhipu = ZhipuClient::with_models(api_key.clone(), chat_model.clone());
            tracing::info!("Created Zhipu provider with full ASR/TTS/Chat capabilities");
            Arc::new(zhipu)
        }
    }
}

/// Create an AI provider from environment variables
///
/// Priority:
/// 1. Combined provider (Doubao ASR/TTS + Zhipu Chat) if both configured
/// 2. Zhipu-only provider if only Zhipu is configured
/// 3. Doubao-only provider if only Doubao is configured
pub fn create_provider_from_env() -> Option<Arc<dyn AiProvider>> {
    static DEFAULT: OnceLock<Option<Arc<dyn AiProvider>>> = OnceLock::new();
    DEFAULT.get_or_init(build_provider_from_env).clone()
}
fn build_provider_from_env() -> Option<Arc<dyn AiProvider>> {
    // // Try to create combined provider first (for best experience)
    // if let Some(combined) = CombinedProvider::from_env() {
    //     tracing::info!("Created combined provider: Doubao (ASR/TTS) + Zhipu (Chat)");
    //     return Some(Arc::new(combined));
    // }
    let default_provider = std::env::var("AI_PROVIDER_DEFAULT")
        .ok()
        .map(|s| s.to_lowercase());

    match default_provider.as_deref() {
        Some("zhipu") => {
            // Try Zhipu alone (has full capabilities)
            if let Some(zhipu) = ZhipuClient::from_env() {
                tracing::info!("Created Zhipu provider with full ASR/TTS/Chat capabilities");
                return Some(Arc::new(zhipu));
            }
        }
        Some("doubao") => {
            if let Some(doubao) = DoubaoClient::from_env() {
                tracing::info!("Created Doubao provider with full ASR/TTS/Chat capabilities");
                return Some(Arc::new(doubao));
            }
        }
        None | Some("") => {
            if let Some(zhipu) = ZhipuClient::from_env() {
                return Some(Arc::new(zhipu));
            }
            if let Some(doubao) = DoubaoClient::from_env() {
                return Some(Arc::new(doubao));
            }
        }
        _ => {}
    }

    tracing::error!("No AI provider configured. Set ZHIPU_API_KEY or DOUBAO_* env vars.");
    None
}

/// Get AI provider for a specific user
///
/// Priority:
/// 1. User's active provider configuration from database
/// 2. System default from environment variables
pub async fn get_user_provider(user_id: i64) -> Option<Arc<dyn AiProvider>> {
    // Try to get user's active provider from database
    let user_provider = with_conn(move |conn| {
        user_ai_providers::table
            .filter(user_ai_providers::user_id.eq(user_id))
            .filter(user_ai_providers::is_active.eq(true))
            .first::<UserAiProvider>(conn)
            .optional()
    })
    .await
    .ok()
    .flatten();

    if let Some(provider) = user_provider {
        if let Some(ai_provider) =
            create_provider_from_user_config(&provider.provider_type, &provider.settings)
        {
            tracing::info!(
                "Using user's {} provider for user_id={}",
                provider.provider_type,
                user_id
            );
            return Some(ai_provider);
        }
    }

    // Fall back to system default
    tracing::info!("Using system default provider for user_id={}", user_id);
    create_provider_from_env()
}

/// Create AI provider from user configuration stored in database
fn create_provider_from_user_config(
    provider_type: &str,
    settings: &Value,
) -> Option<Arc<dyn AiProvider>> {
    match provider_type {
        "zhipu" => {
            let zhipu_settings: ZhipuSettings = serde_json::from_value(settings.clone()).ok()?;
            let config = ProviderConfig::Zhipu {
                api_key: zhipu_settings.api_key,
                chat_model: Some(zhipu_settings.chat_model),
                asr_model: None,
                tts_model: None,
            };
            Some(create_provider(&config))
        }
        "doubao" => {
            let doubao_settings: DoubaoSettings = serde_json::from_value(settings.clone()).ok()?;
            let config = ProviderConfig::Doubao {
                app_id: doubao_settings.app_id,
                access_token: doubao_settings.access_token,
                api_key: doubao_settings.api_key,
                chat_model: Some(doubao_settings.chat_model),
                tts_resource_id: doubao_settings.tts_resource_id,
                voice_type: doubao_settings.voice_type,
            };
            Some(create_provider(&config))
        }
        _ => None,
    }
}
