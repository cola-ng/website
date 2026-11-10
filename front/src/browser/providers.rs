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
                sapid::combobox(id.to_owned())
                    .theme(theme::orange())
                    .label(format!("{label}模型"))
                    .placeholder("搜索建议或输入模型 ID")
                    .empty_message("可直接使用输入的模型 ID")
                    .options(
                        models
                            .iter()
                            .enumerate()
                            .map(|(i, m)| sapid::select_option(format!("m{i}"), m.clone())),
                    )
                    .search(value)
                    .enabled(busy.map(|b| !*b))
                    .on_search_change(move |v| value.revise(|s| *s = v))
            }),
        )),
    )
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
        let kind = scope.cage("zhipu".to_string());
        let selection = scope.cage(Some("zhipu".to_string()));
        let search = scope.cage("智谱 Zhipu".to_string());
        let key = scope.cage(String::new());
        let chat = scope.cage(String::new());
        let tts = scope.cage(String::new());
        let asr = scope.cage(String::new());
        let voice = scope.cage(String::new());
        let region = scope.cage("cn".to_string());
        let chat_on = scope.cage(true);
        let tts_on = scope.cage(true);
        let asr_on = scope.cage(true);
        let editing = scope.cage(None::<i64>);
        let open = scope.cage(false);
        let notice = scope.cage(String::new());
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
            action.error.revise(String::clear);
        };
        let enabled = action.busy.map(|b| !*b);
        view().class("panel").fill((
            heading("AI 服务配置"),p("自备密钥即可使用，无需付费订阅。文本对话、语音合成和语音识别可分别使用不同平台；个人配置失败不会切换到平台密钥。"),
            catalog.status(),providers.status(),access.status(),view().role(Role::Status).class("ai-access-status").fill(text().content(access.data.map(|v|string(v,"message")))),
            action_button().label("添加 AI 配置").enabled(catalog.data.map(|v|!items(v).is_empty())).on_press(move||{reset();kind.revise(|s|*s="zhipu".into());selection.revise(|v|*v=Some("zhipu".into()));search.revise(|s|*s="智谱 Zhipu".into());defaults("zhipu");open.revise(|b|*b=true);}),
            sapid::dialog("provider-editor").theme(theme::orange()).title("AI 服务配置").show_trigger(false).close_label("关闭 AI 配置").open(open).on_open_change(move|v|{if !v{key.revise(String::clear);}open.revise(|b|*b=v);}).content(move||view().class("form-fields dialog-form provider-form").fill((
                view().class("field").fill((text().content("AI 平台（可搜索）"),view().class("provider-select-field").fill(widget(move|inner|{
                    let options=items(&catalog.data.get()).into_iter().map(|v|sapid::select_option(string(&v,"id"),string(&v,"label"))).collect::<Vec<_>>();
                    sapid::combobox("provider-platform").theme(theme::orange()).label("AI 平台").placeholder("搜索平台名称，例如千问、OpenAI、Kimi").empty_message("未找到平台，请尝试中文或英文名称").options(options).search(search).value(selection).enabled(action.busy.map(move|b|!*b&&editing.get().is_none())).on_search_change(move|v|search.revise(|s|*s=v)).on_value_change(move|v|{selection.revise(|s|*s=v.clone());if let Some(id)=v && id!=kind.get(){defaults(&id);kind.revise(|s|*s=id.clone());key.revise(String::clear);}}).build(inner)
                })))),
                field_state("API Key",key,Purpose::Password,false.into(),enabled.into()),
                when(editing.map(|v|v.is_some()),||p("API Key 留空可保留原密钥。")),
                each(catalog.data.map(move|v|items(v).into_iter().filter(|p|string(p,"id")==kind.get()).collect::<Vec<_>>())).key(|v|string(v,"id")).seed(move|def|{
                    let regions=names(&def,"regions");
                    view().class("provider-models").fill((
                        when(regions.len()>1,move||choices_enabled(region,&[("cn","中国区"),("global","国际区")],enabled.into())),
                        p("请使用所选平台和区域的 API Key。模型可输入控制台提供的 ID；关闭不需要的能力后，该配置不会提供对应功能。"),
                        model_editor("provider-chat","文本对话",names(&def,"chat_models"),chat,chat_on,action.busy),
                        model_editor("provider-tts","语音合成 TTS",names(&def,"tts_models"),tts,tts_on,action.busy),
                        when(tts_on,move||field_state("TTS 音色 ID",voice,Purpose::Text,false.into(),enabled.into())),
                        model_editor("provider-asr","语音识别 ASR",names(&def,"asr_models"),asr,asr_on,action.busy),
                    ))
                }),
                p("保存会分别启用本配置中勾选的能力，并替换这些能力当前的默认配置。其他能力保持原来的选择。测试连接会调用模型，可能产生少量费用。"),
                action_button().label(editing.map(|v|if v.is_some(){"保存并启用"}else{"添加并启用"}.into())).enabled(action.busy.map(move|b|!*b&&selection.get().is_some())).on_press(move||{
                    if editing.get().is_none()&&key.get().trim().is_empty(){action.error.revise(|e|*e="请输入 API Key".into());return;}
                    let mut settings=json!({"region":region.get(),"tts_voice":voice.get().trim()});let mut caps=Vec::new();
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
                    action_button().label("编辑").variant(sapid::ButtonVariant::Outline).enabled(enabled).on_press(move||{reset();let platform=string(&source,"provider_type");kind.revise(|s|*s=platform.clone());selection.revise(|s|*s=Some(platform.clone()));search.revise(|s|*s=items(&catalog.data.get()).iter().find(|p|string(p,"id")==platform).map(|p|string(p,"label")).unwrap_or(platform));defaults(&kind.get());for (cap,value,on) in [("chat",chat,chat_on),("tts",tts,tts_on),("asr",asr,asr_on)]{let name=source["settings"].get(format!("{cap}_model")).and_then(Value::as_str).map(str::to_owned);if let Some(name)=name{on.revise(|b|*b= !name.is_empty());value.revise(|s|*s=name);}}voice.revise(|s|*s=source["settings"]["tts_voice"].as_str().unwrap_or("tongtong").into());if let Some(r)=source["settings"]["region"].as_str(){region.revise(|s|*s=r.into());}editing.revise(|v|*v=Some(id));open.revise(|b|*b=true);}),
                    action_button().label("测试连接").enabled(enabled).on_press(move||action.run("POST",format!("/api/ai-providers/{id}/test"),None,move|v|{if v["success"]==true{notice.revise(|s|*s="连接成功；可在对话或语音练习中验证实际效果。".into());}else{action.error.revise(|e|*e=string(&v,"error"));}})),
                    danger_button().label("删除配置").enabled(enabled).on_press(move||{if confirm("删除此 AI 配置？对应能力将停用。其余平台配置不受影响。"){action.run("DELETE",format!("/api/ai-providers/{id}"),None,move|_|{reset();reload();});}}),
                ))));view().class("provider-row provider-card").fill(nodes(children))
            }),action.status(),success_notice(notice,action),
        )).build(scope)
    })
}
