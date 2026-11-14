use std::result::Result;
use super::*;
use crate::services::{AiProvider, chat_stream, get_user_provider};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::mpsc;

pub(super) enum Input {
    Text(String),
    Audio(Vec<u8>),
}

pub(super) async fn prepare(
    req: &mut Request,
    user: i64,
    chat: i64,
) -> Result<(Input, Arc<dyn AiProvider>), StatusError> {
    check_owner(user, chat).await?;
    let bytes = req
        .payload_with_max_size(12 * 1024 * 1024)
        .await
        .map_err(|_| StatusError::bad_request().brief("消息或音频过大"))?;
    let input: ChatSendRequest = serde_json::from_slice(&bytes)
        .map_err(|_| StatusError::bad_request().brief("消息格式无效"))?;
    let input = match input {
        ChatSendRequest::Text { message } => {
            let message = message.trim().to_string();
            if message.is_empty() || message.chars().count() > 4000 {
                return Err(StatusError::bad_request().brief("请输入 1 至 4000 个字符"));
            }
            Input::Text(message)
        }
        ChatSendRequest::Audio { audio_base64 } => {
            let bytes = BASE64
                .decode(audio_base64)
                .map_err(|_| StatusError::bad_request().brief("音频格式无效"))?;
            if bytes.is_empty() || bytes.len() > 10 * 1024 * 1024 {
                return Err(StatusError::bad_request().brief("音频为空或超过 10 MB"));
            }
            Input::Audio(bytes)
        }
    };
    let provider = get_user_provider(user).await?;
    if provider.chat_service().is_none() {
        return Err(StatusError::bad_request().brief("请在 AI 服务配置中启用文本对话模型"));
    }
    if matches!(input, Input::Audio(_)) && provider.asr().is_none() {
        return Err(StatusError::bad_request().brief("请在 AI 服务配置中启用语音识别 ASR 模型"));
    }
    Ok((input, provider))
}

pub(super) async fn check_owner(user: i64, chat: i64) -> Result<(), StatusError> {
    // Ownership must be checked before reading history, calling a provider or writing anything.
    let exists = with_conn!(move |conn| {
        learn_chats::table
            .find(chat)
            .filter(learn_chats::user_id.eq(user))
            .select(learn_chats::id)
            .first::<i64>(conn).await
            .optional()
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("无法读取对话"))?;
    if exists.is_none() {
        return Err(StatusError::not_found().brief("对话不存在"));
    }
    Ok(())
}

async fn event(tx: &mpsc::Sender<Value>, name: &str, data: Value) {
    let _ = tokio::time::timeout(
        Duration::from_secs(5),
        tx.send(json!({"event":name,"data":data})),
    )
    .await;
}

#[handler]
pub async fn send_stream(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user = depot.user_id()?;
    let chat = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("对话编号无效"))?;
    check_owner(user, chat).await?;
    let request_id = req
        .headers()
        .get("X-Conversation-Request")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    if !cancellation::valid_id(&request_id) {
        return Err(StatusError::bad_request().brief("轮次编号无效").into());
    }
    let voice = req
        .headers()
        .get("X-Conversation-Voice")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("background")
        .to_owned();
    if !["background", "browser", "provider"].contains(&voice.as_str()) {
        return Err(StatusError::bad_request().brief("语音输出方式无效").into());
    }
    let job = cancellation::register(user, chat, request_id)?;
    let (input, provider) = match prepare(req, user, chat).await {
        Ok(value) => value,
        Err(error) => {
            job.finish();
            return Err(error.into());
        }
    };
    if voice == "provider" && provider.tts().is_none() {
        job.finish();
        return Err(StatusError::bad_request()
            .brief("请在 AI 服务配置中启用语音合成 TTS 模型")
            .into());
    }
    let (tx, rx) = mpsc::channel(16);
    tokio::spawn(async move {
        if voice == "provider" {
            let (sentences, receive) = mpsc::channel(2);
            let text = async {
                let result = controlled_with_speech(
                    user,
                    chat,
                    input,
                    provider.clone(),
                    &tx,
                    &job,
                    Some(sentences),
                )
                .await;
                if result.is_err() {
                    job.request_cancel();
                }
                result
            };
            let work = async {
                let (result, count) = tokio::join!(
                    text,
                    speech::run(provider.tts().unwrap(), receive, &tx, &job)
                );
                match result {
                    Ok(_) => event(&tx, "voice_done", json!({"segments":count})).await,
                    Err(error) => event(&tx, "error", json!({"message":error.brief})).await,
                }
            };
            tokio::pin!(work);
            tokio::select! { _ = &mut work => {}, _ = tx.closed() => { job.request_cancel(); work.await; } }
        } else {
            match controlled_conversation(user, chat, input, provider.clone(), &tx, &job).await {
                Ok(result) if voice == "background" => {
                    generate_audio(
                        user,
                        provider,
                        result.ai_turn.id,
                        result.ai_turn.content_en,
                        job.clone(),
                    )
                    .await
                }
                Ok(_) => {}
                Err(error) => event(&tx, "error", json!({"message":error.brief})).await,
            }
        }
        job.finish();
    });
    res.add_header("Content-Type", "text/event-stream; charset=utf-8", true)?;
    res.add_header("Cache-Control", "no-cache, no-transform", true)?;
    res.add_header("X-Accel-Buffering", "no", true)?;
    let interval = tokio::time::interval(Duration::from_secs(10));
    res.stream(futures_util::stream::unfold((rx,interval),|(mut rx,mut interval)|async move {
        let text=tokio::select!{
            value=rx.recv()=>match value {Some(value)=>format!("data: {value}\n\n"),None=>return None},
            _=interval.tick()=>": keep-alive\n\n".to_string(),
        };
        Some((Ok::<_,std::io::Error>(text),(rx,interval)))
    }));
    Ok(())
}

