use super::*;

fn names(value: &Value, key: &str) -> Vec<String> {
    items(&value[key])
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect()
}
fn model_editor(
    id: &'static str,
    label: &'static str,
    models: Vec<String>,
    value: Cage<String>,
    on: Cage<bool>,
    busy: Cage<bool>,
) -> Box<dyn Widget> {
    if models.is_empty() {
        return boxed(view().class("provider-capability unavailable").fill((
            heading(label),
            p("本站暂未接入该平台的此项能力，可使用其他平台的配置。"),
        )));
    }
    boxed(
        view().class("provider-capability").fill((
            checkbox_control()
                .label(format!("配置{label}"))
                .checked(on)
                .enabled(busy.map(|b| !*b))
                .on_checked_change(move |v| on.revise(|b| *b = v)),
            when(on, move || {
                model_choice(id, label, models.clone(), value, busy)
            }),
        )),
    )
}

pub(super) fn model_choice(
    id: &'static str,
    label: &'static str,
    models: Vec<String>,
    value: Cage<String>,
    busy: Cage<bool>,
) -> impl Widget {
    let count = models.len();
    widget(move |scope| {
        let expanded = scope.cage(false);
        let query = scope.cage(value.get_untracked());
        let models = models.clone();
        let selected = models.clone();
        let accepted = models.clone();
        let mut previous = value.get_untracked();
        let _ = scope.effect(move || {
            let current = value.get();
            if current != previous {
                previous = current.clone();
                query.revise(|v| *v = current);
            }
        });
        let enabled = busy.map(|b| !*b);
        let combo = sapid::combobox(id.to_owned())
            .theme(theme::orange())
            .label(format!("{label}模型"))
            .placeholder("搜索建议或输入模型 ID")
            .empty_message("未在建议中，可直接使用输入的模型 ID")
            .options(
                models
                    .iter()
                    .enumerate()
                    .map(|(i, m)| sapid::select_option(format!("m{i}"), m.clone())),
            )
            .value(value.map(move |v| {
                selected
                    .iter()
                    .position(|m| m == v)
                    .map(|i| format!("m{i}"))
            }))
            .search(query)
            .open(expanded)
            .on_open_change(move |v| expanded.revise(|b| *b = v))
            .enabled(enabled)
            .on_search_change(move |v| {
                query.revise(|s| *s = v.clone());
                value.revise(|s| *s = v);
            })
            .on_value_change(move |v| {
                if let Some(model) = v
                    .and_then(|s| s.strip_prefix('m').and_then(|s| s.parse::<usize>().ok()))
                    .and_then(|i| accepted.get(i))
                {
                    value.revise(|s| *s = model.clone());
                }
            });
        view()
            .class("provider-model-choice")
            .fill((
                sapid::input_group()
                    .class("provider-select-field")
                    .input(combo)
                    .end(
                        sapid::input_section()
                            .class("provider-select-chevron")
                            .fill(
                                sapid::action_icon(format!("展开{label}模型"))
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
                                    .enabled(enabled)
                                    .on_press(move || {
                                        query.revise(String::clear);
                                        expanded.revise(|b| *b = !*b);
                                    }),
                            ),
                    ),
                text()
                    .class("muted")
                    .content(format!("{count} 个模型，可搜索或手填 ID")),
            ))
            .build(scope)
    })
}

