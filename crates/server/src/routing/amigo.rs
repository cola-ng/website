use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::hoops::require_auth;
use crate::services::amigo::{
    agent::Agent,
    gateway::DeviceGateway,
    model::{Profile, Settings, clean},
    store,
};
use crate::{DepotExt, JsonResult, json_ok};

pub fn router() -> Router {
    router_with_backend(None)
}

pub fn router_with_backend(
    backend: Option<std::sync::Arc<dyn crate::services::amigo::tutor::Backend>>,
) -> Router {
    Router::with_path("amigo")
        .push(Router::with_path("session").get(DeviceGateway { backend }))
        .push(Router::with_path("pairing/start").post(pairing_start))
        .push(Router::with_path("pairing/status").post(pairing_status))
        .push(
            Router::new()
                .hoop(require_auth)
                .get(list)
                .post(provision)
                .push(Router::with_path("pairing/claim").post(pairing_claim))
                .push(
                    Router::with_path("devices")
                        .get(managed_devices)
                        .post(provision)
                        .push(
                            Router::with_path("{device_id}")
                                .get(device_profile)
                                .put(configure_device)
                                .delete(remove_device)
                                .push(Router::with_path("history").get(device_history))
                                .push(Router::with_path("calls").get(device_calls))
                                .push(Router::with_path("ai").get(device_ai).put(save_device_ai))
                                .push(Router::with_path("ai/voices").get(device_ai_voices)),
                        ),
                )
                .push(Router::with_path("profile").get(get_profile))
                .push(Router::with_path("settings").put(put_settings))
                .push(Router::with_path("forget").post(forget))
                .push(Router::with_path("ticket").post(issue_ticket))
                .push(Router::with_path("profile/item").put(edit_item))
                .push(Router::with_path("runtime").get(runtime_status))
                .push(Router::with_path("realtime").get(realtime_options))
                .push(Router::with_path("connection").get(connection_info))
                .push(
                    Router::with_path("learning")
                        .get(get_learning)
                        .put(set_learning),
                )
                .push(
                    Router::with_path("agents")
                        .get(list_agents)
                        .post(create_agent)
                        .push(
                            Router::with_path("{id}")
                                .put(update_agent)
                                .delete(archive_agent),
                        ),
                )
                .push(Router::with_path("{id}").delete(revoke)),
        )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProvisionRequest {
    name: String,
}

#[derive(Serialize)]
struct ProvisionResponse {
    device: store::Device,
    device_token: String,
}

#[handler]
async fn provision(req: &mut Request, depot: &mut Depot) -> JsonResult<ProvisionResponse> {
    let user = depot.user_id()?;
    let input: ProvisionRequest = parse(req).await?;
    if input.name.trim().is_empty() || input.name.len() > 80 {
        return Err(StatusError::bad_request()
            .brief("device name must be 1-80 bytes")
            .into());
    }
    let (device, device_token) = store::provision(user, clean(&input.name, 80))
        .await
        .map_err(|_| StatusError::service_unavailable())?;
    json_ok(ProvisionResponse {
        device,
        device_token,
    })
}

#[handler]
async fn list(depot: &mut Depot) -> JsonResult<Vec<store::Device>> {
    json_ok(
        store::devices(depot.user_id()?)
            .await
            .map_err(|_| StatusError::service_unavailable())?,
    )
}

#[handler]
async fn revoke(req: &mut Request, depot: &mut Depot) -> JsonResult<serde_json::Value> {
    let id: Uuid = req
        .param::<String>("id")
        .and_then(|id| id.parse().ok())
        .ok_or_else(StatusError::bad_request)?;
    if !store::revoke(depot.user_id()?, id)
        .await
        .map_err(|_| StatusError::service_unavailable())?
    {
        return Err(StatusError::not_found().into());
    }
    json_ok(serde_json::json!({"revoked":true}))
}

#[handler]
async fn get_profile(depot: &mut Depot) -> JsonResult<Profile> {
    json_ok(
        store::load(depot.user_id()?)
            .await
            .map_err(|_| StatusError::service_unavailable())?,
    )
}

#[handler]
async fn put_settings(req: &mut Request, depot: &mut Depot) -> JsonResult<Settings> {
    let settings: Settings = parse(req).await?;
    settings
        .validate()
        .map_err(|e| StatusError::bad_request().brief(e))?;
    let profile = store::update(depot.user_id()?, move |profile| profile.settings = settings)
        .await
        .map_err(|_| StatusError::service_unavailable())?;
    json_ok(profile.settings)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ForgetRequest {
    kind: String,
    key: String,
}

#[handler]
async fn forget(req: &mut Request, depot: &mut Depot) -> JsonResult<Profile> {
    let input: ForgetRequest = parse(req).await?;
    if !["memory", "weakness"].contains(&input.kind.as_str())
        || input.key.is_empty()
        || input.key.len() > 320
    {
        return Err(StatusError::bad_request()
            .brief("expected memory/weakness and item key")
            .into());
    }
    json_ok(
        store::update(depot.user_id()?, move |profile| {
            let key = input.key.trim().to_lowercase();
            if input.kind == "memory" {
                profile.memories.retain(|m| m.subject != key);
                for state in profile.agent_states.values_mut() {
                    state.memories.retain(|m| m.subject != key);
                }
            } else {
                profile.weaknesses.retain(|w| w.key != key);
                for state in profile.agent_states.values_mut() {
                    let learning = &mut state.learning;
                    if learning.items.iter().any(|item| item.key == key)
                        || learning.targets.iter().any(|target| target.key == key)
                    {
                        learning.items.retain(|item| item.key != key);
                        learning.targets.retain(|target| target.key != key);
                        learning.evidence.retain(|item| item.target_key != key);
                        if learning
                            .task
                            .as_ref()
                            .is_some_and(|task| task.target_keys.contains(&key))
                        {
                            learning.task = None;
                        }
                        learning.revision += 1;
                    }
                }
            }
            if !profile.forgotten.contains(&key) {
                profile.forgotten.push(key);
            }
        })
        .await
        .map_err(|_| StatusError::service_unavailable())?,
    )
}

async fn parse<T: serde::de::DeserializeOwned>(req: &mut Request) -> Result<T, StatusError> {
    let bytes = req
        .payload_with_max_size(16 * 1024)
        .await
        .map_err(|_| StatusError::bad_request().brief("request exceeds limit"))?;
    serde_json::from_slice(bytes).map_err(|_| StatusError::bad_request().brief("invalid JSON"))
}

#[derive(Deserialize)]
struct TicketRequest {
    device_id: Uuid,
}

#[handler]
async fn issue_ticket(req: &mut Request, depot: &mut Depot) -> JsonResult<serde_json::Value> {
    let input: TicketRequest = parse(req).await?;
    let ticket = store::ticket(depot.user_id()?, input.device_id)
        .await
        .map_err(|_| StatusError::service_unavailable())?
        .ok_or_else(StatusError::not_found)?;
    json_ok(serde_json::json!({"ticket":ticket,"expires_in":30}))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditItem {
    kind: String,
    key: String,
    summary: Option<String>,
    corrected: Option<String>,
    mastered: Option<bool>,
}

#[handler]
async fn edit_item(req: &mut Request, depot: &mut Depot) -> JsonResult<Profile> {
    let input: EditItem = parse(req).await?;
    if !["memory", "weakness"].contains(&input.kind.as_str())
        || input.key.len() > 320
        || input
            .summary
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.len() > 1500)
        || input
            .corrected
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.len() > 1500)
    {
        return Err(StatusError::bad_request()
            .brief("invalid profile correction")
            .into());
    }
    let key = input.key.trim().to_lowercase();
    let current = store::load(depot.user_id()?)
        .await
        .map_err(|_| StatusError::service_unavailable())?;
    if (input.kind == "memory" && !current.memories.iter().any(|m| m.subject == key))
        || (input.kind == "weakness" && !current.weaknesses.iter().any(|w| w.key == key))
    {
        return Err(StatusError::not_found().into());
    }
    json_ok(
        store::update(depot.user_id()?, move |profile| {
            if input.kind == "memory" {
                if let Some(memory) = profile.memories.iter_mut().find(|m| m.subject == key)
                    && let Some(summary) = input.summary
                {
                    memory.summary = clean(&summary, 240);
                    memory.confidence = 1.0;
                    memory.user_confirmed = true;
                    memory.updated_at = chrono::Utc::now();
                }
            } else if let Some(item) = profile.weaknesses.iter_mut().find(|w| w.key == key) {
                let changed = input.corrected.is_some() || input.mastered.is_some();
                if let Some(corrected) = input.corrected {
                    item.corrected = clean(&corrected, 240);
                    item.user_corrected = true;
                }
                if let Some(mastered) = input.mastered {
                    item.interval_days = if mastered { 30 } else { 0 };
                    item.next_review_at =
                        chrono::Utc::now() + chrono::Duration::days(i64::from(item.interval_days));
                }
                item.updated_at = chrono::Utc::now();
                let review_at = input.mastered.map(|_| item.next_review_at);
                if changed {
                    for state in profile.agent_states.values_mut() {
                        let learning = &mut state.learning;
                        let tracked = learning.items.iter().any(|target| target.key == key)
                            || learning.targets.iter().any(|target| target.key == key);
                        if tracked {
                            if let Some(target) =
                                learning.items.iter_mut().find(|target| target.key == key)
                                && let Some(at) = review_at
                            {
                                target.next_review_at = Some(at);
                            }
                            if learning
                                .task
                                .as_ref()
                                .is_some_and(|task| task.target_keys.contains(&key))
                            {
                                learning.task = None;
                            }
                            learning.revision += 1;
                        }
                    }
                }
            }
        })
        .await
        .map_err(|_| StatusError::service_unavailable())?,
    )
}

#[handler]
async fn list_agents(depot: &mut Depot) -> JsonResult<serde_json::Value> {
    let profile = store::load(depot.user_id()?)
        .await
        .map_err(|_| StatusError::service_unavailable())?;
    json_ok(serde_json::json!({
        "agents": profile.agents,
        "active_agent_id": profile.active_agent_id,
    }))
}

#[handler]
async fn create_agent(req: &mut Request, depot: &mut Depot) -> JsonResult<Profile> {
    let user = depot.user_id()?;
    let mut agent: Agent = parse(req).await?;
    if agent.id == "amigo" {
        agent.id = Uuid::new_v4().simple().to_string();
    }
    agent.builtin = false;
    let current = store::load(user)
        .await
        .map_err(|_| StatusError::service_unavailable())?;
    let mut validated = current;
    validated
        .activate(agent.clone())
        .map_err(|e| StatusError::bad_request().brief(e))?;
    let expected = agent.id.clone();
    let profile = store::update(user, move |profile| {
        let _ = profile.activate(agent);
    })
    .await
    .map_err(|_| StatusError::service_unavailable())?;
    if profile.active_agent_id != expected {
        return Err(StatusError::conflict()
            .brief("agent changed concurrently; retry")
            .into());
    }
    json_ok(profile)
}

#[handler]
async fn update_agent(req: &mut Request, depot: &mut Depot) -> JsonResult<Profile> {
    let user = depot.user_id()?;
    let id = req
        .param::<String>("id")
        .ok_or_else(StatusError::bad_request)?;
    let current = store::load(user)
        .await
        .map_err(|_| StatusError::service_unavailable())?;
    let existing = current
        .agents
        .iter()
        .find(|agent| agent.id == id)
        .cloned()
        .ok_or_else(StatusError::not_found)?;
    let mut agent: Agent = parse(req).await?;
    agent.id = id;
    agent.builtin = existing.builtin;
    let mut validated = current;
    validated
        .activate(agent.clone())
        .map_err(|e| StatusError::bad_request().brief(e))?;
    json_ok(
        store::update(user, move |profile| {
            let _ = profile.activate(agent);
        })
        .await
        .map_err(|_| StatusError::service_unavailable())?,
    )
}

#[handler]
async fn archive_agent(req: &mut Request, depot: &mut Depot) -> JsonResult<Profile> {
    let user = depot.user_id()?;
    let id = req
        .param::<String>("id")
        .ok_or_else(StatusError::bad_request)?;
    let current = store::load(user)
        .await
        .map_err(|_| StatusError::service_unavailable())?;
    if id == "amigo" || !current.agents.iter().any(|agent| agent.id == id) {
        return Err(StatusError::bad_request()
            .brief("custom agent not found or built-in agent cannot be deleted")
            .into());
    }
    json_ok(
        store::update(user, move |profile| {
            profile.archive_agent(&id);
        })
        .await
        .map_err(|_| StatusError::service_unavailable())?,
    )
}

#[handler]
async fn runtime_status(depot: &mut Depot) -> JsonResult<serde_json::Value> {
    let user = depot.user_id()?;
    let selection = crate::services::ocr::user_chat_config(user)
        .await
        .map_err(|_| StatusError::service_unavailable())?;
    let (provider, model) = selection
        .map(|(p, s)| (p, s["chat_model"].as_str().unwrap_or_default().to_owned()))
        .unwrap_or(("platform".into(), "账号默认服务".into()));
    let (ready, voice_ready, message) = match crate::services::get_user_provider(user).await {
        Ok(p) => (
            p.chat_service().is_some(),
            p.asr().is_some() && p.tts().is_some(),
            None,
        ),
        Err(e) => (false, false, Some(e.to_string())),
    };
    let realtime_ready=crate::services::amigo::live_voice::Config::resolve(user,&Settings::default()).await.is_ok();
    json_ok(
        serde_json::json!({"provider":provider,"model":model,"configured":ready,"ready":ready&&voice_ready,"voice_ready":voice_ready,"realtime_ready":realtime_ready,"message":message,"vision":"账号所选模型"}),
    )
}

#[handler]
async fn realtime_options(depot: &mut Depot, res: &mut Response) -> JsonResult<serde_json::Value> {
    use diesel::sql_types::{BigInt, Jsonb};
    use horme::prelude::*;
    use crate::db::with_conn;
    #[derive(QueryableByName)]
    struct Row { #[diesel(sql_type=Jsonb)] value: serde_json::Value }
    let user=depot.user_id()?;
    let providers=with_conn!(move|conn|diesel::sql_query("SELECT jsonb_build_object('id',id,'label','OpenAI #'||id) AS value FROM user_ai_providers WHERE user_id=$1 AND provider_type='openai' ORDER BY (tts_active OR is_active) DESC,id DESC")
        .bind::<BigInt,_>(user).load::<Row>(conn).await).await.map_err(|_|StatusError::service_unavailable())?;
    let models=crate::services::provider_catalog::definition("openai").unwrap().tts_models.into_iter()
        .filter(|m|crate::services::openai_realtime::is_model(m)).collect::<Vec<_>>();
    res.headers_mut().insert("Cache-Control","no-store".parse().unwrap());
    json_ok(serde_json::json!({"providers":providers.into_iter().map(|p|p.value).collect::<Vec<_>>(),"models":models}))
}

#[handler]
async fn get_learning(req: &mut Request, depot: &mut Depot) -> JsonResult<serde_json::Value> {
    let profile = store::load(depot.user_id()?)
        .await
        .map_err(|_| StatusError::service_unavailable())?;
    let agent_id = req
        .query::<String>("agent_id")
        .unwrap_or_else(|| profile.active_agent_id.clone());
    if !profile.agents.iter().any(|agent| agent.id == agent_id) {
        return Err(StatusError::not_found().into());
    }
    json_ok(
        serde_json::json!({"agent_id":agent_id,"session":crate::services::amigo::workflow::session(&profile,&agent_id),"catalog":crate::services::amigo::workflow::catalog()}),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LearningControl {
    agent_id: String,
    revision: u64,
    mode: String,
    #[serde(default)]
    goal: String,
    #[serde(default)]
    scene: String,
}
#[handler]
async fn set_learning(req: &mut Request, depot: &mut Depot) -> JsonResult<serde_json::Value> {
    use crate::services::amigo::workflow;
    let input: LearningControl = parse(req).await?;
    let book = workflow::playbook(&input.mode)
        .ok_or_else(|| StatusError::bad_request().brief("unknown learning mode"))?;
    if input.goal.len() > 1500 || input.scene.len() > 5000 {
        return Err(StatusError::bad_request()
            .brief("learning goal or scene exceeds limit")
            .into());
    }
    let user = depot.user_id()?;
    let current = store::load(user)
        .await
        .map_err(|_| StatusError::service_unavailable())?;
    if !current
        .agents
        .iter()
        .any(|agent| agent.id == input.agent_id)
    {
        return Err(StatusError::not_found().into());
    }
    let agent_id = input.agent_id.clone();
    let event = Uuid::new_v4();
    let profile = store::update(user, move |profile| {
        let mut state = workflow::session(profile, &input.agent_id);
        if state.revision != input.revision
            || !profile
                .agents
                .iter()
                .any(|agent| agent.id == input.agent_id)
        {
            return;
        }
        state.id = Uuid::new_v4().to_string();
        state.catalog_version = workflow::catalog().version;
        state.revision += 1;
        if state.mode != "conversation" && input.mode == "conversation" {
            state.practice_ended_at = Some(chrono::Utc::now());
        }
        state.mode = input.mode;
        state.stage = book.entry[0].clone();
        state.goal = clean(&input.goal, 240);
        state.scene = clean(&input.scene, 1000);
        state.strategy = book
            .stages
            .iter()
            .find(|stage| stage.id == state.stage)
            .map(|stage| stage.strategy.clone())
            .unwrap_or_default();
        state.task = None;
        state.targets.clear();
        state.practice_attempts = 0;
        state.awaiting_choice = false;
        state.last_event = Some(event);
        profile
            .agent_states
            .entry(input.agent_id)
            .or_default()
            .learning = state;
    })
    .await
    .map_err(|_| StatusError::service_unavailable())?;
    let state = workflow::session(&profile, &agent_id);
    if state.last_event != Some(event) {
        return Err(StatusError::conflict()
            .brief("学习方式已更新，请刷新后再调整")
            .into());
    }
    json_ok(serde_json::json!({"agent_id":agent_id,"session":state,"catalog":workflow::catalog()}))
}

async fn managed(req: &Request, depot: &Depot) -> Result<store::Device, salvo::http::StatusError> {
    let user = depot.user_id().map_err(|_| StatusError::unauthorized())?;
    let id = req
        .param::<Uuid>("device_id")
        .ok_or_else(StatusError::bad_request)?;
    crate::services::amigo::devices::owned(user, id)
        .await
        .map_err(|_| StatusError::service_unavailable())?
        .ok_or_else(StatusError::not_found)
}
#[handler]
async fn managed_devices(
    depot: &mut Depot,
) -> JsonResult<Vec<crate::services::amigo::devices::Details>> {
    json_ok(
        crate::services::amigo::devices::list(depot.user_id()?)
            .await
            .map_err(|_| StatusError::service_unavailable())?,
    )
}
#[handler]
async fn device_profile(req: &mut Request, depot: &mut Depot) -> JsonResult<Profile> {
    let device = managed(req, depot).await?;
    json_ok(
        crate::services::amigo::devices::load(device)
            .await
            .map_err(|_| StatusError::service_unavailable())?,
    )
}
#[handler]
async fn configure_device(req: &mut Request, depot: &mut Depot) -> JsonResult<Profile> {
    let device = managed(req, depot).await?;
    let mut input: crate::services::amigo::devices::Update = parse(req).await?;
    if input.name.trim().is_empty() || input.name.len() > 80 || input.configuration.volume > 100 {
        return Err(StatusError::bad_request()
            .brief("Invalid name or volume")
            .into());
    }
    input
        .settings
        .validate()
        .map_err(|e| StatusError::bad_request().brief(e))?;
    if input.settings.conversation_mode == crate::services::amigo::model::ConversationMode::Realtime {
        crate::services::amigo::live_voice::Config::for_device(&device, &input.settings)
            .await.map_err(|e|StatusError::bad_request().brief(e))?;
    }
    let profile = store::load(device.user_id)
        .await
        .map_err(|_| StatusError::service_unavailable())?;
    if !profile.agents.iter().any(|a| a.id == input.agent_id) {
        return Err(StatusError::bad_request()
            .brief("Unknown companion role")
            .into());
    }
    if let Some(agent) = input.companion.as_mut() {
        agent.id = input.agent_id.clone();
        agent.builtin = false;
        agent
            .validate()
            .map_err(|e| StatusError::bad_request().brief(e))?;
    }
    json_ok(
        crate::services::amigo::devices::save(device, input)
            .await
            .map_err(|_| StatusError::service_unavailable())?,
    )
}
#[handler]
async fn device_history(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<Vec<crate::services::amigo::devices::Event>> {
    let device = managed(req, depot).await?;
    json_ok(
        crate::services::amigo::devices::history(device)
            .await
            .map_err(|_| StatusError::service_unavailable())?,
    )
}
#[handler]
async fn remove_device(req: &mut Request, depot: &mut Depot) -> JsonResult<serde_json::Value> {
    let device = managed(req, depot).await?;
    store::revoke(device.user_id, device.id)
        .await
        .map_err(|_| StatusError::service_unavailable())?;
    json_ok(serde_json::json!({"revoked":true}))
}

#[handler]
async fn connection_info(req: &mut Request, depot: &mut Depot) -> JsonResult<serde_json::Value> {
    depot.user_id()?;
    let gateway = std::env::var("AMIGO_GATEWAY_URL")
        .ok()
        .filter(|s| url::Url::parse(s).is_ok())
        .unwrap_or_else(|| {
            let host = req
                .headers()
                .get("host")
                .and_then(|s| s.to_str().ok())
                .unwrap_or("localhost");
            let scheme = if req.uri().scheme_str() == Some("https") {
                "wss"
            } else {
                "ws"
            };
            format!("{scheme}://{host}/api/amigo/session")
        });
    json_ok(serde_json::json!({"gateway_url":gateway}))
}

fn pairing_token(req: &Request) -> Result<String, StatusError> {
    req.headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .filter(|s| {
            (s.starts_with("am_") || s.starts_with("lb_"))
                && (32..128).contains(&s.len())
                && s.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        })
        .map(str::to_owned)
        .ok_or_else(StatusError::unauthorized)
}
fn pairing_ip(req: &Request) -> String {
    req.remote_addr()
        .as_ipv4()
        .map(|s| s.ip().to_string())
        .or_else(|| req.remote_addr().as_ipv6().map(|s| s.ip().to_string()))
        .unwrap_or_else(|| "unknown".into())
}
async fn pairing_limit(key: String, maximum: i32) -> Result<(), StatusError> {
    if !crate::services::amigo::pairing::limit(key, maximum)
        .await
        .map_err(|_| StatusError::service_unavailable())?
    {
        return Err(StatusError::too_many_requests().brief("尝试过于频繁，请一分钟后再试"));
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairingStart {
    board: String,
}
#[handler]
async fn pairing_start(req: &mut Request, res: &mut Response) -> JsonResult<serde_json::Value> {
    let token = pairing_token(req)?;
    pairing_limit(format!("start:{}", pairing_ip(req)), 30).await?;
    let input: PairingStart = parse(req).await?;
    if !amigo::protocol::supported_board(&input.board) {
        return Err(StatusError::bad_request().brief("unsupported board").into());
    }
    res.add_header("Cache-Control", "no-store", true)?;
    json_ok(
        crate::services::amigo::pairing::start(&token, input.board)
            .await
            .map_err(|_| StatusError::service_unavailable())?,
    )
}
#[handler]
async fn pairing_status(req: &mut Request, res: &mut Response) -> JsonResult<serde_json::Value> {
    let token = pairing_token(req)?;
    pairing_limit(format!("poll-ip:{}", pairing_ip(req)), 300).await?;
    pairing_limit(format!("poll:{}", crate::auth::hash_code(&token)), 60).await?;
    res.add_header("Cache-Control", "no-store", true)?;
    json_ok(
        crate::services::amigo::pairing::status(&token)
            .await
            .map_err(|_| StatusError::service_unavailable())?,
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairingClaim {
    code: String,
    name: String,
}
#[handler]
async fn pairing_claim(req: &mut Request, depot: &mut Depot) -> JsonResult<store::Device> {
    let user = depot.user_id()?;
    pairing_limit(format!("claim-user:{user}"), 5).await?;
    pairing_limit(format!("claim-ip:{}", pairing_ip(req)), 10).await?;
    let input: PairingClaim = parse(req).await?;
    let code = input.code.trim().replace([' ', '-'], "");
    if code.len() != 6
        || !code.bytes().all(|c| c.is_ascii_digit())
        || input.name.trim().is_empty()
        || input.name.len() > 80
    {
        return Err(StatusError::bad_request()
            .brief("请输入设备屏幕上的六位验证码和设备名称")
            .into());
    }
    let device = crate::services::amigo::pairing::claim(user, code, clean(&input.name, 80))
        .await
        .map_err(|_| StatusError::service_unavailable())?
        .ok_or_else(|| {
            StatusError::bad_request()
                .brief("验证码无效、已过期或已被使用，请查看设备屏幕上的最新验证码")
        })?;
    json_ok(device)
}

#[handler]
async fn device_calls(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<Vec<crate::services::amigo::calls::Call>> {
    let device = managed(req, depot).await?;
    json_ok(
        crate::services::amigo::calls::list(device)
            .await
            .map_err(|_| StatusError::service_unavailable())?,
    )
}

#[handler]
async fn device_ai(req:&mut Request,depot:&mut Depot,res:&mut Response)->JsonResult<serde_json::Value>{
    use crate::services::amigo::ai;
    let device=managed(req,depot).await?;
    let bindings=ai::load(&device).await.map_err(|_|StatusError::service_unavailable())?;
    let mut effective=serde_json::Map::new();
    for cap in ai::CAPS {
        let binding=bindings.get(cap).and_then(Option::as_ref);
        if cap=="realtime" && binding.is_none(){
            let settings=crate::services::amigo::devices::load(device.clone()).await.map_err(|_|StatusError::service_unavailable())?.settings;
            let live=crate::services::amigo::live_voice::Config::for_device(&device,&settings).await;
            effective.insert(cap.into(),match live{Ok(config)=>serde_json::json!({"provider_type":"openai","model":config.model,"voice":config.voice,"region":"global","inherited":true}),Err(error)=>serde_json::json!({"inherited":true,"error":error})});
            continue;
        }
        let row=ai::resolve(device.user_id,cap,binding).await;
        effective.insert(cap.into(),match row {
            Ok(Some(r))=>serde_json::json!({"provider_type":r.kind,"model":r.model(cap),"voice":r.settings["tts_voice"],"region":r.settings["region"],"inherited":binding.is_none()}),
            Ok(None)=>serde_json::json!({"inherited":binding.is_none(),"error":"全局尚未配置该服务"}),
            Err(error)=>serde_json::json!({"inherited":binding.is_none(),"error":error}),
        });
    }
    res.headers_mut().insert("Cache-Control","no-store".parse().unwrap());
    json_ok(serde_json::json!({"bindings":ai::public(&bindings),"effective":effective}))
}
#[handler]
async fn save_device_ai(req:&mut Request,depot:&mut Depot,res:&mut Response)->JsonResult<serde_json::Value>{
    let device=managed(req,depot).await?;
    let input:crate::services::amigo::ai::Bindings=req.parse_json().await.map_err(|_|StatusError::bad_request().brief("设备 AI 配置格式无效"))?;
    let bindings=crate::services::amigo::ai::save(device,input).await.map_err(|e|StatusError::bad_request().brief(e))?;
    res.headers_mut().insert("Cache-Control","no-store".parse().unwrap());
    json_ok(serde_json::json!({"bindings":crate::services::amigo::ai::public(&bindings)}))
}
#[handler]
async fn device_ai_voices(req:&mut Request,depot:&mut Depot)->JsonResult<Vec<crate::services::voices::Voice>>{
    let device=managed(req,depot).await?;
    let cap=req.query::<String>("capability").unwrap_or("tts".into());
    if !["tts","realtime"].contains(&cap.as_str()){return Err(StatusError::bad_request().into())}
    let bindings=crate::services::amigo::ai::load(&device).await.map_err(|_|StatusError::service_unavailable())?;
    let row=crate::services::amigo::ai::resolve(device.user_id,&cap,bindings.get(&cap).and_then(Option::as_ref)).await.map_err(|e|StatusError::bad_request().brief(e))?;
    let kind=req.query::<String>("kind").unwrap_or_default();let model=req.query::<String>("model").unwrap_or_default();
    let Some(mut row)=row.filter(|r|kind.is_empty()||r.kind==kind) else{return json_ok(crate::services::voices::presets_for_model(&kind,&model))};
    if !model.is_empty(){row.settings["tts_model"]=serde_json::json!(model);}
    json_ok(crate::services::voices::list(&row.kind,&row.settings).await.map_err(|e|StatusError::bad_gateway().brief(e))?)
}