fn record(user: i64, chat: i64, speaker: &str, text: String, status: &str) -> NewChatTurn {
    NewChatTurn {
        user_id: user,
        chat_id: chat,
        speaker: speaker.into(),
        use_lang: "en".into(),
        content_en: text,
        content_zh: String::new(),
        audio_path: None,
        duration_ms: None,
        words_per_minute: None,
        issues_count: Some(0),
        hesitation_count: None,
        status: status.into(),
    }
}

async fn pair(
    user: i64,
    chat: i64,
    text: String,
    audio: Option<String>,
) -> Result<(ChatTurn, ChatTurn), StatusError> {
    let result = with_conn!(move |conn| {
        horme::transaction(conn, async |conn| {
            if learn_chats::table
                .find(chat)
                .filter(learn_chats::user_id.eq(user))
                .for_update()
                .first::<Chat>(conn).await
                .optional()?
                .is_none()
            {
                return Ok(Err(false));
            }
            let cutoff = Utc::now() - chrono::Duration::minutes(5);
            if learn_chat_turns::table
                .filter(learn_chat_turns::chat_id.eq(chat))
                .filter(learn_chat_turns::user_id.eq(user))
                .filter(learn_chat_turns::speaker.eq("assistant"))
                .filter(learn_chat_turns::status.eq("processing"))
                .filter(learn_chat_turns::created_at.gt(cutoff))
                .count()
                .get_result::<i64>(conn).await?
                > 0
            {
                return Ok(Err(true));
            }
            diesel::update(
                learn_chat_turns::table
                    .filter(learn_chat_turns::chat_id.eq(chat))
                    .filter(learn_chat_turns::user_id.eq(user))
                    .filter(learn_chat_turns::status.eq("processing"))
                    .filter(learn_chat_turns::created_at.le(cutoff)),
            )
            .set((
                learn_chat_turns::status.eq("error"),
                learn_chat_turns::error.eq(Some("上次回复已超时，请重试")),
            ))
            .execute(conn).await?;
            let mut message = record(user, chat, "user", text, "completed");
            message.audio_path = audio;
            let user_turn = diesel::insert_into(learn_chat_turns::table)
                .values(message)
                .get_result::<ChatTurn>(conn).await?;
            let ai_turn = diesel::insert_into(learn_chat_turns::table)
                .values(record(user, chat, "assistant", String::new(), "processing"))
                .get_result::<ChatTurn>(conn).await?;
            Ok(Ok((user_turn, ai_turn)))
        }).await
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("无法保存消息"))?;
    result.map_err(|busy| {
        if busy {
            StatusError::conflict().brief("上一条回复仍在生成，请稍候")
        } else {
            StatusError::not_found().brief("对话不存在")
        }
    })
}

async fn generate(
    provider: Arc<dyn AiProvider>,
    history: Vec<ChatMessage>,
    text: &str,
    ai: i64,
    events: &mpsc::Sender<Value>,
    job: &cancellation::Job,
    speech: Option<mpsc::Sender<String>>,
) -> Result<StructuredChatResponse, StatusError> {
    let service = provider
        .chat_service()
        .ok_or_else(|| StatusError::service_unavailable().brief("该提供方不支持对话"))?;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let future = service.chat_structured_stream(history, text, chat_stream::SYSTEM_PROMPT, tx);
    tokio::pin!(future);
    let mut raw = String::new();
    let mut phrases = speech::Phrases::default();
    let mut last = (String::new(), String::new());
    let mut saved = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(90), async {
        loop {
            let delta = tokio::select! {
                biased;
                _ = job.wait() => return Err(AiProviderError::Request("cancelled".into())),
                result=&mut future=>{
                    while let Ok(delta)=rx.try_recv(){raw.push_str(&delta);}
                    break result;
                },
                delta=rx.recv()=>delta,
            };
            if let Some(delta) = delta {
                raw.push_str(&delta);
                if raw.len() > chat_stream::MAX_OUTPUT_BYTES {
                    return Err(AiProviderError::Parse("回答过长".into()));
                }
                let current = (
                    chat_stream::string_prefix(&raw, "reply_en").unwrap_or_default(),
                    chat_stream::string_prefix(&raw, "reply_zh").unwrap_or_default(),
                );
                if current != last {
                    speech::feed(&mut phrases, &current.0, false, &speech, job, events).await;
                    event(
                        events,
                        "delta",
                        json!({"reply_en":current.0,"reply_zh":current.1}),
                    )
                    .await;
                    last = current;
                    if saved.elapsed() >= Duration::from_secs(1) {
                        let (en, zh) = last.clone();
                        let _ = with_conn!(move |conn| {
                            diesel::update(
                                learn_chat_turns::table
                                    .find(ai)
                                    .filter(learn_chat_turns::status.eq("processing")),
                            )
                            .set((
                                learn_chat_turns::content_en.eq(en),
                                learn_chat_turns::content_zh.eq(zh),
                            ))
                            .execute(conn).await
                        })
                        .await;
                        saved = std::time::Instant::now();
                    }
                }
            }
        }
    })
    .await;
    match result {
        Ok(Ok(response)) if !job.cancelled() => {
            speech::feed(&mut phrases, &response.reply_en, true, &speech, job, events).await;
            Ok(response)
        }
        _ => {
            let (en, zh) = last;
            let cancelled = job.cancelled();
            let _ = with_conn!(move |conn| {
                diesel::update(
                    learn_chat_turns::table
                        .find(ai)
                        .filter(learn_chat_turns::status.eq("processing")),
                )
                .set((
                    learn_chat_turns::content_en.eq(en),
                    learn_chat_turns::content_zh.eq(zh),
                    learn_chat_turns::status.eq(if cancelled { "cancelled" } else { "error" }),
                    learn_chat_turns::error.eq(if cancelled {
                        None
                    } else {
                        Some("AI 回复中断或超时，请重试")
                    }),
                ))
                .execute(conn).await
            })
            .await;
            if cancelled {
                event(events, "cancelled", json!({"ai_turn_id":ai})).await;
                Err(StatusError::conflict().brief("本轮已取消"))
            } else {
                Err(StatusError::bad_gateway().brief("AI 回复中断或超时，请重试"))
            }
        }
    }
}

