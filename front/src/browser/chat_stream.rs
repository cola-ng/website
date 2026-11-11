use super::*;
use std::cell::Cell;
use std::rc::Rc;

/// A page reopened during generation has no original POST stream to consume.
/// Recover that persisted turn only; normal streamed sends never enter this path.
pub fn pending_turn(id: String, chat: i64, turns: Resource) -> impl Widget {
    widget(move |scope| {
        let id = id.clone();
        let status = scope.cage("AI 正在回复…".to_string());
        spawn(async move {
            for _ in 0..90 {
                gloo_timers::future::TimeoutFuture::new(2000).await;
                if !status.is_alive() {
                    return;
                }
                let result = request("GET", &format!("/api/learn/chats/turns/{id}"), None).await;
                if !status.is_alive() {
                    return;
                }
                match result {
                    Ok(turn) if turn["status"] != "processing" => {
                        turns.load(format!(
                            "/api/learn/chats/{chat}/turns?limit=50&from_latest=true"
                        ));
                        return;
                    }
                    Ok(_) => {}
                    Err(_) => {
                        status.revise(|v| *v = "暂时无法同步回复，请刷新重试。".into());
                        return;
                    }
                }
            }
            if status.is_alive() {
                status.revise(|v| *v = "回复仍在处理中，请稍后刷新。".into());
            }
        });
        view()
            .role(Role::Status)
            .fill(text().content(status))
            .build(scope)
    })
}

pub fn load_older(id: i64, turns: Resource, action: Resource) {
    let before = string(&turns.data.get(), "first_id");
    action.run(
        "GET",
        format!("/api/learn/chats/{id}/turns?limit=50&before_id={before}&from_latest=true"),
        None,
        move |page| {
            turns.data.revise(|current| {
                let mut history = items(&page);
                for message in items(current) {
                    if !history.iter().any(|old| old["id"] == message["id"]) {
                        history.push(message);
                    }
                }
                current["items"] = json!(history);
                current["first_id"] = page["first_id"].clone();
                current["has_prev"] = page["has_prev"].clone();
            });
        },
    );
}

#[derive(Clone)]
pub struct ConversationStream {
    live: Cage<Value>,
    pub running: Cage<bool>,
    voice_enabled: Cage<bool>,
    voice_source: Cage<String>,
    epoch: Rc<Cell<u64>>,
    request_id: Rc<RefCell<Option<String>>>,
    cancelling: Rc<Cell<bool>>,
    voice: Rc<RefCell<Option<JsValue>>>,
    abort: Rc<RefCell<Option<web_sys::AbortController>>>,
}

