mod account;
mod admin;
mod chat_stream;
mod dashboard;
mod dictionary;
mod learning;
mod media;
mod theme;
mod ui;

use crate::model::*;
use glory::prelude::*;
use serde_json::{Value, json};
use std::cell::RefCell;
use theme::{action_button, checkbox_control, danger_button, text_control};
pub use ui::*;
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::JsFuture;

thread_local! {
    static APP: RefCell<Option<glory::web::WebApp>> = const { RefCell::new(None) };
    static SESSION: RefCell<Option<Cage<Value>>> = const { RefCell::new(None) };
}
pub fn window() -> web_sys::Window {
    web_sys::window().expect("browser window")
}
pub fn document() -> web_sys::Document {
    window().document().expect("browser document")
}
pub fn storage() -> Option<web_sys::Storage> {
    window().local_storage().ok().flatten()
}
pub fn token() -> Option<String> {
    storage()?
        .get_item("colang.token")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
}
pub fn save_auth(value: &Value) -> std::result::Result<(), String> {
    let store = storage().ok_or("浏览器未允许保存登录状态，请启用站点存储")?;
    let access_token = value["access_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("登录响应缺少凭证")?;
    let mut user = value["user"].clone();
    user["is_admin"] = value["is_admin"].clone();
    store
        .set_item("colang.user", &user.to_string())
        .map_err(js_error)?;
    store
        .set_item("colang.token", access_token)
        .map_err(js_error)?;
    Ok(())
}
pub fn clear_auth() {
    if let Some(s) = storage() {
        let _ = s.remove_item("colang.token");
        let _ = s.remove_item("colang.user");
    }
    SESSION.with(|s| {
        if let Some(cage) = *s.borrow()
            && cage.is_alive()
        {
            cage.revise(|v| *v = Value::Null);
        }
    });
}
pub fn go(path: &str) {
    let _ = window().location().set_href(path);
}
pub fn param(name: &str) -> Option<String> {
    url::form_urlencoded::parse(
        window()
            .location()
            .search()
            .unwrap_or_default()
            .trim_start_matches('?')
            .as_bytes(),
    )
    .find(|(key, _)| key == name)
    .map(|(_, v)| v.into_owned())
}
pub fn js_error(value: JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "浏览器操作失败，请检查权限或网络连接".into())
}
pub fn spawn(future: impl std::future::Future<Output = ()> + 'static) {
    wasm_bindgen_futures::spawn_local(future);
}
pub async fn request(
    method: &str,
    path: &str,
    body: Option<Value>,
) -> std::result::Result<Value, String> {
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    let headers = web_sys::Headers::new().map_err(js_error)?;
    headers
        .set("Accept", "application/json")
        .map_err(js_error)?;
    if let Some(token) = token() {
        headers
            .set("Authorization", &format!("Bearer {token}"))
            .map_err(js_error)?;
    }
    if let Some(body) = body {
        headers
            .set("Content-Type", "application/json")
            .map_err(js_error)?;
        init.set_body(&JsValue::from_str(&body.to_string()));
    }
    init.set_headers(&headers);
    let controller = web_sys::AbortController::new().map_err(js_error)?;
    init.set_signal(Some(&controller.signal()));
    let timeout = gloo_timers::callback::Timeout::new(90_000, move || controller.abort());
    let result = JsFuture::from(window().fetch_with_str_and_init(path, &init)).await;
    let response: web_sys::Response = result
        .map_err(|_| "请求超时或网络不可用，请重试".to_string())?
        .dyn_into()
        .map_err(js_error)?;
    let status = response.status();
    let raw = JsFuture::from(response.text().map_err(js_error)?)
        .await
        .map_err(js_error)?
        .as_string()
        .unwrap_or_default();
    drop(timeout);
    if !response.ok() {
        if status == 401 && path == "/api/login" {
            return Err("邮箱或密码不正确，请检查后重试".into());
        }
        if status == 401 {
            clear_auth();
        }
        return Err(error_message(&raw, status));
    }
    if raw.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&raw).map_err(|_| "服务器返回了无法识别的数据，请检查 API 代理配置".into())
}
#[derive(Clone, Copy)]
pub struct Resource {
    pub data: Cage<Value>,
    pub busy: Cage<bool>,
    pub error: Cage<String>,
    generation: Cage<u32>,
    path: Cage<String>,
}
impl Resource {
    pub fn new(scope: &Scope) -> Self {
        Self {
            data: scope.cage(Value::Null),
            busy: scope.cage(false),
            error: scope.cage(String::new()),
            generation: scope.cage(0),
            path: scope.cage(String::new()),
        }
    }
    pub fn load(self, path: impl Into<String>) {
        self.generation.revise(|g| *g += 1);
        let generation = self.generation.get_untracked();
        let path = path.into();
        if self.path.get_untracked() != path {
            self.data.revise(|v| *v = Value::Null);
            self.path.revise(|previous| *previous = path.clone());
        }
        self.busy.revise(|v| *v = true);
        self.error.revise(String::clear);
        spawn(async move {
            let result = request("GET", &path, None).await;
            if !self.generation.is_alive() || self.generation.get() != generation {
                return;
            }
            self.busy.revise(|v| *v = false);
            match result {
                Ok(data) => self.data.revise(|v| *v = data),
                Err(error) => self.error.revise(|v| *v = error),
            }
        });
    }
    pub fn run(
        self,
        method: &'static str,
        path: String,
        body: Option<Value>,
        done: impl FnOnce(Value) + 'static,
    ) {
        if self.busy.get() {
            return;
        }
        self.busy.revise(|v| *v = true);
        self.error.revise(String::clear);
        spawn(async move {
            let result = request(method, &path, body).await;
            if !self.busy.is_alive() {
                return;
            }
            self.busy.revise(|v| *v = false);
            match result {
                Ok(value) => done(value),
                Err(error) => self.error.revise(|v| *v = error),
            }
        });
    }
    pub fn status(self) -> impl Widget {
        view().class("feedback").fill((
            when(self.busy, || {
                view()
                    .role(Role::Status)
                    .class("loading-status")
                    .fill(text().content("正在处理，请稍候…"))
            }),
            when(self.error.map(|error| !error.is_empty()), move || {
                view()
                    .role(Role::Alert)
                    .class("error")
                    .fill(text().content(self.error))
            }),
        ))
    }
}
pub fn start() -> std::result::Result<(), JsValue> {
    theme::install()?;
    let route = Route::parse(&window().location().pathname()?);
    let root = document()
        .get_element_by_id("app")
        .ok_or("missing app root")?;
    root.set_text_content(None);
    let web = glory::web::launch_with(root, move |scope| {
        media::viewport(scope);
        let session = scope.cage(Value::Null);
        let ready = scope.cage(token().is_none());
        SESSION.with(|s| *s.borrow_mut() = Some(session));
        if token().is_some() {
            spawn(async move {
                let result = request("GET", "/api/me", None).await;
                if !session.is_alive() {
                    return;
                }
                match result {
                    Ok(user) => {
                        session.revise(|s| *s = user);
                    }
                    Err(error) => {
                        // A network failure must not discard a valid stored credential.
                        if token().is_some() {
                            session.revise(|s| *s = json!({"session_error": error}));
                        }
                    }
                }
                ready.revise(|v| *v = true);
            });
        }
        heartbeat(&route);
        view().class("site sapid-theme-colang-light").fill((
            header(session),
            view().role(Role::Main).class("main").fill((
                when(ready.map(|v| !v), || text().content("正在验证登录状态…")),
                each(scope.bond(move || {
                    if !ready.get() {
                        return Vec::new();
                    }
                    let user = session.get();
                    vec![if !user["session_error"].is_null() {
                        3u8
                    } else if user.is_null() {
                        0
                    } else if user["is_admin"] == true {
                        2
                    } else {
                        1
                    }]
                }))
                .key(|access| *access)
                .seed(move |_| {
                    let route = route.clone();
                    widget(move |scope| {
                        let logged =
                            !session.get().is_null() && session.get()["session_error"].is_null();
                        let mut content = if route.requires_auth() && !logged {
                            if let Some(error) = session.get()["session_error"].as_str() {
                                nodes(vec![
                                    boxed(title("无法验证登录状态")),
                                    boxed(p(error)),
                                    boxed(nav(
                                        "重试",
                                        &window().location().href().unwrap_or_else(|_| "/".into()),
                                    )),
                                ])
                            } else {
                                nodes(vec![boxed(account::login_page(true))])
                            }
                        } else if matches!(route, Route::Admin(_))
                            && !session.get()["is_admin"].as_bool().unwrap_or(false)
                        {
                            nodes(vec![
                                boxed(title("没有管理权限")),
                                boxed(nav("返回首页", "/")),
                            ])
                        } else {
                            page(route.clone(), session)
                        };
                        content.build(scope)
                    })
                }),
            )),
            view().role(Role::ContentInfo).class("footer").fill((
                p("开朗英语 · 每天进步一点"),
                nav("意见反馈", "/feedback"),
                nav("服务条款", "/terms"),
                nav("隐私政策", "/privacy"),
            )),
        ))
    })
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    APP.with(|app| *app.borrow_mut() = Some(web));
    Ok(())
}
fn main_navigation() -> impl Widget {
    view()
        .role(Role::Navigation)
        .name("主导航")
        .class("nav")
        .fill((
            section_nav("天天唠嗑", "/chat"),
            section_nav("温故知新", "/review"),
            section_nav("角色扮演", "/stages"),
            section_nav("大声跟读", "/read"),
            section_nav("词典翻译", "/dict"),
        ))
}
fn header(session: Cage<Value>) -> impl Widget {
    view().role(Role::Banner).class("header").fill(
        view().class("header-inner").fill((
            link().target("/").class("brand").fill((
                image().source("/colang-logo.svg").alt("开朗英语标志"),
                text().content("开朗英语"),
            )),
            view().class("desktop-navigation").fill(main_navigation()),
            view().class("mobile-navigation").fill(
                sapid::drawer("mobile-navigation")
                    .theme(theme::orange())
                    .label("打开导航菜单")
                    .trigger_content(|| text().content("☰"))
                    .title("学习导航")
                    .close_label("关闭导航菜单")
                    .content(main_navigation),
            ),
            when(
                session.map(|s| !s.is_null() && s["session_error"].is_null()),
                move || {
                    sapid::popover("account-menu")
                        .theme(theme::orange())
                        .label(session.map(|s| format!("{} · 我的账户", string(s, "name"))))
                        .close_label("关闭账户菜单")
                        .content(move || {
                            view().class("account-menu").fill((
                                media::avatar(session),
                                text().content(session.map(|s| string(s, "name"))),
                                nav("我的学习", "/me"),
                                nav("我的收藏", "/favorites"),
                                nav("成就", "/me#achievements"),
                                nav("AI 服务设置", "/me#settings"),
                                nav("订阅与钱包", "/subscription"),
                                when(
                                    session.map(|s| s["is_admin"].as_bool().unwrap_or(false)),
                                    || nav("管理后台", "/admin"),
                                ),
                                action_button()
                                    .label("退出登录")
                                    .variant(sapid::ButtonVariant::Subtle)
                                    .on_press(|| {
                                        clear_auth();
                                        go("/");
                                    }),
                            ))
                        })
                },
            )
            .otherwise(|| section_nav("登录 / 注册", "/login")),
        )),
    )
}
fn page(route: Route, session: Cage<Value>) -> Nodes {
    let page: Box<dyn Widget> = match route {
        Route::Landing => boxed(dashboard::landing(session)),
        Route::Login => boxed(account::login_page(false)),
        Route::Auth => boxed(account::authorize()),
        Route::AuthComplete => boxed(account::oauth_complete()),
        Route::Dict(word) => boxed(dictionary::dictionary(word)),
        Route::Translate => boxed(dictionary::translate()),
        Route::Chat(id) => boxed(learning::chat(id)),
        Route::Review => boxed(learning::review()),
        Route::ReviewSession(kind) => boxed(learning::review_session(kind)),
        Route::Stages => boxed(learning::stages()),
        Route::Stage(id) => boxed(learning::stage(id)),
        Route::Read => boxed(learning::reading()),
        Route::Me => boxed(account::profile(session)),
        Route::Favorites => boxed(dictionary::favorites()),
        Route::Subscription => boxed(account::subscription()),
        Route::Admin(path) => boxed(admin::page(path)),
        Route::Terms => boxed(legal(include_str!("../../content/terms.txt"), "服务条款")),
        Route::Privacy => boxed(legal(include_str!("../../content/privacy.txt"), "隐私政策")),
        Route::Feedback => boxed(view().class("panel").fill((
            title("意见反馈"),
            p("分享想法、报告错误或请求功能。"),
            nav(
                "前往项目反馈页面",
                "https://github.com/cola-ng/website/issues",
            ),
        ))),
        Route::NotFound => boxed(view().class("panel").fill((
            title("页面不存在"),
            p("请检查地址，或从导航栏继续学习。"),
            nav("返回首页", "/"),
        ))),
    };
    nodes(vec![page])
}
fn legal(content: &'static str, label: &'static str) -> impl Widget {
    view().class("panel legal").fill((
        title(label),
        nodes(
            content
                .lines()
                .filter(|s| !s.trim().is_empty() && s.trim() != label)
                .map(|s| {
                    let short = s.chars().count() < 60;
                    if short
                        && s.split_once('、').is_some_and(|(number, _)| {
                            !number.is_empty()
                                && number.chars().all(|c| "一二三四五六七八九十".contains(c))
                        })
                    {
                        boxed(heading(s))
                    } else if short
                        && s.split_whitespace().next().is_some_and(|n| {
                            n.contains('.') && n.split('.').all(|part| part.parse::<u16>().is_ok())
                        })
                    {
                        boxed(text().kind(TextKind::Heading(3)).content(s))
                    } else {
                        boxed(p(s))
                    }
                })
                .collect(),
        ),
    ))
}
fn heartbeat(route: &Route) {
    let Some(page_type) = route.study_type() else {
        return;
    };
    spawn(async move {
        loop {
            if token().is_some() && !document().hidden() && document().has_focus().unwrap_or(false)
            {
                let _ = request(
                    "POST",
                    "/api/learn/heartbeat",
                    Some(json!({"page_type":page_type})),
                )
                .await;
            }
            gloo_timers::future::TimeoutFuture::new(20_000).await;
        }
    });
}
