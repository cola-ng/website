//! Native website UI for hardware robots. No separate frontend or account system.
use super::*;

fn poll(scope: &Scope, resource: Resource, path: String) {
    resource.load(path.clone());
    let interval = gloo_timers::callback::Interval::new(5000, move || {
        if resource.data.is_alive() && !resource.busy.get_untracked() {
            resource.load(path.clone());
        }
    });
    let _ = scope.on_cleanup(move || drop(interval));
}
fn stamp(raw: &str) -> String {
    if raw.is_empty() {
        return "尚未连接".into();
    }
    js_sys::Date::new(&JsValue::from_str(raw))
        .to_locale_string("zh-CN", &JsValue::UNDEFINED)
        .into()
}
fn sync(value: &Value) -> &'static str {
    if value["configuration"].is_null() {
        "使用设备现有设置"
    } else if value["configuration"]["revision"] == value["applied_revision"] {
        "设备已应用设置"
    } else if value["online"] == true {
        "正在同步设置"
    } else {
        "等待设备上线同步"
    }
}
fn defaults() -> Value {
    json!({"revision":0,"volume":70,"continuous":true,"voice_interrupt":true,"playback_wake":true})
}

fn call_stage(value: &str) -> &'static str {
    match value {
        "realtime" => "实时语音对话",
        "asr" => "语音识别",
        "route" => "快速路由",
        "plan" => "对话规划",
        "chat" => "大模型回复",
        "tts" => "语音合成",
        "vision" => "视觉理解",
        _ => "模型调用",
    }
}
fn call_state(value: &str) -> &'static str {
    match value {
        "success" => "成功",
        "error" => "失败",
        "cancelled" => "已中断",
        _ => "进行中",
    }
}
fn diagnostic_field(error: &str, name: &str) -> String {
    let value = error.split_once(name).map(|(_, v)| v).unwrap_or_default();
    value
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || "-._:/[]".contains(*c))
        .take(128)
        .collect()
}
fn call_diagnostics(call: Value, all: &Value) -> Value {
    let error = string(&call, "error");
    let timeline = items(all)
        .into_iter()
        .filter(|c| c["turn_id"] == call["turn_id"])
        .rev()
        .collect::<Vec<_>>();
    let hint = if error.contains("未识别到有效语音") || error.contains("no speech") {
        "识别没有得到有效文字，该轮停在语音识别阶段，尚未进入回复生成。"
    } else if error.contains("HTTP 401") || error.contains("invalid_api_key") {
        "上游服务拒绝了身份验证，请检查该平台的密钥。"
    } else if error.contains("HTTP 429")
        || error.contains("rate_limit")
        || error.contains("insufficient_quota")
    {
        "上游返回限流或额度错误，请核对错误代码及平台额度。"
    } else if error.contains("model_not_found") {
        "上游未接受所选模型，请核对模型 ID 和账号权限。"
    } else if call["state"] == "success" {
        "本阶段调用已完成。"
    } else if call["state"] == "cancelled" {
        "本阶段已取消；可结合本轮其他阶段判断是打断、超时还是连接中断。"
    } else {
        "请结合错误日志与本轮调用链定位失败阶段。"
    };
    json!({"call":call,"timeline":timeline,"hint":hint,"upstream":{
        "http_status":diagnostic_field(&error,"HTTP "),"error_code":diagnostic_field(&error,"code="),
        "parameter":diagnostic_field(&error,"param="),"request_id":diagnostic_field(&error,"request_id="),
    }})
}