pub(super) async fn conversation(
    user: i64,
    chat: i64,
    input: Input,
    provider: Arc<dyn AiProvider>,
    events: &mpsc::Sender<Value>,
) -> Result<ChatSendResponse, StatusError> {
    let job = cancellation::Job::new();
    let result = controlled_conversation(user, chat, input, provider.clone(), events, &job).await?;
    let ai_id = result.ai_turn.id;
    let text = result.ai_turn.content_en.clone();
    tokio::spawn(async move {
        generate_audio(user, provider, ai_id, text, job).await;
    });
    Ok(result)
}

async fn controlled_conversation(
    user: i64,
    chat: i64,
    input: Input,
    provider: Arc<dyn AiProvider>,
    events: &mpsc::Sender<Value>,
    job: &cancellation::Job,
) -> Result<ChatSendResponse, StatusError> {
    controlled_with_speech(user, chat, input, provider, events, job, None).await
}
async fn controlled_with_speech(
    user: i64,
    chat: i64,
    input: Input,
    provider: Arc<dyn AiProvider>,
    events: &mpsc::Sender<Value>,
    job: &cancellation::Job,
    speech: Option<mpsc::Sender<String>>,
) -> Result<ChatSendResponse, StatusError> {
    if job.cancelled() {
        return Err(StatusError::conflict().brief("本轮已取消"));
    }
    let (text, audio) = match input {
        Input::Text(text) => (text, None),
        Input::Audio(bytes) => {
            event(events, "phase", json!({"message":"正在识别语音…"})).await;
            let asr = provider.asr().ok_or_else(|| {
                StatusError::service_unavailable().brief("该提供方不支持语音识别")
            })?;
            let response = tokio::select! {
                _ = job.wait() => return Err(StatusError::conflict().brief("本轮已取消")),
                response = tokio::time::timeout(
                Duration::from_secs(30),
                asr.transcribe(bytes.clone(), Some("auto")),
            )
            => response,
            }
            .map_err(|_| StatusError::gateway_timeout().brief("语音识别超时，请重试"))?
            .map_err(|_| StatusError::bad_gateway().brief("语音识别失败，请重试"))?;
            if response.text.trim().is_empty() {
                return Err(StatusError::bad_request().brief("没有识别到有效语音"));
            }
            (
                response.text,
                save_audio_file(user, &bytes, "user", "wav").await,
            )
        }
    };
    if job.cancelled() {
        return Err(StatusError::conflict().brief("本轮已取消"));
    }
    let (user_turn, ai_turn) = pair(user, chat, text.clone(), audio).await?;
    let user_id = user_turn.id;
    let ai_id = ai_turn.id;
    event(
        events,
        "accepted",
        json!({"user_turn":ChatTurnWithIssues::from_turn(user_turn),"ai_turn_id":ai_id}),
    )
    .await;
    let history = match get_chat_latest_turns(chat, user, user_id, 40).await {
        Ok(history) => history,
        Err(error) => {
            let _ = update_ai_turn(
                ai_id,
                None,
                "error".into(),
                Some("无法读取对话历史，请重试".into()),
            )
            .await;
            return Err(error);
        }
    };
    // Provider I/O is cancellable; database futures are always awaited so a
    // detached spawn_blocking write cannot race the next turn.
    let response = generate(provider.clone(), history, &text, ai_id, events, job, speech).await?;
    let result = with_conn!(move |conn| {
        horme::transaction(conn, async |conn| {
            let updated = diesel::update(learn_chat_turns::table.find(user_id))
                .set((
                    learn_chat_turns::use_lang.eq(&response.use_lang),
                    learn_chat_turns::content_en.eq(&response.original_en),
                    learn_chat_turns::content_zh.eq(&response.original_zh),
                    learn_chat_turns::issues_count.eq(response.issues.len() as i32),
                ))
                .get_result::<ChatTurn>(conn).await?;
            let issues = response
                .issues
                .iter()
                .map(|issue| NewChatIssue {
                    user_id: user,
                    chat_id: chat,
                    chat_turn_id: user_id,
                    issue_type: issue.issue_type.clone(),
                    start_position: issue.start_position,
                    end_position: issue.end_position,
                    original_text: Some(issue.original.clone()),
                    suggested_text: Some(issue.suggested.clone()),
                    description_en: Some(issue.description_en.clone()),
                    description_zh: Some(issue.description_zh.clone()),
                    severity: Some(issue.severity.clone()),
                })
                .collect::<Vec<_>>();
            let issues = if issues.is_empty() {
                vec![]
            } else {
                diesel::insert_into(learn_chat_issues::table)
                    .values(issues)
                    .get_results::<ChatIssue>(conn).await?
            };
            let ai = diesel::update(learn_chat_turns::table.find(ai_id))
                .set((
                    learn_chat_turns::content_en.eq(&response.reply_en),
                    learn_chat_turns::content_zh.eq(&response.reply_zh),
                    learn_chat_turns::status.eq("completed"),
                    learn_chat_turns::error.eq(None::<String>),
                ))
                .get_result::<ChatTurn>(conn).await?;
            Ok(ChatSendResponse {
                user_turn: ChatTurnWithIssues::from_turn_with_issues(updated, issues),
                ai_turn: ChatTurnWithIssues::from_turn(ai),
            })
        }).await
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("对话已更改，无法保存回复，请刷新"));
    let result = match result {
        Ok(value) => value,
        Err(error) => {
            let _ = update_ai_turn(ai_id, None, "error".into(), Some(error.brief.clone())).await;
            return Err(error);
        }
    };
    event(
        events,
        "done",
        serde_json::to_value(&result).unwrap_or_default(),
    )
    .await;
    Ok(result)
}

async fn generate_audio(
    user: i64,
    provider: Arc<dyn AiProvider>,
    ai_id: i64,
    text: String,
    job: Arc<cancellation::Job>,
) {
    if let Some(tts) = provider.tts() {
        let audio = tokio::select! {
            biased;
            _ = job.wait() => return,
            audio = tokio::time::timeout(Duration::from_secs(45), tts.synthesize(&text, None, None)) => match audio {
                Ok(Ok(audio)) => audio,
                _ => { tracing::warn!(turn_id=ai_id, "Background speech generation failed or timed out"); return; }
            }
        };
        if !job.cancelled()
            && let Some(path) = save_audio_file(user, &audio.audio_data, "ai", &audio.format).await
        {
            let _ = update_ai_turn(ai_id, Some(path), "completed".into(), None).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ai_provider::{AsrService, ChatService, TtsResponse, TtsService};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct Fake {
        fail: bool,
        audio_done: Arc<AtomicBool>,
    }
    #[async_trait]
    impl ChatService for Fake {
        async fn chat(
            &self,
            _: Vec<ChatMessage>,
            _: Option<f32>,
            _: Option<u32>,
        ) -> Result<String, AiProviderError> {
            unreachable!()
        }
        async fn chat_structured(
            &self,
            _: Vec<ChatMessage>,
            _: &str,
            _: &str,
        ) -> Result<StructuredChatResponse, AiProviderError> {
            unreachable!()
        }
        async fn chat_structured_stream(
            &self,
            history: Vec<ChatMessage>,
            _: &str,
            _: &str,
            tx: mpsc::UnboundedSender<String>,
        ) -> Result<StructuredChatResponse, AiProviderError> {
            assert!(history.len() <= 40);
            let first = r#"{"reply_en":"Hello"#;
            tx.send(first.into()).unwrap();
            tokio::time::sleep(Duration::from_millis(80)).await;
            if self.fail {
                return Err(AiProviderError::Request("test disconnect".into()));
            }
            let rest = r#" there!","reply_zh":"你好","use_lang":"en","original_en":"New input","original_zh":"新输入","issues":[]}"#;
            tx.send(rest.into()).unwrap();
            tokio::time::sleep(Duration::from_millis(40)).await;
            chat_stream::finish(&format!("{first}{rest}"))
        }
    }
    #[async_trait]
    impl TtsService for Fake {
        async fn synthesize(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<f32>,
        ) -> Result<TtsResponse, AiProviderError> {
            tokio::time::sleep(Duration::from_millis(800)).await;
            self.audio_done.store(true, Ordering::SeqCst);
            Ok(TtsResponse {
                audio_data: vec![0; 44],
                format: "wav".into(),
                duration_ms: None,
            })
        }
    }
    #[async_trait]
    impl AiProvider for Fake {
        fn name(&self) -> &'static str {
            "test"
        }
        fn asr(&self) -> Option<Arc<dyn AsrService>> {
            None
        }
        fn tts(&self) -> Option<Arc<dyn TtsService>> {
            Some(Arc::new(Fake {
                fail: self.fail,
                audio_done: self.audio_done.clone(),
            }))
        }
        fn chat_service(&self) -> Option<Arc<dyn ChatService>> {
            Some(Arc::new(Fake {
                fail: self.fail,
                audio_done: self.audio_done.clone(),
            }))
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "requires an explicitly isolated local colang_qa database"]
    async fn chat_stream_database_flow() {
        let database = std::env::var("QA_DATABASE_URL").expect("QA_DATABASE_URL required");
        let url = url::Url::parse(&database).unwrap();
        assert!(
            matches!(url.host_str(), Some("localhost" | "127.0.0.1")) && url.path() == "/colang_qa"
        );
        let config = AppConfig {
            bind_addr: "127.0.0.1:0".into(),
            jwt_secret: crate::auth::random_code(),
            jwt_ttl: Duration::from_secs(600),
            database: crate::config::DbConfig {
                url: database,
                ..Default::default()
            },
            space_path: std::path::PathBuf::from(
                std::env::var("QA_ARTIFACTS").expect("QA_ARTIFACTS required"),
            )
            .join("chat-test-audio")
            .to_string_lossy()
            .into(),
        };
        crate::config::APP_CONFIG.set(config.clone()).unwrap();
        crate::db::init(&config.database).await;
        let suffix = crate::auth::random_code();
        let (user, chat) = with_conn!(move |conn| {
            let user = diesel::insert_into(base_users::table)
                .values(crate::models::NewUser {
                    name: format!("Stream {suffix}"),
                    email: None,
                    phone: None,
                    display_name: None,
                    inviter_id: None,
                    updated_by: None,
                    created_by: None,
                })
                .get_result::<crate::models::User>(conn).await?;
            let chat = diesel::insert_into(learn_chats::table)
                .values(NewChat {
                    user_id: user.id,
                    title: "Streaming QA".into(),
                    context_id: None,
                    duration_ms: None,
                    issues_count: None,
                })
                .get_result::<Chat>(conn).await?;
            for i in 0..55 {
                diesel::insert_into(learn_chat_turns::table)
                    .values(record(
                        user.id,
                        chat.id,
                        "user",
                        format!("message-{i}"),
                        "completed",
                    ))
                    .execute(conn).await?;
            }
            Ok((user.id, chat.id))
        })
        .await
        .unwrap();
        let history = get_chat_latest_turns(chat, user, i64::MAX, 40)
            .await
            .unwrap();
        assert_eq!(history.len(), 40);
        assert_eq!(history[0].content, "message-15");
        assert_eq!(history[39].content, "message-54");
        assert!(
            get_chat_latest_turns(chat, user + 99999, i64::MAX, 40)
                .await
                .unwrap()
                .is_empty()
        );
        let audio_done = Arc::new(AtomicBool::new(false));
        let provider: Arc<dyn AiProvider> = Arc::new(Fake {
            fail: false,
            audio_done: audio_done.clone(),
        });
        let (tx, mut rx) = mpsc::channel(16);
        let task = tokio::spawn(async move {
            conversation(user, chat, Input::Text("New input".into()), provider, &tx).await
        });
        let first = rx.recv().await.unwrap();
        assert_eq!(first["event"], "accepted");
        assert_eq!(
            pair(user, chat, "duplicate".into(), None)
                .await
                .unwrap_err()
                .code,
            StatusCode::CONFLICT
        );
        let mut partial = false;
        while let Some(frame) = rx.recv().await {
            if frame["event"] == "delta" && frame["data"]["reply_en"] == "Hello" {
                partial = true;
            }
            if frame["event"] == "done" {
                assert!(partial);
                assert!(!audio_done.load(Ordering::SeqCst));
                break;
            }
        }
        let result = task.await.unwrap().unwrap();
        assert_eq!(result.ai_turn.content_en, "Hello there!");
        tokio::time::sleep(Duration::from_millis(1000)).await;
        assert!(audio_done.load(Ordering::SeqCst));
        let ai = result.ai_turn.id;
        let stored =
            with_conn!(move |conn| learn_chat_turns::table.find(ai).first::<ChatTurn>(conn).await)
                .await
                .unwrap();
        assert_eq!(stored.status, "completed");
        assert!(stored.audio_path.is_some());
        let (tx, rx) = mpsc::channel(16);
        drop(rx);
        let failed = conversation(
            user,
            chat,
            Input::Text("New input".into()),
            Arc::new(Fake {
                fail: true,
                audio_done: audio_done.clone(),
            }),
            &tx,
        )
        .await;
        assert!(failed.is_err());
        let last = with_conn!(move |conn| {
            learn_chat_turns::table
                .filter(learn_chat_turns::chat_id.eq(chat))
                .order(learn_chat_turns::id.desc())
                .first::<ChatTurn>(conn).await
        })
        .await
        .unwrap();
        assert_eq!(last.status, "error");
        assert_eq!(last.content_en, "Hello");
        // Cancellation retains the visible partial text and releases the chat.
        let job = cancellation::Job::new();
        let worker_job = job.clone();
        let provider: Arc<dyn AiProvider> = Arc::new(Fake {
            fail: false,
            audio_done: audio_done.clone(),
        });
        let (tx, mut rx) = mpsc::channel(16);
        let worker = tokio::spawn(async move {
            controlled_conversation(
                user,
                chat,
                Input::Text("interrupt me".into()),
                provider,
                &tx,
                &worker_job,
            )
            .await
        });
        let mut interrupted_id = 0;
        while let Some(frame) = rx.recv().await {
            if frame["event"] == "accepted" {
                interrupted_id = frame["data"]["ai_turn_id"].as_i64().unwrap();
            }
            if frame["event"] == "delta" {
                job.request_cancel();
                break;
            }
        }
        assert!(worker.await.unwrap().is_err());
        let interrupted = with_conn!(move |conn| {
            learn_chat_turns::table
                .find(interrupted_id)
                .first::<ChatTurn>(conn).await
        })
        .await
        .unwrap();
        assert_eq!(interrupted.status, "cancelled");
        assert_eq!(interrupted.content_en, "Hello");
        assert!(interrupted.error.is_none());
        let history = get_chat_latest_turns(chat, user, i64::MAX, 40)
            .await
            .unwrap();
        assert!(
            !history
                .iter()
                .any(|m| m.role == "assistant" && m.content == "Hello")
        );
        tokio::time::sleep(Duration::from_millis(150)).await;
        let stored = with_conn!(move |conn| {
            learn_chat_turns::table
                .find(interrupted_id)
                .first::<ChatTurn>(conn).await
        })
        .await
        .unwrap();
        assert_eq!(stored.status, "cancelled");
        assert!(pair(user, chat, "retry".into(), None).await.is_ok());
    }
}
