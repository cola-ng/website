use super::*;

pub fn dictionary(mut initial: Option<String>) -> impl Widget {
    widget(move |scope| {
        let query_value = scope.cage(initial.clone().unwrap_or_default());
        let result = Resource::new(scope);
        let history = Resource::new(scope);
        if let Some(word) = initial.take() {
            result.load(format!("/api/dict/lookup?{}", query("word", &word)));
        }
        if token().is_some() {
            history.load("/api/dict/words/searched?limit=30");
        }
        view()
            .fill((
                title("英语词典"),
                p("查词、听发音，将值得记住的词加入复习。"),
                view().class("panel search").fill((
                    field("单词或短语", query_value),
                    action_button()
                        .label("查询")
                        .loading(result.busy)
                        .enabled(result.busy.map(|b| !b))
                        .on_press(move || {
                            let word = query_value.get().trim().to_string();
                            if word.is_empty() {
                                result.error.revise(|e| *e = "请输入要查询的单词".into());
                                return;
                            }
                            result.load(format!("/api/dict/lookup?{}", query("word", &word)));
                            let _ = window().history().and_then(|h| {
                                h.replace_state_with_url(
                                    &JsValue::NULL,
                                    "",
                                    Some(&format!(
                                        "/dict/{}",
                                        query("w", &word).trim_start_matches("w=")
                                    )),
                                )
                            });
                            if token().is_some() {
                                spawn(async move {
                                    let _ = request(
                                        "POST",
                                        "/api/dict/words/searched",
                                        Some(json!({"word":word})),
                                    )
                                    .await;
                                    if history.data.is_alive() {
                                        history.load("/api/dict/words/searched?limit=30");
                                    }
                                });
                            }
                        }),
                )),
                result.status(),
                each(
                    result
                        .data
                        .map(|v| if v.is_null() { vec![] } else { vec![v.clone()] }),
                )
                .key(|v| v.to_string())
                .seed(word_card),
                when(history.data.map(|v| !items(v).is_empty()), move || {
                    view().class("panel").fill((
                        heading("最近查词"),
                        each(history.data.map(items))
                            .key(|v| v.to_string())
                            .seed(move |v| {
                                let word = string(&v, "word");
                                let id = string(&v, "id");
                                view().class("history-row").fill((
                                    nav(
                                        &word,
                                        format!(
                                            "/dict/{}",
                                            query("w", &word).trim_start_matches("w=")
                                        ),
                                    ),
                                    danger_button()
                                        .label("删除记录")
                                        .enabled(history.busy.map(|b| !*b))
                                        .on_press(move || {
                                            history.run(
                                                "DELETE",
                                                format!("/api/dict/words/searched/{id}"),
                                                None,
                                                move |_| {
                                                    history
                                                        .load("/api/dict/words/searched?limit=30")
                                                },
                                            )
                                        }),
                                ))
                            }),
                        danger_button()
                            .label("清空查词记录")
                            .enabled(history.busy.map(|b| !*b))
                            .on_press(move || {
                                if confirm("清空全部查词记录？") {
                                    history.run(
                                        "DELETE",
                                        "/api/dict/words/searched".into(),
                                        None,
                                        move |_| history.load("/api/dict/words/searched?limit=30"),
                                    );
                                }
                            }),
                        history.status(),
                    ))
                }),
            ))
            .build(scope)
    })
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
        for (key, label_text) in [
            ("definitions", "释义"),
            ("sentences", "例句"),
            ("relations", "关联词"),
            ("forms", "词形"),
            ("etymologies", "词源"),
            ("dictionaries", "词典来源"),
        ] {
            let entries = items(&data[key]);
            if entries.is_empty() {
                continue;
            }
            sections.push(boxed(heading(label_text)));
            for entry in entries {
                let keys: &[&str] = match key {
                    "definitions" => &["part_of_speech", "definition", "usage_notes"],
                    "sentences" => &["sentence", "source"],
                    "relations" => &["relation_type", "related_word"],
                    "forms" => &["form_type", "form", "notes"],
                    "etymologies" => &["etymology", "origin_word", "origin_meaning"],
                    _ => &["name_zh", "name_en", "publisher", "license_type"],
                };
                sections.push(boxed(p(keys
                    .iter()
                    .map(|k| string(&entry, k))
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · "))));
            }
        }
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
        view().fill((title("中英翻译"), view().class("panel").fill((
            textarea("需要翻译的内容",source),
            choices(target, &[("zh","译为中文"),("en","译为英文")]),
            choices(tone, &[("original","原文语气"),("casual","日常"),("business","商务"),("academic","学术"),("literary","文学"),("simple","简明")]),
            checkbox_control().label("自动纠正语法与拼写").checked(correct).on_checked_change(move |v| correct.revise(|c| *c=v)),
            checkbox_control().label("保留原文格式").checked(formatting).on_checked_change(move |v| formatting.revise(|c| *c=v)),
            action_button().label("翻译").loading(result.busy).enabled(result.busy.map(|v| !v)).on_press(move || {
                let input=source.get().trim().to_string(); if input.is_empty() || input.len()>5000 { result.error.revise(|s| *s="请输入内容，长度不超过 5000 字节".into()); return; }
                result.data.revise(|v|*v=Value::Null);
                let target=target.get(); let tone=tone.get(); let text=input.clone(); let lang=target.clone(); let style=tone.clone();
                result.run("POST", "/api/dict/translate".into(), Some(json!({"text":input,"source":"auto","target":target,"options":{"tone":tone,"auto_correct":correct.get(),"preserve_formatting":formatting.get()}})), move |mut v| { v["source_text"]=json!(text); v["target_lang"]=json!(lang); v["tone"]=json!(style); result.data.revise(|d| *d=v); notice.revise(String::clear); });
            }), result.status(),
            each(result.data.map(|v| if v.is_null() {vec![]} else {vec![v.clone()]})).key(|v| v.to_string()).seed(move |v| {
                let translated=string(&v,"translated"); let spoken=translated.clone();
                view().class("translation-result").fill((heading("翻译结果"), p(translated), action_button().label("朗读译文").on_press(move || media::speak(&spoken, action.error)),
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