fn call_detail_view(detail: Value) -> impl Widget {
    widget(move |scope| {
        let call = &detail["call"];
        let upstream = &detail["upstream"];
        let notice = scope.cage(String::new());
        let error = scope.cage(String::new());
        let json = serde_json::to_string_pretty(&detail).unwrap_or_default();
        let copy = json.clone();
        let mut fields = vec![
            ("调用 ID", string(call, "id")),
            ("轮次", string(call, "turn_id")),
            ("阶段", call_stage(&string(call, "stage")).into()),
            ("状态", call_state(&string(call, "state")).into()),
            ("提供方", string(call, "provider")),
            ("模型", string(call, "model")),
            ("开始时间", stamp(&string(call, "started_at"))),
            ("耗时", format!("{} ms", call["duration_ms"])),
        ];
        for (label, key) in [
            ("HTTP 状态", "http_status"),
            ("错误代码", "error_code"),
            ("错误参数", "parameter"),
            ("请求编号", "request_id"),
        ] {
            let value = string(upstream, key);
            if !value.is_empty() {
                fields.push((label, value));
            }
        }
        view()
            .class("amigo-call-detail")
            .fill((
                view().class("amigo-call-details-meta").fill(nodes(
                    fields
                        .into_iter()
                        .map(|(label, value)| {
                            boxed(view().fill((
                                text().class("muted").content(label),
                                text().content(value),
                            )))
                        })
                        .collect(),
                )),
                p(string(&detail, "hint")),
                heading("错误日志"),
                text()
                    .class(if call["state"] == "error" {
                        "amigo-diagnostic-log error"
                    } else {
                        "amigo-diagnostic-log"
                    })
                    .content(if call["error"].is_null() {
                        "本阶段没有记录错误。".into()
                    } else {
                        string(call, "error")
                    }),
                heading("本轮调用链"),
                view().class("amigo-call-timeline").fill(nodes(
                    items(&detail["timeline"])
                        .into_iter()
                        .map(|entry| {
                            boxed(
                                view().class("amigo-call-step").fill((
                                    text().content(format!(
                                        "{} · {} · {} ms",
                                        call_stage(&string(&entry, "stage")),
                                        call_state(&string(&entry, "state")),
                                        entry["duration_ms"]
                                    )),
                                    text().class("muted").content(format!(
                                        "{} / {}",
                                        string(&entry, "provider"),
                                        string(&entry, "model")
                                    )),
                                    text()
                                        .class(if entry["state"] == "error" {
                                            "error"
                                        } else {
                                            "muted"
                                        })
                                        .content(string(&entry, "error")),
                                )),
                            )
                        })
                        .collect(),
                )),
                heading("诊断记录"),
                text().class("amigo-diagnostic-log").content(json),
                p("详情包含已记录的诊断数据；旧记录缺少的上游字段不会补写。"),
                action_button()
                    .label("复制诊断信息")
                    .variant(sapid::ButtonVariant::Outline)
                    .on_press(move || media::copy_text(copy.clone(), notice, error)),
                text().class("success").content(notice),
                text().class("error").content(error),
            ))
            .build(scope)
    })
}

