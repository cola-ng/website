use super::*;

pub fn landing(session: Cage<Value>) -> impl Widget {
    widget(move |scope| {
        let summary = Resource::new(scope);
        let daily = Resource::new(scope);
        let stages = Resource::new(scope);
        let history = Resource::new(scope);
        let word = scope.cage(String::new());
        let action = Resource::new(scope);
        stages.load("/api/asset/stages?limit=6");
        if token().is_some() {
            summary.load("/api/learn/summary");
            daily.load("/api/learn/daily");
            history.load("/api/dict/words/searched?limit=6");
        }
        view().class("dashboard").fill((
            view().class("dashboard-primary").fill((
                view().class("panel welcome-card").fill((
                    view().class("welcome-greeting").fill((
                    text().kind(TextKind::Heading(1)).content(session.map(|s| {
                        let name = string(s,"name");
                        if name.is_empty() {"欢迎来到开朗英语 👋".into()} else {format!("你好，{name} 👋")}
                    })),
                    p("今天想学点什么？从一次轻松的对话开始吧。"),
                    view().class("actions primary-actions").fill((nav("开始对话", "/chat"), nav("选择场景", "/stages"))),
                    )),
                    view().class("welcome-search").fill((view().class("search quick-search").fill((submit_field("查找单词或短语",word,false,move||{let value=word.get();if !value.trim().is_empty(){go(&dictionary_path(value.trim()));}}),action_button().label("查词").on_press(move || {
                        let value=word.get(); if !value.trim().is_empty(){go(&dictionary_path(value.trim()));}
                    }))),
                    view().class("word-chips").fill(each(history.data.map(items)).key(|v|v.to_string()).seed(|v|{let word=string(&v,"word");nav(word.clone(),dictionary_path(&word))})),)),
                )),
                learning_data(summary),
                view().class("panel").fill((view().class("section-heading").fill((heading("角色扮演"),nav("查看全部 →","/stages"))),stages.status(),view().class("scene-grid").fill(each(stages.data.map(items)).key(|v|v.to_string()).seed(|v|{
                    link().target(format!("/stages/{}",string(&v,"id"))).class("scene-card").fill((text().class("scene-emoji").content(if string(&v,"icon_emoji").is_empty(){"💬".into()}else{string(&v,"icon_emoji")}),text().content(string(&v,"name_zh")),text().class("muted").content(string(&v,"name_en"))))
                })),empty(stages,"场景正在准备中。"))),
            )),
            view().class("dashboard-secondary").fill((
                view().class("panel").fill((view().class("section-heading").fill((heading("今日任务"),text().class("badge").content(daily.data.map(|v|format!("{}/{}",items(&v["tasks"]).iter().filter(|t|t["status"]=="completed").count(),items(&v["tasks"]).len()))))),daily.status(),
                    each(daily.data.map(|v|items(&v["tasks"]))).key(|v|v.to_string()).seed(move|v|{
                        let id=string(&v,"id");let target=v["target_value"].clone();
                        view().class("task-card").fill((checkbox_control().label(string(&v,"task_title")).checked(v["status"]=="completed").enabled(action.busy.map(|b|!*b)).on_checked_change(move|done|action.run("PUT",format!("/api/learn/daily/tasks/{id}"),Some(json!({"status":if done{"completed"}else{"pending"},"current_value":if done{target.clone()}else{json!(0)}})),move |_|daily.load("/api/learn/daily"))),p(string(&v,"task_description"))))
                    }),action.status(),when(token().is_none(),||nav("登录后查看学习任务","/login")),
                )),
                view().class("panel").fill((heading("学习洞察"),each(daily.data.map(|v|items(&v["insights"]))).key(|v|v.to_string()).seed(|v|{
                    let path=string(&v,"action_link");view().class("insight-card").fill((p(string(&v,"content_zh")),when(path.starts_with('/')&&!path.starts_with("//"),move||nav(string(&v,"action_text"),path.clone()))))
                }),when(daily.data.map(|v|items(&v["insights"]).is_empty()),||view().class("insight-card").fill((p("坚持开口、及时复习，让每次学习都有收获。"),nav("开始今日复习 →","/review")))))),
                view().class("panel help-card").fill((heading("你的学习空间"),nav("我的收藏","/favorites"),nav("个人资料与 AI 设置","/me"),nav("订阅与钱包","/subscription"))),
            )),
        )).build(scope)
    })
}

