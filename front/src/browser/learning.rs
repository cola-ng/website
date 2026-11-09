use super::*;

pub fn chat(id: Option<i64>) -> impl Widget {
    widget(move |scope| {
        let chats = Resource::new(scope);
        chats.load("/api/learn/chats?limit=200");
        let turns = Resource::new(scope);
        let action = Resource::new(scope);
        let message = scope.cage(String::new());
        let title_value = scope.cage(String::new());
        let context = scope.cage(None::<i64>);
        let contexts = Resource::new(scope);
        contexts.load("/api/asset/contexts?limit=200");
        if let Some(id) = id {
            turns.load(format!(
                "/api/learn/chats/{id}/turns?limit=50&from_latest=true"
            ));
        }
        let new_chat = move || {
            let title = title_value.get();
            let title = if title.trim().is_empty() {
                "英语对话".into()
            } else {
                title
            };
            action.run(
                "POST",
                "/api/learn/chats".into(),
                Some(json!({"title":title,"context_id":context.get()})),
                move |v| go(&format!("/chat/{}", string(&v, "id"))),
            );
        };
        view().fill((title("英语对话"),view().class("chat-layout").fill((
            view().class("panel chat-sidebar").fill((heading("我的对话"),field("对话标题",title_value),
                action_button().label("自由对话").enabled(action.busy.map(|b|!*b)).on_press(new_chat),contexts.status(),
                each(contexts.data.map(items)).key(|v|v.to_string()).seed(move|v|{let id=v["id"].as_i64();let name=string(&v,"name_zh");action_button().label(format!("{} {}",string(&v,"icon_emoji"),name)).variant(sapid::ButtonVariant::Outline).on_press(move||{context.revise(|c|*c=id);title_value.revise(|t|*t=name.clone());new_chat();context.revise(|c|*c=None);})}),
                chats.status(),empty(chats,"还没有对话，创建一段开始练习。"),
                each(chats.data.map(items)).key(|v|v.to_string()).seed(|v|nav(string(&v,"title"),format!("/chat/{}",string(&v,"id")))),
            )),
            view().class("panel conversation").fill((
                when(id.is_some(),move||{let id=id.unwrap();view().fill((
                    view().class("actions").fill((action_button().label("修改标题").enabled(action.busy.map(|b|!*b)).variant(sapid::ButtonVariant::Outline).on_press(move||{if title_value.get().trim().is_empty(){action.error.revise(|e|*e="请在左侧填写新标题".into());return;}action.run("PUT",format!("/api/learn/chats/{id}"),Some(json!({"title":title_value.get().trim()})),move |_|chats.load("/api/learn/chats?limit=200"));}),
                        action_button().label("导出对话").variant(sapid::ButtonVariant::Outline).enabled(action.busy.map(|b|!*b)).on_press(move||media::export_chat(id,action)),
                        danger_button().label("重置本次对话").enabled(action.busy.map(|b|!*b)).on_press(move||{if confirm("清空这段对话的全部内容？"){action.run("POST",format!("/api/learn/chats/{id}/reset"),None,move |_|turns.load(format!("/api/learn/chats/{id}/turns?limit=50&from_latest=true")));}}))),
                    turns.status(),empty(turns,"发送一句英语，开始对话。"),
                    action_button().label("加载更早的消息").variant(sapid::ButtonVariant::Outline).enabled(turns.data.map(|v|v["has_prev"].as_bool().unwrap_or(false))).on_press(move||{let before=string(&turns.data.get(),"first_id");turns.load(format!("/api/learn/chats/{id}/turns?limit=50&before_id={before}"));}),
                    each(turns.data.map(items)).key(|v|v.to_string()).seed(move|v|chat_turn(v,id,turns,action)),
                    textarea("输入消息",message),action_button().label("发送消息").enabled(action.busy.map(|b|!*b)).on_press(move||{
                        let text=message.get().trim().to_string();if text.is_empty(){return;}
                        send(id,json!({"type":"text","message":text}),turns,action,Some(message));
                    }),
                    media::recorder(move|audio|send(id,json!({"type":"audio","audio_base64":audio}),turns,action,None),action),
                ))}).otherwise(||view().fill((heading("今天想聊什么？"),p("选择一个场景，或创建自由对话。")))),action.status(),
            )),
        )))).build(scope)
    })
}
fn send(id: i64, body: Value, turns: Resource, action: Resource, message: Option<Cage<String>>) {
    action.run(
        "POST",
        format!("/api/learn/chats/{id}/send"),
        Some(body),
        move |v| {
            if let Some(message) = message {
                message.revise(String::clear);
            }
            turns.load(format!(
                "/api/learn/chats/{id}/turns?limit=50&from_latest=true"
            ));
            let turn_id = v["ai_turn"]["id"].as_i64();
            if v["ai_turn"]["status"] == "processing"
                && let Some(turn_id) = turn_id
            {
                action.busy.revise(|b| *b = true);
                spawn(async move {
                    for _ in 0..12 {
                        let response =
                            request("GET", &format!("/api/learn/chats/turns/{turn_id}"), None)
                                .await;
                        if !action.busy.is_alive() {
                            return;
                        }
                        match response {
                            Ok(v) if v["status"] == "processing" => {
                                gloo_timers::future::TimeoutFuture::new(1000).await
                            }
                            Ok(v) => {
                                action.busy.revise(|b| *b = false);
                                if v["status"] == "error" {
                                    action.error.revise(|e| *e = string(&v, "error"));
                                }
                                turns.load(format!(
                                    "/api/learn/chats/{id}/turns?limit=50&from_latest=true"
                                ));
                                return;
                            }
                            Err(e) => {
                                action.busy.revise(|b| *b = false);
                                action.error.revise(|s| *s = e);
                                return;
                            }
                        }
                    }
                    action.busy.revise(|b| *b = false);
                    action
                        .error
                        .revise(|e| *e = "AI 仍在处理，请稍后刷新对话。".into());
                });
            }
        },
    );
}
fn chat_turn(v: Value, chat_id: i64, turns: Resource, action: Resource) -> impl Widget {
    let id = string(&v, "id");
    let audio_id = id.clone();
    let speaker = string(&v, "speaker");
    let spoken = string(&v, "content_en");
    view()
        .class(if speaker == "user" {
            "message user-message"
        } else {
            "message ai-message"
        })
        .fill((
            text().class("eyebrow").content(if speaker == "user" {
                "你"
            } else {
                "AI 伙伴"
            }),
            p(&spoken),
            p(string(&v, "content_zh")),
            when(v["status"] == "processing", || p("AI 正在回复…")),
            when(v["status"] == "error", {
                let error = string(&v, "error");
                move || p(error.clone())
            }),
            nodes(
                items(&v["issues"])
                    .iter()
                    .map(|issue| {
                        boxed(p(format!(
                            "{} → {}\n{}",
                            string(issue, "original_text"),
                            string(issue, "suggested_text"),
                            string(issue, "description_zh")
                        )))
                    })
                    .collect(),
            ),
            action_button()
                .label("朗读消息")
                .variant(sapid::ButtonVariant::Subtle)
                .on_press(move || {
                    if !v["audio_path"].is_null() {
                        media::play(
                            format!("/api/learn/chats/turns/{audio_id}/audio"),
                            action.error,
                        );
                    } else {
                        media::speak(&spoken, action.error);
                    }
                }),
            danger_button()
                .label("删除消息")
                .enabled(action.busy.map(|b| !*b))
                .enabled(action.busy.map(|b| !*b))
                .on_press(move || {
                    if confirm("删除此条消息？") {
                        action.run(
                            "DELETE",
                            format!("/api/learn/chats/turns/{id}"),
                            None,
                            move |_| {
                                turns.load(format!(
                                    "/api/learn/chats/{chat_id}/turns?limit=50&from_latest=true"
                                ))
                            },
                        );
                    }
                }),
        ))
}
pub fn stages() -> impl Widget {
    widget(|scope| {
        let data = Resource::new(scope);
        data.load("/api/asset/stages?limit=200");
        view()
            .fill((
                title("场景学习"),
                p("在生活里的真实场景中，积累能直接用上的表达。"),
                data.status(),
                empty(data, "暂时没有可用场景。"),
                view()
                    .class("cards")
                    .fill(each(data.data.map(items)).key(|v| v.to_string()).seed(|v| {
                        view().class("panel").fill((
                            heading(format!(
                                "{} {}",
                                string(&v, "icon_emoji"),
                                string(&v, "name_zh")
                            )),
                            p(string(&v, "name_en")),
                            p(string(&v, "description_zh")),
                            p(format!("难度 {}", string(&v, "difficulty"))),
                            nav("进入场景 →", format!("/stages/{}", string(&v, "id"))),
                        ))
                    })),
            ))
            .build(scope)
    })
}
pub fn stage(id: i64) -> impl Widget {
    widget(move |scope| {
        let stage = Resource::new(scope);
        stage.load(format!("/api/asset/stages/{id}"));
        let scripts = Resource::new(scope);
        let selected_script = scope.cage(String::new());
        scripts.load(format!("/api/asset/stages/{id}/scripts"));
        let turns = Resource::new(scope);
        let action = Resource::new(scope);
        view()
            .fill((
                nav("← 所有场景", "/stages"),
                stage.status(),
                each(
                    stage
                        .data
                        .map(|v| if v.is_null() { vec![] } else { vec![v.clone()] }),
                )
                .key(|v| v.to_string())
                .seed(|v| {
                    view().class("panel").fill((
                        title(string(&v, "name_zh")),
                        p(string(&v, "description_zh")),
                    ))
                }),
                scripts.status(),
                empty(scripts, "该场景暂时没有练习脚本。"),
                view().class("cards").fill(
                    each(scripts.data.map(items))
                        .key(|v| v.to_string())
                        .seed(move |v| {
                            let script_id = string(&v, "id");
                            view().class("panel").fill((
                                heading(string(&v, "title_zh")),
                                p(string(&v, "title_en")),
                                p(string(&v, "description_zh")),
                                {
                                    let selected_id = script_id.clone();
                                    action_button()
                                        .label("学习脚本")
                                        .pressed(selected_script.map(move |id| id == &selected_id))
                                        .on_press(move || {
                                            selected_script.revise(|id| *id = script_id.clone());
                                            turns.load(format!(
                                                "/api/asset/scripts/{script_id}/turns"
                                            ))
                                        })
                                },
                            ))
                        }),
                ),
                turns.status(),
                each(turns.data.map(items))
                    .key(|v| v.to_string())
                    .seed(move |v| script_practice(v, action)),
                action.status(),
                nav("与 AI 自由对话 →", "/chat"),
            ))
            .build(scope)
    })
}
pub fn review() -> impl Widget {
    widget(|scope| {
        let data = Resource::new(scope);
        data.load("/api/learn/review/stats");
        let vocabulary = Resource::new(scope);
        vocabulary.load("/api/learn/vocabulary?per_page=100");
        view()
            .fill((
                title("巩固每一点进步"),
                p("选择需要加强的练习类型。"),
                data.status(),
                view().class("cards").fill(nodes(
                    REVIEW_TYPES
                        .iter()
                        .map(|(kind, label)| {
                            let kind = kind.to_string();
                            let count_kind = kind.clone();
                            boxed(view().class("panel").fill((
                                heading(*label),
                                text().content(
                                    data.data.map(move |v| {
                                        format!("{} 项待复习", string(v, &count_kind))
                                    }),
                                ),
                                nav("开始复习 →", format!("/review/session/{kind}")),
                            )))
                        })
                        .collect(),
                )),
                account::data_section("生词本", vocabulary),
            ))
            .build(scope)
    })
}
pub fn review_session(kind: String) -> impl Widget {
    widget(move |scope| {
        let data = Resource::new(scope);
        data.load(format!("/api/learn/review/items?type={kind}&limit=100"));
        let index = scope.cage(0usize);
        let correct = scope.cage(0usize);
        let kind = kind.clone();
        view()
            .fill((
                title("复习练习"),
                nav("← 返回复习", "/review"),
                data.status(),
                text().class("muted").content(
                    data.data
                        .map(move |v| format!("本轮 {} 项", items(v).len())),
                ),
                each(data.data.map(move |v| {
                    items(v)
                        .get(index.get())
                        .cloned()
                        .into_iter()
                        .collect::<Vec<_>>()
                }))
                .key(|v| v.to_string())
                .seed(move |v| review_item(v, kind.clone(), data, index, correct)),
                when(
                    data.data
                        .map(move |v| !v.is_null() && index.get() >= items(v).len()),
                    move || {
                        view().class("panel").fill((
                            heading("本轮练习已完成"),
                            text().content(correct.map(|c| format!("答对 {c} 项。继续保持！"))),
                            nav("选择其他练习", "/review"),
                        ))
                    },
                ),
            ))
            .build(scope)
    })
}
fn review_item(
    item: Value,
    kind: String,
    data: Resource,
    index: Cage<usize>,
    correct: Cage<usize>,
) -> impl Widget {
    widget(move |scope| {
        let answer = scope.cage(String::new());
        let shown = scope.cage(false);
        let rating = scope.cage(String::new());
        let evaluation = Resource::new(scope);
        let action = Resource::new(scope);
        if kind == "pronunciation" {
            scope.effect(move || {
                if evaluation.data.get().is_null() {
                    rating.revise(String::clear);
                    shown.revise(|v| *v = false);
                }
            });
        }
        let started = js_sys::Date::now();
        let word = string(&item, "word");
        let definition = string(&item, "word_zh");
        let source = item["source"].clone();
        let source_id = item["source_id"].clone();
        let review_kind = kind.clone();
        let spoken = word.clone();
        let mut body: Vec<Box<dyn Widget>> = vec![boxed(heading(match kind.as_str() {
            "spelling" => "听音拼写".to_string(),
            "phrase" => {
                let example = string(&item, "example");
                if example.is_empty() {
                    format!("根据释义填写：{}", string(&item, "word_zh"))
                } else {
                    example.replace(&word, "_____")
                }
            }
            _ => word.clone(),
        }))];
        if kind == "meaning" {
            let expected = definition.clone();
            let mut options: Vec<String> = items(&data.data.get())
                .iter()
                .map(|v| string(v, "word_zh"))
                .filter(|v| !v.is_empty() && v != &expected)
                .collect();
            options.sort();
            options.dedup();
            options.truncate(3);
            options.push(expected.clone());
            let len = options.len();
            if len > 1 {
                options.rotate_left((js_sys::Date::now() as usize) % len);
            }
            if expected.is_empty() {
                body.push(boxed(p("此词暂无释义，请先在词典中补充或选择不熟悉。")));
            }
            body.push(boxed(
                view().class("choices").fill(nodes(
                    options
                        .into_iter()
                        .filter(|v| !v.is_empty())
                        .map(|option| {
                            let expected = expected.clone();
                            boxed(
                                action_button()
                                    .label(option.clone())
                                    .enabled(shown.map(|v| !*v))
                                    .on_press(move || {
                                        answer.revise(|s| *s = option.clone());
                                        rating.revise(|r| {
                                            *r = if option == expected {
                                                "correct"
                                            } else {
                                                "incorrect"
                                            }
                                            .into()
                                        });
                                        shown.revise(|v| *v = true);
                                    }),
                            )
                        })
                        .collect(),
                )),
            ));
        } else if kind == "spelling" || kind == "phrase" {
            body.push(boxed(field("输入答案", answer)));
            let expected = word.clone();
            body.push(boxed(
                action_button()
                    .label("检查答案")
                    .enabled(shown.map(|v| !*v))
                    .on_press(move || {
                        rating.revise(|r| {
                            *r = if normalize_answer(&answer.get()) == normalize_answer(&expected) {
                                "correct"
                            } else {
                                "incorrect"
                            }
                            .into()
                        });
                        shown.revise(|v| *v = true);
                    }),
            ))
        } else if kind == "pronunciation" {
            let reference = word.clone();
            body.push(boxed(media::recorder(
                move |audio| {
                    let reference = reference.clone();
                    evaluation.run(
                        "POST",
                        "/api/learn/pronunciation/evaluate".into(),
                        Some(json!({"audio_base64":audio,"reference_text":reference})),
                        move |v| {
                            let score = v["overall_score"].as_i64().unwrap_or(0);
                            rating.revise(|r| {
                                *r = if score >= 80 {
                                    "correct"
                                } else if score >= 60 {
                                    "partial"
                                } else {
                                    "incorrect"
                                }
                                .into()
                            });
                            evaluation.data.revise(|d| *d = v);
                            shown.revise(|s| *s = true);
                        },
                    );
                },
                evaluation,
            )));
            body.push(boxed(account::data_section("发音反馈", evaluation)));
        } else {
            body.push(boxed(
                action_button()
                    .label("显示释义")
                    .on_press(move || shown.revise(|s| *s = true)),
            ));
        }
        body.push(boxed(
            action_button()
                .label("听发音")
                .variant(sapid::ButtonVariant::Outline)
                .on_press(move || media::speak(&spoken, action.error)),
        ));
        let example = string(&item, "example");
        let example_zh = string(&item, "example_zh");
        let solution = word.clone();
        let definition = definition.clone();
        body.push(boxed(when(shown, move || {
            view().class("answer").fill((
                heading(solution.clone()),
                p(definition.clone()),
                p(example.clone()),
                p(example_zh.clone()),
                text().content(rating.map(|v| {
                    match v.as_str() {
                        "correct" => "回答正确",
                        "incorrect" => "需要加强",
                        "partial" => "部分掌握",
                        _ => "请自评掌握程度",
                    }
                    .to_string()
                })),
            ))
        })));
        let mut buttons = vec![];
        for (result, label_text) in [
            ("incorrect", "不熟悉"),
            ("partial", "有印象"),
            ("correct", "已掌握"),
        ] {
            let source = source.clone();
            let source_id = source_id.clone();
            let kind = review_kind.clone();
            let recognition = kind == "recognition";
            buttons.push(boxed(action_button().label(if recognition{label_text}else if result=="incorrect"{"跳过并记为不熟悉"}else if result=="correct"{"提交结果并继续"}else{continue}).enabled(action.busy.map(move|b|!*b&&!evaluation.busy.get()&&(recognition||result=="incorrect"||!rating.get().is_empty()))).on_press(move||{
                let result=if recognition||result=="incorrect"{result.to_string()}else{rating.get()};let is_correct=result=="correct";
                action.run("POST","/api/learn/review/submit".into(),Some(json!({"source":source,"source_id":source_id,"review_type":kind,"result":result,"score":evaluation.data.get()["overall_score"],"time_spent_ms":(js_sys::Date::now()-started).max(0.0) as i64})),move |v|{if v["success"]!=true{action.error.revise(|e|*e="复习记录未保存，请刷新后重试".into());return;}
if is_correct{correct.revise(|c|*c+=1);}index.revise(|i|*i+=1);});
            })));
        }
        body.push(boxed(view().class("actions").fill(nodes(buttons))));
        body.push(boxed(action.status()));
        view()
            .class("panel practice-card")
            .fill(nodes(body))
            .build(scope)
    })
}
pub fn reading() -> impl Widget {
    widget(|scope| {
        let data = Resource::new(scope);
        data.load("/api/asset/read/recommended?limit=10");
        let subjects = Resource::new(scope);
        subjects.load("/api/asset/read/subjects?per_page=100");
        let action = Resource::new(scope);
        let index = scope.cage(0usize);
        let filter = scope.cage(None::<i64>);
        let refresh = move || {
            index.revise(|i| *i = 0);
            data.load(format!(
                "/api/asset/read/recommended?limit=10{}",
                filter
                    .get()
                    .map(|id| format!("&subject_id={id}"))
                    .unwrap_or_default()
            ));
        };
        view()
            .fill((
                title("跟读练习"),
                p("听一句，读一句，逐步掌握自然的英语发音。"),
                subjects.status(),
                view().class("choices").fill((
                    action_button()
                        .label("全部主题")
                        .variant(sapid::ButtonVariant::Outline)
                        .pressed(filter.map(|id| id.is_none()))
                        .on_press(move || {
                            filter.revise(|f| *f = None);
                            refresh();
                        }),
                    each(subjects.data.map(items))
                        .key(|v| v.to_string())
                        .seed(move |v| {
                            let id = v["id"].as_i64();
                            action_button()
                                .label(string(&v, "title_zh"))
                                .pressed(filter.map(move |current| *current == id))
                                .variant(sapid::ButtonVariant::Outline)
                                .on_press(move || {
                                    filter.revise(|f| *f = id);
                                    refresh();
                                })
                        }),
                )),
                data.status(),
                text().content(
                    data.data
                        .map(|v| format!("当前等级 {}", string(v, "user_level"))),
                ),
                each(data.data.map(move |v| {
                    items(v)
                        .get(index.get())
                        .cloned()
                        .into_iter()
                        .collect::<Vec<_>>()
                }))
                .key(|v| v.to_string())
                .seed(move |v| reading_item(v, index, action)),
                when(
                    data.data
                        .map(move |v| !v.is_null() && index.get() >= items(v).len()),
                    move || {
                        view().class("panel").fill((
                            heading("本组练习已完成"),
                            action_button().label("获取下一组").on_press(refresh),
                        ))
                    },
                ),
                view().class("actions").fill((
                    action_button()
                        .label("提升难度")
                        .variant(sapid::ButtonVariant::Outline)
                        .on_press(move || {
                            action.run(
                                "POST",
                                "/api/asset/read/advance-level".into(),
                                Some(json!({"skill_type":"pronunciation"})),
                                move |_| refresh(),
                            )
                        }),
                    danger_button()
                        .label("重置跟读进度")
                        .enabled(action.busy.map(|b| !*b))
                        .on_press(move || {
                            if confirm("重置发音等级并清除跟读练习记录？") {
                                action.run(
                                    "POST",
                                    "/api/asset/read/reset-level".into(),
                                    Some(json!({"skill_type":"pronunciation"})),
                                    move |_| refresh(),
                                );
                            }
                        }),
                )),
                action.status(),
            ))
            .build(scope)
    })
}
fn reading_item(v: Value, index: Cage<usize>, action: Resource) -> impl Widget {
    widget(move |scope| {
        let evaluation = Resource::new(scope);
        let saved = scope.cage(false);
        let sentence = string(&v, "content_en");
        let spoken = sentence.clone();
        let reference = sentence.clone();
        let id = v["id"].clone();
        let audio_id = id.clone();
        let favorite_id = id.clone();
        let practice_id = id.clone();
        let submission = Resource::new(scope);
        scope.effect(move || {
            if evaluation.data.get().is_null() {
                saved.revise(|v| *v = false);
            }
        });
        view()
            .class("panel practice-card")
            .fill((
                heading(sentence),
                p(string(&v, "content_zh")),
                p(string(&v, "phonetic_transcription")),
                view().class("actions").fill((
                    action_button().label("播放原音").on_press(move || {
                        media::play(
                            format!("/api/asset/read/sentences/{audio_id}/audio"),
                            action.error,
                        )
                    }),
                    action_button()
                        .label("浏览器朗读")
                        .variant(sapid::ButtonVariant::Outline)
                        .on_press(move || media::speak(&spoken, action.error)),
                    action_button()
                        .label("收藏 / 取消收藏句子")
                        .enabled(action.busy.map(|b| !*b))
                        .variant(sapid::ButtonVariant::Outline)
                        .on_press(move || {
                            action.run(
                                "POST",
                                "/api/learn/favorites/toggle".into(),
                                Some(json!({"entity":"sentence","record_id":favorite_id})),
                                move |v| action.data.revise(|d| *d = v),
                            )
                        }),
                )),
                media::recorder(
                    move |audio| {
                        if evaluation.busy.get() {
                            return;
                        }
                        let reference = reference.clone();
                        evaluation.run(
                            "POST",
                            "/api/learn/pronunciation/evaluate".into(),
                            Some(json!({"audio_base64":audio,"reference_text":reference})),
                            move |v| {
                                saved.revise(|s| *s = false);
                                evaluation.data.revise(|d| *d = v);
                            },
                        );
                    },
                    evaluation,
                ),
                account::data_section("发音反馈", evaluation),
                action_button()
                    .label(saved.map(|s| {
                        if *s {
                            "已保存"
                        } else {
                            "保存本次成绩"
                        }
                        .to_string()
                    }))
                    .enabled(evaluation.data.map(move |v| {
                        !v.is_null()
                            && !saved.get()
                            && !submission.busy.get()
                            && !evaluation.busy.get()
                    }))
                    .on_press(move || {
                        let mut scores = evaluation.data.get();
                        scores["sentence_id"] = practice_id.clone();
                        scores["practice_id"] =
                            json!(format!("read-{}-{}", practice_id, js_sys::Date::now()));
                        submission.run(
                            "POST",
                            "/api/asset/read/practice".into(),
                            Some(scores),
                            move |v| {
                                saved.revise(|s| *s = true);
                                submission.data.revise(|d| *d = v);
                            },
                        );
                    }),
                submission.status(),
                when(saved, || {
                    view()
                        .role(Role::Status)
                        .class("success")
                        .fill(p("成绩已保存"))
                }),
                action_button()
                    .label("下一句")
                    .enabled(evaluation.busy.map(move |b| !*b && !submission.busy.get()))
                    .variant(sapid::ButtonVariant::Outline)
                    .on_press(move || index.revise(|i| *i += 1)),
            ))
            .build(scope)
    })
}