pub fn console() -> impl Widget {
    widget(move |scope| {
        let devices = Resource::new(scope);
        let runtime = Resource::new(scope);
        let action = Resource::new(scope);
        let adding = scope.cage(false);
        let name = scope.cage("我的 Amigo".to_string());
        let query = scope.cage(String::new());
        let code = scope.cage(String::new());
        poll(scope, devices, "/api/amigo/devices".into());
        poll(scope, runtime, "/api/amigo/runtime".into());
        view().class("amigo-console").fill((
            view().class("section-heading").fill((
                view().fill((text().class("eyebrow").content("COLANG HARDWARE · AMIGO"), title("智能学伴"), p("连接你的 Amigo，定制每一台机器人的角色和陪伴方式。"))),
                action_button().label("＋ 添加设备").on_press(move || {code.revise(String::clear);action.error.revise(String::clear);adding.revise(|v|*v=true);}),
            )),
            view().class("amigo-toolbar").fill((
                text().content(devices.data.map(|v|format!("我的设备 · {} 台 · {} 台在线",items(v).len(),items(v).iter().filter(|d|d["online"]==true).count()))),
                field("搜索设备", query),
            )),
            devices.status(),
            view().class("amigo-grid").fill(each(scope.bond(move || {
                let q=query.get().to_lowercase();
                items(&devices.data.get()).into_iter().filter(|d|string(d,"name").to_lowercase().contains(&q)||string(d,"id").contains(&q)).collect::<Vec<_>>()
            })).key(|v|v.to_string()).seed(move |d| {
                let id=string(&d,"id");let path=format!("/amigo/devices/{id}");
                let board=match string(&d,"board").as_str(){"lckfb-szpi-esp32s3"=>"立创实战派 ESP32-S3".into(),""=>"等待首次连接".into(),v=>v.to_string()};
                view().class("panel amigo-device").fill((
                    view().class("amigo-device-title").fill((text().class("amigo-avatar").content("☷"),view().fill((heading(string(&d,"name")),text().class(if d["online"]==true{"amigo-online"}else{"muted"}).content(if d["online"]==true{"● 在线"}else{"● 离线"}))))),
                    view().class("amigo-meta").fill((view().fill((text().class("muted").content("开发板"),p(board))),view().fill((text().class("muted").content("最近连接"),p(stamp(&string(&d,"last_seen_at"))))))),
                    view().fill((text().class("muted").content(sync(&d)),text().class("muted").content(if d["conversation_mode"]=="realtime"{"实时语音"}else{"标准对话"}))),
                    view().class("actions amigo-card-actions").fill((nav("配置",path.clone()),nav("记录",format!("{path}?tab=history")),nav("连接",format!("{path}?tab=connection")))),
                ))
            })),
            when(devices.data.map(|v|v.is_array()&&items(v).is_empty()),move ||view().class("panel amigo-empty").fill((heading("你的第一位 Amigo，等你连接"),p("让机器人联网，再输入屏幕上的六位验证码。")))),
            view().class("panel amigo-service").fill((text().content(runtime.data.map(|v|if v["ready"]==true{format!("账号全局 AI 已就绪 · {}",string(v,"model"))}else if v["realtime_ready"]==true{"账号全局实时语音可用".into()}else{"账号全局 AI 尚未配置；也可在每台设备中单独设置。".into()})),nav("账号与 AI 服务设置 →","/me#settings"))),
            sapid::dialog("amigo-add").theme(theme::orange()).show_trigger(false).open(adding).on_open_change(move |open|{if !action.busy.get(){adding.revise(|v|*v=open);}}).title("添加 Amigo 设备").close_label("关闭添加设备").content(move ||view().class("amigo-pair-form").fill((
                p("让 Amigo 连接 Wi-Fi，输入设备屏幕上的六位验证码，即可绑定到当前账号。"),
                view().class("field").fill((text().content("六位验证码"),text_control().name("六位验证码").value(code).on_value_change(move |next|{code.revise(|v|*v=next);action.error.revise(String::clear);}))),
                field("设备名称",name),
                text().class("muted").content("验证码 5 分钟内有效，过期后设备会自动显示新码。已经绑定的设备，可在设备 Info → Pair → Confirm 后重新绑定，Wi-Fi 会保留。"),
                action_button().label("绑定设备").loading(action.busy).on_press(move ||{
                    let digits=code.get().chars().filter(|c|!c.is_whitespace()&&*c!='-').collect::<String>();
                    if digits.len()!=6||!digits.bytes().all(|c|c.is_ascii_digit()){action.error.revise(|v|*v="请输入设备屏幕上的六位数字".into());return;}
                    if name.get().trim().is_empty(){action.error.revise(|v|*v="请输入设备名称".into());return;}
                    action.run("POST","/api/amigo/pairing/claim".into(),Some(json!({"code":digits,"name":name.get().trim()})),move |v|{
                        adding.revise(|v|*v=false);devices.load("/api/amigo/devices");go(&format!("/amigo/devices/{}",string(&v,"id")));
                    });
                }),
                action.status(),
                text().class("muted").content("绑定后可设置机器人的角色和参数。语音对话使用当前账号的 AI 服务，可稍后在账号设置中配置。"),
            ))),
        )).build(scope)
    })
}

fn text_setting(
    scope: &Scope,
    label: &str,
    draft: Cage<Value>,
    section: &'static str,
    key: &'static str,
    numeric: bool,
) -> Box<dyn Widget> {
    let value = scope.cage(string(&draft.get()[section], key));
    boxed(
        view().class("field").fill((
            text().content(label.to_string()),
            text_control()
                .name(label.to_string())
                .value(value)
                .on_value_change(move |next| {
                    value.revise(|v| *v = next.clone());
                    draft.revise(|v| {
                        v[section][key] = if numeric {
                            next.parse::<i64>().map_or(Value::Null, |n| json!(n))
                        } else {
                            json!(next)
                        }
                    });
                }),
        )),
    )
}
fn toggle(
    label: &str,
    detail: &str,
    draft: Cage<Value>,
    section: &'static str,
    key: &'static str,
) -> Box<dyn Widget> {
    boxed(
        view().class("amigo-toggle").fill((
            checkbox_control()
                .label(label.to_string())
                .checked(draft.map(move |v| v[section][key] == true))
                .on_checked_change(move |checked| {
                    draft.revise(|v| v[section][key] = json!(checked))
                }),
            text().class("muted").content(detail.to_string()),
        )),
    )
}