fn learning_data(summary: Resource) -> impl Widget {
    widget(move |scope| {
        let period = scope.cage("day".to_string());
        let study = Resource::new(scope);
        if token().is_some() {
            study.load("/api/learn/study-time?days=185");
        }
        view().class("panel dashboard-data").fill((
            view().class("section-heading").fill((heading("学习数据"),nav("开始复习 →","/review"))),summary.status(),
            view().class("dashboard-data-body").fill((
                view().class("stat-grid").fill((stat(summary,"本周对话","weekly_chat_minutes","分钟"),stat(summary,"已掌握词汇","mastered_vocabulary_count","个"),stat(summary,"待复习","pending_review_count","项"),stat(summary,"学习天数","learning_days","天"))),
                view().class("study-chart").fill((view().class("section-heading").fill((text().class("muted").content("学习时长分布"),choices(period,&[("day","日"),("week","周"),("month","月")]))),study.status(),
                    view().class("study-columns").fill(each(scope.bond(move||{
                        let period=period.get();let now=js_sys::Date::new_0();let day_ms=86400000.;let today=(now.get_time()/day_ms).floor()*day_ms;
                        let rows=items(&study.data.get()["daily_stats"]);
                        let mut bins:Vec<Value>=if period=="day"{items(&summary.data.get()["weekly_minutes"]).into_iter().map(|v|json!({"label":v["day"],"minutes":v["minutes"]})).collect()}else{vec![]};
                        if period=="week"{for i in (0..8).rev(){let end=today+(1-i*7) as f64*day_ms;let start=end-7.*day_ms;let seconds:f64=rows.iter().filter(|v|{let t=js_sys::Date::parse(&format!("{}T00:00:00Z",string(v,"stat_date")));t>=start&&t<end}).map(|v|v["total_seconds"].as_f64().unwrap_or_default()).sum();bins.push(json!({"label":if i==0{"近7天".into()}else{format!("{i}周前")},"minutes":seconds/60.}));}}
                        if period=="month"{let current=now.get_utc_full_year() as i32*12+now.get_utc_month() as i32;for i in (0..6).rev(){let month=current-i;let key=format!("{:04}-{:02}",month.div_euclid(12),month.rem_euclid(12)+1);let seconds:f64=rows.iter().filter(|v|string(v,"stat_date").starts_with(&key)).map(|v|v["total_seconds"].as_f64().unwrap_or_default()).sum();bins.push(json!({"label":format!("{}月",month.rem_euclid(12)+1),"minutes":seconds/60.}));}}
                        let max=bins.iter().filter_map(|v|v["minutes"].as_f64()).fold(60.,f64::max);for bin in &mut bins{bin["percent"]=json!(bin["minutes"].as_f64().unwrap_or_default().max(0.)/max*100.);}
                        bins
                    })).key(|v|v.to_string()).seed(|v|view().class("study-column").fill((text().class("muted").content(format!("{:.0}",v["minutes"].as_f64().unwrap_or_default())),view().class("study-column-track").fill(view().class("study-column-fill").style(Style::new().height(Length::Percent(v["percent"].as_f64().unwrap_or_default())))),text().class("muted").content(string(&v,"label")))))),
                    text().class("muted").content("分钟 · 仅展示已记录的学习时长"),
                )),
            )),
        )).build(scope)
    })
}
pub fn stat(
    resource: Resource,
    label: &'static str,
    key: &'static str,
    unit: &'static str,
) -> impl Widget {
    view().class("stat-card").fill((
        text().class("muted").content(label),
        text()
            .class("stat-value")
            .content(resource.data.map(move |v| {
                if v[key].is_null() {
                    "—".into()
                } else {
                    format!("{} {unit}", string(v, key))
                }
            })),
    ))
}
