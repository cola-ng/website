//! AI Services module
//!
//! This module provides AI services using outfox crates:
//! - outfox-doubao for ASR, TTS, and Chat (Bytedance Doubao)
//! - outfox-zhipu for ASR, TTS, and Chat (Zhipu AI GLM models)
//!
//! Both providers now offer full ASR/TTS/Chat capabilities.
//! You can use either provider independently, or combine them.

pub mod ai_provider;
pub mod doubao;
pub mod zhipu;

use std::sync::Arc;

pub use ai_provider::{
    AiProvider, AiProviderError, AsrService, ChatMessage, ChatService, ProviderConfig,
    StructuredChatResponse, TextIssue, TtsService,
};
use async_trait::async_trait;
pub use doubao::DoubaoClient;
pub use zhipu::ZhipuClient;

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
    tracing::info!("Creating provider with config: {:?}", config);

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
    // // Try to create combined provider first (for best experience)
    // if let Some(combined) = CombinedProvider::from_env() {
    //     tracing::info!("Created combined provider: Doubao (ASR/TTS) + Zhipu (Chat)");
    //     return Some(Arc::new(combined));
    // }
    let default_provider = std::env::var("AI_PROVIDER_DEFAULT")
        .ok()
        .map(|s| s.to_lowercase());

    println!("Default provider from env: {:?}", default_provider);
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
        _ => {}
    }

    tracing::error!("No AI provider configured. Set ZHIPU_API_KEY or DOUBAO_* env vars.");
    None
}
