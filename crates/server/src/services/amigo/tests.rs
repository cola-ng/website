//! Opt-in integration test: real TCP/WebSocket, routes and PostgreSQL;
//! deterministic model services explicitly identified by Welcome.test_mode.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use amigo::protocol::*;
use async_trait::async_trait;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use horme::prelude::*;
use salvo::prelude::*;
use std::result::Result;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use super::{agent::Agent, audio, model::*, store, tutor::Backend};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Default)]
struct TestBackend {
    plans: Mutex<Vec<Plan>>,
    workflow_plans: Mutex<std::collections::VecDeque<super::workflow::Decision>>,
}

#[async_trait]
impl Backend for TestBackend {
    async fn plan_turn(
        &self,
        _profile: &Profile,
        _text: &str,
        _proactive: bool,
        _camera_available: Option<bool>,
    ) -> Result<Option<super::workflow::Decision>, String> {
        Ok(self.workflow_plans.lock().unwrap().pop_front())
    }
    async fn reply_stream(
        &self,
        profile: &Profile,
        plan: &Plan,
        text: &str,
        sentences: tokio::sync::mpsc::Sender<String>,
    ) -> Result<Analysis, String> {
        let mut analysis = self.reply(profile, plan, text).await?;
        if analysis.next_task.is_some() {
            sentences
                .send(analysis.reply_en.clone())
                .await
                .map_err(|_| "cancelled")?;
            return Ok(analysis);
        }
        analysis.reply_en = "First sentence. Last sentence.".into();
        sentences
            .send("First sentence.".into())
            .await
            .map_err(|_| "cancelled")?;
        tokio::time::sleep(Duration::from_millis(800)).await;
        if text == "stream fail" {
            return Err("injected stream failure".into());
        }
        sentences
            .send("Last sentence.".into())
            .await
            .map_err(|_| "cancelled")?;
        Ok(analysis)
    }
    fn test_mode(&self) -> bool {
        true
    }
    async fn transcribe(&self, pcm: &[u8]) -> Result<String, String> {
        assert!(pcm.len() >= 3200 && pcm.iter().any(|b| *b != 0));
        assert_eq!(&audio::wav(pcm).unwrap()[..4], b"RIFF");
        if pcm.iter().all(|b| *b == 2) {
            // Real ASR may return punctuation for noise instead of an empty
            // string. It must not start a reply or a speaker/echo feedback loop.
            return Ok("#".into());
        }
        Ok("I go yesterday".into())
    }
    async fn observe(&self, source: Source, media: &[u8]) -> Result<(String, Analysis), String> {
        if source == Source::Speech {
            assert!(!media.is_empty());
        } else {
            assert!(media.starts_with(&[255, 216]));
            if media == [255, 216, 42] {
                return Ok((String::new(), Analysis::default()));
            }
        }
        Ok((
            "I am reading a book about photosynthesis".into(),
            Analysis {
                topics: vec!["plants".into()],
                memories: vec![MemoryEvidence {
                    subject: "plants".into(),
                    summary: "Reading a book about plants".into(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        ))
    }
    async fn reply(&self, _profile: &Profile, plan: &Plan, text: &str) -> Result<Analysis, String> {
        self.plans.lock().unwrap().push(plan.clone());
        if text == "slow" {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        if text == "fail" {
            return Err("injected provider failure".into());
        }
        let mut analysis = Analysis {
            reply_en: if plan.proactive {
                "You are reading about plants. What did you learn yesterday?".into()
            } else {
                "Try: I went yesterday. What did you read?".into()
            },
            hint_zh: "用一句英语回答。".into(),
            ..Default::default()
        };
        if let Some(target) = plan
            .workflow
            .as_ref()
            .and_then(|flow| flow.decision.as_ref())
            .and_then(|decision| decision.task_spec.as_ref())
            .and_then(|task| task.target_keys.first())
        {
            let prompt = format!("Repeat {target} after me.");
            analysis.reply_en = format!("Let's practice together. {prompt}");
            analysis.next_task = Some(super::workflow::PresentedTask {
                prompt,
                target_keys: vec![],
                kind: String::new(),
                support: super::workflow::Support::None,
            });
        }
        if text == "I go yesterday" {
            analysis.issues.push(Issue {
                key: "past tense".into(),
                kind: "grammar".into(),
                original: text.into(),
                corrected: "I went yesterday".into(),
            });
        }
        if text == "I went yesterday" {
            analysis.recalls.push(Recall {
                key: "past tense".into(),
                evidence: text.into(),
                success: true,
            });
        }
        Ok(analysis)
    }
    async fn synthesize(&self, _text: &str, _agent: &Agent) -> Result<Vec<u8>, String> {
        // Non-speech tone, not a fake claim of real synthesized speech.
        let pcm: Vec<_> = (0..1600)
            .flat_map(|i| (((i as f32 * 0.2).sin() * 8000.0) as i16).to_le_bytes())
            .collect();
        audio::decode(audio::wav(&pcm)?, "wav")
    }
}

async fn send(ws: &mut Socket, message: ClientMessage) {
    ws.send(Message::Text(
        serde_json::to_string(&message).unwrap().into(),
    ))
    .await
    .unwrap();
}

async fn event(ws: &mut Socket) -> ServerMessage {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(15), ws.next())
            .await
            .expect("server event timeout")
            .expect("server disconnected")
            .unwrap();
        if msg.is_text() {
            return serde_json::from_str(msg.to_text().unwrap()).unwrap();
        }
        assert!(!msg.is_close(), "unexpected close");
    }
}

async fn connect(base: &str, token: &str) -> Socket {
    connect_mode(base, token, false).await
}
async fn connect_mode(base: &str, token: &str, streaming: bool) -> Socket {
    connect_board(base, token, streaming, "simulator").await
}
async fn connect_board(base: &str, token: &str, streaming: bool, board: &str) -> Socket {
    for _ in 0..30 {
        let mut request = format!("{}/api/amigo/session", base.replacen("http://", "ws://", 1))
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert("Authorization", format!("Bearer {token}").parse().unwrap());
        if let Ok((mut ws, _)) = tokio_tungstenite::connect_async(request).await {
            send(
                &mut ws,
                ClientMessage::Hello {
                    version: VERSION,
                    board: board.into(),
                    sample_rate_hz: 16000,
                    channels: 1,
                    audio: None,
                    streaming,
                    service_status: false,
                },
            )
            .await;
            assert!(matches!(
                event(&mut ws).await,
                ServerMessage::Welcome {
                    test_mode: true,
                    ..
                }
            ));
            return ws;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("could not connect device");
}

async fn reply(ws: &mut Socket, expected: Option<u64>) -> (u64, bool, Vec<String>) {
    let reply = receive_reply(ws, expected).await;
    finish_reply(ws, reply.0).await;
    reply
}

async fn finish_reply(ws: &mut Socket, turn_id: u64) {
    send(ws, ClientMessage::PlaybackDone { turn_id }).await;
    loop {
        match event(ws).await {
            ServerMessage::Ack { turn_id: id, state } if id == turn_id && state == "ready" => {
                return;
            }
            ServerMessage::Error { code, message, .. } => panic!("gateway {code}: {message}"),
            _ => {}
        }
    }
}

async fn receive_reply(ws: &mut Socket, expected: Option<u64>) -> (u64, bool, Vec<String>) {
    let (mut turn, mut proactive, mut targets, mut pcm, mut sequence) =
        (None, false, vec![], vec![], 0);
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(15), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if msg.is_binary() {
            let bytes = msg.into_data();
            let packet = MediaPacket::decode(&bytes).unwrap();
            assert_eq!(packet.kind, MediaKind::AudioOut);
            assert_eq!(Some(packet.turn_id), turn);
            assert_eq!(packet.sequence, sequence);
            sequence += 1;
            pcm.extend_from_slice(packet.payload);
        } else if msg.is_text() {
            match serde_json::from_str::<ServerMessage>(msg.to_text().unwrap()).unwrap() {
                ServerMessage::Reply {
                    turn_id,
                    proactive: p,
                    targets: t,
                    ..
                } => {
                    if let Some(expected) = expected {
                        assert_eq!(turn_id, expected);
                    }
                    turn = Some(turn_id);
                    proactive = p;
                    targets = t;
                }
                ServerMessage::AudioEnd { turn_id, bytes } => {
                    assert_eq!(Some(turn_id), turn);
                    assert_eq!(pcm.len(), bytes);
                    assert!(pcm.iter().any(|b| *b != 0));
                    return (turn_id, proactive, targets);
                }
                ServerMessage::Error { code, message, .. } => panic!("gateway {code}: {message}"),
                _ => {}
            }
        }
    }
}

async fn start_server(backend: Arc<TestBackend>) -> (String, tokio::task::JoinHandle<()>) {
    let acceptor = TcpListener::new("127.0.0.1:0").bind().await;
    let base = format!("http://{}", acceptor.local_addr().unwrap());
    let router = crate::routing::router_with_amigo_backend(Some(backend));
    let task = tokio::spawn(async move {
        Server::new(acceptor).serve(router).await;
    });
    (base, task)
}

async fn verify_voice_ownership(http: &reqwest::Client, base: &str, jwt: &str, other: &str) {
    let root = format!("{base}/api/ai-providers");
    let provider=http.post(&root).bearer_auth(jwt).json(&serde_json::json!({"provider_type":"zhipu","settings":{"api_key":"synthetic-voice-key","chat_model":"","asr_model":"","tts_model":"glm-tts"},"activate_for":[]})).send().await.unwrap().error_for_status().unwrap().json::<serde_json::Value>().await.unwrap();
    let id = provider["id"].as_i64().unwrap();
    let path = format!("{root}/{id}/voices");
    assert_eq!(http.get(&path).send().await.unwrap().status(), 401);
    assert_eq!(
        http.get(&path)
            .bearer_auth(other)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let list = http
        .get(&path)
        .bearer_auth(jwt)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(list.as_array().unwrap().len(), 7);
    assert!(!list.to_string().contains("synthetic-voice-key"));
    assert_eq!(
        http.post(format!("{root}/voice-preview"))
            .bearer_auth(other)
            .json(&serde_json::json!({"provider_id":id,"voice_id":"tongtong"}))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        http.post(format!("{root}/voice-preview"))
            .bearer_auth(jwt)
            .json(&serde_json::json!({"provider_id":id,"voice_id":"invalid\r\nvoice"}))
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    http.delete(format!("{root}/{id}"))
        .bearer_auth(jwt)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

// The production route must keep a real authenticated control socket alive even
// when the owner has no AI access, and apply settings without calling a model.
async fn verify_control_without_ai(user: i64) {
    let acceptor = TcpListener::new("127.0.0.1:0").bind().await;
    let base = format!("http://{}", acceptor.local_addr().unwrap());
    let task = tokio::spawn(async move {
        Server::new(acceptor)
            .serve(crate::routing::router_with_amigo_backend(None))
            .await;
    });
    let (device, token) = store::provision(user, "No AI control".into())
        .await
        .unwrap();
    let mut request = format!("{}/api/amigo/session", base.replacen("http:", "ws:", 1))
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("Authorization", format!("Bearer {token}").parse().unwrap());
    let (mut ws, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    send(
        &mut ws,
        ClientMessage::Hello {
            version: VERSION,
            board: "lckfb-szpi-esp32s3".into(),
            sample_rate_hz: 16000,
            channels: 1,
            audio: None,
            streaming: false,
            service_status: true,
        },
    )
    .await;
    assert!(matches!(
        event(&mut ws).await,
        ServerMessage::Welcome {
            test_mode: false,
            ..
        }
    ));
    assert!(matches!(
        event(&mut ws).await,
        ServerMessage::ServiceState { ready: false }
    ));
    send(
        &mut ws,
        ClientMessage::Text {
            turn_id: 1,
            text: "must not reach a model".into(),
        },
    )
    .await;
    assert!(
        matches!(event(&mut ws).await,ServerMessage::Error{code,..} if code=="ai_setup_required")
    );
    let config = serde_json::json!({"revision":1234567,"volume":85,"continuous":true,"voice_interrupt":true,"playback_wake":true});
    let id = device.id;
    crate::db::with_conn!(move |conn| diesel::sql_query(
        "UPDATE amigo_devices SET configuration=$2 WHERE id=$1"
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Jsonb, _>(config)
    .execute(conn)
    .await)
    .await
    .unwrap();
    assert!(matches!(
        event(&mut ws).await,
        ServerMessage::Configuration {
            revision: 1234567,
            volume: 85,
            ..
        }
    ));
    send(
        &mut ws,
        ClientMessage::ConfigurationApplied { revision: 1234567 },
    )
    .await;
    send(&mut ws, ClientMessage::Ping).await;
    assert!(matches!(event(&mut ws).await, ServerMessage::Pong));
    let listed = super::devices::list(user).await.unwrap();
    let found = listed.iter().find(|d| d.id == id).unwrap();
    assert!(found.online);
    assert_eq!(found.applied_revision, Some(1234567));
    // Real provider construction with a dummy key is local only; no cloud calls.
    let settings = serde_json::json!({"api_key":"test-not-a-real-key","chat_model":"glm-4-flash","asr_model":"glm-asr-2512","tts_model":"glm-tts"});
    crate::db::with_conn!(move |conn|diesel::sql_query("INSERT INTO user_ai_providers(user_id,provider_type,settings,is_active,asr_active,tts_active) VALUES($1,'zhipu',$2,true,true,true)").bind::<diesel::sql_types::BigInt,_>(user).bind::<diesel::sql_types::Jsonb,_>(settings).execute(conn).await).await.unwrap();
    assert!(matches!(
        event(&mut ws).await,
        ServerMessage::ServiceState { ready: true }
    ));
    crate::db::with_conn!(move |conn| diesel::sql_query(
        "DELETE FROM user_ai_providers WHERE user_id=$1"
    )
    .bind::<diesel::sql_types::BigInt, _>(user)
    .execute(conn)
    .await)
    .await
    .unwrap();
    assert!(matches!(
        event(&mut ws).await,
        ServerMessage::ServiceState { ready: false }
    ));
    send(&mut ws, ClientMessage::Ping).await;
    assert!(matches!(event(&mut ws).await, ServerMessage::Pong));
    ws.close(None).await.unwrap();
    store::revoke(user, id).await.unwrap();
    task.abort();
}

// An abrupt reboot must replace a half-open socket without waiting for its
// heartbeat timeout. Old lease cleanup must not clear the replacement lease.
async fn verify_reconnection_takeover(base: &str, token: &str, user: i64, id: Uuid) {
    let mut old = connect(base, token).await;
    let mut fresh = connect(base, token).await;
    send(&mut fresh, ClientMessage::Ping).await;
    assert!(matches!(event(&mut fresh).await, ServerMessage::Pong));
    let start = std::time::Instant::now();
    tokio::time::timeout(Duration::from_secs(8), async {
        while let Some(Ok(message)) = old.next().await {
            if message.is_close() {
                break;
            }
        }
    })
    .await
    .expect("replaced connection was not retired");
    assert!(start.elapsed() < Duration::from_secs(8));
    send(&mut fresh, ClientMessage::Ping).await;
    assert!(matches!(event(&mut fresh).await, ServerMessage::Pong));
    assert!(
        super::devices::list(user)
            .await
            .unwrap()
            .iter()
            .find(|d| d.id == id)
            .unwrap()
            .online
    );
    fresh.close(None).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
}

// Physical-code enrollment uses the real database and the normal account guard.
async fn verify_pairing(
    http: &reqwest::Client,
    api: &str,
    user: i64,
    other: i64,
    jwt: &str,
    other_jwt: &str,
) {
    use super::pairing;
    use serde_json::{Value, json};
    let secret = format!("am_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    assert_eq!(
        http.post(format!("{api}/pairing/start"))
            .json(&json!({"board":"lckfb-szpi-esp32s3"}))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        http.post(format!("{api}/pairing/claim"))
            .json(&json!({"code":"123456","name":"test"}))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        http.post(format!("{api}/pairing/start"))
            .bearer_auth(&secret)
            .json(&json!({"board":"invalid"}))
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    let response = http
        .post(format!("{api}/pairing/start"))
        .bearer_auth(&secret)
        .json(&json!({"board":"lckfb-szpi-esp32s3"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let started: Value = response.json().await.unwrap();
    let code = started["code"].as_str().unwrap();
    assert_eq!(code.len(), 6);
    assert!(code.bytes().all(|b| b.is_ascii_digit()));
    assert!((1..=300).contains(&started["expires_in"].as_i64().unwrap()));
    assert_eq!(
        pairing::start(&secret, "lckfb-szpi-esp32s3".into())
            .await
            .unwrap()["code"],
        code
    );
    let waiting = http
        .post(format!("{api}/pairing/status"))
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    assert_eq!(waiting["state"], "waiting");
    assert!(waiting.get("code").is_none());
    let claim = http
        .post(format!("{api}/pairing/claim"))
        .bearer_auth(jwt)
        .json(&json!({"code":format!("{} {}",&code[..3],&code[3..]),"name":"Paired board"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    let id: Uuid = claim["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(claim["user_id"], user);
    assert!(!claim.to_string().contains(&secret));
    assert_eq!(
        pairing::status(&secret).await.unwrap(),
        json!({"state":"paired","device_id":id,"ai_ready":false})
    );
    assert_eq!(
        pairing::start(&secret, "lckfb-szpi-esp32s3".into())
            .await
            .unwrap()["state"],
        "paired"
    );
    let retry = http
        .post(format!("{api}/pairing/claim"))
        .bearer_auth(jwt)
        .json(&json!({"code":code,"name":"Retry"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    assert_eq!(retry["id"], claim["id"]);
    assert_eq!(
        http.post(format!("{api}/pairing/claim"))
            .bearer_auth(other_jwt)
            .json(&json!({"code":code,"name":"Steal"}))
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    assert!(store::revoke(user, id).await.unwrap());
    assert_eq!(pairing::status(&secret).await.unwrap()["state"], "revoked");
    assert!(
        pairing::claim(user, code.into(), "Revoked retry".into())
            .await
            .unwrap()
            .is_none()
    );

    // A claim is atomic: competing accounts cannot both acquire the same code.
    let concurrent_secret = format!("am_{}", Uuid::new_v4().simple());
    let pending = pairing::start(&concurrent_secret, "lckfb-szpi-esp32s3".into())
        .await
        .unwrap();
    let code = pending["code"].as_str().unwrap().to_owned();
    let (a, b) = tokio::join!(
        pairing::claim(user, code.clone(), "Concurrent A".into()),
        pairing::claim(other, code, "Concurrent B".into())
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(u8::from(a.is_some()) + u8::from(b.is_some()), 1);
    let device = a.or(b).unwrap();
    assert!(store::revoke(device.user_id, device.id).await.unwrap());

    // Expired codes cannot bind. A fresh request renews the code automatically.
    let expired_secret = format!("am_{}", Uuid::new_v4().simple());
    let expired = pairing::start(&expired_secret, "lckfb-szpi-esp32s3".into())
        .await
        .unwrap();
    let hash = crate::auth::hash_code(&expired_secret);
    crate::db::with_conn!(move |conn| diesel::sql_query(
        "UPDATE amigo_pairings SET expires_at=NOW()-INTERVAL '1 second' WHERE token_hash=$1"
    )
    .bind::<diesel::sql_types::Text, _>(hash)
    .execute(conn)
    .await)
    .await
    .unwrap();
    assert_eq!(
        pairing::status(&expired_secret).await.unwrap()["state"],
        "expired"
    );
    assert!(
        pairing::claim(
            user,
            expired["code"].as_str().unwrap().into(),
            "Expired".into()
        )
        .await
        .unwrap()
        .is_none()
    );
    let fresh = pairing::start(&expired_secret, "lckfb-szpi-esp32s3".into())
        .await
        .unwrap();
    assert_eq!(fresh["state"], "waiting");

    // Leading zeroes survive both database storage and JSON transport.
    let zero_secret = format!("am_{}", Uuid::new_v4().simple());
    pairing::start(&zero_secret, "lckfb-szpi-esp32s3".into())
        .await
        .unwrap();
    let hash = crate::auth::hash_code(&zero_secret);
    crate::db::with_conn!(move |conn| diesel::sql_query(
        "UPDATE amigo_pairings SET code='000007' WHERE token_hash=$1"
    )
    .bind::<diesel::sql_types::Text, _>(hash)
    .execute(conn)
    .await)
    .await
    .unwrap();
    assert_eq!(
        pairing::start(&zero_secret, "lckfb-szpi-esp32s3".into())
            .await
            .unwrap()["code"],
        "000007"
    );
    let zero = pairing::claim(user, "000007".into(), "Leading zero".into())
        .await
        .unwrap()
        .unwrap();
    assert!(store::revoke(user, zero.id).await.unwrap());
    let hash = crate::auth::hash_code(&zero_secret);
    crate::db::with_conn!(move |conn| diesel::sql_query(
        "DELETE FROM amigo_pairings WHERE token_hash=$1"
    )
    .bind::<diesel::sql_types::Text, _>(hash)
    .execute(conn)
    .await)
    .await
    .unwrap();
    // Bound brute-force attempts; neither API response exposes permanent credentials.
    for _ in 0..3 {
        assert_eq!(
            http.post(format!("{api}/pairing/claim"))
                .bearer_auth(jwt)
                .json(&json!({"code":"bad","name":"test"}))
                .send()
                .await
                .unwrap()
                .status(),
            400
        );
    }
    assert_eq!(
        http.post(format!("{api}/pairing/claim"))
            .bearer_auth(jwt)
            .json(&json!({"code":"bad","name":"test"}))
            .send()
            .await
            .unwrap()
            .status(),
        429
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires an isolated AMIGO_TEST_DATABASE_URL; see docs/AMIGO_GATEWAY.md"]
async fn gateway_end_to_end() {
    let database_url =
        std::env::var("AMIGO_TEST_DATABASE_URL").expect("set isolated test database URL");
    let url = url::Url::parse(&database_url).unwrap();
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    assert!(
        url.path().starts_with("/amigo_test"),
        "test database name must start with amigo_test"
    );
    let db_config = crate::DbConfig {
        url: database_url,
        pool_size: 4,
        min_idle: 1,
        ..Default::default()
    };
    crate::config::APP_CONFIG
        .set(crate::AppConfig {
            bind_addr: "127.0.0.1:0".into(),
            jwt_secret: Uuid::new_v4().to_string(),
            jwt_ttl: Duration::from_secs(3600),
            database: db_config.clone(),
            space_path: concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../target/verification/amigo/test-space"
            )
            .into(),
        })
        .unwrap();
    crate::db::init(&db_config).await;
    let user = crate::db::with_conn!(move |conn| {
        diesel::insert_into(crate::db::schema::base_users::table)
            .values(
                crate::db::schema::base_users::name.eq(format!("amigo-test-{}", Uuid::new_v4())),
            )
            .returning(crate::db::schema::base_users::id)
            .get_result::<i64>(conn)
            .await
    })
    .await
    .unwrap();
    let other = crate::db::with_conn!(move |conn| {
        diesel::insert_into(crate::db::schema::base_users::table)
            .values(
                crate::db::schema::base_users::name.eq(format!("amigo-test-{}", Uuid::new_v4())),
            )
            .returning(crate::db::schema::base_users::id)
            .get_result::<i64>(conn)
            .await
    })
    .await
    .unwrap();
    // Registration triggers seed onboarding vocabulary; isolate this scenario
    // from those unrelated learning targets in the dedicated test database.
    crate::db::with_conn!(move |conn| {
        diesel::delete(
            crate::db::schema::learn_issue_words::table
                .filter(crate::db::schema::learn_issue_words::user_id.eq_any([user, other])),
        )
        .execute(conn)
        .await
    })
    .await
    .unwrap();
    let user_token =
        crate::auth::issue_access_token(user, 0, &crate::AppConfig::get().jwt_secret, 3600).unwrap();
    let other_token =
        crate::auth::issue_access_token(other, 0, &crate::AppConfig::get().jwt_secret, 3600).unwrap();
    let backend = Arc::new(TestBackend::default());
    let (base, server) = start_server(backend.clone()).await;
    let http = reqwest::Client::new();
    // Hardware follows the website access policy even when platform keys exist.
    assert!(
        super::tutor::ProductionBackend::for_user(user)
            .await
            .is_err()
    );
    let api = format!("{base}/api/amigo");
    verify_pairing(&http, &api, user, other, &user_token, &other_token).await;
    verify_voice_ownership(&http, &base, &user_token, &other_token).await;
    verify_control_without_ai(user).await;
    assert_eq!(
        http.get(format!("{api}/profile"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        http.get(format!("{api}/session"))
            .bearer_auth(&user_token)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let provision: serde_json::Value = http
        .post(&api)
        .bearer_auth(&user_token)
        .json(&serde_json::json!({"name":"test Korvo"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let token = provision["device_token"].as_str().unwrap().to_owned();
    let id: Uuid = provision["device"]["id"].as_str().unwrap().parse().unwrap();
    verify_reconnection_takeover(&base, &token, user, id).await;
    // Browser tickets are owner-scoped, short lived and atomically single use.
    assert!(store::ticket(other, id).await.unwrap().is_none());
    let ticket = store::ticket(user, id).await.unwrap().unwrap();
    let (first, second) = tokio::join!(
        store::consume_ticket(&ticket),
        store::consume_ticket(&ticket)
    );
    assert_eq!(
        u32::from(first.unwrap().is_some()) + u32::from(second.unwrap().is_some()),
        1
    );
    let expired = store::ticket(user, id).await.unwrap().unwrap();
    let hash = crate::auth::hash_code(&expired);
    crate::db::with_conn!(move |conn| diesel::sql_query(
        "UPDATE amigo_ws_tickets SET expires_at=NOW()-INTERVAL '1 second' WHERE token_hash=$1"
    )
    .bind::<diesel::sql_types::Text, _>(hash)
    .execute(conn)
    .await)
    .await
    .unwrap();
    assert!(store::consume_ticket(&expired).await.unwrap().is_none());
    assert_eq!(
        http.delete(format!("{api}/{id}"))
            .bearer_auth(&other_token)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let device_list: serde_json::Value = http
        .get(&api)
        .bearer_auth(&user_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(!device_list.to_string().contains(&token));
    let mut settings = Settings {
        camera_enabled: false,
        ambient_audio_enabled: false,
        proactive_enabled: false,
        quiet_start_minute: 0,
        quiet_end_minute: 0,
        idle_secs: 1,
        observation_interval_secs: 10,
        proactive_cooldown_secs: 10,
        ..Default::default()
    };
    assert!(
        http.put(format!("{api}/settings"))
            .bearer_auth(&user_token)
            .json(&settings)
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );

    // Device management is owner-scoped and configured profiles are independent.
    let (managed, managed_token) = store::provision(user, "managed Amigo".into())
        .await
        .unwrap();
    let (sibling, _) = store::provision(user, "sibling Amigo".into())
        .await
        .unwrap();
    let device_url = format!("{api}/devices/{}", managed.id);
    for method in [
        reqwest::Method::GET,
        reqwest::Method::PUT,
        reqwest::Method::DELETE,
    ] {
        assert_eq!(
            http.request(method, &device_url)
                .bearer_auth(&other_token)
                .json(&serde_json::json!({}))
                .send()
                .await
                .unwrap()
                .status(),
            404
        );
    }
    assert_eq!(http.get(&device_url).send().await.unwrap().status(), 401);
    assert_eq!(
        http.get(format!("{device_url}/history"))
            .bearer_auth(&other_token)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let mut local_settings = settings.clone();
    local_settings.english_level = "A2".into();
    let mut body = serde_json::json!({"name":"bedside Amigo","agent_id":"amigo","settings":local_settings,"configuration":{"volume":101}});
    assert_eq!(
        http.put(&device_url)
            .bearer_auth(&user_token)
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    body["configuration"]["volume"] = serde_json::json!(85);
    assert!(
        http.put(&device_url)
            .bearer_auth(&user_token)
            .json(&body)
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );
    assert_eq!(
        super::devices::load(managed.clone())
            .await
            .unwrap()
            .settings
            .english_level,
        "A2"
    );
    assert_eq!(
        super::devices::load(sibling.clone())
            .await
            .unwrap()
            .settings
            .english_level,
        "B1"
    );
    assert_eq!(
        store::load(user).await.unwrap().settings.english_level,
        "B1"
    );
    let mut managed_ws = connect_board(&base, &managed_token, false, LICHUANG_BOARD).await;
    let revision = match event(&mut managed_ws).await {
        ServerMessage::Configuration {
            revision,
            volume: 85,
            ..
        } => revision,
        other => panic!("expected device settings, got {other:?}"),
    };
    send(
        &mut managed_ws,
        ClientMessage::ConfigurationApplied {
            revision: revision - 1,
        },
    )
    .await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        super::devices::list(user)
            .await
            .unwrap()
            .iter()
            .find(|d| d.id == managed.id)
            .unwrap()
            .applied_revision
            .is_none()
    );
    send(
        &mut managed_ws,
        ClientMessage::ConfigurationApplied { revision },
    )
    .await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        super::devices::list(user)
            .await
            .unwrap()
            .iter()
            .find(|d| d.id == managed.id)
            .unwrap()
            .applied_revision,
        Some(revision)
    );
    body["configuration"]["volume"] = serde_json::json!(90);
    assert!(
        http.put(&device_url)
            .bearer_auth(&user_token)
            .json(&body)
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );
    assert!(matches!(
        event(&mut managed_ws).await,
        ServerMessage::Configuration { volume: 90, .. }
    ));
    let listed = http
        .get(format!("{api}/devices"))
        .bearer_auth(&user_token)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!listed.contains(&managed_token));
    managed_ws.close(None).await.unwrap();
    assert!(
        http.delete(&device_url)
            .bearer_auth(&user_token)
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );
    assert_eq!(
        http.get(&device_url)
            .bearer_auth(&user_token)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    store::revoke(user, sibling.id).await.unwrap();

    // Exercise the real-board handshake through the same complete media,
    // cancellation and observation flow as the existing simulator.
    let mut ws = connect_board(&base, &token, false, LICHUANG_BOARD).await;
    let duplicate_ticket = store::ticket(user, id).await.unwrap().unwrap();
    let mut duplicate = format!("{}/api/amigo/session", base.replacen("http://", "ws://", 1))
        .into_client_request()
        .unwrap();
    duplicate.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        format!("amigo.v1, {duplicate_ticket}").parse().unwrap(),
    );
    assert!(tokio_tungstenite::connect_async(duplicate).await.is_err());
    send(
        &mut ws,
        ClientMessage::Wake {
            turn_id: 1,
            trigger: Trigger::Button,
        },
    )
    .await;
    assert!(matches!(event(&mut ws).await, ServerMessage::Ack { .. }));
    for sequence in 0..5 {
        ws.send(Message::Binary(
            MediaPacket {
                kind: MediaKind::AudioIn,
                turn_id: 1,
                sequence,
                payload: &[1; 640],
            }
            .encode()
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
    }
    send(&mut ws, ClientMessage::EndTurn { turn_id: 1 }).await;
    assert!(!reply(&mut ws, Some(1)).await.1);
    let profile = store::load(user).await.unwrap();
    assert_eq!(profile.weaknesses[0].key, "past tense");
    assert_eq!(profile.weaknesses[0].failures, 1);
    let calls_url = format!("{api}/devices/{id}/calls");
    assert_eq!(http.get(&calls_url).send().await.unwrap().status(), 401);
    assert_eq!(
        http.get(&calls_url)
            .bearer_auth(&other_token)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let diagnostic = http
        .get(&calls_url)
        .bearer_auth(&user_token)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    for stage in ["asr", "chat", "tts"] {
        assert!(
            diagnostic
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["stage"] == stage && v["state"] == "success")
        );
    }
    let serialized = diagnostic.to_string();
    assert!(
        !serialized.contains(&token)
            && !serialized.contains("api_key")
            && !serialized.contains("I go yesterday")
    );

    // Real periodic camera command, binary JPEG upload and persistent context.
    settings.camera_enabled = true;
    http.put(format!("{api}/settings"))
        .bearer_auth(&user_token)
        .json(&settings)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let (request, source) = loop {
        if let ServerMessage::Capture {
            request_id, source, ..
        } = event(&mut ws).await
        {
            break (request_id, source);
        }
    };
    assert_eq!(source, Source::Book);
    let mut jpeg = vec![];
    image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
        .encode(&[255; 12], 2, 2, image::ExtendedColorType::Rgb8)
        .unwrap();
    send(
        &mut ws,
        ClientMessage::ObservationStart {
            request_id: request,
            source,
            bytes: jpeg.len(),
        },
    )
    .await;
    ws.send(Message::Binary(
        MediaPacket {
            kind: MediaKind::Observation,
            turn_id: request,
            sequence: 0,
            payload: &jpeg,
        }
        .encode()
        .unwrap()
        .into(),
    ))
    .await
    .unwrap();
    loop {
        if matches!(
            event(&mut ws).await,
            ServerMessage::ObservationStored { .. }
        ) {
            break;
        }
    }
    assert_eq!(
        store::load(user).await.unwrap().memories[0].subject,
        "plants"
    );

    // The server, not the client, chooses the moment and lesson targets.
    settings.camera_enabled = false;
    settings.proactive_enabled = true;
    http.put(format!("{api}/settings"))
        .bearer_auth(&user_token)
        .json(&settings)
        .send()
        .await
        .unwrap();
    let (outreach, proactive, targets) = reply(&mut ws, None).await;
    assert!(proactive && outreach >= SERVER_TURN_START);
    assert_eq!(targets, ["past tense"]);
    send(
        &mut ws,
        ClientMessage::Feedback {
            turn_id: outreach,
            accepted: true,
        },
    )
    .await;
    send(
        &mut ws,
        ClientMessage::Text {
            turn_id: 2,
            text: "I went yesterday".into(),
        },
    )
    .await;
    reply(&mut ws, Some(2)).await;
    let profile = store::load(user).await.unwrap();
    assert_eq!(profile.weaknesses[0].successful_recalls, 1);
    assert_eq!(profile.weaknesses[0].interval_days, 1);
    assert!(
        backend
            .plans
            .lock()
            .unwrap()
            .iter()
            .any(|p| p.proactive && p.situation.contains("photosynthesis"))
    );

    // A cancelled in-flight response cannot update the profile or speak later.
    send(
        &mut ws,
        ClientMessage::Text {
            turn_id: 3,
            text: "slow".into(),
        },
    )
    .await;
    loop {
        if matches!(event(&mut ws).await, ServerMessage::Ack { turn_id: 3, .. }) {
            break;
        }
    }
    send(&mut ws, ClientMessage::Cancel { turn_id: 3 }).await;
    loop {
        if matches!(
            event(&mut ws).await,
            ServerMessage::Cancelled { turn_id: 3 }
        ) {
            break;
        }
    }
    tokio::time::sleep(Duration::from_millis(650)).await;
    assert!(
        !store::load(user)
            .await
            .unwrap()
            .history
            .iter()
            .any(|h| h.text == "slow")
    );
    send(
        &mut ws,
        ClientMessage::Text {
            turn_id: 4,
            text: "fail".into(),
        },
    )
    .await;
    loop {
        if let ServerMessage::Error { code, .. } = event(&mut ws).await {
            assert_eq!(code, "pipeline_failed");
            break;
        }
    }
    send(
        &mut ws,
        ClientMessage::Text {
            turn_id: 5,
            text: "Hello".into(),
        },
    )
    .await;
    reply(&mut ws, Some(5)).await;

    let created: serde_json::Value = http
        .post(format!("{api}/agents"))
        .bearer_auth(&user_token)
        .json(&serde_json::json!({
            "id":"coach","name":"Coach","role_prompt":"A direct but kind speaking coach"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(created["active_agent_id"], "coach");
    assert_eq!(
        http.put(format!("{api}/agents/coach"))
            .bearer_auth(&other_token)
            .json(&created["agents"][1])
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    send(
        &mut ws,
        ClientMessage::Text {
            turn_id: 6,
            text: "@amigo 请过来".into(),
        },
    )
    .await;
    reply(&mut ws, Some(6)).await;
    send(
        &mut ws,
        ClientMessage::Text {
            turn_id: 7,
            text: "@coach 请过来".into(),
        },
    )
    .await;
    reply(&mut ws, Some(7)).await;
    assert!(backend.plans.lock().unwrap().last().unwrap().agent.id == "coach");
    send(
        &mut ws,
        ClientMessage::Text {
            turn_id: 8,
            text: "创建一个名叫 Joey 的角色，模仿《老友记》中 Joey 的风格".into(),
        },
    )
    .await;
    reply(&mut ws, Some(8)).await;
    let joey_id = {
        let plans = backend.plans.lock().unwrap();
        let introduction = plans.last().unwrap();
        assert_eq!(introduction.agent.name, "Joey");
        assert_eq!(introduction.response_act.as_str(), "introduce");
        introduction.agent.id.clone()
    };
    assert_eq!(store::load(user).await.unwrap().active_agent_id, joey_id);
    send(
        &mut ws,
        ClientMessage::Text {
            turn_id: 9,
            text: "@amigo 请过来".into(),
        },
    )
    .await;
    reply(&mut ws, Some(9)).await;

    // Periodic short audio observations use their own request/size boundaries.
    settings.proactive_enabled = false;
    settings.ambient_audio_enabled = true;
    http.put(format!("{api}/settings"))
        .bearer_auth(&user_token)
        .json(&settings)
        .send()
        .await
        .unwrap();
    store::update(user, |p| p.last_observation_at = None)
        .await
        .unwrap();
    let request = loop {
        if let ServerMessage::Capture {
            request_id,
            source: Source::Speech,
            ..
        } = event(&mut ws).await
        {
            break request_id;
        }
    };
    send(
        &mut ws,
        ClientMessage::ObservationStart {
            request_id: request,
            source: Source::Speech,
            bytes: 3200,
        },
    )
    .await;
    ws.send(Message::Binary(
        MediaPacket {
            kind: MediaKind::Observation,
            turn_id: request,
            sequence: 0,
            payload: &[1; 3200],
        }
        .encode()
        .unwrap()
        .into(),
    ))
    .await
    .unwrap();
    loop {
        if matches!(
            event(&mut ws).await,
            ServerMessage::ObservationStored { .. }
        ) {
            break;
        }
    }
    settings.ambient_audio_enabled = false;
    http.put(format!("{api}/settings"))
        .bearer_auth(&user_token)
        .json(&settings)
        .send()
        .await
        .unwrap();
    ws.close(None).await.unwrap();
    server.abort();

    // Rebuild the server/connection from PostgreSQL; there is no profile cache.
    let (base, restarted) = start_server(backend.clone()).await;
    let mut ws = connect(&base, &token).await;
    let profile = store::load(user).await.unwrap();
    assert_eq!(profile.weaknesses[0].successful_recalls, 1);
    assert_eq!(profile.observations.len(), 2);
    assert_eq!(profile.agents.len(), 3);
    assert_eq!(profile.active_agent_id, "amigo");
    let other_profile: Profile = http
        .get(format!("{base}/api/amigo/profile"))
        .bearer_auth(&other_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(other_profile.memories.is_empty() && other_profile.weaknesses.is_empty());

    if let Ok(executable) = std::env::var("AMIGO_SIMULATOR_EXE") {
        let (sim_device, sim_token) = store::provision(user, "CLI simulator".into())
            .await
            .unwrap();
        let mut command = std::process::Command::new(executable);
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/verification/amigo/simulator-test-input.wav"
        );
        tokio::fs::write(fixture, audio::wav(&[1; 3200]).unwrap())
            .await
            .unwrap();
        command.env("AMIGO_DEVICE_TOKEN", sim_token).args([
            "--server",
            &base,
            "--duration-secs",
            "3",
            "--wav",
            fixture,
            "--trigger",
            "wake_word",
        ]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let status = tokio::task::spawn_blocking(move || command.status().unwrap())
            .await
            .unwrap();
        assert!(status.success());
        assert!(store::revoke(user, sim_device.id).await.unwrap());
    }

    let (bad_device, bad_token) = store::provision(user, "bad sequence fixture".into())
        .await
        .unwrap();
    let mut bad_ws = connect(&base, &bad_token).await;
    send(
        &mut bad_ws,
        ClientMessage::Wake {
            turn_id: 1,
            trigger: Trigger::Button,
        },
    )
    .await;
    assert!(matches!(
        event(&mut bad_ws).await,
        ServerMessage::Ack { .. }
    ));
    bad_ws
        .send(Message::Binary(
            MediaPacket {
                kind: MediaKind::AudioIn,
                turn_id: 1,
                sequence: 1,
                payload: &[1; 640],
            }
            .encode()
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
    assert!(
        matches!(event(&mut bad_ws).await,ServerMessage::Error {ref code,..} if code=="invalid_media")
    );
    let _ = bad_ws.close(None).await;
    store::revoke(user, bad_device.id).await.unwrap();

    // Forgetting is persistent and cannot cross owner boundaries.
    let response = http
        .post(format!("{base}/api/amigo/forget"))
        .bearer_auth(&user_token)
        .json(&serde_json::json!({"kind":"weakness","key":"past tense"}))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    assert!(store::load(user).await.unwrap().weaknesses.is_empty());
    assert!(store::revoke(user, id).await.unwrap());
    assert!(store::ticket(user, id).await.unwrap().is_none());
    // Revocation closes the live connection at the next lease refresh.
    let closed = tokio::time::timeout(Duration::from_secs(8), async {
        while let Some(message) = ws.next().await {
            if message.is_err() || message.unwrap().is_close() {
                break;
            }
        }
    })
    .await;
    assert!(closed.is_ok());
    assert!(store::authenticate(&token).await.unwrap().is_none());
    // Streaming audio is audible before delayed generation completes, and an
    // empty audio queue between sentences must not prematurely end the turn.
    store::update(user, |p| {
        p.settings.proactive_enabled = false;
        p.settings.camera_enabled = false;
        p.settings.ambient_audio_enabled = false;
    })
    .await
    .unwrap();
    let (stream_device, stream_token) = store::provision(user, "streaming fixture".into())
        .await
        .unwrap();
    let mut streaming = connect_mode(&base, &stream_token, true).await;
    let started = std::time::Instant::now();
    send(
        &mut streaming,
        ClientMessage::Text {
            turn_id: 1,
            text: "stream slow".into(),
        },
    )
    .await;
    let mut first_audio = false;
    let mut bytes = 0;
    let mut updated = false;
    loop {
        let message = tokio::time::timeout(Duration::from_secs(5), streaming.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if message.is_binary() {
            if !first_audio {
                assert!(started.elapsed() < Duration::from_millis(700));
                first_audio = true;
            }
            bytes += MediaPacket::decode(&message.into_data())
                .unwrap()
                .payload
                .len();
        } else if message.is_text() {
            match serde_json::from_str::<ServerMessage>(message.to_text().unwrap()).unwrap() {
                ServerMessage::ReplyUpdate { text, .. } => {
                    assert_eq!(text, "First sentence. Last sentence.");
                    updated = true;
                }
                ServerMessage::AudioEnd {
                    turn_id,
                    bytes: total,
                } => {
                    assert!(updated && first_audio);
                    assert!(started.elapsed() >= Duration::from_millis(800));
                    assert_eq!(bytes, total);
                    assert_eq!(bytes, 6400);
                    send(&mut streaming, ClientMessage::PlaybackDone { turn_id }).await;
                    break;
                }
                ServerMessage::Error { message, .. } => panic!("stream failed {message}"),
                _ => {}
            }
        }
    }
    assert_eq!(
        store::load(user)
            .await
            .unwrap()
            .history
            .last()
            .unwrap()
            .text,
        "First sentence. Last sentence.",
        "persist the complete reply, not the first spoken segment"
    );
    // Interrupt while generation is in the gap after its first sentence.
    send(
        &mut streaming,
        ClientMessage::Text {
            turn_id: 2,
            text: "stream slow".into(),
        },
    )
    .await;
    loop {
        let message = streaming.next().await.unwrap().unwrap();
        if message.is_binary() {
            break;
        }
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    send(&mut streaming, ClientMessage::Cancel { turn_id: 2 }).await;
    assert!(matches!(
        event(&mut streaming).await,
        ServerMessage::Cancelled { turn_id: 2 }
    ));
    send(
        &mut streaming,
        ClientMessage::Text {
            turn_id: 3,
            text: "next turn".into(),
        },
    )
    .await;
    reply(&mut streaming, Some(3)).await;
    let before = store::load(user).await.unwrap().history.len();
    send(
        &mut streaming,
        ClientMessage::Text {
            turn_id: 4,
            text: "stream fail".into(),
        },
    )
    .await;
    loop {
        match event(&mut streaming).await {
            ServerMessage::Error {
                turn_id: Some(4), ..
            } => break,
            ServerMessage::AudioEnd { .. } => panic!("failed generation cannot end normally"),
            _ => {}
        }
    }
    assert_eq!(
        store::load(user).await.unwrap().history.len(),
        before,
        "invalid partial generation must not update the learner profile"
    );
    let diagnostic = super::calls::list(stream_device.clone()).await.unwrap();
    assert!(diagnostic.iter().any(|v| {
        v.turn_id == "4"
            && v.stage == "chat"
            && v.state == "error"
            && v.error
                .as_ref()
                .is_some_and(|e| e.contains("injected stream failure"))
    }));

    send(
        &mut streaming,
        ClientMessage::Text {
            turn_id: 5,
            text: "recover after failure".into(),
        },
    )
    .await;
    reply(&mut streaming, Some(5)).await;
    // An empty ASR result is recoverable on the same connection and must not
    // fabricate a conversation or modify the learner profile.
    let before = serde_json::to_value(store::load(user).await.unwrap()).unwrap();
    send(
        &mut streaming,
        ClientMessage::Wake {
            turn_id: 6,
            trigger: Trigger::Button,
        },
    )
    .await;
    assert!(matches!(
        event(&mut streaming).await,
        ServerMessage::Ack { .. }
    ));
    for sequence in 0..5 {
        streaming
            .send(Message::Binary(
                MediaPacket {
                    kind: MediaKind::AudioIn,
                    turn_id: 6,
                    sequence,
                    payload: &[2; 640],
                }
                .encode()
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();
    }
    send(&mut streaming, ClientMessage::EndTurn { turn_id: 6 }).await;
    let empty_result = loop {
        let message = event(&mut streaming).await;
        if !matches!(message, ServerMessage::Ack { .. }) {
            break message;
        }
    };
    assert!(
        matches!(&empty_result,
        ServerMessage::Error { turn_id: Some(6), code, .. } if code == "no_speech"),
        "unexpected empty recognition result: {empty_result:?}"
    );
    assert_eq!(
        serde_json::to_value(store::load(user).await.unwrap()).unwrap(),
        before
    );
    send(
        &mut streaming,
        ClientMessage::Wake {
            turn_id: 7,
            trigger: Trigger::Button,
        },
    )
    .await;
    assert!(matches!(
        event(&mut streaming).await,
        ServerMessage::Ack { .. }
    ));
    for sequence in 0..5 {
        streaming
            .send(Message::Binary(
                MediaPacket {
                    kind: MediaKind::AudioIn,
                    turn_id: 7,
                    sequence,
                    payload: &[1; 640],
                }
                .encode()
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();
    }
    send(&mut streaming, ClientMessage::EndTurn { turn_id: 7 }).await;
    reply(&mut streaming, Some(7)).await;
    use super::workflow::{self, ContextRequest, Decision, TaskPolicy};
    let scripted =
        |mode: &str, stage: &str, switch_evidence: &str, context: ContextRequest| Decision {
            resume_practice: false,
            practice_attempt: false,
            check_in: false,
            pronunciation: None,
            support_provided: super::workflow::Support::None,
            task_spec: None,
            mode: mode.into(),
            stage: stage.into(),
            confidence: 0.95,
            switch_evidence: switch_evidence.into(),
            goal: "Understand the current material".into(),
            strategy: "Respond to the actual task".into(),
            scene: String::new(),
            action: super::agent::ResponseAct::Respond,
            context,
            task_policy: TaskPolicy::Keep,
            assess_user: false,
            targets: vec![],
            explanation: "Explicit test-mode planning fixture".into(),
        };
    store::update(user, |p| {
        p.settings.camera_enabled = true;
        p.last_observation_at = Some(Utc::now());
    })
    .await
    .unwrap();
    let before = store::load(user).await.unwrap().observations.len();
    send(&mut streaming, ClientMessage::CameraState { enabled: true }).await;
    backend.workflow_plans.lock().unwrap().push_back(scripted(
        "reading",
        "ground",
        "摄像头打开了，你现在给我读。",
        ContextRequest::FreshCamera,
    ));
    send(
        &mut streaming,
        ClientMessage::Text {
            turn_id: 8,
            text: "摄像头打开了，你现在给我读。".into(),
        },
    )
    .await;
    loop {
        match event(&mut streaming).await {
            ServerMessage::Capture { request_id: 8, .. } => break,
            ServerMessage::Ack { .. }
            | ServerMessage::Transcript { .. }
            | ServerMessage::LearningState { .. } => {}
            message => panic!("expected workflow camera request {message:?}"),
        }
    }
    send(
        &mut streaming,
        ClientMessage::ObservationStart {
            request_id: 8,
            source: Source::Book,
            bytes: jpeg.len(),
        },
    )
    .await;
    streaming
        .send(Message::Binary(
            MediaPacket {
                kind: MediaKind::Observation,
                turn_id: 8,
                sequence: 0,
                payload: &jpeg,
            }
            .encode()
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
    receive_reply(&mut streaming, Some(8)).await;
    let before_drain = store::load(user).await.unwrap();
    assert_eq!(
        workflow::session(&before_drain, &before_drain.active_agent_id).mode,
        "conversation"
    );
    assert_eq!(before_drain.observations.len(), before);
    finish_reply(&mut streaming, 8).await;
    let observed_profile = store::load(user).await.unwrap();
    let learning_agent = observed_profile.active_agent_id.clone();
    assert_eq!(observed_profile.observations.len(), before + 1);
    assert_eq!(
        workflow::session(&observed_profile, &learning_agent).mode,
        "reading"
    );
    assert_eq!(
        backend
            .plans
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .visual
            .as_ref()
            .unwrap()
            .status,
        "captured"
    );
    // A contextual continuation reuses the selected material, rather than taking
    // another photo merely because the camera is on.
    backend.workflow_plans.lock().unwrap().push_back(scripted(
        "reading",
        "read",
        "",
        ContextRequest::CurrentMaterial,
    ));
    send(
        &mut streaming,
        ClientMessage::Text {
            turn_id: 9,
            text: "继续。".into(),
        },
    )
    .await;
    reply(&mut streaming, Some(9)).await;
    assert_eq!(
        backend
            .plans
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .visual
            .as_ref()
            .unwrap()
            .status,
        "material"
    );
    assert_eq!(
        store::load(user).await.unwrap().observations.len(),
        before + 1
    );
    backend.workflow_plans.lock().unwrap().push_back(scripted(
        "conversation",
        "converse",
        "Let's chat about my day.",
        ContextRequest::None,
    ));
    send(
        &mut streaming,
        ClientMessage::Text {
            turn_id: 10,
            text: "Let's chat about my day.".into(),
        },
    )
    .await;
    reply(&mut streaming, Some(10)).await;
    assert!(
        backend
            .plans
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .visual
            .is_none()
    );
    // Cancel while the workflow is waiting for its photo. Stale bytes cannot
    // become learning evidence or close the next conversation.
    backend.workflow_plans.lock().unwrap().push_back(scripted(
        "reading",
        "ground",
        "Read this page",
        ContextRequest::FreshCamera,
    ));
    send(
        &mut streaming,
        ClientMessage::Text {
            turn_id: 11,
            text: "Read this page".into(),
        },
    )
    .await;
    loop {
        if matches!(
            event(&mut streaming).await,
            ServerMessage::Capture { request_id: 11, .. }
        ) {
            break;
        }
    }
    send(&mut streaming, ClientMessage::Cancel { turn_id: 11 }).await;
    loop {
        if matches!(
            event(&mut streaming).await,
            ServerMessage::Cancelled { turn_id: 11 }
        ) {
            break;
        }
    }
    send(
        &mut streaming,
        ClientMessage::ObservationStart {
            request_id: 11,
            source: Source::Book,
            bytes: jpeg.len(),
        },
    )
    .await;
    streaming
        .send(Message::Binary(
            MediaPacket {
                kind: MediaKind::Observation,
                turn_id: 11,
                sequence: 0,
                payload: &jpeg,
            }
            .encode()
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
    backend.workflow_plans.lock().unwrap().push_back(scripted(
        "conversation",
        "converse",
        "",
        ContextRequest::None,
    ));
    send(
        &mut streaming,
        ClientMessage::Text {
            turn_id: 12,
            text: "Let's keep chatting.".into(),
        },
    )
    .await;
    reply(&mut streaming, Some(12)).await;
    assert_eq!(
        store::load(user).await.unwrap().observations.len(),
        before + 1
    );
    let state = workflow::session(&store::load(user).await.unwrap(), &learning_agent);
    assert_eq!(state.revision, 4);
    let learning_url = format!("{base}/api/amigo/learning");
    let selected=http.put(&learning_url).bearer_auth(&user_token).json(&serde_json::json!({"agent_id":learning_agent,"revision":state.revision,"mode":"vocabulary","goal":"Learn resilient","scene":""})).send().await.unwrap();
    assert!(selected.status().is_success());
    assert_eq!(http.put(&learning_url).bearer_auth(&user_token).json(&serde_json::json!({"agent_id":learning_agent,"revision":state.revision,"mode":"roleplay"})).send().await.unwrap().status(),409);
    assert_eq!(
        workflow::session(&store::load(user).await.unwrap(), &learning_agent).mode,
        "vocabulary"
    );
    let other_state: serde_json::Value = http
        .get(&learning_url)
        .bearer_auth(&other_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(other_state["session"]["revision"], 0);
    // Malformed/uncertain planning is an operational fallback, never a keyword
    // router or a teaching update.
    let mut invalid = scripted("reading", "ground", "Read it", ContextRequest::FreshCamera);
    invalid.confidence = 0.1;
    backend.workflow_plans.lock().unwrap().push_back(invalid);
    send(
        &mut streaming,
        ClientMessage::Text {
            turn_id: 13,
            text: "Read it".into(),
        },
    )
    .await;
    reply(&mut streaming, Some(13)).await;
    assert_eq!(
        workflow::session(&store::load(user).await.unwrap(), &learning_agent).revision,
        5
    );
    assert!(
        backend
            .plans
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .workflow
            .as_ref()
            .unwrap()
            .decision
            .is_none()
    );
    backend.workflow_plans.lock().unwrap().push_back(scripted(
        "vocabulary",
        "explain",
        "",
        ContextRequest::None,
    ));
    send(
        &mut streaming,
        ClientMessage::Text {
            turn_id: 14,
            text: "continue the vocabulary activity".into(),
        },
    )
    .await;
    loop {
        if matches!(
            event(&mut streaming).await,
            ServerMessage::Reply { turn_id: 14, .. }
        ) {
            break;
        }
    }
    let changed=http.put(&learning_url).bearer_auth(&user_token).json(&serde_json::json!({"agent_id":learning_agent,"revision":5,"mode":"conversation","goal":"Keep chatting"})).send().await.unwrap();
    assert!(changed.status().is_success());
    let mut conflict_seen = false;
    loop {
        match event(&mut streaming).await {
            ServerMessage::LearningState { status, mode, .. } if status == "state_changed" => {
                assert_eq!(mode, "conversation");
                conflict_seen = true;
            }
            ServerMessage::AudioEnd { turn_id: 14, .. } => {
                send(&mut streaming, ClientMessage::PlaybackDone { turn_id: 14 }).await;
                break;
            }
            ServerMessage::Error { message, .. } => {
                panic!("workflow conflict disrupted media: {message}")
            }
            _ => {}
        }
    }
    assert!(conflict_seen);
    let state = workflow::session(&store::load(user).await.unwrap(), &learning_agent);
    assert_eq!(state.revision, 6);
    assert_eq!(state.mode, "conversation");
    streaming.close(None).await.unwrap();
    store::revoke(user, stream_device.id).await.unwrap();
    // Explicit learner corrections update both the legacy weakness and the new
    // workflow schedule, without fabricating successful learning evidence.
    let correction_key = "conditional practice".to_owned();
    let tracked_key = correction_key.clone();
    let tracked_agent = learning_agent.clone();
    store::update(user, move |profile| {
        profile.weaknesses.push(Weakness {
            user_corrected: false,
            key: tracked_key.clone(),
            kind: "grammar".into(),
            original: "If I will study".into(),
            corrected: "If I study".into(),
            failures: 1,
            successful_recalls: 0,
            interval_days: 0,
            next_review_at: Utc::now(),
            updated_at: Utc::now(),
        });
        let learning = &mut profile
            .agent_states
            .get_mut(&tracked_agent)
            .unwrap()
            .learning;
        learning.items.push(workflow::Item {
            key: tracked_key.clone(),
            independent: 2,
            next_review_at: Some(Utc::now()),
            ..Default::default()
        });
        learning.task = Some(workflow::Task {
            id: "before-user-correction".into(),
            prompt: "Old exercise".into(),
            target_keys: vec![tracked_key],
            kind: "sentence_use".into(),
            support: workflow::Support::None,
        });
    })
    .await
    .unwrap();
    let correction_url = format!("{base}/api/amigo/profile/item");
    for (mastered, expected_revision) in [(true, 7), (false, 8)] {
        assert!(
            http.put(&correction_url)
                .bearer_auth(&user_token)
                .json(
                    &serde_json::json!({"kind":"weakness","key":correction_key,"mastered":mastered})
                )
                .send()
                .await
                .unwrap()
                .status()
                .is_success()
        );
        let corrected = store::load(user).await.unwrap();
        let learning = workflow::session(&corrected, &learning_agent);
        let target = learning
            .items
            .iter()
            .find(|target| target.key == correction_key)
            .unwrap();
        let weakness = corrected
            .weaknesses
            .iter()
            .find(|target| target.key == correction_key)
            .unwrap();
        assert_eq!(target.next_review_at, Some(weakness.next_review_at));
        assert_eq!(target.independent, 2);
        assert_eq!(learning.revision, expected_revision);
        assert!(learning.task.is_none());
        assert_eq!(weakness.interval_days, if mastered { 30 } else { 0 });
    }
    // A scheduled acoustic focus uses the same speaker path as an explicit
    // request. Even AudioEnd leaves the old scene active until the device drains.
    store::update(user, |profile| {
        profile.settings.proactive_enabled = true;
        profile.settings.idle_secs = 1;
        profile.settings.quiet_start_minute = 0;
        profile.settings.quiet_end_minute = 0;
        profile.settings.camera_enabled = false;
        profile.settings.ambient_audio_enabled = false;
        profile.last_outreach_at = None;
        profile.outreaches_today = 0;
        let state = &mut profile
            .agent_states
            .get_mut(&profile.active_agent_id)
            .unwrap()
            .learning;
        state.pronunciation_focus = Some("three".into());
        state.practice_ended_at = None;
    })
    .await
    .unwrap();
    let mut drill = scripted("pronunciation", "model", "", ContextRequest::None);
    drill.targets = vec![workflow::Target {
        key: "three".into(),
        kind: "pronunciation".into(),
        meaning: "Practice three".into(),
    }];
    drill.task_policy = TaskPolicy::Replace;
    drill.task_spec = Some(workflow::TaskSpec {
        target_keys: vec!["three".into()],
        kind: "pronunciation_repeat".into(),
        support: workflow::Support::Answer,
        objective: "Model three and invite repetition".into(),
    });
    backend
        .workflow_plans
        .lock()
        .unwrap()
        .push_back(drill.clone());
    let (guided_device, guided_token) = store::provision(user, "guided practice fixture".into())
        .await
        .unwrap();
    let mut guided = connect_mode(&base, &guided_token, true).await;
    let (turn, proactive, _) = receive_reply(&mut guided, None).await;
    assert!(proactive && turn >= SERVER_TURN_START);
    assert_eq!(
        workflow::session(&store::load(user).await.unwrap(), &learning_agent).mode,
        "conversation"
    );
    finish_reply(&mut guided, turn).await;
    let active = workflow::session(&store::load(user).await.unwrap(), &learning_agent);
    assert_eq!(active.mode, "pronunciation");
    assert_eq!(active.task.unwrap().kind, "pronunciation_repeat");

    let mut exit = scripted(
        "conversation",
        "converse",
        "Let's stop the drill",
        ContextRequest::None,
    );
    exit.task_policy = TaskPolicy::Clear;
    backend.workflow_plans.lock().unwrap().push_back(exit);
    send(
        &mut guided,
        ClientMessage::Text {
            turn_id: 1,
            text: "Let's stop the drill".into(),
        },
    )
    .await;
    receive_reply(&mut guided, Some(1)).await;
    assert_eq!(
        workflow::session(&store::load(user).await.unwrap(), &learning_agent).mode,
        "pronunciation"
    );
    finish_reply(&mut guided, 1).await;
    let exited = workflow::session(&store::load(user).await.unwrap(), &learning_agent);
    assert_eq!(exited.mode, "conversation");
    assert!(exited.task.is_none());

    drill.switch_evidence = "Please practice pronunciation with me".into();
    backend.workflow_plans.lock().unwrap().push_back(drill);
    send(
        &mut guided,
        ClientMessage::Text {
            turn_id: 2,
            text: "Please practice pronunciation with me".into(),
        },
    )
    .await;
    receive_reply(&mut guided, Some(2)).await;
    send(&mut guided, ClientMessage::Cancel { turn_id: 2 }).await;
    assert!(matches!(
        event(&mut guided).await,
        ServerMessage::Cancelled { turn_id: 2 }
    ));
    send(&mut guided, ClientMessage::PlaybackDone { turn_id: 2 }).await;
    send(&mut guided, ClientMessage::Ping).await;
    assert!(matches!(event(&mut guided).await, ServerMessage::Pong));
    let interrupted = workflow::session(&store::load(user).await.unwrap(), &learning_agent);
    assert_eq!(interrupted.revision, exited.revision);
    assert_eq!(interrupted.mode, "conversation");
    guided.close(None).await.unwrap();
    store::revoke(user, guided_device.id).await.unwrap();
    restarted.abort();

    // Shared proactive budget is atomic across devices/server processes.
    store::update(user, |p| {
        p.settings.proactive_enabled = true;
        p.last_outreach_at = None;
        p.outreaches_today = 0;
    })
    .await
    .unwrap();
    let (a, b) = tokio::join!(
        store::reserve_outreach(user, Utc::now()),
        store::reserve_outreach(user, Utc::now())
    );
    assert_eq!(u32::from(a.unwrap()) + u32::from(b.unwrap()), 1);
    store::update(user, |p| {
        p.settings.camera_enabled = true;
        p.last_observation_at = None;
    })
    .await
    .unwrap();
    let now = Utc::now();
    let (a, b) = tokio::join!(
        store::reserve_observation(user, now, Source::Speech),
        store::reserve_observation(user, now, Source::Speech)
    );
    assert_eq!(
        u32::from(a.unwrap().is_some()) + u32::from(b.unwrap().is_some()),
        1
    );
    // Replaying the same server event cannot increment evidence twice.
    let (device, _) = store::provision(user, "dedup fixture".into())
        .await
        .unwrap();
    let lease = Uuid::new_v4();
    assert!(store::claim(device.id, lease).await.unwrap());
    let event_id = Uuid::new_v4();
    let before = store::load(user).await.unwrap().dismissals;
    for _ in 0..2 {
        store::record(
            device.clone(),
            lease,
            event_id,
            "feedback",
            serde_json::json!({}),
            |p| p.dismissals += 1,
        )
        .await
        .unwrap();
    }
    assert_eq!(store::load(user).await.unwrap().dismissals, before + 1);
    store::release(device.id, lease).await.unwrap();

    let mut queries = Vec::new();
    for _ in 0..48 {
        queries.push(tokio::spawn(async {
            crate::db::with_conn!(move |conn| {
                diesel::select(1_i32.into_sql::<diesel::sql_types::Integer>())
                    .get_result::<i32>(conn)
                    .await
            })
            .await
        }));
    }
    for query in futures_util::future::join_all(queries).await {
        assert_eq!(query.unwrap().unwrap(), 1);
    }
    let status = crate::db::state();
    assert!(status.size <= status.max_size);
    assert_eq!(status.available, status.max_size);

    crate::db::with_conn!(move |conn| {
        diesel::delete(
            crate::db::schema::base_users::table
                .filter(crate::db::schema::base_users::id.eq_any([user, other])),
        )
        .execute(conn)
        .await
    })
    .await
    .unwrap();
    assert!(Utc::now() > profile.weaknesses[0].updated_at);
}
