use super::*;

pub fn dictionary(initial: Option<String>) -> impl Widget {
    widget(move |scope| {
        let query_value = scope.cage(initial.clone().unwrap_or_default());
        let result = Resource::new(scope);
        let history = Resource::new(scope);
        let history_epoch = scope.cage(0u32);
        if token().is_some() {
            history.load("/api/dict/words/searched?limit=30");
        } else {
            let entries = storage()
                .and_then(|s| s.get_item("dict_search_history").ok().flatten())
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                .map(|v| items(&v))
                .unwrap_or_default();
            history.data.revise(|v| *v = json!(entries));
        }
        let lookup = std::rc::Rc::new(move || {
            if result.busy.get() {
                return;
            }
            let word = query_value.get().trim().to_string();
            if word.is_empty() {
                result.error.revise(|e| *e = "请输入要查询的单词".into());
                return;
            }
            let current = window().location().pathname().unwrap_or_default();
            if Route::parse(&current) != Route::Dict(Some(word.clone())) {
                go(&dictionary_path(&word));
                return;
            }
            let lookup_epoch = history_epoch.get();
            result.data.revise(|v| *v = Value::Null);
            result.run("GET",format!("/api/dict/lookup?{}",query("word",&word)),None,move|value|{
                result.data.revise(|v|*v=value);
                if history_epoch.get()!=lookup_epoch{return;}
                if token().is_some(){spawn(async move{match request("POST","/api/dict/words/searched",Some(json!({"word":word}))).await{Ok(_)=>{if history.data.is_alive(){history.load("/api/dict/words/searched?limit=30");}},Err(e)=>{if history.error.is_alive(){history.error.revise(|v|*v=e);}}}});}else{
                    let mut entries=items(&history.data.get());entries.retain(|v|!string(v,"word").eq_ignore_ascii_case(&word));entries.insert(0,json!({"word":word,"searched_at":js_sys::Date::new_0().to_iso_string().as_string()}));entries.truncate(50);local_history(entries,history);
                }
            });
        });
        if initial.is_some() {
            lookup();
        }
        let keyboard_lookup = lookup.clone();
        view()
            .class("dictionary-page")
            .fill((
                title("词典翻译"),
                dictionary_tabs(),
                view().class("panel search").fill((
                    submit_field("单词或短语", query_value, false, move || {
                        keyboard_lookup()
                    }),
                    action_button()
                        .label("查询")
                        .loading(result.busy)
                        .enabled(result.busy.map(|b| !*b))
                        .on_press(move || lookup()),
                )),
                view().class("dictionary-layout").fill((
                    view().class("dictionary-main").fill((
                        result.status(),
                        each(
                            result
                                .data
                                .map(|v| if v.is_null() { vec![] } else { vec![v.clone()] }),
                        )
                        .key(|v| v.to_string())
                        .seed(word_card),
                        when(
                            result.data.map(move |v| {
                                v.is_null() && !result.busy.get() && result.error.get().is_empty()
                            }),
                            || {
                                view().class("panel dictionary-empty").fill((
                                    heading("探索每一个单词"),
                                    p("输入单词或短语，查看释义、发音、例句与词形。"),
                                ))
                            },
                        ),
                    )),
                    view().class("panel dictionary-history").fill((
                        heading("最近查词"),
                        history.status(),
                        each(history.data.map(items))
                            .key(|v| v.to_string())
                            .seed(move |v| {
                                let word = string(&v, "word");
                                let remove_word = word.clone();
                                let id = string(&v, "id");
                                view().class("history-row").fill((
                                    nav(word.clone(), dictionary_path(&word)),
                                    danger_button()
                                        .label("删除记录")
                                        .enabled(history.busy.map(|b| !*b))
                                        .on_press(move || {
                                            history_epoch.revise(|n| *n += 1);
                                            if token().is_some() {
                                                history.run(
                                                    "DELETE",
                                                    format!("/api/dict/words/searched/{id}"),
                                                    None,
                                                    move |_| {
                                                        history.load(
                                                            "/api/dict/words/searched?limit=30",
                                                        )
                                                    },
                                                );
                                            } else {
                                                let mut entries = items(&history.data.get());
                                                entries.retain(|v| {
                                                    !string(v, "word")
                                                        .eq_ignore_ascii_case(&remove_word)
                                                });
                                                local_history(entries, history);
                                            }
                                        }),
                                ))
                            }),
                        empty(history, "查过的单词会保存在这里。"),
                        when(history.data.map(|v| !items(v).is_empty()), move || {
                            danger_button()
                                .label("清空查词记录")
                                .enabled(history.busy.map(|b| !*b))
                                .on_press(move || {
                                    if confirm("清空全部查词记录？") {
                                        history_epoch.revise(|n| *n += 1);
                                        if token().is_some() {
                                            history.run(
                                                "DELETE",
                                                "/api/dict/words/searched".into(),
                                                None,
                                                move |_| {
                                                    history
                                                        .load("/api/dict/words/searched?limit=30")
                                                },
                                            );
                                        } else {
                                            local_history(vec![], history);
                                        }
                                    }
                                })
                        }),
                    )),
                )),
            ))
            .build(scope)
    })
}
fn local_history(entries: Vec<Value>, history: Resource) {
    let data = json!(entries);
    if let Some(store) = storage()
        && store
            .set_item("dict_search_history", &data.to_string())
            .is_err()
    {
        history
            .error
            .revise(|v| *v = "浏览器暂时无法保存查词记录".into());
    }
    history.data.revise(|v| *v = data);
}
fn word_card(data: Value) -> impl Widget {
    widget(move |scope| {
        let action = Resource::new(scope);
        let saved = scope.cage(String::new());
        let id = data["word"]["id"].as_i64().unwrap_or_default();
        let word = string(&data["word"], "word");
        let spoken = word.clone();
        let vocabulary_word = word.clone();
        let mut sections = vec![
            boxed(heading(&word)),
            boxed(p(string(&data["word"], "memory"))),
        ];
        sections.push(boxed(
            view().class("actions").fill((
                action_button()
                    .label("朗读")
                    .variant(sapid::ButtonVariant::Outline)
                    .on_press(move || media::speak(&spoken, action.error)),
                when(token().is_some(), move || {
                    view().class("actions").fill((
                        action_button()
                            .label("收藏 / 取消收藏")
                            .enabled(action.busy.map(|b| !b))
                            .on_press(move || {
                                action.run(
                                    "POST",
                                    "/api/learn/favorites/toggle".into(),
                                    Some(json!({"entity":"word","record_id":id})),
                                    move |v| {
                                        saved.revise(|s| {
                                            *s = if v["is_favorited"].as_bool().unwrap_or(false) {
                                                "已收藏"
                                            } else {
                                                "已取消收藏"
                                            }
                                            .into()
                                        })
                                    },
                                )
                            }),
                        {
                            let word = vocabulary_word.clone();
                            action_button()
                                .label("加入 / 移出生词本")
                                .variant(sapid::ButtonVariant::Outline)
                                .enabled(action.busy.map(|b| !b))
                                .on_press(move || {
                                    action.run(
                                        "POST",
                                        "/api/learn/vocabulary/toggle".into(),
                                        Some(json!({"word":word})),
                                        move |v| {
                                            saved.revise(|s| {
                                                *s = if v["in_vocabulary"]
                                                    .as_bool()
                                                    .or(v["is_added"].as_bool())
                                                    .unwrap_or(true)
                                                {
                                                    "生词本已更新"
                                                } else {
                                                    "已移出生词本"
                                                }
                                                .into()
                                            })
                                        },
                                    )
                                })
                        },
                    ))
                })
                .otherwise(|| nav("登录后收藏和复习", "/login")),
            )),
        ));
        for pronunciation in items(&data["pronunciations"]) {
            let p_id = string(&pronunciation, "id");
            sections.push(boxed(
                view().class("pronunciation").fill((
                    p(format!(
                        "{} /{}/",
                        string(&pronunciation, "dialect"),
                        string(&pronunciation, "ipa")
                    )),
                    action_button()
                        .label("播放词典发音")
                        .variant(sapid::ButtonVariant::Subtle)
                        .on_press(move || {
                            media::play(
                                format!("/api/dict/pronunciations/{p_id}/audio"),
                                action.error,
                            )
                        }),
                )),
            ));
        }
        let compact = media::compact(scope);
        let selected = scope.cage(String::new());
        let sections_data = data.clone();
        sections.push(boxed(
            each(scope.bond(move || vec![compact.get()]))
                .key(|compact| *compact)
                .seed(move |compact| word_sections(sections_data.clone(), compact, selected)),
        ));
        for image_value in items(&data["images"]) {
            let src = string(&image_value, "image_url");
            if src.starts_with("https://") || src.starts_with('/') && !src.starts_with("//") {
                sections.push(boxed(
                    image()
                        .source(src)
                        .alt(string(&image_value, "alt_text_zh"))
                        .class("word-image"),
                ));
            }
        }
        sections.push(boxed(success_notice(saved, action)));
        sections.push(boxed(action.status()));
        view()
            .class("panel word-card")
            .fill(nodes(sections))
            .build(scope)
    })
}
fn word_sections(data: Value, compact: bool, selected: Cage<String>) -> Box<dyn Widget> {
    let mut tabs = sapid::tabs("word-sections")
        .theme(theme::orange())
        .class("word-tabs")
        .orientation(if compact {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        })
        .value(selected)
        .on_value_change(move |v| selected.revise(|s| *s = v));
    let mut first = true;
    for (key, label_text) in [
        ("definitions", "基本释义"),
        ("sentences", "例句"),
        ("relations", "关联词"),
        ("forms", "词形变化"),
        ("etymologies", "词源"),
        ("dictionaries", "词典来源"),
    ] {
        let entries = items(&data[key]);
        if entries.is_empty() {
            continue;
        }
        if first {
            if selected.get().is_empty() {
                selected.revise(|v| *v = key.into());
            }
            first = false;
        }
        tabs = tabs.tab(sapid::tab(key).label(label_text).content(move || {
            view().class("word-section").fill((
                heading(label_text),
                nodes(
                    entries
                        .iter()
                        .map(|entry| {
                            let keys: &[&str] = match key {
                                "definitions" => &["part_of_speech", "definition", "usage_notes"],
                                "sentences" => &["sentence", "translation", "source"],
                                "relations" => &["relation_type", "related_word"],
                                "forms" => &["form_type", "form", "notes"],
                                "etymologies" => &["etymology", "origin_word", "origin_meaning"],
                                _ => &["name_zh", "name_en", "publisher", "license_type"],
                            };
                            let audio = if key == "sentences" {
                                entry["id"].as_i64().map(|id| {
                                    boxed(super::sentence_audio::player(
                                        "dict",
                                        id,
                                        None,
                                        "播放例句发音",
                                    ))
                                })
                            } else {
                                None
                            };
                            boxed(
                                view().class("definition-entry").fill((
                                    p(keys
                                        .iter()
                                        .map(|k| string(entry, k))
                                        .filter(|v| !v.is_empty())
                                        .collect::<Vec<_>>()
                                        .join(" · ")),
                                    nodes(audio.into_iter().collect()),
                                )),
                            )
                        })
                        .collect(),
                ),
            ))
        }));
    }
    if first {
        boxed(p("此词条暂未收录详细释义。"))
    } else {
        boxed(tabs)
    }
}
fn dictionary_tabs() -> impl Widget {
    view()
        .role(Role::Navigation)
        .name("词典与翻译")
        .class("dictionary-tabs")
        .fill((nav("英语词典", "/dict"), nav("中英翻译", "/translate")))
}
pub fn translate() -> impl Widget {
    widget(|scope| {
        let source = scope.cage(String::new());
        let target = scope.cage("zh".to_string());
        let tone = scope.cage("original".to_string());
        let correct = scope.cage(false);
        let formatting = scope.cage(true);
        let result = Resource::new(scope);
        let action = Resource::new(scope);
        let notice = scope.cage(String::new());
        let compact = media::compact(scope);
        let options_open = scope.cage(!compact.get());
        let previous_compact = scope.cage(compact.get());
        scope.effect(move || {
            let current = compact.get();
            if previous_compact.get_untracked() != current {
                previous_compact.revise(|v| *v = current);
                options_open.revise(|v| *v = !current);
            }
        });
        view().class("translation-page").fill((title("词典翻译"),dictionary_tabs(), view().class("panel translation-workspace").fill((
            view().class("translation-source").fill((heading("原文"),textarea("需要翻译的内容",source))),
            sapid::accordion("translation-options").theme(theme::orange()).class("translation-options").content_policy(sapid::ContentPolicy::KeepMounted).open(options_open.map(|open|if *open{vec!["options".into()]}else{vec![]})).on_open_change(move|v|options_open.revise(|b|*b = !v.is_empty())).item(sapid::accordion_item("options").label("翻译设置").content(move||view().class("translation-settings-fields").fill((
            choices(target, &[("zh","译为中文"),("en","译为英文")]),
            choices(tone, &[("original","原文语气"),("casual","日常"),("business","商务"),("academic","学术"),("literary","文学"),("simple","简明")]),
            checkbox_control().label("自动纠正语法与拼写").checked(correct).on_checked_change(move |v| correct.revise(|c| *c=v)),
            checkbox_control().label("保留原文格式").checked(formatting).on_checked_change(move |v| formatting.revise(|c| *c=v)),
            )))),
            action_button().class("translate-submit").label("翻译").loading(result.busy).enabled(result.busy.map(|v| !v)).on_press(move || {
                let input=source.get().trim().to_string(); if input.is_empty() || input.len()>5000 { result.error.revise(|s| *s="请输入内容，长度不超过 5000 字节".into()); return; }
            result.data.revise(|v|*v=Value::Null);
                let target=target.get(); let tone=tone.get(); let text=input.clone(); let lang=target.clone(); let style=tone.clone();
                result.run("POST", "/api/dict/translate".into(), Some(json!({"text":input,"source":"auto","target":target,"options":{"tone":tone,"auto_correct":correct.get(),"preserve_formatting":formatting.get()}})), move |mut v| { v["source_text"]=json!(text); v["target_lang"]=json!(lang); v["tone"]=json!(style); result.data.revise(|d| *d=v); notice.revise(String::clear); });
            }), result.status(),
            each(result.data.map(|v| if v.is_null() {vec![]} else {vec![v.clone()]})).key(|v| v.to_string()).seed(move |v| {
                let translated=string(&v,"translated"); let spoken=translated.clone();let copied=translated.clone();
                view().class("translation-result").fill((heading("翻译结果"), p(translated), action_button().label("复制译文").variant(sapid::ButtonVariant::Outline).on_press(move||media::copy_text(copied.clone(),notice,action.error)),action_button().label("朗读译文").on_press(move || media::speak(&spoken, action.error)),
                    when(token().is_some(), move || { let v=v.clone(); action_button().label("收藏 / 取消收藏译文").enabled(action.busy.map(|b| !b)).on_press(move || { let source_lang=v["detected_source"].as_str().unwrap_or(if v["target_lang"]=="zh" {"en"} else {"zh"}); action.run("POST", "/api/dict/translate/favorite".into(), Some(json!({"source_text":v["source_text"],"translated_text":v["translated"],"source_lang":source_lang,"target_lang":v["target_lang"],"tone":v["tone"]})), move |v| notice.revise(|n| *n=if v["is_favorited"]==true {"已收藏"} else {"已取消收藏"}.into())); }) })
                ))
            }), action.status(), success_notice(notice,action),
        )))).build(scope)
    })
}
pub fn favorites() -> impl Widget {
    widget(|scope| {
        let kind = scope.cage("word".to_string());
        let offset = scope.cage(0_i64);
        let data = Resource::new(scope);
        let action = Resource::new(scope);
        let refresh = move || {
            data.load(format!(
                "/api/learn/favorites?entity={}&limit=30&offset={}",
                kind.get(),
                offset.get()
            ))
        };
        let previous_kind = scope.cage(kind.get_untracked());
        scope.effect(move || {
            let next = kind.get();
            if previous_kind.get_untracked() != next {
                previous_kind.revise(|k| *k = next);
                offset.revise(|n| *n = 0);
            }
            refresh();
        });
        view()
            .fill((
                title("我的收藏"),
                choices(
                    kind,
                    &[
                        ("word", "单词"),
                        ("sentence", "句子"),
                        ("translation", "翻译"),
                    ],
                ),
                data.status(),
                empty(data, "还没有收藏，去词典或跟读中添加吧。"),
                each(data.data.map(items))
                    .key(|v| v.to_string())
                    .seed(move |v| {
                        let entity = string(&v, "entity");
                        let id = v["record_id"].clone();
                        let label_text = match entity.as_str() {
                            "word" => string(&v, "word"),
                            "sentence" => format!(
                                "{}\n{}",
                                string(&v, "sentence_en"),
                                string(&v, "sentence_zh")
                            ),
                            _ => format!(
                                "{}\n{}",
                                string(&v, "source_text"),
                                string(&v, "translated_text")
                            ),
                        };
                        let spoken = if entity == "translation" {
                            string(&v, "translated_text")
                        } else if entity == "sentence" {
                            string(&v, "sentence_en")
                        } else {
                            string(&v, "word")
                        };
                        view().class("panel").fill((
                            heading(label_text),
                            p(string(&v, "word_zh")),
                            action_button()
                                .label("朗读")
                                .variant(sapid::ButtonVariant::Outline)
                                .on_press(move || media::speak(&spoken, action.error)),
                            danger_button()
                                .label("移出收藏")
                                .enabled(action.busy.map(|b| !*b))
                                .enabled(action.busy.map(|b| !b))
                                .on_press(move || {
                                    action.run(
                                        "DELETE",
                                        format!(
                                            "/api/learn/favorites?entity={entity}&record_id={id}"
                                        ),
                                        None,
                                        move |_| refresh(),
                                    )
                                }),
                        ))
                    }),
                view().class("actions").fill((
                    action_button()
                        .label("上一页")
                        .enabled(offset.map(move |n| *n > 0 && !data.busy.get()))
                        .on_press(move || offset.revise(|n| *n = (*n - 30).max(0))),
                    action_button()
                        .label("下一页")
                        .enabled(
                            data.data
                                .map(move |v| items(v).len() == 30 && !data.busy.get()),
                        )
                        .on_press(move || offset.revise(|n| *n += 30)),
                )),
                action.status(),
                nav("去复习 →", "/review"),
            ))
            .build(scope)
    })
}
