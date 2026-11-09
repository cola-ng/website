use super::*;
use glory::ui::fragment;

pub fn chat(id: Option<i64>) -> impl Widget {
    widget(move |scope| {
        let chats = Resource::new(scope);
        chats.load("/api/learn/chats?limit=200");
        let turns = Resource::new(scope);
        let action = Resource::new(scope);
        let stream = chat_stream::ConversationStream::new(scope);
        let message = scope.cage(String::new());
        let title_value = scope.cage(String::new());
        let context = scope.cage(None::<i64>);
        let picker = scope.cage(false);
        let rename_open = scope.cage(false);
        let contexts = Resource::new(scope);
        contexts.load("/api/asset/contexts?limit=200");
        let display = [
            "conv_showBotEn",
            "conv_showBotZh",
            "conv_showUserEn",
            "conv_showUserZh",
        ]
        .map(|key| {
            scope.cage(
                storage()
                    .and_then(|s| s.get_item(key).ok().flatten())
                    .is_none_or(|v| v != "false"),
            )
        });
        if let Some(id) = id {
            turns.load(format!(
                "/api/learn/chats/{id}/turns?limit=50&from_latest=true"
            ));
        }
        let new_chat = move || {
            let title = if context.get().is_some() {
                title_value.get()
            } else {
                "英语对话".into()
            };
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
        let text_stream = stream.clone();
        let send = std::rc::Rc::new(move || {
            if action.busy.get() {
                return;
            }
            let Some(id) = id else {
                return;
            };
            let text = message.get().trim().to_string();
            if text.is_empty() {
                return;
            }
            text_stream.clone().send(
                id,
                json!({"type":"text","message":text}),
                turns,
                action,
                Some(message),
            );
        });
        let keyboard_send = send.clone();
        let result=view().class("chat-page").fill((
            view().class("sr-only").fill(title("天天唠嗑")),
            when(id.is_none(),move||chat_navigation(chats,picker,new_chat)),

            sapid::dialog("chat-contexts").theme(theme::orange()).show_trigger(false).open(picker).on_open_change(move|v|picker.revise(|b|*b=v)).title("选择对话场景").close_label("关闭场景选择").content(move||view().class("scene-grid").fill((contexts.status(),each(contexts.data.map(items)).key(|v|v.to_string()).seed(move|v|{
                let id=v["id"].as_i64();let name=string(&v,"name_zh");action_button().label(format!("{} {}",string(&v,"icon_emoji"),name)).variant(sapid::ButtonVariant::Outline).enabled(action.busy.map(|b|!*b)).on_press(move||{context.revise(|c|*c=id);title_value.revise(|t|*t=name.clone());new_chat();})
            })))),
            view().class("chat-layout").fill((
                view().class("chat-sidebar").fill((
                    view().class("chat-new-actions").fill((action_button().label("自由对话").enabled(action.busy.map(|b|!*b)).on_press(new_chat),action_button().label("选择场景").variant(sapid::ButtonVariant::Outline).on_press(move||picker.revise(|b|*b=true)))),
                    chat_links(chats),
                )),
                view().class(scope.bond(move||format!("conversation{}{}{}{}",if !display[0].get(){" hide-ai-en"}else{""},if !display[1].get(){" hide-ai-zh"}else{""},if !display[2].get(){" hide-user-en"}else{""},if !display[3].get(){" hide-user-zh"}else{""}))).fill((
                    when(id.is_some(),move||{
                        let id=id.unwrap();let stream=stream.clone();let audio_stream=stream.clone();let send=send.clone();let keyboard_send=keyboard_send.clone();
                        view().class("chat-content").fill((
                            view().class("chat-toolbar").fill((chat_navigation(chats,picker,new_chat),
                                text().class("chat-title").content(chats.data.map(move|v|items(v).iter().find(|c|c["id"].as_i64()==Some(id)).map(|c|string(c,"title")).unwrap_or_else(||"英语对话".into()))),
                                sapid::dialog("rename-chat").theme(theme::orange()).open(rename_open).on_open_change(move|v|{if v{let current=items(&chats.data.get()).iter().find(|c|c["id"].as_i64()==Some(id)).map(|c|string(c,"title")).unwrap_or_default();title_value.revise(|s|*s=current);}rename_open.revise(|b|*b=v);}).label("修改标题").trigger_content(||fragment((text().content("✎"),text().class("toolbar-action-label").content("修改标题")))).title("重命名对话").close_label("关闭重命名").content(move||view().class("dialog-form").fill((field("对话标题",title_value),action_button().label("保存标题").enabled(action.busy.map(|b|!*b)).on_press(move||{if title_value.get().trim().is_empty(){action.error.revise(|e|*e="请输入对话标题".into());return;}action.run("PUT",format!("/api/learn/chats/{id}"),Some(json!({"title":title_value.get().trim()})),move |_|{rename_open.revise(|b|*b=false);chats.load("/api/learn/chats?limit=200");});}),action.status()))),
                                sapid::popover("chat-tools").theme(theme::orange()).label("对话操作").trigger_content(||fragment((text().content("⋯"),text().class("toolbar-action-label").content("对话操作")))).close_label("关闭对话操作").content(move||view().class("dialog-form").fill((
                                    action_button().label("导出对话").variant(sapid::ButtonVariant::Outline).enabled(action.busy.map(|b|!*b)).on_press(move||media::export_chat(id,action)),
                                    danger_button().label("重置本次对话").enabled(action.busy.map(|b|!*b)).on_press(move||{if confirm("清空这段对话的全部内容？"){action.run("POST",format!("/api/learn/chats/{id}/reset"),None,move |_|turns.load(format!("/api/learn/chats/{id}/turns?limit=50&from_latest=true")));}}),
                                ))),
                            )),
                            view().class("chat-display-options").fill(nodes(display.iter().enumerate().map(|(i,cage)|{
                                let cage=*cage;let label=["AI 英","AI 中","我 英","我 中"][i];let key=["conv_showBotEn","conv_showBotZh","conv_showUserEn","conv_showUserZh"][i];
                                boxed(action_button().label(label).variant(sapid::ButtonVariant::Subtle).pressed(cage).on_press(move||{cage.revise(|b|*b = !*b);if let Some(s)=storage(){let _=s.set_item(key,if cage.get(){"true"}else{"false"});}}))
                            }).collect())),
                            view().class("chat-messages").fill((turns.status(),empty(turns,"发送一句英语，开始对话。"),
                                when(turns.data.map(|v|v["has_prev"]==true),move||action_button().label("加载更早的消息").variant(sapid::ButtonVariant::Outline).enabled(action.busy.map(|b|!*b)).on_press(move||chat_stream::load_older(id,turns,action))),
                                each(turns.data.map(items)).key(|v|v.to_string()).seed(move|v|chat_turn(v,id,turns,action)),stream.view(),
                            )),
                            view().class("chat-composer").fill((
                                view().class("compose-text").fill((submit_field("输入消息",message,true,move||keyboard_send()),action_button().label("发送消息").enabled(action.busy.map(|b|!*b)).on_press(move||send()))),
                                media::recorder(move|audio|audio_stream.clone().send(id,json!({"type":"audio","audio_base64":audio}),turns,action,None),action),
                                text().class("muted").content("Enter 发送 · Shift + Enter 换行"),
                            )),
                        ))
                    }).otherwise(move||view().class("chat-empty").fill((heading("今天想聊什么？"),p("选择一个场景，或创建自由对话。"),view().class("actions").fill((action_button().label("新建自由对话").on_press(new_chat),action_button().label("挑选场景").variant(sapid::ButtonVariant::Outline).on_press(move||picker.revise(|b|*b=true))))))),
                    action.status(),
                )),
            )),
        )).build(scope);
        media::follow_chat(scope);
        result
    })
}
fn chat_navigation<F: Fn() + Clone + 'static>(
    chats: Resource,
    picker: Cage<bool>,
    new_chat: F,
) -> impl Widget {
    view().class("mobile-chat-navigation").fill(
        sapid::drawer("conversation-list")
            .theme(theme::orange())
            .label("我的对话")
            .trigger_content(|| text().content("☷"))
            .title("我的对话")
            .close_label("关闭对话列表")
            .content(move || {
                let create = new_chat.clone();
                view().fill((
                    view().class("chat-new-actions").fill((
                        action_button().label("新建自由对话").on_press(create),
                        action_button()
                            .label("选择场景")
                            .on_press(move || picker.revise(|b| *b = true)),
                    )),
                    chat_links(chats),
                ))
            }),
    )
}
fn chat_links(chats: Resource) -> impl Widget {
    view().class("chat-links").fill((
        chats.status(),
        empty(chats, "还没有对话，创建一段开始练习。"),
        each(chats.data.map(items))
            .key(|v| v.to_string())
            .seed(|v| {
                view().class("chat-list-row").fill((
                    nav(string(&v, "title"), format!("/chat/{}", string(&v, "id"))),
                    text()
                        .class("muted")
                        .content(string(&v, "last_message_preview")),
                ))
            }),
    ))
}
fn chat_turn(v: Value, chat_id: i64, turns: Resource, action: Resource) -> impl Widget {
    let id = string(&v, "id");
    let audio_id = id.clone();
    let pending_id = id.clone();
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
            text()
                .kind(TextKind::Paragraph)
                .class("message-en")
                .content(spoken.clone()),
            text()
                .kind(TextKind::Paragraph)
                .class("message-zh")
                .content(string(&v, "content_zh")),
            when(v["status"] == "processing", move || {
                chat_stream::pending_turn(pending_id.clone(), chat_id, turns)
            }),
            when(v["status"] == "error", {
                let error = string(&v, "error");
                move || {
                    view()
                        .role(Role::Alert)
                        .class("error")
                        .fill(p(error.clone()))
                }
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
            view().class("message-actions").fill((
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
            )),
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
        view()
            .class("review-page")
            .fill((
                title("温故知新"),
                p("科学复习，牢记所学。"),
                data.status(),
                link()
                    .class("panel smart-review")
                    .target("/review/session")
                    .fill((
                        text().class("scene-emoji").content("✨"),
                        heading("智能选择"),
                        text().class("stat-value").content(
                            data.data
                                .map(|v| format!("{} 项待复习", string(v, "total"))),
                        ),
                        p("根据遗忘曲线智能推荐复习内容"),
                    )),
                view().class("cards review-modes").fill(nodes(
                    REVIEW_TYPES
                        .iter()
                        .enumerate()
                        .map(|(i, (kind, label))| {
                            let kind = kind.to_string();
                            let count_kind = kind.clone();
                            let descriptions = [
                                "练习准确发音，查看评分反馈",
                                "巩固单词释义，理解词语用法",
                                "听音拼写，强化记忆",
                                "翻卡片复习，自评掌握程度",
                                "填空练习，掌握常用搭配",
                            ];
                            let icons = ["🎙", "📖", "✍", "🧠", "🧩"];
                            boxed(
                                link()
                                    .class("panel review-mode")
                                    .target(format!("/review/session/{kind}"))
                                    .fill((
                                        text().class("scene-emoji").content(icons[i]),
                                        view().class("section-heading").fill((
                                            heading(*label),
                                            text().class("badge").content(
                                                data.data.map(move |v| string(v, &count_kind)),
                                            ),
                                        )),
                                        p(descriptions[i]),
                                    )),
                            )
                        })
                        .collect(),
                )),
                view().class("actions").fill((
                    nav("查看收藏词汇", "/favorites"),
                    nav("去词典添加生词", "/dict"),
                )),
            ))
            .build(scope)
    })
}
pub fn review_session(kind: String) -> impl Widget {
    widget(move |scope| {
        let data = Resource::new(scope);
        data.load(if kind == "smart" {
            "/api/learn/review/items?limit=20".into()
        } else {
            format!("/api/learn/review/items?type={kind}&limit=20")
        });
        let index = scope.cage(0usize);
        let correct = scope.cage(0usize);
        let kind = kind.clone();
        view()
            .fill((
                title("复习练习"),
                nav("← 返回复习", "/review"),
                data.status(),
                text().class("muted").content(data.data.map(move |v| {
                    format!(
                        "进度 {} / {}",
                        (index.get() + 1).min(items(v).len()),
                        items(v).len()
                    )
                })),
                each(data.data.map(move |v| {
                    items(v)
                        .get(index.get())
                        .cloned()
                        .into_iter()
                        .collect::<Vec<_>>()
                }))
                .key(|v| v.to_string())
                .seed(move |v| {
                    let mode = if kind == "smart" {
                        string(&v, "review_type")
                    } else {
                        kind.clone()
                    };
                    review_item(v, mode, data, index, correct)
                }),
                when(
                    data.data.map(move |v| index.get() < items(v).len()),
                    move || {
                        action_button()
                            .label("跳过此项")
                            .variant(sapid::ButtonVariant::Subtle)
                            .on_press(move || index.revise(|i| *i += 1))
                    },
                ),
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
            body.push(boxed(pronunciation_feedback(evaluation)));
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
        let subject_open = scope.cage(false);
        let refresh = move || {
            subject_open.revise(|b| *b = false);
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
                title("大声跟读"),
                p("听一句，读一句，逐步掌握自然的英语发音。"),
                subjects.status(),
                sapid::dialog("reading-subjects")
                    .open(subject_open)
                    .on_open_change(move |v| subject_open.revise(|b| *b = v))
                    .theme(theme::orange())
                    .label("选择跟读主题")
                    .title("选择跟读主题")
                    .close_label("关闭主题选择")
                    .content(move || {
                        view().class("choices subject-grid").fill((
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
                        ))
                    }),
                data.status(),
                text()
                    .class("reading-progress badge")
                    .content(data.data.map(move |v| {
                        format!(
                            "Lv.{} · {} / {}",
                            string(v, "user_level"),
                            (index.get() + 1).min(items(v).len()),
                            items(v).len()
                        )
                    })),
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
                    super::sentence_audio::player(
                        "read",
                        audio_id.as_i64().unwrap_or_default(),
                        if v["audio_path"].as_str().is_some_and(|s| !s.is_empty()) {
                            Some(format!("/api/asset/read/sentences/{audio_id}/audio"))
                        } else {
                            None
                        },
                        "播放原音",
                    ),
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
                pronunciation_feedback(evaluation),
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

fn pronunciation_feedback(evaluation: Resource) -> impl Widget {
    view().class("pronunciation-feedback").fill((
        evaluation.status(),
        when(evaluation.data.map(|v| !v.is_null()), move || {
            view().class("panel").fill((
                heading("发音反馈"),
                view().class("stat-grid").fill((
                    dashboard::stat(evaluation, "综合评分", "overall_score", ""),
                    dashboard::stat(evaluation, "发音", "pronunciation_score", ""),
                    dashboard::stat(evaluation, "流利度", "fluency_score", ""),
                    dashboard::stat(evaluation, "语调", "intonation_score", ""),
                )),
                text().content(
                    evaluation
                        .data
                        .map(|v| format!("识别结果：{}", string(v, "transcribed_text"))),
                ),
                text().content(evaluation.data.map(|v| display_value(&v["feedback"]))),
            ))
        }),
    ))
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
                super::sentence_audio::player(
                    "script",
                    id.as_i64().unwrap_or_default(),
                    if v["audio_path"].as_str().is_some_and(|s| !s.is_empty()) {
                        Some(format!("/api/asset/scripts/turns/{id}/audio"))
                    } else {
                        None
                    },
                    "播放原音",
                ),
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
                        pronunciation_feedback(evaluation),
                    ))
                })
                .otherwise(|| nav("登录后跟读练习", "/login")),
            ))
            .build(scope)
    })
}
