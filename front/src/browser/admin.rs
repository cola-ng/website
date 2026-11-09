use super::*;
use glory::render::DocumentPart;

#[derive(Clone, Copy)]
struct Field {
    key: &'static str,
    kind: Kind,
    required: bool,
}
#[derive(Clone, Copy)]
enum Kind {
    Text,
    Number,
    Bool,
    Long,
}
const fn t(key: &'static str) -> Field {
    Field {
        key,
        kind: Kind::Text,
        required: false,
    }
}
const fn r(key: &'static str) -> Field {
    Field {
        key,
        kind: Kind::Text,
        required: true,
    }
}
const fn n(key: &'static str) -> Field {
    Field {
        key,
        kind: Kind::Number,
        required: false,
    }
}
const fn b(key: &'static str) -> Field {
    Field {
        key,
        kind: Kind::Bool,
        required: false,
    }
}
const fn long(key: &'static str) -> Field {
    Field {
        key,
        kind: Kind::Long,
        required: false,
    }
}
const WORD: &[Field] = &[
    r("word"),
    t("word_type"),
    t("language"),
    n("frequency"),
    n("difficulty"),
    n("syllable_count"),
    n("word_count"),
    b("is_lemma"),
    b("is_active"),
    long("memory"),
    long("notes"),
];
const STAGE: &[Field] = &[
    r("code"),
    r("name_en"),
    r("name_zh"),
    long("description_en"),
    long("description_zh"),
    t("icon_emoji"),
    n("display_order"),
    n("difficulty"),
    b("is_active"),
    long("ai_prompt"),
];
const DOMAIN: &[Field] = &[r("code"), r("name_en"), r("name_zh")];
const CATEGORY: &[Field] = &[
    r("code"),
    r("name_en"),
    r("name_zh"),
    Field {
        key: "domain_id",
        kind: Kind::Number,
        required: true,
    },
    n("parent_id"),
];
const SUBJECT: &[Field] = &[
    r("code"),
    r("title_en"),
    r("title_zh"),
    long("description_en"),
    long("description_zh"),
    t("icon_emoji"),
    n("difficulty"),
    t("subject_type"),
];
const SENTENCE: &[Field] = &[
    Field {
        key: "subject_id",
        kind: Kind::Number,
        required: true,
    },
    Field {
        key: "sentence_order",
        kind: Kind::Number,
        required: true,
    },
    r("content_en"),
    r("content_zh"),
    t("phonetic_transcription"),
    t("audio_path"),
    n("difficulty"),
];
const DEFINITION: &[Field] = &[
    r("language"),
    r("definition"),
    t("part_of_speech"),
    n("definition_order"),
    t("register"),
    t("region"),
    long("context"),
    long("usage_notes"),
    b("is_primary"),
    b("is_summary"),
];
const PRONUNCIATION: &[Field] = &[
    r("ipa"),
    t("dialect"),
    t("gender"),
    t("audio_url"),
    t("audio_path"),
    b("is_primary"),
];

