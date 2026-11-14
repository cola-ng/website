//! AI services provided by the supported model clients.
use std::result::Result;
pub mod amigo;
pub mod asr_stream;
pub mod minimax;
pub mod ocr;


pub mod activity;
pub mod provider_catalog;
pub mod model_catalog;
pub mod openai_realtime;
pub mod compatible;
pub mod ai_provider;
pub mod chat_stream;
pub mod study_time;
pub mod zhipu;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

pub use ai_provider::{
    AiProvider, AiProviderError, AsrService, ChatMessage, ChatService, ProviderConfig,
    StructuredChatResponse, TextIssue, TtsService,
};
use horme::prelude::*;
use serde_json::Value;
pub use zhipu::ZhipuClient;

use crate::db::schema::user_ai_providers;
use crate::db::with_conn;
use crate::models::ai_provider::UserAiProvider;

/// Create an AI provider from configuration.
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
        ProviderConfig::Catalog { provider_type, settings } => {
            if provider_type == "zhipu" {
                Arc::new(ConfiguredZhipu { client: ZhipuClient::with_settings(settings), settings: settings.clone() })
            } else {
                Arc::new(compatible::CompatibleClient::new(provider_type, settings.clone()))
            }
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

/// Create the supported platform provider from environment variables.
pub fn create_provider_from_env() -> Option<Arc<dyn AiProvider>> {
    ProviderConfig::from_env().map(|config| create_provider(&config))
}

/// Only a paid, unexpired subscription with a settled payment may use platform keys.
/// A personal active configuration always takes precedence, including on failure.
#[derive(serde::Serialize, salvo::oapi::ToSchema)]
pub struct AiAccess {
    pub source: String,
    pub platform_allowed: bool,
    pub active_provider_id: Option<i64>,
    pub message: String,
    pub chat_provider_id: Option<i64>,
    pub tts_provider_id: Option<i64>,
    pub asr_provider_id: Option<i64>,
}

async fn provider_selection(
    user_id: i64,
) -> Result<(Vec<UserAiProvider>, bool), salvo::http::StatusError> {
    with_conn!(move |conn| {
        let personal = user_ai_providers::table
            .filter(user_ai_providers::user_id.eq(user_id))
            .filter(user_ai_providers::is_active.eq(true).or(user_ai_providers::tts_active.eq(true)).or(user_ai_providers::asr_active.eq(true)))
            .order(user_ai_providers::id.desc())
            .load::<UserAiProvider>(conn).await?;
        #[derive(diesel::QueryableByName)]
        struct Paid {
            #[diesel(sql_type = diesel::sql_types::Bool)]
            allowed: bool,
        }
        let paid = diesel::sql_query(
            "SELECT EXISTS (
            SELECT 1 FROM user_subscriptions s
            JOIN subscription_plans p ON p.id = s.plan_id
            JOIN payment_orders o ON o.id = s.last_payment_id AND o.user_id = s.user_id
            WHERE s.user_id = $1 AND s.status = 'active' AND s.started_at <= now()
              AND (s.expires_at IS NULL OR s.expires_at > now())
              AND p.tier <> 'free' AND p.price_credits > 0
              AND o.status = 'paid' AND o.paid_at IS NOT NULL
              AND (o.amount_cents > 0 OR o.amount_credits > 0)
        ) AS allowed",
        )
        .bind::<diesel::sql_types::BigInt, _>(user_id)
        .get_result::<Paid>(conn).await?
        .allowed;
        Ok((personal, paid))
    })
    .await
    .map_err(|_| {
        salvo::http::StatusError::service_unavailable()
            .brief("无法验证 AI 配置与付费资格，请稍后重试")
    })
}

pub async fn ai_access(user_id: i64) -> Result<AiAccess, salvo::http::StatusError> {
    let (personal, paid) = provider_selection(user_id).await?;
    let id = personal.iter().find(|p|p.is_active).map(|p|p.id);
    let (source, message) = if !personal.is_empty() {
        (
            "personal",
            "当前使用你自己的 AI 服务，费用由对应提供方收取。",
        )
    } else if paid && create_provider_from_env().is_some() {
        ("platform", "当前使用付费订阅提供的平台 AI 服务。")
    } else {
        (
            "none",
            "请添加并启用自己的 AI 服务配置。平台 AI 仅向已付费且订阅有效的用户提供，免费与试用账户需自备密钥。",
        )
    };
    Ok(AiAccess {
        source: source.into(),
        platform_allowed: paid,
        active_provider_id: id,
        message: message.into(),
        chat_provider_id: personal.iter().find(|p|p.is_active).map(|p|p.id),
        tts_provider_id: personal.iter().find(|p|p.tts_active).map(|p|p.id),
        asr_provider_id: personal.iter().find(|p|p.asr_active).map(|p|p.id),
    })
}

