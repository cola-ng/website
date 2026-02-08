use chrono::Utc;
use diesel::prelude::*;
use salvo::prelude::*;
use serde_json::Value;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::ai_provider::*;
use crate::services::{self, ProviderConfig};
use crate::{AppResult, DepotExt, JsonResult, hoops, json_ok};

pub fn router() -> Router {
    Router::with_path("ai-providers")
        .hoop(hoops::require_auth)
        .get(list_providers)
        .post(create_provider)
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

/// Mask sensitive fields in settings JSON
fn mask_settings(settings: &Value) -> Value {
    let mut masked = settings.clone();
    if let Some(obj) = masked.as_object_mut() {
        for (key, value) in obj.iter_mut() {
            if key.contains("key") || key.contains("token") || key.contains("secret") {
                if let Some(s) = value.as_str() {
                    let len = s.len();
                    if len > 8 {
                        *value = Value::String(format!("{}****{}", &s[..4], &s[len - 4..]));
                    } else {
                        *value = Value::String("****".to_string());
                    }
                }
            }
        }
    }
    masked
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

    // Validate provider type
    let provider_type = input.provider_type.to_lowercase();
    if provider_type != "zhipu" && provider_type != "doubao" {
        return Err(StatusError::bad_request()
            .brief("invalid provider_type, must be 'zhipu' or 'doubao'")
            .into());
    }

    let set_active = input.set_active;
    let settings = input.settings.clone();

    let provider: UserAiProvider = with_conn(move |conn| {
        // If set_active is true, deactivate all other providers first
        if set_active {
            diesel::update(
                user_ai_providers::table
                    .filter(user_ai_providers::user_id.eq(user_id))
                    .filter(user_ai_providers::is_active.eq(true)),
            )
            .set((
                user_ai_providers::is_active.eq(false),
                user_ai_providers::updated_at.eq(Utc::now()),
            ))
            .execute(conn)?;
        }

        let new_provider = NewUserAiProvider {
            user_id,
            provider_type,
            is_active: set_active,
            settings,
        };

        diesel::insert_into(user_ai_providers::table)
            .values(&new_provider)
            .get_result::<UserAiProvider>(conn)
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

    let is_active = input.is_active;
    let settings = input.settings.clone();

    let provider: UserAiProvider = with_conn(move |conn| {
        // Verify ownership
        let existing = user_ai_providers::table
            .filter(user_ai_providers::id.eq(provider_id))
            .filter(user_ai_providers::user_id.eq(user_id))
            .first::<UserAiProvider>(conn)?;

        // If activating this provider, deactivate others first
        if is_active == Some(true) && !existing.is_active {
            diesel::update(
                user_ai_providers::table
                    .filter(user_ai_providers::user_id.eq(user_id))
                    .filter(user_ai_providers::is_active.eq(true)),
            )
            .set((
                user_ai_providers::is_active.eq(false),
                user_ai_providers::updated_at.eq(Utc::now()),
            ))
            .execute(conn)?;
        }

        let update = UpdateUserAiProvider {
            is_active,
            settings,
            updated_at: Some(Utc::now()),
            ..Default::default()
        };

        diesel::update(user_ai_providers::table.filter(user_ai_providers::id.eq(provider_id)))
            .set(&update)
            .get_result::<UserAiProvider>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update provider"))?;

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

    with_conn(move |conn| {
        diesel::delete(
            user_ai_providers::table
                .filter(user_ai_providers::id.eq(provider_id))
                .filter(user_ai_providers::user_id.eq(user_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete provider"))?;

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

    let provider: UserAiProvider = with_conn(move |conn| {
        // Verify ownership
        let _ = user_ai_providers::table
            .filter(user_ai_providers::id.eq(provider_id))
            .filter(user_ai_providers::user_id.eq(user_id))
            .first::<UserAiProvider>(conn)?;

        // Deactivate all other providers
        diesel::update(
            user_ai_providers::table
                .filter(user_ai_providers::user_id.eq(user_id))
                .filter(user_ai_providers::is_active.eq(true)),
        )
        .set((
            user_ai_providers::is_active.eq(false),
            user_ai_providers::updated_at.eq(Utc::now()),
        ))
        .execute(conn)?;

        // Activate this provider
        diesel::update(user_ai_providers::table.filter(user_ai_providers::id.eq(provider_id)))
            .set((
                user_ai_providers::is_active.eq(true),
                user_ai_providers::updated_at.eq(Utc::now()),
            ))
            .get_result::<UserAiProvider>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to activate provider"))?;

    json_ok(to_response(provider))
}

// ============================================================================
// Provider Testing Helper
// ============================================================================

async fn test_provider_connection(provider_type: &str, settings: &Value) -> Result<(), String> {
    let config = match provider_type {
        "zhipu" => {
            let zhipu_settings: ZhipuSettings = serde_json::from_value(settings.clone())
                .map_err(|e| format!("Invalid Zhipu settings: {}", e))?;
            ProviderConfig::Zhipu {
                api_key: zhipu_settings.api_key,
                chat_model: Some(zhipu_settings.chat_model),
                asr_model: None,
                tts_model: None,
            }
        }
        "doubao" => {
            let doubao_settings: DoubaoSettings = serde_json::from_value(settings.clone())
                .map_err(|e| format!("Invalid Doubao settings: {}", e))?;
            ProviderConfig::Doubao {
                app_id: doubao_settings.app_id,
                access_token: doubao_settings.access_token,
                api_key: doubao_settings.api_key,
                chat_model: Some(doubao_settings.chat_model),
                tts_resource_id: doubao_settings.tts_resource_id,
                voice_type: doubao_settings.voice_type,
            }
        }
        _ => return Err(format!("Unknown provider type: {}", provider_type)),
    };

    let provider = services::create_provider(&config);

    // Test chat service with a simple message
    if let Some(chat) = provider.chat_service() {
        let messages = vec![services::ChatMessage {
            role: "user".to_string(),
            content: "Hello, please respond with 'OK' if you can hear me.".to_string(),
        }];

        chat.chat(messages, None, None)
            .await
            .map_err(|e| format!("Chat test failed: {}", e))?;

        Ok(())
    } else {
        Err("Provider does not support chat service".to_string())
    }
}