impl ConversationStream {
    pub fn new(scope: &Scope) -> Self {
        let abort = Rc::new(RefCell::new(None::<web_sys::AbortController>));
        let cleanup = abort.clone();
        let epoch = Rc::new(Cell::new(0u64));
        let cleanup_epoch = epoch.clone();
        let voice = Rc::new(RefCell::new(None::<JsValue>));
        let cleanup_voice = voice.clone();
        let _ = scope.on_cleanup(move || {
            cleanup_epoch.set(cleanup_epoch.get() + 1);
            if let Some(session) = cleanup_voice.borrow_mut().take() {
                media::close_voice(&session);
            }
            media::stop_audio();
            if let Some(controller) = cleanup.borrow_mut().take() {
                controller.abort();
            }
        });
        Self {
            live: scope.cage(Value::Null),
            running: scope.cage(false),
            voice_enabled: scope.cage(false),
            voice_source: scope.cage(
                storage()
                    .and_then(|s| s.get_item("chat.voice_output").ok().flatten())
                    .filter(|v| v == "browser")
                    .unwrap_or_else(|| "provider".into()),
            ),
            epoch,
            voice,
            request_id: Rc::new(RefCell::new(None)),
            cancelling: Rc::new(Cell::new(false)),
            abort,
        }
    }
    pub fn close_voice(&self) {
        let session = self.voice.borrow_mut().take();
        if let Some(session) = session {
            media::close_voice(&session);
        }
    }
    pub async fn cancel(
        &self,
        chat: i64,
        turns: Resource,
        action: Resource,
    ) -> std::result::Result<(), String> {
        media::stop_audio();
        let request_id = self.request_id.borrow().clone();
        while self.cancelling.get() {
            gloo_timers::future::TimeoutFuture::new(20).await;
        }
        if *self.request_id.borrow() != request_id {
            return Ok(());
        }
        let Some(request_id) = request_id else {
            return Ok(());
        };
        self.cancelling.set(true);
        self.epoch.set(self.epoch.get() + 1);
        if let Some(controller) = self.abort.borrow_mut().take() {
            controller.abort();
        }
        if self.live.is_alive() {
            let current = self.live.get();
            if !current["ai_turn_id"].is_null() {
                merge(
                    turns,
                    &json!({"id":current["ai_turn_id"],"speaker":"assistant","content_en":current["reply_en"].as_str().unwrap_or_default(),"content_zh":current["reply_zh"].as_str().unwrap_or_default(),"status":"cancelled"}),
                );
            }
            self.live.revise(|v| *v = Value::Null);
            self.running.revise(|v| *v = false);
        }
        let result = request(
            "POST",
            &format!("/api/learn/chats/{chat}/cancel"),
            Some(json!({"request_id":request_id})),
        )
        .await;
        self.cancelling.set(false);
        if result.is_ok() {
            self.request_id.borrow_mut().take();
        }
        if action.busy.is_alive() {
            action.busy.revise(|v| *v = false);
        }
        result.map(|_| ())
    }
    pub fn voice_controls(self, chat: i64, turns: Resource, action: Resource) -> impl Widget {
        widget(move |scope| {
            let enabled = self.voice_enabled;
            let source = self.voice_source;
            let status = scope.cage(String::new());
            let stream = self.clone();
            let voice_enabled = enabled;
            let voice_epoch = Rc::new(Cell::new(0u64));
            let callback =
                Closure::<dyn FnMut(String, String)>::new(move |kind: String, value: String| {
                    if !status.is_alive() {
                        return;
                    }
                    match kind.as_str() {
                        "status" => status.revise(|v| *v = value),
                        "error" => action.error.revise(|v| *v = value),
                        "closed" => {
                            voice_epoch.set(voice_epoch.get() + 1);
                            voice_enabled.revise(|v| *v = false);
                            status.revise(|v| *v = value);
                        }
                        "audio" | "interrupt" => {
                            let stream = stream.clone();
                            if kind == "interrupt" {
                                voice_epoch.set(voice_epoch.get() + 1);
                            }
                            let ticket = voice_epoch.get();
                            let voice_epoch = voice_epoch.clone();
                            let audio = (kind == "audio").then_some(value);
                            spawn(async move {
                                match stream.cancel(chat, turns, action).await {
                                    Ok(())
                                        if enabled.is_alive()
                                            && enabled.get()
                                            && voice_epoch.get() == ticket =>
                                    {
                                        if let Some(audio) = audio {
                                            stream.send(
                                                chat,
                                                json!({"type":"audio","audio_base64":audio}),
                                                turns,
                                                action,
                                                None,
                                            );
                                        }
                                    }
                                    Err(e) if action.error.is_alive() => {
                                        action.error.revise(|v| *v = e);
                                        stream.close_voice();
                                    }
                                    _ => {}
                                }
                            });
                        }
                        _ => {}
                    }
                });
            let callback = Rc::new(callback);
            let keep_callback = callback.clone();
            let cleanup = self.clone();
            let _ = scope.on_cleanup(move || {
                cleanup.close_voice();
                drop(keep_callback);
            });
            let toggle = self.clone();
            let stop = self.clone();
            view().class("chat-voice-controls").fill((
                action_button().label(enabled.map(|v| if *v { "结束连续语音" } else { "开启连续语音" }.to_string()))
                    .pressed(enabled).enabled(action.busy.map(move |v| !*v || self.running.get()))
                    .on_press(move || {
                        if enabled.get() { toggle.close_voice(); return; }
                        media::stop_audio(); action.error.revise(String::clear); enabled.revise(|v| *v = true);
                        let session = media::open_voice(callback.as_ref().as_ref().unchecked_ref(), &source.get());
                        *toggle.voice.borrow_mut() = Some(session);
                        let stream = toggle.clone();
                        spawn(async move { if let Err(e) = stream.cancel(chat,turns,action).await && action.error.is_alive() { action.error.revise(|v| *v = e); stream.close_voice(); } });
                    }),
                action_button().label("打断回复").variant(sapid::ButtonVariant::Outline)
                    .on_press(move || {
                        media::stop_audio();
                        let session = stop.voice.borrow().clone();
                        if let Some(session) = session { media::voice_reset(&session); }
                        let stream=stop.clone();
                        spawn(async move { if let Err(e)=stream.cancel(chat,turns,action).await && action.error.is_alive() { action.error.revise(|v| *v=e); } });
                    }),
                sapid::popover("voice-settings").theme(theme::orange()).label("语音设置").trigger_content(||text().content("⚙"))
                    .close_label("关闭语音设置").content(move||view().class("dialog-form").fill((
                        text().content("连续语音音源"),
                        action_button().label("用户配置的 TTS").pressed(source.map(|v|v=="provider")).enabled(enabled.map(|v|!*v)).on_press(move||{source.revise(|v|*v="provider".into());if let Some(s)=storage(){let _=s.set_item("chat.voice_output","provider");}}),
                        action_button().label("浏览器系统语音").pressed(source.map(|v|v=="browser")).enabled(enabled.map(|v|!*v)).on_press(move||{source.revise(|v|*v="browser".into());if let Some(s)=storage(){let _=s.set_item("chat.voice_output","browser");}}),
                        text().class("muted").content("结束连续语音后可切换音源；TTS 失败会提示，不自动切换。"),
                    ))),
                view().role(Role::Status).fill(text().content(status)),
                text().class("muted").content("连续语音：停顿自动发送，播完可接话；Esc 或切到后台结束，30 秒无接话自动关闭。"),
            )).build(scope)
        })
    }
    pub fn view(self) -> impl Widget {
        let live = self.live;
        when(live.map(|v| !v.is_null()), move || {
            view().class("message ai-message streaming-reply").fill((
                text().class("eyebrow").content("AI 伙伴"),
                text()
                    .kind(TextKind::Paragraph)
                    .class("message-en")
                    .content(live.map(|v| string(v, "reply_en"))),
                text()
                    .kind(TextKind::Paragraph)
                    .class("message-zh")
                    .content(live.map(|v| string(v, "reply_zh"))),
                view()
                    .role(Role::Status)
                    .class("loading-status")
                    .fill(text().content(live.map(|v| string(v, "phase")))),
            ))
        })
    }
    pub fn send(
        self,
        id: i64,
        body: Value,
        turns: Resource,
        action: Resource,
        draft: Option<Cage<String>>,
    ) {
        if action.busy.get() || self.cancelling.get() {
            return;
        }
        action.busy.revise(|v| *v = true);
        spawn(async move {
            match self.cancel(id, turns, action).await {
                Ok(()) if action.busy.is_alive() => {
                    action.busy.revise(|v| *v = false);
                    self.send_inner(id, body, turns, action, draft);
                }
                Err(e) if action.error.is_alive() => {
                    action.busy.revise(|v| *v = false);
                    action.error.revise(|v| *v = e);
                }
                _ => {}
            }
        });
    }
    fn send_inner(
        self,
        id: i64,
        body: Value,
        turns: Resource,
        action: Resource,
        draft: Option<Cage<String>>,
    ) {
        if action.busy.get() {
            return;
        }
        media::stop_audio();
        let ticket = self.epoch.get() + 1;
        self.epoch.set(ticket);
        self.running.revise(|v| *v = true);
        let request_id = format!(
            "web-{}-{}",
            js_sys::Date::now() as u64,
            (js_sys::Math::random() * u64::MAX as f64) as u64
        );
        *self.request_id.borrow_mut() = Some(request_id.clone());
        let epoch = self.epoch.clone();
        if let Some(session) = self.voice.borrow().as_ref() {
            media::voice_waiting(session);
        }
        let voice = self.voice.clone();
        let running = self.running;
        let voice_mode = if self.voice_enabled.get() {
            self.voice_source.get()
        } else {
            "background".into()
        };
        action.busy.revise(|v| *v = true);
        action.error.revise(String::clear);
        self.live.revise(|v| *v = json!({"phase":"正在连接 AI…"}));
        let live = self.live;
        let abort = self.abort;
        spawn(async move {
            let mut accepted = false;
            let mut text_done = false;
            let result = receive(id, body, request_id, voice_mode.clone(), abort, |frame| {
                if !action.busy.is_alive() || epoch.get() != ticket {
                    return Err("页面已关闭".into());
                }
                let data = &frame["data"];
                match frame["event"].as_str() {
                    Some("phase") => live.revise(|v| v["phase"] = data["message"].clone()),
                    Some("accepted") => {
                        accepted = true;
                        if let Some(draft) = draft {
                            draft.revise(String::clear);
                        }
                        merge(turns, &data["user_turn"]);
                        live.revise(|v| {
                            *v = json!({"phase":"AI 正在回复…","ai_turn_id":data["ai_turn_id"]})
                        });
                    }
                    Some("delta") => {
                        if let Some(session) = voice.borrow().as_ref() {
                            media::voice_reply(
                                session,
                                data["reply_en"].as_str().unwrap_or_default(),
                                false,
                            );
                        }
                        live.revise(|v| {
                            v["reply_en"] = data["reply_en"].clone();
                            v["reply_zh"] = data["reply_zh"].clone();
                            v["phase"] = json!("正在生成回复与学习反馈…");
                        });
                    }
                    Some("done") => {
                        if let Some(session) = voice.borrow().as_ref() {
                            media::voice_reply(
                                session,
                                data["ai_turn"]["content_en"].as_str().unwrap_or_default(),
                                true,
                            );
                        }
                        merge(turns, &data["user_turn"]);
                        merge(turns, &data["ai_turn"]);
                        live.revise(|v| *v = Value::Null);
                        text_done = true;
                        return Ok(voice_mode != "provider");
                    }
                    Some(
                        "audio_start" | "audio_chunk" | "audio_end" | "voice_error" | "voice_done",
                    ) => {
                        if let Some(session) = voice.borrow().as_ref() {
                            media::voice_audio(session, &frame.to_string());
                        }
                        if frame["event"] == "voice_done" {
                            if !text_done {
                                return Err("回复文字未完整返回".into());
                            }
                            return Ok(true);
                        }
                    }
                    Some("error") => {
                        return Err(data["message"]
                            .as_str()
                            .unwrap_or("回复失败，请重试")
                            .into());
                    }
                    _ => {}
                }
                Ok(false)
            })
            .await;
            if !action.busy.is_alive() || epoch.get() != ticket {
                return;
            }
            running.revise(|v| *v = false);
            live.revise(|v| *v = Value::Null);
            action.busy.revise(|v| *v = false);
            if let Err(error) = result {
                let session = voice.borrow_mut().take();
                if let Some(session) = session {
                    media::close_voice(&session);
                }
                action.error.revise(|v| *v = error);
                if accepted {
                    turns.load(format!(
                        "/api/learn/chats/{id}/turns?limit=50&from_latest=true"
                    ));
                }
            }
        });
    }
}