fn time_setting(
    scope: &Scope,
    label: &str,
    draft: Cage<Value>,
    key: &'static str,
) -> Box<dyn Widget> {
    let n = draft.get()["settings"][key].as_u64().unwrap_or_default();
    let value = scope.cage(format!("{:02}:{:02}", n / 60, n % 60));
    boxed(
        view().class("field").fill((
            text().content(label.to_string()),
            text_control()
                .name(label.to_string())
                .value(value)
                .on_value_change(move |next| {
                    value.revise(|v| *v = next.clone());
                    let parts = next
                        .split(':')
                        .map(str::parse::<u32>)
                        .collect::<std::result::Result<Vec<_>, _>>();
                    let minutes = parts
                        .ok()
                        .filter(|v| v.len() == 2 && v[0] < 24 && v[1] < 60)
                        .map(|v| v[0] * 60 + v[1]);
                    draft
                        .revise(|v| v["settings"][key] = minutes.map_or(Value::Null, |n| json!(n)));
                }),
        )),
    )
}

pub fn device(id: String) -> impl Widget {
    widget(move |scope| {
        let id = id.clone();
        let profile = Resource::new(scope);
        let history = Resource::new(scope);
        let calls = Resource::new(scope);
        let devices = Resource::new(scope);
        let connection = Resource::new(scope);
        let path = format!("/api/amigo/devices/{id}");
        profile.load(path.clone());
        history.load(format!("{path}/history"));
        poll(scope, calls, format!("{path}/calls"));
        connection.load("/api/amigo/connection");
        poll(scope, devices, "/api/amigo/devices".into());
        let key = id.clone();
        let editor_id = scope.cage(id.clone());
        view().class("amigo-editor").fill((nav("← 我的设备","/amigo"),profile.status(),each(scope.bond(move||{let p=profile.data.get();let d=items(&devices.data.get()).into_iter().find(|d|string(d,"id")==editor_id.get());if p.is_null()||d.is_none(){vec![]}else{vec![json!({"profile":p,"device":d})]}})).key(|_|"editor").seed(move|profile_value|{
            let id=id.clone();let path=path.clone();let connection=connection;
            widget(move |scope| {
                let action=Resource::new(scope);let notice=scope.cage(String::new());let confirm=scope.cage(false);
                let ai_device=scope.cage(id.clone());let call_open=scope.cage(false);let selected_call=scope.cage(Value::Null);
                let d=profile_value["device"].clone();
                let profile_value=profile_value["profile"].clone();
                let name=scope.cage(string(&d,"name"));
                let agent_id=scope.cage(string(&profile_value,"active_agent_id"));
                let companion=scope.cage(items(&profile_value["agents"]).into_iter().find(|a|a["id"]==profile_value["active_agent_id"]).unwrap_or_default());
                let partner=scope.cage(string(&companion.get(),"name"));let prompt=scope.cage(string(&companion.get(),"role_prompt"));let voice=scope.cage(string(&companion.get()["voice"],"voice_id"));
                let level=scope.cage(string(&profile_value["settings"],"english_level"));
                let mode=scope.cage(if profile_value["settings"]["conversation_mode"]=="realtime"{"realtime".to_owned()}else{"standard".to_owned()});
                let realtime_model=scope.cage(profile_value["settings"]["realtime_model"].as_str().unwrap_or("gpt-realtime-2.1-mini").to_owned());
                let realtime_voice=scope.cage(profile_value["settings"]["realtime_voice"].as_str().unwrap_or("marin").to_owned());
                let realtime_provider=scope.cage(profile_value["settings"]["realtime_provider_id"].as_i64());
                let config=if d["configuration"].is_null(){defaults()}else{d["configuration"].clone()};
                let draft=scope.cage(json!({"settings":profile_value["settings"],"configuration":config}));
                let tab=scope.cage(param("tab").filter(|t|["history","connection","calls"].contains(&t.as_str())).unwrap_or("settings".into()));
                let save_path=scope.cage(path.clone());let delete_path=scope.cage(path.clone());let chosen_id=id.clone();let state_id=id.clone();
                let role_buttons=items(&profile_value["agents"]).into_iter().map(|agent|{let selected=string(&agent,"id");let matched=selected.clone();boxed(action_button().label(string(&agent,"name")).pressed(agent_id.map(move|v|v==&matched)).on_press(move||{agent_id.revise(|v|*v=selected.clone());partner.revise(|v|*v=string(&agent,"name"));prompt.revise(|v|*v=string(&agent,"role_prompt"));voice.revise(|v|*v=string(&agent["voice"],"voice_id"));companion.revise(|v|*v=agent.clone());}))}).collect();
                let mut fields=vec![text_setting(scope,"音量（0–100）",draft,"configuration","volume",true)];
                for (label,detail,key) in [("连续对话","回复后继续聆听，无需每次按 Talk。","continuous"),("说话打断","播音时说话可以打断；嘈杂环境可关闭。","voice_interrupt"),("播音时响应唤醒词","当前立创实战派本地唤醒词为 Hi ESP。","playback_wake")] {fields.push(toggle(label,detail,draft,"configuration",key));}
                fields.push(boxed(heading("学习与陪伴")));
                fields.push(boxed(choices(level,&[("A1","A1"),("A2","A2"),("B1","B1"),("B2","B2"),("C1","C1"),("C2","C2")])));
                for (label,detail,key) in [("主动陪伴","结合学习目标和情景主动开场。","proactive_enabled"),("摄像头观察","允许 Photo 拍照和计划中的画面观察。","camera_enabled"),("环境语音观察","空闲时采集短语音理解情景。","ambient_audio_enabled")] {fields.push(toggle(label,detail,draft,"settings",key));}
                for (label,key) in [("观察间隔（秒）","observation_interval_secs"),("主动开场间隔（秒）","proactive_cooldown_secs"),("空闲等待（秒）","idle_secs"),("每日主动开场上限（0–24）","daily_outreach_limit"),("时区偏移（分钟，UTC+8 为 480）","utc_offset_minutes")] {fields.push(text_setting(scope,label,draft,"settings",key,true));}
                fields.push(time_setting(scope,"免打扰开始",draft,"quiet_start_minute"));
                fields.push(time_setting(scope,"免打扰结束",draft,"quiet_end_minute"));
                fields.push(boxed(p("开始与结束时间相同表示关闭免打扰。")));
                view().class("panel").fill((
                    text().kind(TextKind::Heading(1)).content(name),text().class("muted").content(devices.data.map(move|v|items(v).iter().find(|d|string(d,"id")==state_id).map(|d|sync(d).to_string()).unwrap_or("设备已移除".into()))),
                    choices(tab,&[("settings","设备配置"),("history","对话记录"),("calls","调用日志"),("connection","连接信息")]),
                    view().class_fn(move||if tab.get()=="settings" {"amigo-role-choices choices"}else{"amigo-values-hidden"}.into()).fill(nodes(role_buttons)),
                    when(tab.map(|t|t=="settings"),move||field("伙伴称呼",partner)),
                    view().class_fn(move||if tab.get()=="settings" {"form-fields amigo-settings"}else{"form-fields amigo-settings amigo-values-hidden"}.into()).fill((
                        field("设备名称",name),textarea("角色与说话方式",prompt),
                        heading("语音对话模式"),choices(mode,&[("standard","标准对话"),("realtime","实时语音")]),
                        when(mode.map(|m|m=="standard"),||p("使用下方选择的语音识别、文本对话与语音合成服务，保留学习规划。")),
                        when(mode.map(|m|m=="realtime"),||p("直接聆听并流式回复，使用下方的实时语音服务配置。")),
                        super::device_ai::panel(ai_device.get()),
                        text().class("muted").content("角色、音色及陪伴参数只应用于这台机器人。"),
                        // Keep the editor scope alive across detail tabs.
                    )),
                    view().class_fn(move||if tab.get()=="settings" {"amigo-settings-values"} else {"amigo-settings-values amigo-values-hidden"}.into()).fill(nodes(fields)),
                    when(tab.map(|t|t=="settings"),move ||action_button().label("保存并同步到设备").loading(action.busy).on_press(move||{
                        let mut input=draft.get();
                        let volume=input["configuration"]["volume"].as_i64();
                        if name.get().trim().is_empty()||partner.get().trim().is_empty()||!volume.is_some_and(|n|(0..=100).contains(&n)){action.error.revise(|e|*e="请填写设备名称、伙伴称呼及 0–100 的音量".into());return;}
                        input["name"]=json!(name.get().trim());input["agent_id"]=json!(agent_id.get());input["settings"]["english_level"]=json!(level.get());
                        input["settings"]["conversation_mode"]=json!(mode.get());input["settings"]["realtime_model"]=json!(realtime_model.get());input["settings"]["realtime_voice"]=json!(realtime_voice.get());input["settings"]["realtime_provider_id"]=json!(realtime_provider.get());
                        let mut agent=companion.get();agent["name"]=json!(partner.get().trim());agent["role_prompt"]=json!(prompt.get());agent["voice"]["voice_id"]=if voice.get().trim().is_empty(){Value::Null}else{json!(voice.get().trim())};input["companion"]=agent;
                        action.run("PUT",save_path.get(),Some(input),move|_|{notice.revise(|n|*n="设置已保存；在线设备约 5 秒内同步，离线设备重连后同步。".into());devices.load("/api/amigo/devices");});
                    })),
                    view().fill((
                    when(tab.map(|t|t=="history"),move ||view().class("amigo-history").fill((history.status(),each(history.data.map(items)).key(|e|string(e,"id")).seed(|event|view().class("amigo-event").fill((text().class("muted").content(stamp(&string(&event,"created_at"))),p(format!("我：{}",string(&event["content"],"transcript"))),p(format!("Amigo：{}",string(&event["content"]["analysis"],"reply_en"))),p(string(&event["content"]["context"],"text"))))),empty(history,"这台设备还没有记录。"),p("仅显示这台设备最近 50 条记录。")))),
                    when(tab.map(|t|t=="calls"),move||view().class("amigo-call-log").fill((
                        p("本设备最近 100 条调用。中断记录可能来自说话打断、超时或重新连接。"),calls.status(),
                        each(calls.data.map(items)).key(|v|v.to_string()).seed(move|call|{
                            let stage=call_stage(&string(&call,"stage"));let state=call_state(&string(&call,"state"));let detail=call.clone();
                            view().class("amigo-call-row").fill((
                                view().class("amigo-call-heading").fill((heading(format!("{stage} · {state}")),action_button().label("查看详情").variant(sapid::ButtonVariant::Outline).on_press(move||{selected_call.revise(|v|*v=call_diagnostics(detail.clone(),&calls.data.get()));call_open.revise(|v|*v=true);}))),
                                p(format!("{} · {} ms · 轮次 {}",stamp(&string(&call,"started_at")),call["duration_ms"],string(&call,"turn_id"))),
                                text().class("muted").content(format!("{} / {}",string(&call,"provider"),string(&call,"model"))),
                                text().class(if call["state"]=="error"{"error"}else{"muted"}).content(string(&call,"error")),
                            ))
                        }),empty(calls,"还没有调用日志。发起一次对话后，这里会自动更新。"),
                    ))),
                    sapid::dialog("amigo-call-details").theme(theme::orange()).show_trigger(false).open(call_open).on_open_change(move|v|call_open.revise(|b|*b=v)).title("调用详情").close_label("关闭调用详情").content(move||view().class("amigo-call-dialog").fill(each(selected_call.map(|v|if v.is_null(){vec![]}else{vec![v.clone()]})).key(|v|v.to_string()).seed(call_detail_view))),
                    )),
                    when(tab.map(|t|t=="connection"),move ||view().class("form-fields").fill((p(format!("设备 ID：{chosen_id}")),text().class("amigo-code").content(connection.data.map(|v|string(v,"gateway_url"))),p("此设备通过验证码绑定。更换账号请在设备上选择 Info → Pair → Confirm，再到新账号输入验证码。移除设备将立即撤销连接权限。"),danger_button().label("移除设备").on_press(move||confirm.revise(|v|*v=true))))),
                    view().fill((when(notice.map(|n|!n.is_empty()),move||text().class("success").content(notice)),action.status())),
                    sapid::dialog("amigo-revoke").theme(theme::orange()).show_trigger(false).open(confirm).on_open_change(move|v|confirm.revise(|b|*b=v)).title("移除设备").close_label("取消移除").content(move||view().class("dialog-form").fill((p(format!("确认移除“{}”？设备的连接权限会立即失效。",name.get())),danger_button().label("确认移除").loading(action.busy).on_press(move||action.run("DELETE",delete_path.get(),None,move |_|go("/amigo"))),action.status()))),
                )).build(scope)
            })
        }),text().class("muted").content(format!("Amigo · {key}")),
        )).build(scope)
    })
}