pub async fn get_user_provider(
    user_id: i64,
) -> Result<Arc<dyn AiProvider>, salvo::http::StatusError> {
    let (personal, paid) = provider_selection(user_id).await?;
    if !personal.is_empty() {
        let mut selected = SelectedProvider {name:"personal",chat:None,tts:None,asr:None};
        for row in personal {
            let p = create_provider_from_user_config(&row.provider_type, &row.settings).ok_or_else(||salvo::http::StatusError::bad_request().brief("个人 AI 配置无效，请修正配置；不会切换到平台密钥"))?;
            if row.is_active { selected.name=p.name(); selected.chat=p.chat_service(); }
            if row.tts_active { selected.tts=p.tts(); }
            if row.asr_active { selected.asr=p.asr(); }
        }
        return Ok(Arc::new(selected));
    }
    if !paid {
        return Err(salvo::http::StatusError::payment_required()
            .brief("请先添加并启用自己的 AI 服务配置；平台 AI 仅供已付费且订阅有效的用户使用"));
    }
    create_provider_from_env().ok_or_else(|| {
        salvo::http::StatusError::service_unavailable()
            .brief("平台 AI 暂不可用，请配置自己的 AI 服务或稍后重试")
    })
}

/// Create AI provider from user configuration stored in database
pub fn create_provider_from_user_config(provider_type: &str, settings: &Value) -> Option<Arc<dyn AiProvider>> {
    let settings=provider_catalog::validate(provider_type,settings.clone()).ok()?;
    Some(create_provider(&ProviderConfig::Catalog {provider_type:provider_type.into(),settings}))
}
struct ConfiguredZhipu { client:ZhipuClient, settings:Value }
impl AiProvider for ConfiguredZhipu {
    fn name(&self)->&'static str {"zhipu"}
    fn chat_service(&self)->Option<Arc<dyn ChatService>> {
        let model=provider_catalog::model(&self.settings,"chat")?;
        if model.starts_with("glm-5") {Some(Arc::new(compatible::CompatibleClient::new("zhipu",self.settings.clone())))}
        else {self.client.chat_service()}
    }
    fn tts(&self)->Option<Arc<dyn TtsService>> {provider_catalog::model(&self.settings,"tts").and_then(|_|self.client.tts())}
    fn asr(&self)->Option<Arc<dyn AsrService>> {provider_catalog::model(&self.settings,"asr").and_then(|_|self.client.asr())}
}
struct SelectedProvider { name:&'static str, chat:Option<Arc<dyn ChatService>>, tts:Option<Arc<dyn TtsService>>, asr:Option<Arc<dyn AsrService>> }
impl AiProvider for SelectedProvider {
    fn name(&self)->&'static str {self.name}
    fn chat_service(&self)->Option<Arc<dyn ChatService>> {self.chat.clone()}
    fn tts(&self)->Option<Arc<dyn TtsService>> {self.tts.clone()}
    fn asr(&self)->Option<Arc<dyn AsrService>> {self.asr.clone()}
}

pub async fn personal_tts(user:i64)->Result<(String,Value,Arc<dyn TtsService>),salvo::http::StatusError>{
    let row=with_conn!(move|conn|user_ai_providers::table.filter(user_ai_providers::user_id.eq(user)).filter(user_ai_providers::tts_active.eq(true)).first::<UserAiProvider>(conn).await.optional()).await.map_err(|_|salvo::http::StatusError::service_unavailable().brief("无法读取个人 TTS 配置"))?.ok_or_else(||salvo::http::StatusError::bad_request().brief("请先添加并启用自己的语音合成 TTS 配置"))?;
    // Use the same normalized settings for the request and its audit record.
    // Legacy rows omitted model/voice fields, but their actual SDK request used defaults.
    let settings=provider_catalog::validate(&row.provider_type,row.settings).map_err(|_|salvo::http::StatusError::bad_request().brief("个人 TTS 配置无效"))?;
    let provider=create_provider_from_user_config(&row.provider_type,&settings).ok_or_else(||salvo::http::StatusError::bad_request().brief("个人 TTS 配置无效"))?;
    let service=provider.tts().ok_or_else(||salvo::http::StatusError::bad_request().brief("请先启用自己的 TTS 模型"))?;
    Ok((row.provider_type,settings,service))
}

pub mod voices;
