use chrono::Utc;
use diesel::prelude::*;
use salvo::prelude::*;
use serde_json::Value;

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
        .push(
            Router::with_path("{provider_id}")
                .get(get_provider)
                .put(update_provider)
                .delete(delete_provider)
                .push(Router::with_path("test").post(test_provider))
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
    provider_catalog::validate(kind,settings).map_err(|e|StatusError::bad_request().brief(e))
}
#[endpoint(tags("AI Provider"))]
pub async fn catalog() -> JsonResult<Vec<provider_catalog::ProviderDefinition>> {json_ok(provider_catalog::catalog())}
fn active_caps(settings:&Value,requested:Option<Vec<String>>,all:bool)->Result<Option<Vec<String>>,StatusError> {
    let requested=if all && requested.is_none() {Some(["chat","tts","asr"].into_iter().filter(|cap|provider_catalog::model(settings,cap).is_some()).map(str::to_owned).collect())}else{requested};
    if let Some(caps)=requested.as_ref() {
        if caps.len()>3 || caps.iter().any(|c|!matches!(c.as_str(),"chat"|"tts"|"asr") || provider_catalog::model(settings,c).is_none()) {return Err(StatusError::bad_request().brief("只能启用已配置的文本、TTS 或 ASR 模型"));}
    }
    Ok(requested)
}
fn deactivate(conn:&mut diesel::PgConnection,user:i64,caps:&[String])->diesel::QueryResult<()> {
    let rows=user_ai_providers::table.filter(user_ai_providers::user_id.eq(user));
    if caps.iter().any(|c|c=="chat"){diesel::update(rows).set(user_ai_providers::is_active.eq(false)).execute(conn)?;}
    if caps.iter().any(|c|c=="tts"){diesel::update(rows).set(user_ai_providers::tts_active.eq(false)).execute(conn)?;}
    if caps.iter().any(|c|c=="asr"){diesel::update(rows).set(user_ai_providers::asr_active.eq(false)).execute(conn)?;}
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

    let providers: Vec<UserAiProvider> = with_conn(move |conn| {
        user_ai_providers::table
            .filter(user_ai_providers::user_id.eq(user_id))
            .order(user_ai_providers::created_at.desc())
            .load::<UserAiProvider>(conn)
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
    let caps=active_caps(&settings,input.activate_for,input.set_active)?.unwrap_or_default();
    let provider: UserAiProvider = with_conn(move |conn| {
        conn.transaction(|conn| {
            base_users::table
                .find(user_id)
                .for_update()
                .select(base_users::id)
                .first::<i64>(conn)?;
            deactivate(conn,user_id,&caps)?;
            let new_provider = NewUserAiProvider {
                user_id,
                provider_type,
                is_active: caps.iter().any(|c|c=="chat"),
                tts_active: caps.iter().any(|c|c=="tts"),
                asr_active: caps.iter().any(|c|c=="asr"),
                settings,
            };

            diesel::insert_into(user_ai_providers::table)
                .values(&new_provider)
                .get_result::<UserAiProvider>(conn)
        })
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

    let provider: UserAiProvider = with_conn(move |conn| {
        user_ai_providers::table
            .filter(user_ai_providers::id.eq(provider_id))
            .filter(user_ai_providers::user_id.eq(user_id))
            .first::<UserAiProvider>(conn)
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

    let deleted = with_conn(move |conn| {
        conn.transaction(|conn| {
            base_users::table
                .find(user_id)
                .for_update()
                .select(base_users::id)
                .first::<i64>(conn)?;
            diesel::delete(
                user_ai_providers::table
                    .filter(user_ai_providers::id.eq(provider_id))
                    .filter(user_ai_providers::user_id.eq(user_id)),
            )
            .execute(conn)
        })
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
    let provider: UserAiProvider = with_conn(move |conn| {
        user_ai_providers::table
            .filter(user_ai_providers::id.eq(provider_id))
            .filter(user_ai_providers::user_id.eq(user_id))
            .first::<UserAiProvider>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("provider not found"))?;

    // Create provider config and test
    let test_result = test_provider_connection(&provider.provider_type, &provider.settings).await;
    let now = Utc::now();

    // Update test status in database
    let (test_status, test_error) = match &test_result {
        Ok(_) => ("success".to_string(), None),
        Err(e) => ("failed".to_string(), Some(e.clone())),
    };

    let test_status_clone = test_status.clone();
    let test_error_clone = test_error.clone();

    with_conn(move |conn| {
        diesel::update(user_ai_providers::table.filter(user_ai_providers::id.eq(provider_id)))
            .set((
                user_ai_providers::last_tested_at.eq(Some(now)),
                user_ai_providers::test_status.eq(Some(test_status_clone)),
                user_ai_providers::test_error.eq(test_error_clone),
                user_ai_providers::updated_at.eq(now),
            ))
            .execute(conn)
    })
    .await
    .ok();

    json_ok(TestProviderResponse {
        success: test_result.is_ok(),
        error: test_result.err(),
        tested_at: now,
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
    if active==Some(false) && requested.as_ref().is_some_and(|c|!c.is_empty()) {return Err(StatusError::bad_request().brief("停用配置时不能同时启用能力"));}
    with_conn(move |conn| {
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            base_users::table
                .find(user_id)
                .for_update()
                .select(base_users::id)
                .first::<i64>(conn)?;
            let Some(existing) = user_ai_providers::table
                .find(id)
                .filter(user_ai_providers::user_id.eq(user_id))
                .first::<UserAiProvider>(conn)
                .optional()?
            else {
                return Ok(Err(StatusError::not_found().brief("AI 配置不存在")));
            };
            let mut settings = existing.settings.clone();
            if let Some(patch) = patch {
                let Some(fields) = patch.as_object() else {
                    return Ok(Err(StatusError::bad_request().brief("配置格式无效")));
                };
                for (key, value) in fields {
                    if secret_field(key)
                        && (value.is_null()
                            || value
                                .as_str()
                                .is_some_and(|s| s.trim().is_empty() || s.chars().all(|c| c == '*')))
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
            let caps=match active_caps(&settings,requested,active==Some(true)) {Ok(c)=>c,Err(e)=>return Ok(Err(e))};
            if let Some(caps)=caps.as_ref(){deactivate(conn,user_id,caps)?;}
            let enabled=|cap:&str,old:bool| {
                if active==Some(false) || provider_catalog::model(&settings,cap).is_none(){false}
                else {caps.as_ref().map(|c|c.iter().any(|s|s==cap)).unwrap_or(old)}
            };
            let updated = diesel::update(user_ai_providers::table.find(id))
                .set(UpdateUserAiProvider {
                    settings: Some(settings.clone()),
                    is_active: Some(enabled("chat",existing.is_active)),
                    tts_active: Some(enabled("tts",existing.tts_active)),
                    asr_active: Some(enabled("asr",existing.asr_active)),
                    updated_at: Some(Utc::now()),
                    last_tested_at: None,
                    test_status: None,
                    test_error: None,
                })
                .get_result::<UserAiProvider>(conn)?;
            Ok(Ok(updated))
        })
    })
    .await
    .map_err(|_| provider_error())?
}

// ============================================================================
// Provider Testing Helper
// ============================================================================

async fn test_provider_connection(provider_type: &str, settings: &Value) -> Result<(), String> {
    let provider = services::create_provider_from_user_config(provider_type,settings).ok_or("AI 配置无效")?;
    // Test chat service with a simple message
    if let Some(chat) = provider.chat_service() {
        let messages = vec![services::ChatMessage {
            role: "user".to_string(),
            content: "Hello, please respond with 'OK' if you can hear me.".to_string(),
        }];

        tokio::time::timeout(
            std::time::Duration::from_secs(20),
            chat.chat(messages, Some(0.0), Some(if provider_type=="zhipu"{16}else{1024})),
        )
        .await
        .map_err(|_| "连接测试超时，请检查网络与模型配置".to_string())?
        .map_err(|_| "连接失败，请检查密钥、模型权限与网络".to_string())?;

        Ok(())
    } else {
        if let Some(tts)=provider.tts() {
            tokio::time::timeout(std::time::Duration::from_secs(20),tts.synthesize("Hello",None,None)).await.map_err(|_|"语音测试超时")?.map_err(|_|"语音连接失败，请检查密钥、模型与音色权限")?;
            Ok(())
        } else if matches!(provider_type,"openai"|"groq") {
            services::compatible::CompatibleClient::new(provider_type,settings.clone()).probe_asr().await.map_err(|_|"ASR 连接失败，请检查密钥与模型权限".into())
        } else {
            Err("请在语音练习页面录音验证 ASR 配置".into())
        }
    }
}
