use super::*;

fn label(cap: &str) -> &'static str {
    match cap {
        "chat" => "文本对话",
        "asr" => "语音识别",
        "tts" => "语音合成",
        _ => "实时语音",
    }
}
fn global_summary(cap: &str, providers: &Value, fallback: &Value) -> String {
    let rows = items(providers);
    let row = if cap == "realtime" {
        rows.iter()
            .find(|p| {
                p["provider_type"] == "openai"
                    && (p["tts_active"] == true || p["is_active"] == true)
            })
            .or_else(|| rows.iter().find(|p| p["provider_type"] == "openai"))
    } else {
        let flag = match cap {
            "chat" => "is_active",
            "asr" => "asr_active",
            _ => "tts_active",
        };
        rows.iter().find(|p| p[flag] == true)
    };
    if let Some(row) = row {
        let field = match cap {
            "chat" => "chat_model",
            "asr" => "asr_model",
            _ => "tts_model",
        };
        let model = string(&row["settings"], field);
        let model = if cap == "realtime" && !model.starts_with("gpt-realtime") {
            "gpt-realtime-2.1-mini".into()
        } else {
            model
        };
        format!("全局：{} / {model}", string(row, "provider_type"))
    } else if fallback["inherited"] == true {
        if fallback["error"].is_string() {
            string(fallback, "error")
        } else {
            format!(
                "全局：{} / {}",
                string(fallback, "provider_type"),
                string(fallback, "model")
            )
        }
    } else {
        "账号全局尚未配置此项服务。".into()
    }
}
fn select(
    id: String,
    label: &'static str,
    options: Vec<(String, String)>,
    value: Cage<String>,
) -> impl Widget {
    widget(move |scope| {
        let open = scope.cage(false);
        let search = scope.cage(String::new());
        let accepted = options.clone();
        view()
            .class("field")
            .fill((
                text().content(label),
                sapid::input_group()
                    .class("provider-select-field")
                    .input(
                        sapid::combobox(id.clone())
                            .theme(theme::orange())
                            .label(label)
                            .placeholder("请选择")
                            .options(
                                options.iter().map(|(id, name)| {
                                    sapid::select_option(id.clone(), name.clone())
                                }),
                            )
                            .value(value.map(|v| if v.is_empty() { None } else { Some(v.clone()) }))
                            .search(search)
                            .open(open)
                            .on_search_change(move |v| search.revise(|s| *s = v))
                            .on_open_change(move |v| open.revise(|b| *b = v))
                            .on_value_change(move |id| {
                                if let Some(id) =
                                    id.filter(|id| accepted.iter().any(|(v, _)| v == id))
                                {
                                    value.revise(|v| *v = id);
                                }
                            }),
                    )
                    .end(
                        sapid::input_section()
                            .class("provider-select-chevron")
                            .fill(
                                sapid::action_icon(format!("展开{label}"))
                                    .theme(theme::orange())
                                    .variant(sapid::ActionIconVariant::Subtle)
                                    .icon(
                                        glory::ui::graphic()
                                            .drawing(
                                                sapid::Icon::ChevronDown.drawing(
                                                    glory::style::Color(120, 113, 108, 255),
                                                ),
                                            )
                                            .style(
                                                glory::style::Style::new()
                                                    .width(glory::style::Length::Px(16.))
                                                    .height(glory::style::Length::Px(16.)),
                                            ),
                                    )
                                    .on_press(move || {
                                        search.revise(String::clear);
                                        open.revise(|v| *v = !*v);
                                    }),
                            ),
                    ),
            ))
            .build(scope)
    })
}
fn capability(
    device: String,
    cap: &'static str,
    binding: Value,
    effective: Value,
    draft: Cage<Value>,
    providers: Resource,
    catalog: Resource,
) -> impl Widget {
    widget(move |scope| {
        let source = scope.cage(
            if binding.is_null() {
                "inherit"
            } else if binding["provider_id"].is_number() {
                "account"
            } else {
                "custom"
            }
            .to_owned(),
        );
        let kind = scope.cage(
            binding["provider_type"]
                .as_str()
                .or(effective["provider_type"].as_str())
                .unwrap_or(if cap == "realtime" { "openai" } else { "zhipu" })
                .to_owned(),
        );
        let selected = scope.cage(
            binding["provider_id"]
                .as_i64()
                .map(|id| format!("p{id}"))
                .unwrap_or_default(),
        );
        let key = scope.cage(String::new());
        let model_field = if cap == "realtime" {
            "tts_model"
        } else {
            match cap {
                "chat" => "chat_model",
                "asr" => "asr_model",
                _ => "tts_model",
            }
        };
        let model = scope.cage(
            binding["settings"][model_field]
                .as_str()
                .or(effective["model"].as_str())
                .unwrap_or_default()
                .to_owned(),
        );
        let voice = scope.cage(
            binding["settings"]["tts_voice"]
                .as_str()
                .or(effective["voice"].as_str())
                .unwrap_or_default()
                .to_owned(),
        );
        let region = scope.cage(
            binding["settings"]["region"]
                .as_str()
                .or(effective["region"].as_str())
                .unwrap_or("global")
                .to_owned(),
        );
        let workspace = scope.cage(
            binding["settings"]["workspace_id"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        );
        let discovery = Resource::new(scope);
        let live = scope.cage(Value::Null);
        let busy = scope.cage(false);
        let model_key = if cap == "realtime" {
            "tts_models"
        } else {
            match cap {
                "chat" => "chat_models",
                "asr" => "asr_models",
                _ => "tts_models",
            }
        };
        let compatible = move |p: &Value| {
            if cap == "realtime" {
                string(p, "provider_type") == "openai"
            } else {
                p["settings"][model_field]
                    .as_str()
                    .is_some_and(|m| !m.is_empty())
            }
        };
        let mut previous = (source.get_untracked(), selected.get_untracked());
        let _ = scope.effect(move || {
            let rows = items(&providers.data.get());
            let state = (source.get(), selected.get());
            if state.0 == "account" && state.1.is_empty() {
                if let Some(row) = rows.iter().find(|r| compatible(r)) {
                    selected.revise(|v| *v = format!("p{}", row["id"]));
                }
                return;
            }
            if previous == state {
                return;
            }
            previous = state.clone();
            if state.0 == "account"
                && let Some(row) = rows.iter().find(|p| format!("p{}", p["id"]) == state.1)
            {
                kind.revise(|v| *v = string(row, "provider_type"));
                region.revise(|v| *v = string(&row["settings"], "region"));
                key.revise(String::clear);
                let chosen = string(&row["settings"], model_field);
                model.revise(|v| {
                    *v = if cap == "realtime" && !chosen.starts_with("gpt-realtime") {
                        "gpt-realtime-2.1-mini".into()
                    } else {
                        chosen
                    }
                });
                voice.revise(|v| {
                    *v = if cap == "realtime" {
                        "marin".into()
                    } else {
                        string(&row["settings"], "tts_voice")
                    }
                });
            }
        });
        let mut previous_kind = kind.get_untracked();
        let _ = scope.effect(move || {
            let current = kind.get();
            let rows = items(&catalog.data.get());
            if let Some(def) = rows.iter().find(|p| p["id"] == current) {
                if previous_kind != current {
                    previous_kind = current;
                    live.revise(|v| *v = Value::Null);
                    key.revise(String::clear);
                    if source.get_untracked() == "custom" {
                        model.revise(String::clear);
                        voice.revise(|v| *v = string(def, "voice"));
                        region.revise(|v| {
                            *v = items(&def["regions"])
                                .first()
                                .and_then(Value::as_str)
                                .unwrap_or("global")
                                .into()
                        });
                    }
                }
                if model.get_untracked().is_empty() {
                    model.revise(|v| {
                        *v = items(&def[model_key])
                            .iter()
                            .filter_map(Value::as_str)
                            .find(|m| cap != "realtime" || m.starts_with("gpt-realtime"))
                            .unwrap_or_default()
                            .into()
                    });
                }
            }
        });
        let _=scope.effect(move||{
            let source=source.get();let kind=kind.get();let selected=selected.get();let model=model.get();let key=key.get();let region=region.get();let voice=voice.get();let workspace=workspace.get();
            let value=if source=="inherit"{Value::Null}else{
                let mut settings=json!({"chat_model":"","asr_model":"","tts_model":"","region":region,"tts_voice":voice,"workspace_id":workspace});settings[model_field]=json!(model);
                let id=if source=="account"{selected.strip_prefix('p').and_then(|s|s.parse::<i64>().ok())}else{settings["api_key"]=json!(key);None};
                json!({"provider_type":kind,"provider_id":id,"settings":settings})
            };draft.revise(|v|v[cap]=value);
        });
        let provider = scope.bond(move || {
            if source.get() == "account" {
                selected
                    .get()
                    .strip_prefix('p')
                    .and_then(|s| s.parse::<i64>().ok())
            } else {
                None
            }
        });
        let device_id = scope.cage(device.clone());
        let inherited_fallback = scope.cage(effective.clone());
        let inherited_label = scope
            .bond(move || global_summary(cap, &providers.data.get(), &inherited_fallback.get()));
        let has_key = binding["has_api_key"] == true;
        let model_choices = scope.bond(move || {
            let def = items(&catalog.data.get())
                .into_iter()
                .find(|p| p["id"] == kind.get())
                .unwrap_or_default();
            let found = live.get();
            vec![if found["id"] == kind.get() {
                found[model_key].clone()
            } else {
                def[model_key].clone()
            }]
        });
        view().class(format!("device-ai-cap device-ai-{cap}")).fill((
            heading(label(cap)),p(if cap=="realtime"{"实时语音模式使用此项配置。"}else{"标准对话模式使用此项配置。"}),
            choices(source,&[("inherit","跟随全局"),("account","账号配置"),("custom","设备专用")]),
            when(source.map(|v|v=="inherit"),move||text().class("muted").content(inherited_label)),
            when(source.map(|v|v=="account"),move||view().fill(each(providers.data.map(move|v|vec![Value::Array(items(v).into_iter().filter(|p|compatible(p)).collect())])).key(|v|v.to_string()).seed(move|rows|{
                select(format!("device-ai-{cap}-account"),"账号提供商配置",items(&rows).iter().map(|p|(format!("p{}",p["id"]),format!("{} · {}",string(p,"provider_type"),string(&p["settings"],model_field)))).collect(),selected)
            }))),
            when(source.map(|v|v=="custom"),move||view().class("device-ai-private form-fields").fill((
                each(catalog.data.map(move|v|vec![Value::Array(items(v).into_iter().filter(|p|if cap=="realtime"{p["id"]=="openai"}else{!items(&p[model_key]).is_empty()}).collect())])).key(|v|v.to_string()).seed(move|rows|{
                    select(format!("device-ai-{cap}-kind"),"AI 平台",items(&rows).iter().map(|p|(string(p,"id"),string(p,"label"))).collect(),kind)
                }),
                password(if has_key{"API Key（留空保留设备密钥）"}else{"设备 API Key"},key),
                p("此密钥只供这台设备使用，不修改账号全局配置。"),
            ))),
            when(source.map(|v|v!="inherit"),move||view().class("device-ai-model form-fields").fill((
                when(kind.map(|v|v=="minimax"||v=="qwen"),move||choices(region,&[("cn","中国区"),("global","国际区")])),
                when(kind.map(|v|v=="qwen"),move||field("百炼业务空间 ID",workspace)),
                action_button().label("读取官方模型列表").variant(sapid::ButtonVariant::Outline).loading(discovery.busy).on_press(move||{
                    let body=json!({"provider_type":kind.get(),"provider_id":provider.get(),"api_key":key.get(),"region":region.get(),"workspace_id":workspace.get(),"device_id":device_id.get(),"capability":cap});
                    let identity=(kind.get(),source.get(),selected.get(),key.get(),region.get());
                    discovery.run("POST","/api/ai-providers/models".into(),Some(body),move|v|{if identity==(kind.get(),source.get(),selected.get(),key.get(),region.get()){live.revise(|l|*l=v);}});
                }),discovery.status(),
                each(model_choices).key(|v|v.to_string()).seed(move|list|{
                    let models=items(&list).iter().filter_map(Value::as_str).filter(|m|cap!="realtime"||m.starts_with("gpt-realtime")&&!m.contains("whisper")&&!m.contains("translate")&&!m.contains("transcrib")).map(str::to_owned).collect();
                    super::providers::model_choice(match cap{"chat"=>"device-chat-model","asr"=>"device-asr-model","tts"=>"device-tts-model",_=>"device-realtime-model"},label(cap),models,model,busy)
                }),
                when(cap=="tts"||cap=="realtime",move||super::voices::picker(kind.into(),provider.into(),voice,false,Some(super::voices::Draft {model:model.into(),key:key.into(),region:region.into(),device:Some(device_id.get()),capability:Some(cap.into())}))),
            ))),
        )).build(scope)
    })
}

pub fn panel(device: String) -> impl Widget {
    widget(move |scope| {
        let device_id = scope.cage(device.clone());
        let config = Resource::new(scope);
        let path = scope.cage(format!("/api/amigo/devices/{device}/ai"));
        config.load(path.get());
        let providers = Resource::new(scope);
        providers.load("/api/ai-providers");
        let catalog = Resource::new(scope);
        catalog.load("/api/ai-providers/catalog");
        let action = Resource::new(scope);
        let notice = scope.cage(String::new());
        view().class("device-ai-settings").fill((
            heading("此设备的 AI 服务"),p("默认跟随账号全局设置，也可分别指定提供商、模型、密钥与音色。模型和密钥需点击“保存设备 AI 配置”；下方“保存并同步到设备”只保存设备与陪伴设置。"),
            config.status(),providers.status(),catalog.status(),
            each(config.data.map(|v|if v.is_null(){vec![]}else{vec![v.clone()]})).key(|v|v.to_string()).seed(move|record|{
                widget(move|scope|{
                    let draft=scope.cage(record["bindings"].clone());let save=path;let reset=path;
                    let fields=["chat","asr","tts","realtime"].into_iter().map(|cap|boxed(capability(device_id.get(),cap,record["bindings"][cap].clone(),record["effective"][cap].clone(),draft,providers,catalog))).collect();
                    view().fill((nodes(fields),view().class("actions").fill((
                        action_button().label("保存设备 AI 配置").loading(action.busy).on_press(move||action.run("PUT",save.get(),Some(draft.get()),move|_|{notice.revise(|v|*v="设备 AI 配置已保存，约 5 秒内生效；账号全局设置保持不变。".into());config.load(save.get());})),
                        action_button().label("全部恢复全局配置").variant(sapid::ButtonVariant::Outline).enabled(action.busy.map(|b|!*b)).on_press(move||action.run("PUT",reset.get(),Some(json!({"chat":null,"asr":null,"tts":null,"realtime":null})),move|_|{notice.revise(|v|*v="已恢复为跟随账号全局配置。".into());config.load(reset.get());})),
                    )))).build(scope)
                })
            }),
            text().class("success").content(notice),action.status(),
        )).build(scope)
    })
}
