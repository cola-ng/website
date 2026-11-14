use chrono::Utc;
use horme::prelude::*;
use salvo::prelude::*;
use serde_json::Value;
use std::result::Result;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::ai_provider::*;
use crate::services::{self, provider_catalog};
use crate::{AppResult, DepotExt, JsonResult, hoops, json_ok};

pub fn router() -> Router {
    Router::with_path("ai-providers")
        .hoop(hoops::require_auth)
        .get(list_providers)
        .post(create_provider)
        .push(Router::with_path("access").get(access))
        .push(Router::with_path("catalog").get(catalog))
        .push(Router::with_path("models").post(discover_models))
        .push(Router::with_path("voice-catalog/{kind}").get(voice_presets))
        .push(Router::with_path("voices").get(active_voices))
        .push(Router::with_path("voice-preview").post(voice_preview))
        .push(
            Router::with_path("{provider_id}")
                .get(get_provider)
                .put(update_provider)
                .delete(delete_provider)
                .push(Router::with_path("test").post(test_provider))
                .push(Router::with_path("voices").get(saved_voices))
                .push(Router::with_path("activate").post(activate_provider)),
        )
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_provider_id(req: &Request) -> Result<i64, StatusError> {
    let raw = req
        .params()
        .get("provider_id")
        .cloned()
        .ok_or_else(|| StatusError::bad_request().brief("missing provider_id"))?;
    raw.parse()
        .map_err(|_| StatusError::bad_request().brief("invalid provider_id"))
}

fn secret_field(key: &str) -> bool {
    key.contains("key") || key.contains("token") || key.contains("secret")
}

fn mask_settings(settings: &Value) -> Value {
    let mut masked = settings.clone();
    if let Some(obj) = masked.as_object_mut() {
        for (key, value) in obj {
            if secret_field(key) {
                *value = Value::String("********".into());
            }
        }
    }
    masked
}

fn validated_settings(kind: &str, settings: Value) -> Result<Value, StatusError> {
    provider_catalog::validate(kind, settings).map_err(|e| StatusError::bad_request().brief(e))
}
#[endpoint(tags("AI Provider"))]
pub async fn catalog() -> JsonResult<Vec<provider_catalog::ProviderDefinition>> {
    json_ok(provider_catalog::catalog())
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelQuery {
    provider_type: String,
    provider_id: Option<i64>,
    api_key: Option<String>,
    region: String,
    workspace_id: Option<String>,
    device_id: Option<uuid::Uuid>,
    capability: Option<String>,
}
#[handler]
async fn discover_models(req: &mut Request, depot: &mut Depot, res: &mut Response) -> JsonResult<Value> {
    let user = depot.user_id()?;
    let input: ModelQuery = req.parse_json().await.map_err(|_| StatusError::bad_request())?;
    let definition = provider_catalog::definition(&input.provider_type).ok_or_else(StatusError::bad_request)?;
    if !definition.regions.contains(&input.region.as_str()) { return Err(StatusError::bad_request().brief("服务区域无效").into()); }
    let mut settings = if let Some(device_id) = input.device_id {
        let row=device_provider(user,device_id,input.capability.as_deref().unwrap_or("tts")).await?;
        if let Some(id)=input.provider_id {let p=voice_provider(user,Some(id)).await?;if p.provider_type!=input.provider_type{return Err(StatusError::bad_request().into())}p.settings}
        else{row.filter(|p|p.kind==input.provider_type).map(|p|p.settings).unwrap_or_else(||serde_json::json!({}))}
    } else if let Some(id) = input.provider_id {
        let p = voice_provider(user, Some(id)).await?;
        if p.provider_type != input.provider_type { return Err(StatusError::bad_request().brief("配置平台不匹配").into()); }
        p.settings
    } else { serde_json::json!({}) };
    if let Some(key) = input.api_key.filter(|s| !s.trim().is_empty()) {
        let key = key.trim();
        if key.len() > 4096 || key.chars().any(char::is_control) || key.chars().all(|c| c == '*') { return Err(StatusError::bad_request().brief("API Key 格式无效").into()); }
        settings["api_key"] = serde_json::json!(key);
    }
    if let Some(workspace) = input.workspace_id {
        let workspace = workspace.trim();
        if !workspace.is_empty() && !services::model_catalog::valid_workspace(workspace) { return Err(StatusError::bad_request().brief("业务空间 ID 格式无效").into()); }
        settings["workspace_id"] = serde_json::json!(workspace);
    }
    settings["region"] = serde_json::json!(input.region);
    res.headers_mut().insert("Cache-Control", "no-store".parse().unwrap());
    json_ok(services::model_catalog::discover(&input.provider_type, &settings).await)
}
fn active_caps(
    settings: &Value,
    requested: Option<Vec<String>>,
    all: bool,
) -> Result<Option<Vec<String>>, StatusError> {
    let requested = if all && requested.is_none() {
        Some(
            ["chat", "tts", "asr"]
                .into_iter()
                .filter(|cap| provider_catalog::model(settings, cap).is_some())
                .map(str::to_owned)
                .collect(),
        )
    } else {
        requested
    };
    if let Some(caps) = requested.as_ref() {
        if caps.len() > 3
            || caps.iter().any(|c| {
                !matches!(c.as_str(), "chat" | "tts" | "asr")
                    || provider_catalog::model(settings, c).is_none()
            })
        {
            return Err(StatusError::bad_request().brief("只能启用已配置的文本、TTS 或 ASR 模型"));
        }
    }
    Ok(requested)
}
async fn deactivate(
    conn: &mut horme::diesel_async::AsyncPgConnection,
    user: i64,
    caps: &[String],
) -> horme::Result<()> {
    let rows = user_ai_providers::table.filter(user_ai_providers::user_id.eq(user));
    if caps.iter().any(|c| c == "chat") {
        diesel::update(rows)
            .set(user_ai_providers::is_active.eq(false))
            .execute(conn)
            .await?;
    }
    if caps.iter().any(|c| c == "tts") {
        diesel::update(rows)
            .set(user_ai_providers::tts_active.eq(false))
            .execute(conn)
            .await?;
    }
    if caps.iter().any(|c| c == "asr") {
        diesel::update(rows)
            .set(user_ai_providers::asr_active.eq(false))
            .execute(conn)
            .await?;
    }
    Ok(())
}

fn provider_error() -> StatusError {
    StatusError::internal_server_error().brief("无法保存 AI 配置，请重试")
}

#[endpoint(tags("AI Provider"))]
pub async fn access(depot: &mut Depot) -> JsonResult<services::AiAccess> {
    json_ok(services::ai_access(depot.user_id()?).await?)
}

/// Convert UserAiProvider to response with masked settings
fn to_response(provider: UserAiProvider) -> UserAiProvider {
    UserAiProvider {
        settings: mask_settings(&provider.settings),
        ..provider
    }
}

// ============================================================================
// CRUD Endpoints
// ============================================================================

/// List all AI providers for the current user
#[endpoint(tags("AI Provider"))]
pub async fn list_providers(depot: &mut Depot) -> JsonResult<Vec<UserAiProvider>> {
    let user_id = depot.user_id()?;

    let providers: Vec<UserAiProvider> = with_conn!(move |conn| {
        user_ai_providers::table
            .filter(user_ai_providers::user_id.eq(user_id))
            .order(user_ai_providers::created_at.desc())
            .load::<UserAiProvider>(conn)
            .await
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list providers"))?;

    json_ok(providers.into_iter().map(to_response).collect())
}

/// Create a new AI provider configuration
#[endpoint(tags("AI Provider"))]
pub async fn create_provider(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateAiProviderRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let provider_type = input.provider_type.to_lowercase();
    let settings = validated_settings(&provider_type, input.settings)?;
    let caps = active_caps(&settings, input.activate_for, input.set_active)?.unwrap_or_default();
    let provider: UserAiProvider = with_conn!(move |conn| {
        horme::transaction(conn, async |conn| {
            base_users::table
                .find(user_id)
                .for_update()
                .select(base_users::id)
                .first::<i64>(conn)
                .await?;
            deactivate(conn, user_id, &caps).await?;
            let new_provider = NewUserAiProvider {
                user_id,
                provider_type,
                is_active: caps.iter().any(|c| c == "chat"),
                tts_active: caps.iter().any(|c| c == "tts"),
                asr_active: caps.iter().any(|c| c == "asr"),
                settings,
            };

            diesel::insert_into(user_ai_providers::table)
                .values(&new_provider)
                .get_result::<UserAiProvider>(conn)
                .await
        })
        .await
    })
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
            StatusError::conflict().brief("provider already exists for this user")
        } else {
            StatusError::internal_server_error().brief("failed to create provider")
        }
    })?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(to_response(provider)));
    Ok(())
}