pub fn page(path: Vec<String>) -> impl Widget {
    widget(move |scope| {
        let section = path.first().map(String::as_str).unwrap_or("");
        let id = path
            .last()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0);
        let content = match section {
            "" => boxed(dashboard()),
            "users" => {
                if let Some(id) = id {
                    boxed(user(id))
                } else {
                    boxed(users())
                }
            }
            "finance" => boxed(finance(path.get(1).cloned())),
            "dict" => boxed(view().fill((
                crud("词典管理", "/api/dict/words".into(), WORD, id),
                when(id.is_some(), move || {
                    let id = id.unwrap();
                    view().fill((
                        crud(
                            "释义",
                            format!("/api/dict/words/{id}/definitions"),
                            DEFINITION,
                            None,
                        ),
                        crud(
                            "发音",
                            format!("/api/dict/words/{id}/pronunciations"),
                            PRONUNCIATION,
                            None,
                        ),
                    ))
                }),
            ))),
            "taxonomy" => {
                let category = path.get(1).is_some_and(|s| s == "categories");
                boxed(
                    view().fill((
                        title("分类管理"),
                        sapid::tabs("taxonomy-tabs")
                            .theme(theme::orange())
                            .default_value(if category { "categories" } else { "domains" })
                            .tab(sapid::tab("domains").label("领域").content(move || {
                                crud(
                                    "领域管理",
                                    "/api/asset/domains".into(),
                                    DOMAIN,
                                    if !category { id } else { None },
                                )
                            }))
                            .tab(sapid::tab("categories").label("分类").content(move || {
                                crud(
                                    "分类管理",
                                    "/api/asset/categories".into(),
                                    CATEGORY,
                                    if category { id } else { None },
                                )
                            })),
                    )),
                )
            }
            "chat-scenarios" => boxed(crud("对话场景管理", "/api/admin/stages".into(), STAGE, id)),
            "read" => boxed(
                view().fill((
                    title("跟读管理"),
                    sapid::tabs("read-management")
                        .theme(theme::orange())
                        .default_value("subjects")
                        .tab(sapid::tab("subjects").label("跟读主题").content(|| {
                            crud(
                                "跟读主题管理",
                                "/api/admin/read-subjects".into(),
                                SUBJECT,
                                None,
                            )
                        }))
                        .tab(sapid::tab("sentences").label("跟读句子").content(|| {
                            crud(
                                "跟读句子管理",
                                "/api/admin/read-sentences".into(),
                                SENTENCE,
                                None,
                            )
                        })),
                )),
            ),
            _ => boxed(view().fill((title("管理页面不存在"), nav("返回管理首页", "/admin")))),
        };
        view()
            .fill((
                view()
                    .role(Role::Navigation)
                    .name("后台导航")
                    .class("admin-nav")
                    .fill((
                        nav("概览", "/admin"),
                        section_nav("用户", "/admin/users"),
                        section_nav("财务", "/admin/finance"),
                        section_nav("词典", "/admin/dict"),
                        section_nav("分类", "/admin/taxonomy"),
                        section_nav("对话场景", "/admin/chat-scenarios"),
                        section_nav("跟读内容", "/admin/read"),
                    )),
                nodes(vec![content]),
            ))
            .build(scope)
    })
}
fn dashboard() -> impl Widget {
    widget(|scope| {
        let stats = Resource::new(scope);
        stats.load("/api/admin/stats");
        let users = Resource::new(scope);
        users.load("/api/admin/stats/users/daily?days=30");
        let ai = Resource::new(scope);
        ai.load("/api/admin/stats/ai/today");
        let top = Resource::new(scope);
        top.load("/api/admin/stats/ai/top-users?limit=20");
        view()
            .fill((
                title("管理概览"),
                view().class("stat-grid").fill((
                    dashboard::stat(stats, "注册用户", "total_users", "人"),
                    dashboard::stat(stats, "今日新增", "users_today", "人"),
                    dashboard::stat(stats, "词典单词", "total_words", "个"),
                    dashboard::stat(stats, "管理员", "total_admins", "人"),
                )),
                stats.status(),
                view().class("panel").fill((
                    heading("用户增长"),
                    table(
                        users,
                        &[
                            ("date", "日期"),
                            ("new_users", "新增用户"),
                            ("total_users", "累计用户"),
                        ],
                    ),
                )),
                view().class("panel").fill((
                    heading("今日 AI 使用"),
                    ai.status(),
                    view().class("stat-grid").fill((
                        dashboard::stat(ai, "请求数", "total_requests", "次"),
                        dashboard::stat(ai, "Token 用量", "total_tokens", ""),
                        dashboard::stat(ai, "语音识别", "total_asr_seconds", "秒"),
                        dashboard::stat(ai, "语音合成", "total_tts_seconds", "秒"),
                    )),
                )),
                view().class("panel").fill((
                    heading("AI 使用排行"),
                    table(
                        top,
                        &[
                            ("user_id", "用户编号"),
                            ("user_name", "名称"),
                            ("total_requests", "请求数"),
                            ("total_tokens", "Token 用量"),
                        ],
                    ),
                )),
            ))
            .build(scope)
    })
}
fn users() -> impl Widget {
    widget(|scope| {
        let data = Resource::new(scope);
        let search = scope.cage(String::new());
        let page = scope.cage(1_i64);
        let action = Resource::new(scope);
        let administrators = Resource::new(scope);
        administrators.load("/api/admin/admins");
        let user_id = scope.cage(String::new());
        let refresh = move || {
            data.load(format!(
                "/api/admin/users?page={}&per_page=30&{}",
                page.get(),
                query("search", &search.get())
            ))
        };
        reset_pagination(scope, page, search.into());
        scope.effect(refresh);
        view()
            .fill((
                title("用户管理"),
                field("搜索用户", search),
                data.status(),
                empty(data, "没有匹配的用户。"),
                table(
                    data,
                    &[
                        ("id", "编号"),
                        ("name", "名称"),
                        ("handle", "Handle"),
                        ("email", "邮箱"),
                        ("created_at", "注册时间"),
                    ],
                ),
                pagination(page, data),
                heading("管理员"),
                account::data_section("管理员列表", administrators),
                field("用户编号", user_id),
                action_button()
                    .label("授予管理员权限")
                    .enabled(action.busy.map(|b| !*b))
                    .on_press(move || {
                        let Ok(id) = user_id.get().parse::<i64>() else {
                            action.error.revise(|e| *e = "请输入有效用户编号".into());
                            return;
                        };
                        if confirm("确认授予此用户管理员权限？") {
                            action.run(
                                "POST",
                                "/api/admin/admins/grant".into(),
                                Some(json!({"user_id":id})),
                                move |_| administrators.load("/api/admin/admins"),
                            );
                        }
                    }),
                danger_button()
                    .label("撤销管理员权限")
                    .enabled(action.busy.map(|b| !*b))
                    .on_press(move || {
                        let Ok(id) = user_id.get().parse::<i64>() else {
                            action.error.revise(|e| *e = "请输入有效用户编号".into());
                            return;
                        };
                        if confirm("确认撤销此用户的管理员权限？") {
                            action.run(
                                "POST",
                                "/api/admin/admins/revoke".into(),
                                Some(json!({"user_id":id})),
                                move |_| administrators.load("/api/admin/admins"),
                            );
                        }
                    }),
                action.status(),
            ))
            .build(scope)
    })
}
fn user(id: i64) -> impl Widget {
    widget(move |scope| {
        let data = Resource::new(scope);
        data.load(format!("/api/admin/users/{id}"));
        view()
            .fill((
                title("用户详情"),
                nav("← 用户列表", "/admin/users"),
                account::data_section("账户与学习数据", data),
            ))
            .build(scope)
    })
}
fn finance(tab: Option<String>) -> impl Widget {
    widget(move |scope| {
        let stats = Resource::new(scope);
        stats.load("/api/admin/stats/finance");
        let orders = Resource::new(scope);
        let page = scope.cage(1_i64);
        let search = scope.cage(String::new());
        let status = scope.cage(String::new());
        let subscriptions = tab.as_deref() == Some("subscriptions");
        let endpoint = if subscriptions {
            "/api/admin/subscriptions"
        } else {
            "/api/admin/orders"
        };
        reset_pagination(
            scope,
            page,
            scope
                .bond(move || format!("{}\0{}", search.get(), status.get()))
                .into(),
        );
        scope.effect(move || {
            orders.load(format!(
                "{endpoint}?page={}&per_page=30&{}&{}",
                page.get(),
                query("search", &search.get()),
                query("status", &status.get())
            ))
        });
        view()
            .fill((
                title("财务管理"),
                view().class("stat-grid").fill((
                    money_stat(stats, "总收入", "total_revenue_cents"),
                    money_stat(stats, "今日收入", "revenue_today_cents"),
                    dashboard::stat(stats, "订单数量", "total_orders", "笔"),
                    dashboard::stat(stats, "有效订阅", "active_subscriptions", "个"),
                )),
                stats.status(),
                view().class("actions").fill((
                    nav(
                        "订单",
                        if tab.is_none() {
                            "/admin/finance"
                        } else {
                            "/admin/finance/orders"
                        },
                    ),
                    nav("订阅", "/admin/finance/subscriptions"),
                )),
                field("搜索", search),
                field("状态筛选（留空显示全部）", status),
                orders.status(),
                empty(orders, "暂无记录。"),
                table(
                    orders,
                    if subscriptions {
                        &[
                            ("id", "编号"),
                            ("user_id", "用户"),
                            ("plan_code", "套餐"),
                            ("status", "状态"),
                            ("expires_at", "到期时间"),
                        ]
                    } else {
                        &[
                            ("order_no", "订单号"),
                            ("user_id", "用户"),
                            ("plan_code", "套餐"),
                            ("amount_cents", "金额（分）"),
                            ("status", "状态"),
                            ("created_at", "创建时间"),
                        ]
                    },
                ),
                pagination(page, orders),
            ))
            .build(scope)
    })
}
fn money_stat(resource: Resource, label: &'static str, key: &'static str) -> impl Widget {
    view().class("stat-card").fill((
        text().class("muted").content(label),
        text()
            .class("stat-value")
            .content(resource.data.map(move |v| {
                v[key]
                    .as_f64()
                    .map(|c| format!("¥ {:.2}", c / 100.))
                    .unwrap_or_else(|| "—".into())
            })),
    ))
}
fn table(resource: Resource, columns: &'static [(&'static str, &'static str)]) -> impl Widget {
    view().fill((
        resource.status(),
        empty(resource, "暂无记录。"),
        view()
            .class("table-scroll")
            .role(Role::Group)
            .name("数据表格，可横向滚动")
            .focus(glory::render::FocusMode::TabStop)
            .fill(
                view()
                    .document_part(DocumentPart::Table)
                    .class("data-table")
                    .fill((
                        view().document_part(DocumentPart::TableHead).fill(
                            view().document_part(DocumentPart::TableRow).fill(nodes(
                                columns
                                    .iter()
                                    .map(|(_, name)| {
                                        boxed(
                                            view()
                                                .document_part(DocumentPart::ColumnHeader)
                                                .fill(text().content(*name)),
                                        )
                                    })
                                    .collect(),
                            )),
                        ),
                        view().document_part(DocumentPart::TableBody).fill(
                            each(resource.data.map(items))
                                .key(|v| v.to_string())
                                .seed(move |v| {
                                    view().document_part(DocumentPart::TableRow).fill(nodes(
                                        columns
                                            .iter()
                                            .map(|(key, name)| {
                                                let cell: Box<dyn Widget> = if *key == "id"
                                                    || *key == "user_id"
                                                {
                                                    boxed(nav(
                                                        display_value(&v[*key]),
                                                        format!("/admin/users/{}", string(&v, key)),
                                                    ))
                                                } else {
                                                    boxed(text().content(display_value(&v[*key])))
                                                };
                                                boxed(
                                                    view()
                                                        .document_part(DocumentPart::TableCell)
                                                        .fill((cell_label(*name), cell)),
                                                )
                                            })
                                            .collect(),
                                    ))
                                }),
                        ),
                    )),
            ),
    ))
}
fn pagination(page: Cage<i64>, data: Resource) -> impl Widget {
    view().class("actions").fill((
        action_button()
            .label("上一页")
            .enabled(page.map(move |n| *n > 1 && !data.busy.get()))
            .on_press(move || page.revise(|n| *n -= 1)),
        text().content(page.map(|n| format!("第 {n} 页"))),
        action_button()
            .label("下一页")
            .enabled(data.data.map(move |v| {
                if data.busy.get() {
                    return false;
                }
                let per = v["per_page"].as_i64().unwrap_or(30);
                v["total"]
                    .as_i64()
                    .map(|total| page.get() * per < total)
                    .unwrap_or_else(|| items(v).len() >= per as usize)
            }))
            .on_press(move || page.revise(|n| *n += 1)),
    ))
}
fn defaults(fields: &[Field]) -> Value {
    let mut v = json!({});
    for field in fields {
        v[field.key] = if matches!(field.kind, Kind::Bool) {
            json!(field.key == "is_active")
        } else {
            Value::Null
        };
    }
    v
}
fn cell_label(value: impl Into<String>) -> impl Widget {
    view()
        .class("cell-label")
        .semantics(Some(Box::new(Semantics {
            inert: true,
            ..Default::default()
        })))
        .fill(text().content(value.into()))
}
fn selection_cell(
    id: i64,
    label_text: &'static str,
    selected: Cage<Vec<i64>>,
    action: Resource,
) -> impl Widget {
    view().document_part(DocumentPart::TableCell).fill((
        cell_label("编号"),
        view().class("cell-main").fill((
            text().content(id.to_string()),
            when(label_text == "词典管理", move || {
                checkbox_control()
                    .enabled(action.busy.map(|b| !*b))
                    .name(format!("选择单词 {id}"))
                    .label("")
                    .checked(selected.map(move |ids| ids.contains(&id)))
                    .on_checked_change(move |checked| {
                        selected.revise(|ids| {
                            ids.retain(|v| *v != id);
                            if checked {
                                ids.push(id);
                            }
                        })
                    })
            }),
        )),
    ))
}
fn visible_fields(fields: &[Field]) -> Vec<Field> {
    let keys: &[&str] = if fields.iter().any(|f| f.key == "word") {
        &["word", "is_active", "difficulty", "frequency", "created_at"]
    } else if fields.iter().any(|f| f.key == "content_en") {
        &["content_en", "content_zh", "difficulty", "subject_id"]
    } else if fields.iter().any(|f| f.key == "name_zh")
        && fields.iter().any(|f| f.key == "is_active")
    {
        &["name_zh", "name_en", "difficulty", "is_active"]
    } else if fields.iter().any(|f| f.key == "title_zh") {
        &["title_zh", "title_en", "code", "is_active"]
    } else {
        return fields
            .iter()
            .filter(|f| !matches!(f.kind, Kind::Long))
            .take(5)
            .copied()
            .collect();
    };
    keys.iter()
        .map(|key| {
            fields
                .iter()
                .find(|f| f.key == *key)
                .copied()
                .unwrap_or_else(|| t(key))
        })
        .collect()
}
fn record_value(value: &Value, key: &str) -> impl Widget + use<> {
    if key == "is_active" {
        boxed(
            text()
                .class(if value[key] == true {
                    "status-badge active"
                } else {
                    "status-badge inactive"
                })
                .content(if value[key] == true {
                    "启用"
                } else {
                    "停用"
                }),
        )
    } else {
        boxed(
            text()
                .class("record-cell")
                .content(display_value(&value[key])),
        )
    }
}
fn crud(
    label_text: &'static str,
    endpoint: String,
    fields: &'static [Field],
    initial: Option<i64>,
) -> impl Widget {
    widget(move |scope| {
        let data = Resource::new(scope);
        let action = Resource::new(scope);
        let draft = scope.cage(defaults(fields));
        let edit_id = scope.cage(initial);
        let editor_open = scope.cage(initial.is_some());
        let attempted = scope.cage(false);
        let page = scope.cage(1_i64);
        let search = scope.cage(String::new());
        let notice = scope.cage(String::new());
        let selected = scope.cage(Vec::<i64>::new());
        let base = endpoint.clone();
        let refresh_base = base.clone();
        let refresh = std::rc::Rc::new(move || {
            data.load(format!(
                "{refresh_base}?page={}&per_page=30&{}",
                page.get(),
                query("search", &search.get())
            ))
        });
        let initial_refresh = refresh.clone();
        reset_pagination(scope, page, search.into());
        scope.effect(move || initial_refresh());
        if let Some(id) = initial {
            let endpoint = base.clone();
            spawn(async move {
                match request("GET", &format!("{endpoint}/{id}"), None).await {
                    Ok(v) => {
                        if draft.is_alive() {
                            draft.revise(|d| *d = v);
                        }
                    }
                    Err(e) => {
                        if action.error.is_alive() {
                            action.error.revise(|s| *s = e);
                        }
                    }
                }
            });
        }
        let save_base = base.clone();
        let save_refresh = refresh.clone();
        let rows_base = base.clone();
        let batch_refresh = refresh.clone();
        view()
            .class("admin-section")
            .fill((
                view().class("section-heading").fill((
                    heading(label_text),
                    action_button().label("新建记录").on_press(move || {
                        attempted.revise(|v| *v = false);
                        edit_id.revise(|v| *v = None);
                        draft.revise(|d| *d = defaults(fields));
                        editor_open.revise(|v| *v = true);
                    }),
                )),
                when(label_text == "词典管理", move || {
                    let refresh = batch_refresh.clone();
                    let delete_refresh = refresh.clone();
                    view().class("actions").fill((
                        action_button()
                            .label("选择本页全部单词")
                            .variant(sapid::ButtonVariant::Outline)
                            .on_press(move || {
                                selected.revise(|ids| {
                                    *ids = items(&data.data.get())
                                        .iter()
                                        .filter_map(|v| v["id"].as_i64())
                                        .collect()
                                })
                            }),
                        action_button()
                            .label("批量停用")
                            .enabled(selected.map(move |ids| !ids.is_empty() && !action.busy.get()))
                            .on_press(move || {
                                if confirm("停用选中的单词？") {
                                    batch_words(selected, action, notice, refresh.clone(), false);
                                }
                            }),
                        action_button()
                            .label("批量删除")
                            .enabled(selected.map(move |ids| !ids.is_empty() && !action.busy.get()))
                            .variant(sapid::ButtonVariant::Danger)
                            .on_press(move || {
                                if confirm("永久删除选中的单词？") {
                                    batch_words(
                                        selected,
                                        action,
                                        notice,
                                        delete_refresh.clone(),
                                        true,
                                    );
                                }
                            }),
                    ))
                }),
                field("搜索内容", search),
                data.status(),
                empty(data, "没有匹配的记录。"),
                view()
                    .class("table-scroll")
                    .role(Role::Group)
                    .name("数据表格，可横向滚动")
                    .focus(glory::render::FocusMode::TabStop)
                    .fill(
                        view()
                            .document_part(DocumentPart::Table)
                            .class("data-table")
                            .fill((
                                view().document_part(DocumentPart::TableHead).fill(
                                    view().document_part(DocumentPart::TableRow).fill((
                                        view()
                                            .document_part(DocumentPart::ColumnHeader)
                                            .fill(text().content("编号")),
                                        nodes(
                                            visible_fields(fields)
                                                .into_iter()
                                                .map(|f| {
                                                    boxed(
                                                        view()
                                                            .document_part(
                                                                DocumentPart::ColumnHeader,
                                                            )
                                                            .fill(text().content(label(f.key))),
                                                    )
                                                })
                                                .collect(),
                                        ),
                                        view()
                                            .document_part(DocumentPart::ColumnHeader)
                                            .fill(text().content("操作")),
                                    )),
                                ),
                                view().document_part(DocumentPart::TableBody).fill(
                                    each(data.data.map(items)).key(|v| v.to_string()).seed(
                                        move |v| {
                                            let id = v["id"].as_i64().unwrap_or_default();
                                            let editing = v.clone();
                                            let base = rows_base.clone();
                                            let refresh = refresh.clone();
                                            view()
                                                .document_part(DocumentPart::TableRow)
                                                .class("record")
                                                .fill((
                                                    selection_cell(
                                                        id, label_text, selected, action,
                                                    ),
                                                    nodes(
                                                        visible_fields(fields)
                                                            .into_iter()
                                                            .map(|f| {
                                                                boxed(
                                                                    view()
                                                                        .document_part(
                                                                            DocumentPart::TableCell,
                                                                        )
                                                                        .fill((
                                                                            cell_label(label(
                                                                                f.key,
                                                                            )),
                                                                            record_value(&v, f.key),
                                                                        )),
                                                                )
                                                            })
                                                            .collect(),
                                                    ),
                                                    view()
                                                        .document_part(DocumentPart::TableCell)
                                                        .fill(
                                                        view().class("actions").fill((
                                                            action_button()
                                                                .label("编辑")
                                                                .enabled(action.busy.map(|b| !*b))
                                                                .variant(
                                                                    sapid::ButtonVariant::Outline,
                                                                )
                                                                .on_press(move || {
                                                                    editor_open
                                                                        .revise(|v| *v = true);
                                                                    attempted
                                                                        .revise(|a| *a = false);
                                                                    edit_id
                                                                        .revise(|i| *i = Some(id));
                                                                    draft.revise(|d| {
                                                                        *d = editing.clone()
                                                                    });
                                                                    notice.revise(String::clear);
                                                                }),
                                                            when(
                                                                label_text == "词典管理",
                                                                move || {
                                                                    nav(
                                                                        "编辑释义和发音",
                                                                        format!("/admin/dict/{id}"),
                                                                    )
                                                                },
                                                            ),
                                                            danger_button()
                                                                .label("删除")
                                                                .enabled(action.busy.map(|b| !*b))
                                                                .enabled(action.busy.map(|b| !*b))
                                                                .on_press(move || {
                                                                    if confirm("确认删除此记录？")
                                                                    {
                                                                        let refresh =
                                                                            refresh.clone();
                                                                        action.run(
                                                                            "DELETE",
                                                                            format!("{base}/{id}"),
                                                                            None,
                                                                            move |_| {
                                                                                refresh();
                                                                                notice.revise(|n| {
                                                                                *n = "记录已删除"
                                                                                    .into()
                                                                            });
                                                                            },
                                                                        );
                                                                    }
                                                                }),
                                                        )),
                                                    ),
                                                ))
                                        },
                                    ),
                                ),
                            )),
                    ),
                pagination(page, data),
                success_notice(notice, action),
                action.status(),
                sapid::dialog(format!("editor-{}", endpoint.replace('/', "-")))
                    .theme(theme::orange())
                    .show_trigger(false)
                    .open(editor_open)
                    .on_open_change(move |v| editor_open.revise(|b| *b = v))
                    .title(label_text)
                    .close_label("关闭记录编辑")
                    .content(move || {
                        let save_base = save_base.clone();
                        let save_refresh = save_refresh.clone();
                        view().class("editor dialog-form").fill((
                            text().kind(TextKind::Heading(3)).content(edit_id.map(|id| {
                                id.map(|id| format!("编辑记录 #{id}"))
                                    .unwrap_or_else(|| "新建记录".into())
                            })),
                            nodes(
                                fields
                                    .iter()
                                    .map(|spec| {
                                        let spec = *spec;
                                        let key = spec.key;
                                        let label_text = format!(
                                            "{}{}",
                                            label(key),
                                            if spec.required { " *" } else { "" }
                                        );
                                        if matches!(spec.kind, Kind::Bool) {
                                            boxed(
                                                checkbox_control()
                                                    .enabled(action.busy.map(|b| !*b))
                                                    .label(label_text)
                                                    .checked(draft.map(move |v| {
                                                        v[key].as_bool().unwrap_or(false)
                                                    }))
                                                    .on_checked_change(move |v| {
                                                        draft.revise(|d| d[key] = json!(v))
                                                    }),
                                            )
                                        } else {
                                            boxed(
                                                view().class("field").fill((
                                                    text().content(label_text.clone()),
                                                    text_control()
                                                        .enabled(action.busy.map(|b| !*b))
                                                        .invalid(draft.map(move |value| {
                                                            if !attempted.get() {
                                                                return false;
                                                            }
                                                            let raw = string(value, key);
                                                            (spec.required && raw.trim().is_empty())
                                                                || (matches!(
                                                                    spec.kind,
                                                                    Kind::Number
                                                                ) && !raw.trim().is_empty()
                                                                    && raw
                                                                        .trim()
                                                                        .parse::<i64>()
                                                                        .is_err())
                                                        }))
                                                        .name(label_text)
                                                        .multiline(matches!(spec.kind, Kind::Long))
                                                        .purpose(
                                                            if matches!(spec.kind, Kind::Number) {
                                                                Purpose::Number
                                                            } else {
                                                                Purpose::Text
                                                            },
                                                        )
                                                        .value(draft.map(move |v| string(v, key)))
                                                        .on_value_change(move |v| {
                                                            draft.revise(|d| d[key] = json!(v))
                                                        }),
                                                )),
                                            )
                                        }
                                    })
                                    .collect(),
                            ),
                            action_button()
                                .label("保存记录")
                                .enabled(action.busy.map(|b| !*b))
                                .on_press(move || {
                                    attempted.revise(|a| *a = true);
                                    let mut body = json!({});
                                    let value = draft.get();
                                    for field in fields {
                                        let raw = string(&value, field.key);
                                        if field.required && raw.trim().is_empty() {
                                            action.error.revise(|e| {
                                                *e = format!("请填写{}", label(field.key))
                                            });
                                            return;
                                        }
                                        body[field.key] = match field.kind {
                                            Kind::Bool => {
                                                json!(value[field.key].as_bool().unwrap_or(false))
                                            }
                                            Kind::Number => {
                                                if raw.trim().is_empty() {
                                                    Value::Null
                                                } else {
                                                    match raw.trim().parse::<i64>() {
                                                        Ok(n) => json!(n),
                                                        Err(_) => {
                                                            action.error.revise(|e| {
                                                                *e = format!(
                                                                    "{}必须为整数",
                                                                    label(field.key)
                                                                )
                                                            });
                                                            return;
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {
                                                if raw.trim().is_empty() && !field.required {
                                                    Value::Null
                                                } else {
                                                    json!(raw.trim())
                                                }
                                            }
                                        };
                                    }
                                    let (method, path) = edit_id
                                        .get()
                                        .map(|id| ("PUT", format!("{save_base}/{id}")))
                                        .unwrap_or(("POST", save_base.clone()));
                                    let refresh = save_refresh.clone();
                                    action.run(method, path, Some(body), move |_| {
                                        attempted.revise(|a| *a = false);
                                        edit_id.revise(|i| *i = None);
                                        draft.revise(|d| *d = defaults(fields));
                                        editor_open.revise(|v| *v = false);
                                        notice.revise(|n| *n = "记录已保存".into());
                                        refresh();
                                    });
                                }),
                            action_button()
                                .label("取消编辑")
                                .enabled(action.busy.map(|b| !*b))
                                .variant(sapid::ButtonVariant::Outline)
                                .on_press(move || {
                                    editor_open.revise(|v| *v = false);
                                    attempted.revise(|a| *a = false);
                                    edit_id.revise(|i| *i = None);
                                    draft.revise(|d| *d = defaults(fields));
                                }),
                            action.status(),
                            success_notice(notice, action),
                        ))
                    }),
            ))
            .build(scope)
    })
}

fn batch_words(
    selected: Cage<Vec<i64>>,
    action: Resource,
    notice: Cage<String>,
    refresh: std::rc::Rc<dyn Fn()>,
    delete: bool,
) {
    if action.busy.get() {
        return;
    }
    let ids = selected.get();
    action.busy.revise(|b| *b = true);
    action.error.revise(String::clear);
    spawn(async move {
        let mut completed = 0;
        for id in ids {
            let result = request(
                if delete { "DELETE" } else { "PUT" },
                &format!("/api/dict/words/{id}"),
                if delete {
                    None
                } else {
                    Some(json!({"is_active":false}))
                },
            )
            .await;
            if !selected.is_alive() {
                return;
            }
            match result {
                Ok(_) => {
                    completed += 1;
                    selected.revise(|ids| ids.retain(|v| *v != id));
                }
                Err(error) => {
                    action
                        .error
                        .revise(|e| *e = format!("已处理 {completed} 项；{error}"));
                    action.busy.revise(|b| *b = false);
                    refresh();
                    return;
                }
            }
        }
        action.busy.revise(|b| *b = false);
        notice.revise(|n| *n = format!("已处理 {completed} 项"));
        refresh();
    });
}
