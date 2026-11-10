use super::*;
pub struct Nodes(pub Vec<Box<dyn Widget>>);
impl Widget for Nodes {
    fn build(&mut self, scope: &Scope) -> glory::prelude::Result {
        for widget in std::mem::take(&mut self.0) {
            widget.show_in(scope)?;
        }
        Ok(())
    }
}
pub fn nodes(children: Vec<Box<dyn Widget>>) -> Nodes {
    Nodes(children)
}
pub fn boxed(w: impl Widget) -> Box<dyn Widget> {
    Box::new(w)
}
pub fn title(value: impl Into<String>) -> impl Widget {
    text().kind(TextKind::Heading(1)).content(value.into())
}
pub fn heading(value: impl Into<String>) -> impl Widget {
    text().kind(TextKind::Heading(2)).content(value.into())
}
pub fn p(value: impl Into<String>) -> impl Widget {
    text().kind(TextKind::Paragraph).content(value.into())
}
pub fn nav(label: impl Into<String>, path: impl Into<String>) -> impl Widget {
    navigation_link(label.into(), path.into(), false)
}
pub fn section_nav(label: impl Into<String>, path: impl Into<String>) -> impl Widget {
    navigation_link(label.into(), path.into(), true)
}
pub fn route_link(path: impl Into<String>) -> glory::ui::Link {
    let path = path.into();
    let internal = internal_location(&path).is_some();
    let target = path.clone();
    let link = link().target(path);
    if internal {
        link.semantics(Some(Box::new(Semantics {
            link_request: true,
            ..Default::default()
        })))
        .on(activate, move |_| go(&target))
    } else {
        link
    }
}
fn navigation_link(label: String, path: String, section: bool) -> impl Widget {
    let current_path = window().location().pathname().unwrap_or_default();
    let target = path.split(['?', '#']).next().unwrap_or_default();
    let current =
        if current_path == target || section && target == "/dict" && current_path == "/translate" {
            Some(Current::Page)
        } else if section
            && target != "/"
            && current_path
                .strip_prefix(target)
                .is_some_and(|tail| tail.starts_with('/'))
        {
            Some(Current::Location)
        } else {
            None
        };
    let internal = internal_location(&path).is_some();
    route_link(path)
        .label(label)
        .semantics(Some(Box::new(Semantics {
            current,
            link_request: internal,
            ..Default::default()
        })))
}
pub fn field(label: &str, value: Cage<String>) -> impl Widget + use<> {
    field_state(label, value, Purpose::Text, false.into(), true.into())
}
pub fn password(label: &str, value: Cage<String>) -> impl Widget + use<> {
    field_state(label, value, Purpose::Password, false.into(), true.into())
}
pub fn field_state(
    label: &str,
    value: Cage<String>,
    purpose: Purpose,
    invalid: Vary<bool>,
    enabled: Vary<bool>,
) -> impl Widget + use<> {
    view().class("field").fill((
        text().content(label.to_string()),
        text_control()
            .name(label.to_string())
            .purpose(purpose)
            .invalid(invalid)
            .enabled(enabled)
            .value(value)
            .on_value_change(move |v| value.revise(|s| *s = v)),
    ))
}
pub fn textarea(label: &str, value: Cage<String>) -> impl Widget + use<> {
    view().class("field").fill((
        text().content(label.to_string()),
        text_control()
            .name(label.to_string())
            .multiline(true)
            .value(value)
            .on_value_change(move |v| value.revise(|s| *s = v)),
    ))
}
/// Editable controls must explicitly opt in to Glory's submit command. An
/// ancestor alone cannot consume native editing keys (including IME input).
pub fn submit_field<F: FnMut() + 'static>(
    label: &str,
    value: Cage<String>,
    multiline: bool,
    mut submit: F,
) -> impl Widget + use<F> {
    view().class("field").fill((
        text().content(label.to_string()),
        text_control()
            .name(label.to_string())
            .multiline(multiline)
            .value(value)
            .on_value_change(move |next| value.revise(|v| *v = next))
            .on(glory::render::EventKind::Keyboard, move |event| {
                if let Event::Keyboard {
                    command: glory::render::KeyboardCommand::Submit,
                    ..
                } = event
                {
                    submit();
                }
            }),
    ))
}
pub fn choices(selected: Cage<String>, options: &[(&str, &str)]) -> impl Widget + use<> {
    choices_enabled(selected, options, true.into())
}
pub fn choices_enabled(
    selected: Cage<String>,
    options: &[(&str, &str)],
    enabled: Vary<bool>,
) -> impl Widget + use<> {
    view().class("choices").fill(nodes(
        options
            .iter()
            .map(|(key, label)| {
                let key = key.to_string();
                let active = key.clone();
                boxed(
                    action_button()
                        .label(label.to_string())
                        .enabled(enabled.clone())
                        .pressed(selected.map(move |v| v == &active))
                        .variant(sapid::ButtonVariant::Outline)
                        .on_press(move || selected.revise(|s| *s = key.clone())),
                )
            })
            .collect(),
    ))
}
pub fn confirm(label: &str) -> bool {
    window().confirm_with_message(label).unwrap_or(false)
}
pub fn empty(resource: Resource, message: &'static str) -> impl Widget {
    when(
        resource.data.map(move |v| {
            !resource.busy.get()
                && resource.error.get().is_empty()
                && !v.is_null()
                && items(v).is_empty()
        }),
        move || {
            view()
                .role(Role::Status)
                .class("empty-state")
                .fill(p(message))
        },
    )
}
pub fn success_notice(message: Cage<String>, resource: Resource) -> impl Widget {
    when(
        message
            .map(move |s| !s.is_empty() && !resource.busy.get() && resource.error.get().is_empty()),
        move || {
            view()
                .role(Role::Status)
                .class("success")
                .fill(text().content(message))
        },
    )
}
pub fn reset_pagination(scope: &Scope, page: Cage<i64>, filter: Vary<String>) {
    let previous = scope.cage(filter.get());
    scope.effect(move || {
        let next = filter.get();
        if previous.get_untracked() != next {
            previous.revise(|v| *v = next);
            page.revise(|v| *v = 1);
        }
    });
}
pub fn display_value(v: &Value) -> String {
    match v {
        Value::Null => "—".into(),
        Value::Bool(v) => if *v { "是" } else { "否" }.into(),
        Value::String(v) => v.clone(),
        Value::Array(v) => v.iter().map(display_value).collect::<Vec<_>>().join(" · "),
        Value::Object(v) => v
            .iter()
            .filter(|(k, _)| !k.contains("secret") && !k.contains("key") && *k != "password_hash")
            .map(|(k, v)| format!("{}：{}", label(k), display_value(v)))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => v.to_string(),
    }
}
pub fn details(value: &Value) -> impl Widget + use<> {
    let rows = value
        .as_object()
        .map(|v| {
            v.iter()
                .filter(|(k, v)| {
                    !v.is_null()
                        && !k.contains("secret")
                        && !k.contains("password")
                        && !k.contains("api_key")
                })
                .map(|(k, v)| {
                    boxed(view().class("detail").fill((
                        text().class("muted").content(label(k)),
                        text().content(display_value(v)),
                    )))
                })
                .collect()
        })
        .unwrap_or_default();
    view().class("details").fill(nodes(rows))
}
pub fn label(key: &str) -> String {
    match key {
        "id" => "编号",
        "name" => "名称",
        "title" => "标题",
        "title_en" => "英文标题",
        "title_zh" => "中文标题",
        "email" => "邮箱",
        "phone" => "手机",
        "created_at" => "创建时间",
        "updated_at" => "更新时间",
        "is_admin" => "管理员",
        "is_active" => "启用",
        "name_en" => "英文名称",
        "name_zh" => "中文名称",
        "description_en" => "英文说明",
        "description_zh" => "中文说明",
        "difficulty" => "难度",
        "display_order" => "显示顺序",
        "code" => "代码",
        "icon_emoji" => "图标",
        "ai_prompt" => "对话引导",
        "content_en" => "英文内容",
        "content_zh" => "中文内容",
        "subject_id" => "主题编号",
        "domain_id" => "领域编号",
        "category_id" => "分类编号",
        "parent_id" => "上级编号",
        "sentence_order" => "句子顺序",
        "audio_path" => "音频路径",
        "phonetic_transcription" => "音标",
        "word" => "单词",
        "definition" => "释义",
        "language" => "语言",
        "part_of_speech" => "词性",
        "ipa" => "国际音标",
        "dialect" => "口音",
        "notes" => "备注",
        "memory" => "记忆提示",
        "frequency" => "频率",
        "word_type" => "词语类型",
        "is_primary" => "首选",
        "is_summary" => "概要释义",
        "definition_order" => "释义顺序",
        "gender" => "声音",
        "overall_score" => "综合评分",
        "pronunciation_score" => "发音评分",
        "fluency_score" => "流利度",
        "intonation_score" => "语调",
        "feedback" => "改进建议",
        "transcribed_text" => "识别内容",
        "total" => "总计",
        "total_seconds" => "学习秒数",
        "today_seconds" => "今日秒数",
        "balance" => "余额",
        "status" => "状态",
        "amount" => "金额",
        "price" => "价格",
        "total_users" => "用户总数",
        "total_chats" => "对话总数",
        "total_words" => "单词总数",
        "is_pinned" => "置顶",
        "source" => "来源",
        "mastery_level" => "掌握程度",
        "next_review_at" => "下次复习",
        "new_level" => "当前等级",
        "success" => "成功",
        "success_rate" => "成功率",
        "weekly_chat_minutes" => "本周对话（分钟）",
        "mastered_vocabulary_count" => "已掌握单词",
        "pending_review_count" => "待复习单词",
        "issue_words_count" => "易错单词",
        "total_vocabulary_count" => "生词数量",
        "learning_days" => "学习天数",
        "total_review_times" => "复习次数",
        "average_mastery" => "平均掌握程度（%）",
        "weekly_minutes" => "本周学习时间",
        "daily_stats" => "每日学习时间",
        "stat_date" => "日期",
        "date" => "日期",
        "minutes" => "分钟",
        "word_zh" => "中文释义",
        "practice_count" => "练习次数",
        "correct_count" => "答对次数",
        "last_practiced_at" => "上次练习",
        "provider_type" => "服务商",
        "settings" => "配置",
        _ => return key.replace('_', " "),
    }
    .into()
}
