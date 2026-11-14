use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Landing,
    Amigo(Option<String>),
    Login,
    Auth,
    AuthComplete,
    Dict(Option<String>),
    Translate,
    Chat(Option<i64>),
    Review,
    ReviewSession(String),
    Stages,
    Stage(i64),
    Read,
    Me,
    Favorites,
    Subscription,
    Feedback,
    Terms,
    Privacy,
    Admin(Vec<String>),
    NotFound,
}
impl Route {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Landing => "Colang · 英语学习",
            Self::Amigo(_) => "智能学伴 · Colang",
            Self::Login => "登录 · Colang",
            Self::Auth | Self::AuthComplete => "账户授权 · Colang",
            Self::Dict(_) => "词典 · Colang",
            Self::Translate => "翻译 · Colang",
            Self::Chat(_) => "英语对话 · Colang",
            Self::Review | Self::ReviewSession(_) => "复习练习 · Colang",
            Self::Stages | Self::Stage(_) => "学习场景 · Colang",
            Self::Read => "阅读跟读 · Colang",
            Self::Me => "账户设置 · Colang",
            Self::Favorites => "我的收藏 · Colang",
            Self::Subscription => "订阅 · Colang",
            Self::Feedback => "反馈 · Colang",
            Self::Terms => "服务条款 · Colang",
            Self::Privacy => "隐私政策 · Colang",
            Self::Admin(_) => "内容管理 · Colang",
            Self::NotFound => "页面未找到 · Colang",
        }
    }

    pub fn parse(path: &str) -> Self {
        let parts: Vec<_> = path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        let id = |s: &str| s.parse::<i64>().ok().filter(|n| *n > 0);
        match parts.as_slice() {
            [] => Self::Landing,
            ["amigo"] => Self::Amigo(None),
            ["amigo", "devices", id] => Self::Amigo(Some((*id).into())),
            ["login"] => Self::Login,
            ["auth"] => Self::Auth,
            ["auth", "complete"] => Self::AuthComplete,
            ["dict"] => Self::Dict(None),
            ["dict", word] => Self::Dict(Some(
                url::form_urlencoded::parse(format!("w={word}").replace('+', "%2B").as_bytes())
                    .next()
                    .map(|(_, v)| v.into_owned())
                    .unwrap_or_default(),
            )),
            ["translate"] => Self::Translate,
            ["chat"] => Self::Chat(None),
            ["chat", n] => id(n).map(|n| Self::Chat(Some(n))).unwrap_or(Self::NotFound),
            ["review"] => Self::Review,
            ["review", "session"] => Self::ReviewSession("smart".into()),
            ["review", "session", kind] if REVIEW_TYPES.iter().any(|(k, _)| k == kind) => {
                Self::ReviewSession((*kind).into())
            }
            ["stages"] => Self::Stages,
            ["stages", n] => id(n).map(Self::Stage).unwrap_or(Self::NotFound),
            ["read"] => Self::Read,
            ["me"] => Self::Me,
            ["favorites"] => Self::Favorites,
            ["subscription"] => Self::Subscription,
            ["feedback"] => Self::Feedback,
            ["terms"] => Self::Terms,
            ["privacy"] => Self::Privacy,
            ["admin", tail @ ..] => Self::Admin(tail.iter().map(|s| (*s).into()).collect()),
            _ => Self::NotFound,
        }
    }
    pub fn requires_auth(&self) -> bool {
        matches!(
            self,
            Self::Amigo(_)
                | Self::Chat(_)
                | Self::Translate
                | Self::Review
                | Self::ReviewSession(_)
                | Self::Read
                | Self::Me
                | Self::Favorites
                | Self::Admin(_)
        )
    }
    pub fn study_type(&self) -> Option<&'static str> {
        match self {
            Self::Dict(_) => Some("dictionary"),
            Self::Translate => Some("translate"),
            Self::Chat(_) => Some("chat"),
            Self::Review | Self::ReviewSession(_) => Some("review"),
            Self::Stages | Self::Stage(_) => Some("stage"),
            Self::Read => Some("read"),
            _ => None,
        }
    }
}
pub const REVIEW_TYPES: &[(&str, &str)] = &[
    ("pronunciation", "发音纠正"),
    ("meaning", "词义复习"),
    ("spelling", "拼写练习"),
    ("recognition", "单词识记"),
    ("phrase", "短语搭配"),
];
pub fn string(v: &Value, key: &str) -> String {
    match &v[key] {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        v => v.to_string(),
    }
}
pub fn items(v: &Value) -> Vec<Value> {
    v.as_array()
        .or_else(|| {
            [
                "items",
                "words",
                "users",
                "domains",
                "categories",
                "orders",
                "subscriptions",
                "stages",
                "subjects",
                "sentences",
                "transactions",
            ]
            .iter()
            .find_map(|key| v[*key].as_array())
        })
        .cloned()
        .unwrap_or_default()
}
pub fn query(key: &str, value: &str) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .append_pair(key, value)
        .finish()
}
pub fn dictionary_path(word: &str) -> String {
    format!(
        "/dict/{}",
        query("w", word)
            .trim_start_matches("w=")
            .replace('+', "%20")
    )
}
pub fn safe_return(path: &str) -> &str {
    if path.starts_with('/') && !path.starts_with("//") && !path.contains(['\\', '\r', '\n']) {
        path
    } else {
        "/me"
    }
}
pub fn desktop_callback(raw: &str) -> bool {
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    url.username().is_empty()
        && url.password().is_none()
        && ((url.scheme() == "http"
            && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")))
            || (matches!(url.scheme(), "colang" | "amigo") && url.host_str().is_some()))
}
pub fn error_message(body: &str, status: u16) -> String {
    if status == 401 {
        return "登录已过期，请重新登录".into();
    }
    if status == 403 {
        return "没有权限执行此操作".into();
    }
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        for p in ["/error/brief", "/error/message", "/brief", "/message"] {
            if let Some(s) = v
                .pointer(p)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
            {
                return s.into();
            }
        }
    }
    match status {
        404 => "未找到所请求的内容",
        409 => "数据冲突，请刷新后重试",
        429 => "请求过于频繁，请稍后重试",
        500..=599 => "服务暂时不可用，请稍后重试",
        _ => "请求失败，请检查输入或网络连接",
    }
    .into()
}
pub fn normalize_answer(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_and_deep_links() {
        for word in ["ice cream", "C++", "a/b", "你好"] {
            assert_eq!(
                Route::parse(&dictionary_path(word)),
                Route::Dict(Some(word.into()))
            );
        }
        assert_eq!(
            Route::parse("/dict/ice%20cream"),
            Route::Dict(Some("ice cream".into()))
        );
        assert_eq!(
            Route::parse("/dict/C%2B%2B"),
            Route::Dict(Some("C++".into()))
        );
        assert_eq!(Route::parse("/chat/42"), Route::Chat(Some(42)));
        assert_eq!(Route::parse("/stages/-1"), Route::NotFound);
        assert_eq!(Route::parse("/review/session/bogus"), Route::NotFound);
        assert_eq!(Route::parse("/unknown"), Route::NotFound);
    }
    #[test]
    fn redirect_validation() {
        for bad in ["//evil.test", "https://evil.test", "/\\evil.test", "/\nfoo"] {
            assert_eq!(safe_return(bad), "/me");
        }
        assert_eq!(safe_return("/auth?state=abc"), "/auth?state=abc");
        for bad in [
            "javascript:alert(1)",
            "https://evil.test",
            "http://localhost.evil.test",
            "http://user@localhost",
        ] {
            assert!(!desktop_callback(bad));
        }
        assert!(desktop_callback("http://127.0.0.1:4319/callback"));
        assert!(desktop_callback("amigo://callback"));
    }
    #[test]
    fn response_handling() {
        assert_eq!(items(&serde_json::json!({"words":[{"id":1}]})).len(), 1);
        assert_eq!(
            error_message("{\"error\":{\"brief\":\"invalid password\"}}", 400),
            "invalid password"
        );
        assert!(!error_message("<html>proxy error</html>", 502).contains("<html>"));
        assert_eq!(normalize_answer("  Ice   CREAM "), "ice cream");
        for key in [
            "users",
            "domains",
            "categories",
            "orders",
            "subscriptions",
            "stages",
            "subjects",
            "sentences",
            "transactions",
        ] {
            let mut response = serde_json::json!({"total":1});
            response[key] = serde_json::json!([{"id":42}]);
            assert_eq!(items(&response)[0]["id"], 42);
        }
    }
}