fn script_practice(v: Value, action: Resource) -> impl Widget {
    widget(move |scope| {
        let evaluation = Resource::new(scope);
        let recording = scope.cage(String::new());
        let id = v["id"].clone();
        let spoken = string(&v, "content_en");
        let reference = spoken.clone();
        view()
            .class("panel")
            .fill((
                heading(string(&v, "speaker_role")),
                p(spoken.clone()),
                p(string(&v, "content_zh")),
                p(string(&v, "phonetic_transcription")),
                action_button().label("播放原音").on_press(move || {
                    media::play(format!("/api/asset/scripts/turns/{id}/audio"), action.error)
                }),
                action_button()
                    .label("浏览器朗读")
                    .variant(sapid::ButtonVariant::Outline)
                    .on_press(move || media::speak(&spoken, action.error)),
                when(token().is_some(), move || {
                    let reference = reference.clone();
                    view().fill((
                        media::recorder(
                            move |audio| {
                                recording.revise(|r| *r = audio.clone());
                                evaluation.run(
                                    "POST",
                                    "/api/learn/pronunciation/evaluate".into(),
                                    Some(json!({"audio_base64":audio,"reference_text":reference})),
                                    move |value| evaluation.data.revise(|v| *v = value),
                                );
                            },
                            evaluation,
                        ),
                        action_button()
                            .label("回放我的录音")
                            .enabled(recording.map(|r| !r.is_empty()))
                            .on_press(move || {
                                media::play(
                                    format!("data:audio/wav;base64,{}", recording.get()),
                                    evaluation.error,
                                )
                            }),
                        account::data_section("发音反馈", evaluation),
                    ))
                })
                .otherwise(|| nav("登录后跟读练习", "/login")),
            ))
            .build(scope)
    })
}