/// Get a specific AI provider configuration
#[endpoint(tags("AI Provider"))]
pub async fn get_provider(req: &mut Request, depot: &mut Depot) -> JsonResult<UserAiProvider> {
    let user_id = depot.user_id()?;
    let provider_id = get_provider_id(req)?;

    let provider: UserAiProvider = with_conn!(move |conn| {
        user_ai_providers::table
            .filter(user_ai_providers::id.eq(provider_id))
            .filter(user_ai_providers::user_id.eq(user_id))
            .first::<UserAiProvider>(conn)
            .await
    })
    .await
    .map_err(|_| StatusError::not_found().brief("provider not found"))?;

    json_ok(to_response(provider))
}

/// Update an AI provider configuration
#[endpoint(tags("AI Provider"))]
pub async fn update_provider(req: &mut Request, depot: &mut Depot) -> JsonResult<UserAiProvider> {
    let user_id = depot.user_id()?;
    let provider_id = get_provider_id(req)?;
    let input: UpdateAiProviderRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let provider = change_provider(
        user_id,
        provider_id,
        input.settings.clone(),
        input.is_active,
        input.activate_for,
    )
    .await?;

    json_ok(to_response(provider))
}

/// Delete an AI provider configuration
#[endpoint(tags("AI Provider"))]
pub async fn delete_provider(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<serde_json::Value> {
    let user_id = depot.user_id()?;
    let provider_id = get_provider_id(req)?;

    let deleted = with_conn!(move |conn| {
        horme::transaction(conn, async |conn| {
            base_users::table
                .find(user_id)
                .for_update()
                .select(base_users::id)
                .first::<i64>(conn)
                .await?;
            diesel::delete(
                user_ai_providers::table
                    .filter(user_ai_providers::id.eq(provider_id))
                    .filter(user_ai_providers::user_id.eq(user_id)),
            )
            .execute(conn)
            .await
        })
        .await
    })
    .await
    .map_err(|_| provider_error())?;
    if deleted == 0 {
        return Err(StatusError::not_found().brief("AI 配置不存在").into());
    }

    json_ok(serde_json::json!({ "deleted": true }))
}

// ============================================================================
// Test & Activate Endpoints
// ============================================================================

/// Test an AI provider configuration
#[endpoint(tags("AI Provider"))]
pub async fn test_provider(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<TestProviderResponse> {
    let user_id = depot.user_id()?;
    let provider_id = get_provider_id(req)?;

    // Get the provider config
    let provider: UserAiProvider = with_conn!(move |conn| {
        user_ai_providers::table
            .filter(user_ai_providers::id.eq(provider_id))
            .filter(user_ai_providers::user_id.eq(user_id))
            .first::<UserAiProvider>(conn)
            .await
    })
    .await
    .map_err(|_| StatusError::not_found().brief("provider not found"))?;

    let capabilities = test_provider_connection(&provider.provider_type, &provider.settings).await;
    let success = !capabilities.is_empty() && capabilities.iter().all(|c| c.success);
    let error = if success {
        None
    } else {
        Some(
            capabilities
                .iter()
                .filter_map(|c| c.error.as_deref())
                .collect::<Vec<_>>()
                .join("；"),
        )
    };
    let now = Utc::now();
    let saved_error = error.clone();
    // A concurrent settings edit invalidates this result; do not mark new credentials as tested.
    with_conn!(move |conn| {
        diesel::update(
            user_ai_providers::table
                .filter(user_ai_providers::id.eq(provider_id))
                .filter(user_ai_providers::user_id.eq(user_id))
                .filter(user_ai_providers::settings.eq(provider.settings)),
        )
        .set((
            user_ai_providers::last_tested_at.eq(Some(now)),
            user_ai_providers::test_status.eq(Some(if success { "success" } else { "failed" })),
            user_ai_providers::test_error.eq(saved_error),
            user_ai_providers::updated_at.eq(now),
        ))
        .execute(conn)
        .await
    })
    .await
    .map_err(|_| provider_error())?;
    json_ok(TestProviderResponse {
        success,
        error,
        tested_at: now,
        capabilities,
    })
}

/// Activate an AI provider (deactivate others)
#[endpoint(tags("AI Provider"))]
pub async fn activate_provider(req: &mut Request, depot: &mut Depot) -> JsonResult<UserAiProvider> {
    let user_id = depot.user_id()?;
    let provider_id = get_provider_id(req)?;

    let provider = change_provider(user_id, provider_id, None, Some(true), None).await?;

    json_ok(to_response(provider))
}

async fn change_provider(
    user_id: i64,
    id: i64,
    patch: Option<Value>,
    active: Option<bool>,
    requested: Option<Vec<String>>,
) -> Result<UserAiProvider, StatusError> {
    if active == Some(false) && requested.as_ref().is_some_and(|c| !c.is_empty()) {
        return Err(StatusError::bad_request().brief("停用配置时不能同时启用能力"));
    }
    with_conn!(move |conn| {
        horme::transaction(conn, async |conn| {
            base_users::table
                .find(user_id)
                .for_update()
                .select(base_users::id)
                .first::<i64>(conn)
                .await?;
            let Some(existing) = user_ai_providers::table
                .find(id)
                .filter(user_ai_providers::user_id.eq(user_id))
                .first::<UserAiProvider>(conn)
                .await
                .optional()?
            else {
                return Ok(Err(StatusError::not_found().brief("AI 配置不存在")));
            };
            let settings_changed = patch.is_some();
            let mut settings = existing.settings.clone();
            if let Some(patch) = patch {
                let Some(fields) = patch.as_object() else {
                    return Ok(Err(StatusError::bad_request().brief("配置格式无效")));
                };
                for (key, value) in fields {
                    if secret_field(key)
                        && (value.is_null()
                            || value.as_str().is_some_and(|s| {
                                s.trim().is_empty() || s.chars().all(|c| c == '*')
                            }))
                    {
                        continue;
                    }
                    settings[key] = value.clone();
                }
            }
            let settings = match validated_settings(&existing.provider_type, settings) {
                Ok(s) => s,
                Err(e) => return Ok(Err(e)),
            };
            let caps = match active_caps(&settings, requested, active == Some(true)) {
                Ok(c) => c,
                Err(e) => return Ok(Err(e)),
            };
            if let Some(caps) = caps.as_ref() {
                deactivate(conn, user_id, caps).await?;
            }
            let enabled = |cap: &str, old: bool| {
                if active == Some(false) || provider_catalog::model(&settings, cap).is_none() {
                    false
                } else {
                    caps.as_ref()
                        .map(|c| c.iter().any(|s| s == cap))
                        .unwrap_or(old)
                }
            };
            let updated = diesel::update(user_ai_providers::table.find(id))
                .set(UpdateUserAiProvider {
                    settings: Some(settings.clone()),
                    is_active: Some(enabled("chat", existing.is_active)),
                    tts_active: Some(enabled("tts", existing.tts_active)),
                    asr_active: Some(enabled("asr", existing.asr_active)),
                    updated_at: Some(Utc::now()),
                    last_tested_at: settings_changed.then_some(None),
                    test_status: settings_changed.then_some(None),
                    test_error: settings_changed.then_some(None),
                })
                .get_result::<UserAiProvider>(conn)
                .await?;
            Ok(Ok(updated))
        })
        .await
    })
    .await
    .map_err(|_| provider_error())?
}

// ============================================================================
// Provider Testing Helper
// ============================================================================

async fn capability_test(
    capability: &'static str,
    label: &'static str,
    future: impl std::future::Future<Output = Result<(), services::AiProviderError>>,
) -> CapabilityTest {
    let result = tokio::time::timeout(std::time::Duration::from_secs(20), future).await;
    let error = match result {
        Ok(Ok(())) => None,
        Ok(Err(_)) => Some(format!(
            "{label}连接失败，请检查密钥、模型权限、音色、余额与服务区域"
        )),
        Err(_) => Some(format!("{label}连接测试超时，请检查网络与模型配置")),
    };
    CapabilityTest {
        capability: capability.into(),
        success: error.is_none(),
        error,
    }
}

async fn test_provider_connection(provider_type: &str, settings: &Value) -> Vec<CapabilityTest> {
    let Ok(settings) = provider_catalog::validate(provider_type, settings.clone()) else {
        return vec![CapabilityTest {
            capability: "configuration".into(),
            success: false,
            error: Some("AI 配置无效".into()),
        }];
    };
    let Some(provider) = services::create_provider_from_user_config(provider_type, &settings)
    else {
        return vec![CapabilityTest {
            capability: "configuration".into(),
            success: false,
            error: Some("AI 配置无效".into()),
        }];
    };
    let chat_test = async {
        if let Some(chat) = provider.chat_service() {
            Some(
                capability_test("chat", "文本", async {
                    chat.chat(
                        vec![services::ChatMessage {
                            role: "user".into(),
                            content: "Please reply with OK.".into(),
                        }],
                        Some(0.0),
                        Some(if provider_type == "zhipu" { 16 } else { 1024 }),
                    )
                    .await
                    .map(|_| ())
                })
                .await,
            )
        } else {
            None
        }
    };
    let tts_test = async {
        if let Some(tts) = provider.tts() {
            Some(
                capability_test("tts", "TTS", async {
                    let audio = tts.synthesize("Hello", None, None).await?;
                    if super::sentence_audio::audio_format(&audio.audio_data, &audio.format)
                        .is_none()
                    {
                        return Err(services::AiProviderError::Parse("TTS 音频无效".into()));
                    }
                    Ok(())
                })
                .await,
            )
        } else {
            None
        }
    };
    let asr_test = async {
        if let Some(asr) = provider.asr() {
            Some(
                capability_test("asr", "ASR", async {
                    if provider_type == "zhipu" {
                        asr.transcribe(services::ai_provider::probe_audio(), None)
                            .await
                            .map(|_| ())
                    } else {
                        services::compatible::CompatibleClient::new(provider_type, settings.clone())
                            .probe_asr()
                            .await
                    }
                })
                .await,
            )
        } else {
            None
        }
    };
    let (chat, tts, asr) = tokio::join!(chat_test, tts_test, asr_test);
    [chat, tts, asr].into_iter().flatten().collect()
}

async fn voice_provider(user: i64, id: Option<i64>) -> Result<UserAiProvider, StatusError> {
    with_conn!(move |conn| async {
        let rows = user_ai_providers::table.filter(user_ai_providers::user_id.eq(user));
        if let Some(id) = id {
            rows.filter(user_ai_providers::id.eq(id)).first(conn).await
        } else {
            rows.filter(user_ai_providers::tts_active.eq(true))
                .first(conn)
                .await
        }
    })
    .await
    .map_err(|_| StatusError::not_found().brief("请先保存并选择 TTS 配置"))
}
async fn device_provider(user:i64,id:uuid::Uuid,cap:&str)->Result<Option<services::amigo::ai::Resolved>,StatusError>{
    if !services::amigo::ai::CAPS.contains(&cap){return Err(StatusError::bad_request())}
    let device=services::amigo::devices::owned(user,id).await.map_err(|_|StatusError::service_unavailable())?.ok_or_else(StatusError::not_found)?;
    let bindings=services::amigo::ai::load(&device).await.map_err(|_|StatusError::service_unavailable())?;
    services::amigo::ai::resolve(user,cap,bindings.get(cap).and_then(Option::as_ref)).await.map_err(|e|StatusError::bad_request().brief(e))
}
#[handler]
async fn voice_presets(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<Vec<services::voices::Voice>> {
    depot.user_id()?;
    json_ok(services::voices::presets_for_model(
        &req.param::<String>("kind").unwrap_or_default(),
        &req.query::<String>("model").unwrap_or_default(),
    ))
}
#[handler]
async fn saved_voices(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<Vec<services::voices::Voice>> {
    let mut p = voice_provider(depot.user_id()?, Some(get_provider_id(req)?)).await?;
    if let Some(model) = req.query::<String>("model").filter(|m| !m.is_empty()) {p.settings["tts_model"]=serde_json::json!(model);}
    json_ok(
        services::voices::list(&p.provider_type, &p.settings)
            .await
            .map_err(|e| StatusError::bad_gateway().brief(e))?,
    )
}
#[handler]
async fn active_voices(depot: &mut Depot) -> JsonResult<Vec<services::voices::Voice>> {
    let p = voice_provider(depot.user_id()?, None).await?;
    json_ok(
        services::voices::list(&p.provider_type, &p.settings)
            .await
            .map_err(|e| StatusError::bad_gateway().brief(e))?,
    )
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct VoicePreview {
    provider_id: Option<i64>,
    voice_id: String,
    provider_type: Option<String>,
    tts_model: Option<String>,
    api_key: Option<String>,
    region: Option<String>,
    device_id: Option<uuid::Uuid>,
    capability: Option<String>,
}
#[handler]
async fn voice_preview(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> JsonResult<Value> {
    use base64::Engine;
    let input: VoicePreview = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request())?;
    if !services::voices::valid(&input.voice_id) {
        return Err(StatusError::bad_request().brief("请选择有效音色").into());
    }
    let user=depot.user_id()?;
    let (kind,mut settings)=if let Some(id)=input.device_id {
        let resolved=device_provider(user,id,input.capability.as_deref().unwrap_or("tts")).await?;
        if let Some(provider)=input.provider_id {let p=voice_provider(user,Some(provider)).await?;if input.provider_type.as_ref().is_some_and(|kind|*kind!=p.provider_type){return Err(StatusError::bad_request().into())}(p.provider_type,p.settings)}
        else if let Some(row)=resolved.filter(|r|input.provider_type.as_ref().is_none_or(|kind|*kind==r.kind)){(row.kind,row.settings)}
        else {(input.provider_type.clone().ok_or_else(StatusError::bad_request)?,serde_json::json!({"chat_model":"","asr_model":"","tts_model":""}))}
    }else if input.provider_id.is_some() || input.provider_type.is_none() {
        let p=voice_provider(user,input.provider_id).await?;
        if input.provider_type.as_ref().is_some_and(|kind| *kind!=p.provider_type){return Err(StatusError::bad_request().brief("配置平台不匹配").into());}
        (p.provider_type,p.settings)
    } else {
        (input.provider_type.unwrap(),serde_json::json!({"chat_model":"","asr_model":"","tts_model":""}))
    };
    if let Some(model)=input.tts_model {settings["tts_model"]=serde_json::json!(model);}
    if let Some(region)=input.region.filter(|s|!s.is_empty()) {settings["region"]=serde_json::json!(region);}
    if let Some(key)=input.api_key.filter(|s|!s.trim().is_empty()) {settings["api_key"]=serde_json::json!(key);}
    settings["tts_voice"]=serde_json::json!(input.voice_id);
    let settings=validated_settings(&kind,settings)?;
    let output = services::voices::preview(&kind, &settings, &input.voice_id)
        .await
        .map_err(|e| StatusError::bad_gateway().brief(e))?;
    let mime = match output.format.as_str() {
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        _ => return Err(StatusError::bad_gateway().brief("不支持的试听格式").into()),
    };
    res.add_header("Cache-Control", "no-store", true)?;
    json_ok(
        serde_json::json!({"audio":base64::engine::general_purpose::STANDARD.encode(output.audio_data),"mime":mime}),
    )
}
