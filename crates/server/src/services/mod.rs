pub mod ai_provider;
pub mod bigmodel;
pub mod doubao;

pub use ai_provider::{AiProvider, AiProviderError, ChatMessage, ProviderConfig};
pub use bigmodel::BigModelClient;
pub use doubao::DoubaoClient;

use std::sync::Arc;

/// Create an AI provider from configuration
pub fn create_provider(config: &ProviderConfig) -> Arc<dyn AiProvider> {
    match config {
        ProviderConfig::BigModel {
            api_key,
            asr_model,
            tts_model,
            chat_model,
        } => Arc::new(BigModelClient::with_models(
            api_key.clone(),
            asr_model.clone(),
            tts_model.clone(),
            chat_model.clone(),
        )),
        ProviderConfig::Doubao {
            app_id,
            access_token,
            chat_api_key,
            chat_model,
        } => Arc::new(DoubaoClient::with_model(
            app_id.clone(),
            access_token.clone(),
            chat_api_key.clone(),
            chat_model.clone(),
        )),
    }
}

/// Create an AI provider from environment variables
/// Tries BigModel first, then Doubao
pub fn create_provider_from_env() -> Option<Arc<dyn AiProvider>> {
    ProviderConfig::from_env().map(|config| create_provider(&config))
}

/// Legacy exports for backward compatibility
pub use bigmodel::BigModelError;