fn merge(turns: Resource, turn: &Value) {
    if turn["id"].is_null() {
        return;
    }
    let turn = turn.clone();
    turns.data.revise(|data| {
        let mut list = items(data);
        if let Some(old) = list.iter_mut().find(|v| v["id"] == turn["id"]) {
            *old = turn;
        } else {
            list.push(turn);
        }
        if data.is_object() {
            data["items"] = json!(list);
        } else {
            *data = json!({"items":list,"has_prev":false});
        }
    });
}

async fn receive(
    id: i64,
    body: Value,
    request_id: String,
    voice_mode: String,
    abort: Rc<RefCell<Option<web_sys::AbortController>>>,
    mut on_frame: impl FnMut(Value) -> std::result::Result<bool, String>,
) -> std::result::Result<(), String> {
    let controller = web_sys::AbortController::new().map_err(js_error)?;
    *abort.borrow_mut() = Some(controller.clone());
    let timeout_controller = controller.clone();
    let timeout = gloo_timers::callback::Timeout::new(125000, move || timeout_controller.abort());
    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    init.set_signal(Some(&controller.signal()));
    let headers = web_sys::Headers::new().map_err(js_error)?;
    headers
        .set("Content-Type", "application/json")
        .map_err(js_error)?;
    if voice_mode != "background" {
        headers
            .set("X-Conversation-Voice", &voice_mode)
            .map_err(js_error)?;
    }
    headers
        .set("X-Conversation-Request", &request_id)
        .map_err(js_error)?;
    headers
        .set("Accept", "text/event-stream, application/json")
        .map_err(js_error)?;
    if let Some(token) = token() {
        headers
            .set("Authorization", &format!("Bearer {token}"))
            .map_err(js_error)?;
    }
    init.set_headers(&headers);
    init.set_body(&JsValue::from_str(&body.to_string()));
    let result = async {
        let response: web_sys::Response = JsFuture::from(
            window().fetch_with_str_and_init(&format!("/api/learn/chats/{id}/send/stream"), &init),
        )
        .await
        .map_err(|_| "连接中断或超时，请重试".to_string())?
        .dyn_into()
        .map_err(js_error)?;
        if !response.ok() {
            let status = response.status();
            let text = JsFuture::from(response.text().map_err(js_error)?)
                .await
                .map_err(js_error)?
                .as_string()
                .unwrap_or_default();
            if status == 401 {
                clear_auth();
            }
            return Err(error_message(&text, status));
        }
        let reader = response
            .body()
            .ok_or("服务器没有返回回复流")?
            .get_reader()
            .dyn_into::<web_sys::ReadableStreamDefaultReader>()
            .map_err(|e| js_error(e.into()))?;
        let mut bytes = Vec::new();
        loop {
            let chunk = JsFuture::from(reader.read())
                .await
                .map_err(|_| "回复连接中断，请重试".to_string())?;
            if js_sys::Reflect::get(&chunk, &JsValue::from_str("done"))
                .map_err(js_error)?
                .as_bool()
                == Some(true)
            {
                return Err("回复未完整返回，请重试".into());
            }
            let value =
                js_sys::Reflect::get(&chunk, &JsValue::from_str("value")).map_err(js_error)?;
            bytes.extend(js_sys::Uint8Array::new(&value).to_vec());
            if bytes.len() > 256 * 1024 {
                return Err("回复数据超过长度限制".into());
            }
            while let Some(end) = bytes.windows(2).position(|v| v == b"\n\n") {
                let frame = String::from_utf8(bytes.drain(..end + 2).collect())
                    .map_err(|_| "回复编码无效".to_string())?;
                let payload = frame
                    .lines()
                    .filter_map(|line| line.strip_prefix("data:"))
                    .map(str::trim_start)
                    .collect::<Vec<_>>()
                    .join("\n");
                if payload.is_empty() {
                    continue;
                }
                let frame =
                    serde_json::from_str(&payload).map_err(|_| "回复格式无效".to_string())?;
                if on_frame(frame)? {
                    return Ok(());
                }
            }
        }
    }
    .await;
    drop(timeout);
    if abort
        .borrow()
        .as_ref()
        .is_some_and(|active| active == &controller)
    {
        abort.borrow_mut().take();
    }
    controller.abort();
    result
}
