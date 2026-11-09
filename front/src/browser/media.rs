use super::*;
#[wasm_bindgen(module = "/src/browser/platform.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = copyText)]
    async fn clipboard_text(text: &str) -> std::result::Result<(), JsValue>;
    #[wasm_bindgen(js_name = followChat)]
    fn watch_chat() -> js_sys::Function;
    #[wasm_bindgen(js_name = cancelRecording)]
    fn cancel_recording(owner: u32);
    #[wasm_bindgen(js_name = downloadText)]
    fn download_text(filename: &str, text: &str);
    #[wasm_bindgen(catch, js_name = avatarData)]
    async fn avatar_data(token: &str) -> std::result::Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, js_name = startRecording)]
    async fn start_recording() -> std::result::Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, js_name = stopRecording)]
    async fn stop_recording() -> std::result::Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, js_name = playAudio)]
    async fn play_audio(path: &str, token: &str) -> std::result::Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, js_name = speakText)]
    fn speak_text(text: &str) -> std::result::Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = uploadAvatar)]
    async fn upload_avatar(token: &str) -> std::result::Result<JsValue, JsValue>;
}
pub fn copy_text(value: String, notice: Cage<String>, error: Cage<String>) {
    spawn(async move {
        match clipboard_text(&value).await {
            Ok(()) if notice.is_alive() => notice.revise(|s| *s = "已复制译文".into()),
            Err(e) if error.is_alive() => error.revise(|s| *s = message(e)),
            _ => {}
        }
    });
}
pub fn follow_chat(scope: &Scope) {
    let stop = watch_chat();
    let _ = scope.on_cleanup(move || {
        let _ = stop.call0(&JsValue::NULL);
    });
}
pub fn export_chat(id: i64, action: Resource) {
    if action.busy.get() {
        return;
    }
    action.busy.revise(|b| *b = true);
    spawn(async move {
        let mut after = 0;
        let mut transcript = String::new();
        loop {
            let result = request(
                "GET",
                &format!("/api/learn/chats/{id}/turns?limit=500&after_id={after}"),
                None,
            )
            .await;
            if !action.busy.is_alive() {
                return;
            }
            let value = match result {
                Ok(v) => v,
                Err(e) => {
                    action.busy.revise(|b| *b = false);
                    action.error.revise(|v| *v = e);
                    return;
                }
            };
            for turn in items(&value) {
                transcript.push_str(&format!(
                    "{}\n{}\n{}\n\n",
                    string(&turn, "speaker"),
                    string(&turn, "content_en"),
                    string(&turn, "content_zh")
                ));
            }
            let next = value["last_id"].as_i64().unwrap_or(after);
            if value["has_next"] != true || next <= after {
                break;
            }
            after = next;
        }
        action.busy.revise(|b| *b = false);
        download_text(&format!("colang-chat-{id}.txt"), &transcript);
    });
}
fn message(e: JsValue) -> String {
    js_sys::Reflect::get(&e, &"message".into())
        .ok()
        .and_then(|s| s.as_string())
        .unwrap_or_else(|| js_error(e))
}
pub fn speak(text: &str, error: Cage<String>) {
    if let Err(e) = speak_text(text) {
        error.revise(|v| *v = message(e));
    }
}
pub fn play(path: String, error: Cage<String>) {
    spawn(async move {
        if let Err(e) = play_audio(&path, &token().unwrap_or_default()).await
            && error.is_alive()
        {
            error.revise(|v| *v = message(e));
        }
    });
}
pub fn recorder(on_complete: impl Fn(String) + 'static, resource: Resource) -> impl Widget {
    let error = resource.error;
    let complete = std::rc::Rc::new(on_complete);
    widget(move |scope| {
        let recording = scope.cage(false);
        let busy = scope.cage(false);
        let owner = std::rc::Rc::new(std::cell::Cell::new(None::<u32>));
        let cleanup_owner = owner.clone();
        scope
            .on_cleanup(move || {
                if let Some(id) = cleanup_owner.get() {
                    cancel_recording(id);
                    if resource.busy.is_alive() {
                        resource.busy.revise(|b| *b = false);
                    }
                }
            })
            .map_err(|_| glory::ui::Error::Gone)?;
        let complete = complete.clone();
        view()
            .class("recorder")
            .fill((
                action_button()
                    .label(recording.map(|v| {
                        if *v {
                            "停止录音并提交"
                        } else {
                            "开始录音"
                        }
                        .to_string()
                    }))
                    .loading(busy)
                    .enabled(busy.map(move |v| !*v && (recording.get() || !resource.busy.get())))
                    .on_press(move || {
                        if busy.get() || (!recording.get() && resource.busy.get()) {
                            return;
                        }
                        busy.revise(|b| *b = true);
                        error.revise(String::clear);
                        let complete = complete.clone();
                        let owner = owner.clone();
                        spawn(async move {
                            if recording.get() {
                                let result = stop_recording().await;
                                if !recording.is_alive() {
                                    return;
                                }
                                recording.revise(|r| *r = false);
                                busy.revise(|b| *b = false);
                                resource.busy.revise(|b| *b = false);
                                owner.set(None);
                                match result {
                                    Ok(v) => complete(v.as_string().unwrap_or_default()),
                                    Err(e) => error.revise(|v| *v = message(e)),
                                }
                            } else {
                                resource.data.revise(|v| *v = Value::Null);
                                resource.busy.revise(|b| *b = true);
                                let result = start_recording().await;
                                if !recording.is_alive() {
                                    if let Ok(value) = result {
                                        cancel_recording(value.as_f64().unwrap_or(0.) as u32);
                                    }
                                    return;
                                }
                                busy.revise(|b| *b = false);
                                match result {
                                    Ok(value) => {
                                        owner.set(Some(value.as_f64().unwrap_or(0.) as u32));
                                        recording.revise(|r| *r = true);
                                    }
                                    Err(e) => {
                                        resource.busy.revise(|b| *b = false);
                                        error.revise(|v| *v = message(e));
                                    }
                                }
                            }
                        });
                    }),
                text()
                    .class("muted")
                    .content("录音最长 60 秒。请允许麦克风权限，录完后点击停止提交。"),
            ))
            .build(scope)
    })
}
pub fn avatar_upload(action: Resource, session: Cage<Value>) {
    if action.busy.get() {
        return;
    }
    action.busy.revise(|b| *b = true);
    spawn(async move {
        let result = upload_avatar(&token().unwrap_or_default()).await;
        if !action.busy.is_alive() {
            return;
        }
        action.busy.revise(|b| *b = false);
        match result {
            Ok(raw) => {
                if let Some(raw) = raw.as_string()
                    && let Ok(mut user) = serde_json::from_str::<Value>(&raw)
                {
                    user["is_admin"] = session.get()["is_admin"].clone();
                    session.revise(|s| *s = user);
                }
            }
            Err(e) => action.error.revise(|v| *v = message(e)),
        }
    });
}

pub fn avatar(session: Cage<Value>) -> impl Widget {
    widget(move |scope| {
        let source = scope.cage("/colang-logo.svg".to_string());
        let generation = scope.cage(0u32);
        scope.effect(move || {
            let has_avatar = !session.get()["avatar"].is_null();
            generation.revise(|v| *v += 1);
            let current = generation.get_untracked();
            if !has_avatar {
                source.revise(|s| *s = "/colang-logo.svg".into());
                return;
            }
            spawn(async move {
                if let Ok(value) = avatar_data(&token().unwrap_or_default()).await
                    && source.is_alive()
                    && generation.get() == current
                {
                    source.revise(|s| {
                        *s = value
                            .as_string()
                            .unwrap_or_else(|| "/colang-logo.svg".into())
                    });
                }
            });
        });
        image()
            .source(source)
            .alt("我的头像")
            .class("profile-avatar")
            .build(scope)
    })
}
