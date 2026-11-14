use super::*;

#[derive(Clone)]
pub struct Draft {
    pub model: Vary<String>,
    pub key: Vary<String>,
    pub region: Vary<String>,
    pub device: Option<String>,
    pub capability: Option<String>,
}
fn matches(v: &Value, gender: &str, language: &str, query: &str) -> bool {
    (gender == "all" || v["gender"] == gender)
        && (language == "all" || v["language"] == language || v["language"] == "multi")
        && (query.is_empty()
            || format!("{} {}", string(v, "name"), string(v, "id"))
                .to_lowercase()
                .contains(&query.to_lowercase()))
}
/// Account and per-device voice choices. The draft key is sent only in a
/// protected preview POST; list URLs contain model IDs, never credentials.
pub fn picker(
    kind: Vary<String>,
    provider: Vary<Option<i64>>,
    value: Cage<String>,
    inherit: bool,
    draft: Option<Draft>,
) -> impl Widget {
    widget(move |scope| {
        let rows = Resource::new(scope);
        let action = Resource::new(scope);
        let gender = scope.cage("all".to_string());
        let language = scope.cage("all".to_string());
        let epoch = scope.cage(0u32);
        let search = scope.cage(String::new());
        let open = scope.cage(false);
        let advanced = scope.cage(false);
        let visible = scope.cage(12usize);
        let source_kind = kind.clone();
        let source_provider = provider.clone();
        let source_draft = draft.clone();
        let mut previous = String::new();
        let _ = scope.effect(move || {
            let kind = source_kind.get();
            let model = source_draft
                .as_ref()
                .map(|d| d.model.get())
                .unwrap_or_default();
            let mut path = if let Some(id) = source_provider.get() {
                format!("/api/ai-providers/{id}/voices")
            } else if kind == "active" {
                "/api/ai-providers/voices".into()
            } else {
                format!("/api/ai-providers/voice-catalog/{kind}")
            };
            if let Some(device) = source_draft.as_ref().and_then(|d| d.device.as_ref()) {
                path = format!("/api/amigo/devices/{device}/ai/voices");
            }
            if !model.is_empty() {
                path.push('?');
                path.push_str(&{
                    let mut query = url::form_urlencoded::Serializer::new(String::new());
                    query.append_pair("model", &model);
                    if let Some(d) = source_draft.as_ref().filter(|d| d.device.is_some()) {
                        query
                            .append_pair("capability", d.capability.as_deref().unwrap_or("tts"))
                            .append_pair("kind", &kind);
                    }
                    query.finish()
                });
            }
            if path != previous {
                previous = path.clone();
                search.revise(String::clear);
                open.revise(|v| *v = false);
                gender.revise(|v| *v = "all".into());
                language.revise(|v| *v = "all".into());
                visible.revise(|v| *v = 12);
                rows.load(path);
            }
        });
        let _ = scope.effect(move || {
            let data = rows.data.get();
            let g = gender.get();
            let l = language.get();
            let custom = advanced.get();
            let current = value.get_untracked();
            if custom || inherit && current.is_empty() {
                return;
            }
            let candidates = items(&data)
                .into_iter()
                .filter(|v| matches(v, &g, &l, ""))
                .collect::<Vec<_>>();
            let known_openai = [
                "marin", "cedar", "alloy", "ash", "ballad", "coral", "echo", "fable", "nova",
                "onyx", "sage", "shimmer", "verse",
            ]
            .contains(&current.as_str());
            if (current.is_empty() || known_openai || g != "all" || l != "all")
                && !candidates.iter().any(|v| string(v, "id") == current)
                && let Some(first) = candidates.first()
            {
                value.revise(|v| *v = string(first, "id"));
                search.revise(String::clear);
            }
        });
        let cancel_kind = kind.clone();
        let cancel_provider = provider.clone();
        let cancel_draft = draft.clone();
        let mut initialized = false;
        let _ = scope.effect(move || {
            let _ = value.get();
            let _ = cancel_kind.get();
            let _ = cancel_provider.get();
            if let Some(d) = &cancel_draft {
                let _ = d.model.get();
                let _ = d.key.get();
                let _ = d.region.get();
            }
            if initialized {
                epoch.revise(|v| *v += 1);
                media::stop_audio();
                action.error.revise(String::clear);
                search.revise(String::clear);
            }
            initialized = true;
        });
        let preview_kind = kind.clone();
        let preview_provider = provider.clone();
        let preview_draft = draft.clone();
        view().class("voice-picker").fill((
   heading(if inherit{"设备音色"}else{"TTS 音色"}),
   when(inherit,move||view().class("actions").fill((p("留空时跟随账号的默认音色。"),action_button().label("使用账号默认").variant(sapid::ButtonVariant::Outline).on_press(move||value.revise(String::clear))))),
   view().class("voice-filters").fill((
    when(rows.data.map(|v|items(v).iter().any(|v|v["gender"]!="unknown")),move||choices(gender,&[("all","全部音色"),("female","女声"),("male","男声"),("unknown","未标注")])),
    when(rows.data.map(|v|items(v).iter().any(|v|v["language"]!="multi")),move||choices(language,&[("all","全部语言"),("en","英语"),("zh","中文"),("other","其他")])),
    when(rows.data.map(|v|!items(v).is_empty()&&items(v).iter().all(|v|v["gender"]=="unknown")),||p("官方未标注性别，可点击音色后试听选择。")),
   )),
   rows.status(),
   view().class("voice-selection").fill((
    text().class("muted").content(rows.data.map(|v|format!("{} 种可选音色，点击下方选择",items(v).len()))),
    each(scope.bond(move||vec![json!(items(&rows.data.get()).into_iter().filter(|v|matches(v,&gender.get(),&language.get(),"")).collect::<Vec<_>>())])).key(|v|v.to_string()).seed(move|filtered|{
     let options=items(&filtered);let selected=options.clone();let accepted=options.clone();
     widget(move|scope|{
      let options=options.clone();let selected=selected.clone();let accepted=accepted.clone();
      let combo=sapid::combobox("voice-choice").theme(theme::orange()).label("搜索音色").placeholder("搜索音色名称").enabled(rows.data.map(Value::is_array)).open(open).on_open_change(move|v|open.revise(|b|*b=v)).empty_message("没有符合条件的音色")
       .options(options.iter().enumerate().map(|(i,v)|sapid::select_option(format!("v{i}"),format!("{} · {}",string(v,"name"),string(v,"id")))))
       .value(value.map(move|id|selected.iter().position(|v|string(v,"id")==*id).map(|i|format!("v{i}"))))
       .search(search).on_search_change(move|s|{search.revise(|v|*v=s);visible.revise(|v|*v=12);})
       .on_value_change(move|key|{if let Some(v)=key.and_then(|s|s.strip_prefix('v').and_then(|s|s.parse::<usize>().ok())).and_then(|i|accepted.get(i)){value.revise(|s|*s=string(v,"id"));}});
      sapid::input_group().class("provider-select-field").input(combo).end(sapid::input_section().class("provider-select-chevron").fill(
       sapid::action_icon("展开音色列表").theme(theme::orange()).variant(sapid::ActionIconVariant::Subtle)
        .icon(glory::ui::graphic().drawing(sapid::Icon::ChevronDown.drawing(glory::style::Color(120,113,108,255))).style(glory::style::Style::new().width(glory::style::Length::Px(16.)).height(glory::style::Length::Px(16.))))
        .enabled(rows.data.map(Value::is_array)).on_press(move||{search.revise(String::clear);open.revise(|v|*v= !*v);})
      )).build(scope)
     })
    }),
    view().class("voice-options").fill(each(scope.bond(move||items(&rows.data.get()).into_iter().filter(|v|matches(v,&gender.get(),&language.get(),&search.get())).take(visible.get()).collect::<Vec<_>>())).key(|v|v.to_string()).seed(move|v|{
     let id=string(&v,"id");let selected=id.clone();let name=string(&v,"name").replace(" · 官方推荐"," · 推荐");
     action_button().class("voice-option").label(name).variant(sapid::ButtonVariant::Outline).pressed(value.map(move|v|*v==selected)).on_press(move||{value.revise(|v|*v=id.clone());search.revise(String::clear);open.revise(|v|*v=false);})
    })),
    when(scope.bond(move||items(&rows.data.get()).iter().filter(|v|matches(v,&gender.get(),&language.get(),&search.get())).count()>visible.get()),move||action_button().label("显示更多音色").variant(sapid::ButtonVariant::Subtle).on_press(move||visible.revise(|v|*v+=12))),
   )),
   view().class("voice-current").role(Role::Status).fill((
    text().content(rows.data.map(move|v|{let id=value.get();if id.is_empty(){"当前音色：跟随账号默认".into()}else if let Some(voice)=items(v).iter().find(|v|string(v,"id")==id){let label=match string(voice,"gender").as_str(){"female"=>" · 女声","male"=>" · 男声",_=>""};format!("当前音色：{}{label}",string(voice,"name"))}else{format!("当前音色：{id}")}})),
    text().class("muted").content("点选后可直接试听，保存后应用到账号或设备。"),
   )),
   text().class("muted").content(rows.data.map(move|v|items(v).iter().find(|v|string(v,"id")==value.get()).map(|v|string(v,"description")).unwrap_or_default())),
   view().class("voice-advanced").fill((
    action_button().label(advanced.map(|v|if *v{"收起自定义音色"}else{"高级：自定义音色 ID"}.into())).variant(sapid::ButtonVariant::Subtle).pressed(advanced).on_press(move||advanced.revise(|v|*v= !*v)),
    when(advanced,move||view().fill((p("仅使用平台的自定义音色时需要填写 ID；普通音色直接从上方选择。"),field("自定义音色 ID",value)))),
   )),
   view().class("actions").fill((
    action_button().label("试听所选音色").loading(action.busy).enabled(scope.bond(move||!value.get().is_empty()&&(advanced.get()||(gender.get()=="all"&&language.get()=="all")||items(&rows.data.get()).iter().any(|v|string(v,"id")==value.get()&&matches(v,&gender.get(),&language.get(),""))))).on_press(move||{
     let source=preview_kind.get();let mut body=json!({"provider_id":preview_provider.get(),"voice_id":value.get()});
     if source!="active"{body["provider_type"]=json!(source);}
     if let Some(d)=&preview_draft{
      if d.device.is_none()&&preview_provider.get().is_none()&&d.key.get().trim().is_empty(){action.error.revise(|v|*v="填写 API Key 后即可试听，无需先保存配置。".into());return;}
      if d.model.get().trim().is_empty(){action.error.revise(|v|*v="请先选择语音合成模型。".into());return;}
      body["tts_model"]=json!(d.model.get());body["api_key"]=json!(d.key.get());body["region"]=json!(d.region.get());
      if let Some(device)=&d.device{body["device_id"]=json!(device);body["capability"]=json!(d.capability);}
     }
     epoch.revise(|e|*e+=1);let current=epoch.get();
     action.run("POST","/api/ai-providers/voice-preview".into(),Some(body),move|output|{if epoch.get()==current{media::play_data(output,action.error);}});
    }),
    action_button().label("停止试听").variant(sapid::ButtonVariant::Outline).on_press(move||{epoch.revise(|e|*e+=1);media::stop_audio();}),
   )),
   p("试听为 AI 生成的简短语音，费用由提供方收取。"),action.status(),
  )).build(scope)
    })
}
