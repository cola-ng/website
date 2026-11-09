use super::*;
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
    abort: Rc<RefCell<Option<web_sys::AbortController>>>,
}

impl ConversationStream {
    pub fn new(scope: &Scope) -> Self {
        let abort = Rc::new(RefCell::new(None::<web_sys::AbortController>));
        let cleanup = abort.clone();
        let _ = scope.on_cleanup(move || {
            if let Some(controller) = cleanup.borrow_mut().take() {
                controller.abort();
            }
        });
        Self {
            live: scope.cage(Value::Null),
            abort,
        }
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
        if action.busy.get() {
            return;
        }
        action.busy.revise(|v| *v = true);
        action.error.revise(String::clear);
        self.live.revise(|v| *v = json!({"phase":"正在连接 AI…"}));
        let live = self.live;
        let abort = self.abort;
        spawn(async move {
            let mut accepted = false;
            let result = receive(id, body, abort, |frame| {
                if !action.busy.is_alive() {
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
                        live.revise(|v| *v = json!({"phase":"AI 正在回复…"}));
                    }
                    Some("delta") => live.revise(|v| {
                        v["reply_en"] = data["reply_en"].clone();
                        v["reply_zh"] = data["reply_zh"].clone();
                        v["phase"] = json!("正在生成回复与学习反馈…");
                    }),
                    Some("done") => {
                        merge(turns, &data["user_turn"]);
                        merge(turns, &data["ai_turn"]);
                        live.revise(|v| *v = Value::Null);
                        return Ok(true);
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
            if !action.busy.is_alive() {
                return;
            }
            live.revise(|v| *v = Value::Null);
            action.busy.revise(|v| *v = false);
            if let Err(error) = result {
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
    abort.borrow_mut().take();
    controller.abort();
    result
}