pub fn panel() -> impl Widget {
    widget(|scope| {
        let providers = Resource::new(scope);
        providers.load("/api/ai-providers");
        let catalog = Resource::new(scope);
        catalog.load("/api/ai-providers/catalog");
        let access = Resource::new(scope);
        access.load("/api/ai-providers/access");
        let action = Resource::new(scope);
        let discovery = Resource::new(scope);
        let model_data = scope.cage(Value::Null);
        let model_epoch = scope.cage(0u32);
        let kind = scope.cage("zhipu".to_string());
        let selection = scope.cage(Some("zhipu".to_string()));
        let platform_open = scope.cage(false);
        let search = scope.cage("智谱 Zhipu".to_string());
        let key = scope.cage(String::new());
        let chat = scope.cage(String::new());
        let tts = scope.cage(String::new());
        let asr = scope.cage(String::new());
        let voice = scope.cage(String::new());
        let region = scope.cage("cn".to_string());
        let workspace = scope.cage(String::new());
        let chat_on = scope.cage(true);
        let tts_on = scope.cage(true);
        let asr_on = scope.cage(true);
        let editing = scope.cage(None::<i64>);
        let open = scope.cage(false);
        let notice = scope.cage(String::new());
        let fetch_models = move || {
            model_epoch.revise(|e| *e += 1);
            let epoch = model_epoch.get_untracked();
            let source = kind.get_untracked();
            let area = region.get_untracked();
            let credential = key.get_untracked();
            let space = workspace.get_untracked();
            let body = json!({"provider_type":source,"provider_id":editing.get_untracked(),"region":area,"api_key":credential,"workspace_id":space});
            discovery.busy.revise(|v| *v = true);
            discovery.error.revise(String::clear);
            spawn(async move {
                let result = request("POST", "/api/ai-providers/models", Some(body)).await;
                if !model_epoch.is_alive() || model_epoch.get_untracked() != epoch {
                    return;
                }
                discovery.busy.revise(|v| *v = false);
                if kind.get_untracked() != source
                    || region.get_untracked() != area
                    || key.get_untracked() != credential
                    || workspace.get_untracked() != space
                {
                    return;
                }
                match result {
                    Ok(v) => model_data.revise(|data| *data = v),
                    Err(e) => {
                        model_data.revise(|v| *v = Value::Null);
                        discovery
                            .error
                            .revise(|v| *v = format!("{e}；当前显示静态建议。"));
                    }
                }
            });
        };
        let mut previous_model_source = None;
        let _ = scope.effect(move || {
            let state = (open.get(), kind.get(), region.get(), editing.get());
            if previous_model_source.as_ref() == Some(&state) {
                return;
            }
            previous_model_source = Some(state.clone());
            model_epoch.revise(|e| *e += 1);
            model_data.revise(|v| *v = Value::Null);
            if state.0 {
                fetch_models();
            }
        });
        let reload = move || {
            providers.load("/api/ai-providers");
            access.load("/api/ai-providers/access");
        };
        let defaults = move |id: &str| {
            if let Some(def) = items(&catalog.data.get())
                .into_iter()
                .find(|v| string(v, "id") == id)
            {
                for (field, value, on) in [
                    ("chat_models", chat, chat_on),
                    ("tts_models", tts, tts_on),
                    ("asr_models", asr, asr_on),
                ] {
                    let models = names(&def, field);
                    value.revise(|s| *s = models.first().cloned().unwrap_or_default());
                    on.revise(|b| *b = !models.is_empty());
                }
                voice.revise(|s| *s = string(&def, "voice"));
                region.revise(|s| *s = names(&def, "regions").first().cloned().unwrap_or_default());
            }
        };
        let reset = move || {
            open.revise(|b| *b = false);
            editing.revise(|v| *v = None);
            key.revise(String::clear);
            workspace.revise(String::clear);
            model_data.revise(|v| *v = Value::Null);
            model_epoch.revise(|e| *e += 1);
            action.error.revise(String::clear);
        };
        let enabled = action.busy.map(|b| !*b);
        view().class("panel").fill((
            heading("AI 服务配置"),p("自备密钥即可使用，无需付费订阅。文本对话、语音合成和语音识别可分别使用不同平台；个人配置失败不会切换到平台密钥。"),
            catalog.status(),providers.status(),access.status(),view().role(Role::Status).class("ai-access-status").fill(text().content(access.data.map(|v|string(v,"message")))),
            action_button().label("添加 AI 配置").enabled(catalog.data.map(|v|!items(v).is_empty())).on_press(move||{reset();kind.revise(|s|*s="zhipu".into());selection.revise(|v|*v=Some("zhipu".into()));search.revise(|s|*s="智谱 Zhipu".into());defaults("zhipu");open.revise(|b|*b=true);}),
            sapid::dialog("provider-editor").theme(theme::orange()).title("AI 服务配置").show_trigger(false).close_label("关闭 AI 配置").open(open).on_open_change(move|v|{if !v{key.revise(String::clear);}open.revise(|b|*b=v);}).content(move||view().class("form-fields dialog-form provider-form").fill((
                view().class("field").fill((
                    text().content("AI 平台（可搜索）"),
                    sapid::input_group().class("provider-select-field")
                        .input(widget(move|inner|{
                            let options=items(&catalog.data.get()).into_iter().map(|v|sapid::select_option(string(&v,"id"),string(&v,"label"))).collect::<Vec<_>>();
                            sapid::combobox("provider-platform").theme(theme::orange()).label("AI 平台").placeholder("搜索平台名称，例如千问、OpenAI、Kimi").empty_message("未找到平台，请尝试中文或英文名称").options(options).open(platform_open).on_open_change(move|v|platform_open.revise(|b|*b=v)).search(search).value(selection).enabled(action.busy.map(move|b|!*b&&editing.get().is_none())).on_search_change(move|v|search.revise(|s|*s=v)).on_value_change(move|v|{selection.revise(|s|*s=v.clone());if let Some(id)=v && id!=kind.get(){defaults(&id);kind.revise(|s|*s=id.clone());key.revise(String::clear);}}).build(inner)
                        }))
                        .end(sapid::input_section().class("provider-select-chevron").fill(
                            sapid::action_icon("展开 AI 平台").theme(theme::orange()).variant(sapid::ActionIconVariant::Subtle)
                                .icon(glory::ui::graphic().drawing(sapid::Icon::ChevronDown.drawing(glory::style::Color(120,113,108,255)))
                                    .style(glory::style::Style::new().width(glory::style::Length::Px(16.)).height(glory::style::Length::Px(16.))))
                                .enabled(action.busy.map(move|b|!*b&&editing.get().is_none()))
                                .on_press(move||platform_open.revise(|b|*b = !*b))
                        )),
                )),
                view().class("field").fill((text().content("API Key"),text_control().name("API Key").purpose(Purpose::Password).enabled(enabled).value(key).on_value_change(move|v|{key.revise(|s|*s=v);model_data.revise(|v|*v=Value::Null);model_epoch.revise(|e|*e+=1);discovery.busy.revise(|v|*v=false);}))),
                when(editing.map(|v|v.is_some()),||p("API Key 留空可保留原密钥。")),
                each(catalog.data.map(move|v|items(v).into_iter().filter(|p|string(p,"id")==kind.get()).map(|p|{let live=model_data.get();if live["id"]==p["id"]{live}else{p}}).collect::<Vec<_>>())).key(|v|v.to_string()).seed(move|def|{
                    let regions=names(&def,"regions");
                    view().class("provider-models").fill((
                        when(regions.len()>1,move||choices_enabled(region,&[("cn","中国区"),("global","国际区")],enabled.into())),
                        when(kind.map(|v|v=="qwen"),move||view().class("field").fill((text().content("百炼业务空间 ID（读取官方列表）"),text_control().name("百炼业务空间 ID（读取官方列表）").value(workspace).on_value_change(move|v|{workspace.revise(|s|*s=v);model_data.revise(|v|*v=Value::Null);model_epoch.revise(|e|*e+=1);discovery.busy.revise(|v|*v=false);})) )),
                        view().class("provider-catalog-info").fill((
                            text().class("muted").content(format!("建议列表核对日期：{}",string(&def,"checked_at"))),
                            route_link(string(&def,"docs_url")).label("查看官方完整模型列表"),
                            text().class("model-list-source").content(if def["source"]=="official"{format!("官方实时列表 · {}",string(&def,"fetched_at"))}else{"静态建议列表".into()}),
                            p(string(&def,"discovery_notice")),
                            action_button().label("读取官方模型列表").variant(sapid::ButtonVariant::Outline).loading(discovery.busy).enabled(discovery.busy.map(|v|!*v)).on_press(fetch_models),
                            discovery.status(),
                        )),
                        p("请使用所选平台和区域的 API Key。模型可输入控制台提供的 ID；关闭不需要的能力后，该配置不会提供对应功能。"),
                        view().class("provider-model-sections").fill((
                        text().class("muted").content(if names(&def,"chat_models").is_empty(){"文本对话：当前未接入"}else if def["capability_sources"]["chat"]=="official"{"文本对话：官方返回列表"}else{"文本对话：静态建议"}),
                        model_editor("provider-chat","文本对话",names(&def,"chat_models"),chat,chat_on,action.busy),
                        text().class("muted").content(if names(&def,"tts_models").is_empty(){"语音合成 TTS：当前未接入"}else if def["capability_sources"]["tts"]=="official"{"语音合成 TTS：官方返回列表"}else{"语音合成 TTS：静态建议"}),
                        model_editor("provider-tts","语音合成 TTS",names(&def,"tts_models"),tts,tts_on,action.busy),
                        when(tts_on,move||super::voices::picker(kind.into(),editing.into(),voice,false,Some(super::voices::Draft{model:tts.into(),key:key.into(),region:region.into(),device:None,capability:None}))),
                        text().class("muted").content(if names(&def,"asr_models").is_empty(){"语音识别 ASR：当前未接入"}else if def["capability_sources"]["asr"]=="official"{"语音识别 ASR：官方返回列表"}else{"语音识别 ASR：静态建议"}),
                        model_editor("provider-asr","语音识别 ASR",names(&def,"asr_models"),asr,asr_on,action.busy),
                        )),
                    ))
                }),
                p("保存会分别启用本配置中勾选的能力，并替换这些能力当前的默认配置。其他能力保持原来的选择。测试连接会分别调用已配置的文本、TTS 和 ASR 模型，可能产生少量费用。"),
                action_button().label(editing.map(|v|if v.is_some(){"保存并启用"}else{"添加并启用"}.into())).enabled(action.busy.map(move|b|!*b&&selection.get().is_some())).on_press(move||{
                    if editing.get().is_none()&&key.get().trim().is_empty(){action.error.revise(|e|*e="请输入 API Key".into());return;}
                    let mut settings=json!({"region":region.get(),"tts_voice":voice.get().trim(),"workspace_id":workspace.get().trim()});let mut caps=Vec::new();
                    for (cap,value,on) in [("chat",chat,chat_on),("tts",tts,tts_on),("asr",asr,asr_on)] {let value=value.get().trim().to_owned();if on.get()&&value.is_empty(){action.error.revise(|e|*e="请为已勾选的能力填写模型 ID".into());return;}settings[format!("{cap}_model")]=json!(if on.get(){value}else{String::new()});if on.get(){caps.push(cap);}}
                    if caps.is_empty(){action.error.revise(|e|*e="请至少配置一种模型".into());return;}
                    if !key.get().trim().is_empty(){settings["api_key"]=json!(key.get().trim());}
                    let (method,path,body)=if let Some(id)=editing.get(){("PUT",format!("/api/ai-providers/{id}"),json!({"settings":settings,"activate_for":caps}))}else{("POST","/api/ai-providers".into(),json!({"provider_type":kind.get(),"settings":settings,"activate_for":caps}))};
                    action.run(method,path,Some(body),move|_|{reset();reload();notice.revise(|s|*s="配置已保存并启用".into());});
                }),action.status(),
            ))),
            each(providers.data.map(move|v|{
                let definitions=items(&catalog.data.get());
                items(v).into_iter().map(|mut row|{row["catalog_definition"]=definitions.iter().find(|p|string(p,"id")==string(&row,"provider_type")).cloned().unwrap_or(Value::Null);row}).collect::<Vec<_>>()
            })).key(|v|v.to_string()).seed(move|row|{
                let id=row["id"].as_i64().unwrap_or_default();let source=row.clone();let platform=string(&row,"provider_type");let definition=row["catalog_definition"].clone();
                let label=if definition.is_null(){platform}else{string(&definition,"label")};
                let states=[("chat","文本",row["is_active"]==true),("tts","TTS",row["tts_active"]==true),("asr","ASR",row["asr_active"]==true)];
                let mut children=vec![boxed(heading(label)),boxed(p("个人密钥 · 费用由对应平台收取"))];
                for (cap,label,active) in states {
                    let model=row["settings"].get(format!("{cap}_model")).and_then(Value::as_str).map(str::to_owned).unwrap_or_else(||names(&definition,&format!("{cap}_models")).first().cloned().unwrap_or_default());
                    if !model.is_empty() {
                    children.push(boxed(view().class("provider-usage-row").fill((p(format!("{label}：{model}{}",if active{" · 使用中"}else{" · 未启用"})),action_button().label(format!("{}{label}",if active{"停用"}else{"启用"})).variant(sapid::ButtonVariant::Outline).enabled(enabled).on_press(move||{
                        let selected=states.iter().filter(|(c,_,on)|if *c==cap{!active}else{*on}).map(|(c,_,_)|*c).collect::<Vec<_>>();
                        action.run("PUT",format!("/api/ai-providers/{id}"),Some(json!({"activate_for":selected})),move|_|reload());
                    })))));
                }}
                children.push(boxed(view().class("actions").fill((
                    action_button().label("编辑").variant(sapid::ButtonVariant::Outline).enabled(enabled).on_press(move||{reset();let platform=string(&source,"provider_type");kind.revise(|s|*s=platform.clone());selection.revise(|s|*s=Some(platform.clone()));search.revise(|s|*s=items(&catalog.data.get()).iter().find(|p|string(p,"id")==platform).map(|p|string(p,"label")).unwrap_or(platform));defaults(&kind.get());for (cap,value,on) in [("chat",chat,chat_on),("tts",tts,tts_on),("asr",asr,asr_on)]{let name=source["settings"].get(format!("{cap}_model")).and_then(Value::as_str).map(str::to_owned);if let Some(name)=name{on.revise(|b|*b= !name.is_empty());value.revise(|s|*s=name);}}voice.revise(|s|*s=source["settings"]["tts_voice"].as_str().unwrap_or("tongtong").into());if let Some(r)=source["settings"]["region"].as_str(){region.revise(|s|*s=r.into());}workspace.revise(|s|*s=source["settings"]["workspace_id"].as_str().unwrap_or_default().into());editing.revise(|v|*v=Some(id));open.revise(|b|*b=true);}),
                    action_button().label("测试连接").enabled(enabled).on_press(move||action.run("POST",format!("/api/ai-providers/{id}/test"),None,move|v|{if v["success"]==true{notice.revise(|s|*s=format!("连接成功；已验证：{}。",items(&v["capabilities"]).iter().filter(|c|c["success"]==true).map(|c|match c["capability"].as_str(){Some("chat")=>"文本",Some("tts")=>"TTS",Some("asr")=>"ASR",_=>"配置"}).collect::<Vec<_>>().join("、")));}else{action.error.revise(|e|*e=string(&v,"error"));}})),
                    danger_button().label("删除配置").enabled(enabled).on_press(move||{if confirm("删除此 AI 配置？对应能力将停用。其余平台配置不受影响。"){action.run("DELETE",format!("/api/ai-providers/{id}"),None,move|_|{reset();reload();});}}),
                ))));view().class("provider-row provider-card").fill(nodes(children))
            }),action.status(),success_notice(notice,action),
        )).build(scope)
    })
}
