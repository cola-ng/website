pub mod ai_provider;
pub mod doubao;
pub mod zhipu;

pub use ai_provider::{AiProvider, AiProviderError, ChatMessage, ProviderConfig, TextIssue, UserInputAnalysis};
pub use doubao::DoubaoClient;
pub use zhipu::ZhipuClient;

use std::sync::Arc;

/// Create an AI provider from configuration
pub fn create_provider(config: &ProviderConfig) -> Arc<dyn AiProvider> {
    println!("Creating provider with config: {:#?}", config);
    match config {
        ProviderConfig::Doubao {
            app_id,
            access_token,
            api_key,
            chat_model,
            tts_resource_id,
        } => Arc::new(DoubaoClient::with_options(
            app_id.clone(),
            access_token.clone(),
            api_key.clone(),
            chat_model.clone(),
            tts_resource_id.clone(),
        )),
        ProviderConfig::Zhipu {
            api_key,
            asr_model,
            tts_model,
            chat_model,
        } => Arc::new(ZhipuClient::with_models(
            api_key.clone(),
            asr_model.clone(),
            tts_model.clone(),
            chat_model.clone(),
        )),
    }
}

/// Create an AI provider from environment variables
pub fn create_provider_from_env() -> Option<Arc<dyn AiProvider>> {
    ProviderConfig::from_env().map(|config| create_provider(&config))
}

/// Legacy exports for backward compatibility
pub use zhipu::ZhipuError;
