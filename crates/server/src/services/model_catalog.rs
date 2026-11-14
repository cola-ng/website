//! Read-only discovery against fixed official endpoints. No credentials or raw
//! upstream responses leave this module. Static recommendations are explicit fallbacks.
use super::provider_catalog;
use serde_json::{Value, json};
use std::{collections::BTreeSet, time::Duration};

pub fn supports(kind: &str) -> bool {
    matches!(
        kind,
        "openai"
            | "deepseek"
            | "kimi"
            | "qwen"
            | "gemini"
            | "anthropic"
            | "siliconflow"
            | "openrouter"
            | "xai"
            | "mistral"
            | "groq"
            | "baidu"
    )
}
pub fn valid_workspace(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.:/".contains(&c))
}
fn fallback(kind: &str, reason: &str) -> Value {
    let mut out =
        serde_json::to_value(provider_catalog::definition(kind).expect("validated provider"))
            .unwrap();
    out["source"] = json!("static");
    out["discovery_notice"] = json!(reason);
    out["capability_sources"] = json!({"chat":"static","tts":"static","asr":"static"});
    out
}
pub async fn discover(kind: &str, settings: &Value) -> Value {
    let result = if !supports(kind) {
        return fallback(
            kind,
            "该平台当前文档未提供适用于此配置的模型列表接口，使用已核对的静态建议。",
        );
    } else if kind != "openrouter" && settings["api_key"].as_str().unwrap_or_default().is_empty() {
        return fallback(kind, "填写 API Key 后可读取官方列表；当前显示静态建议。");
    } else {
        tokio::time::timeout(Duration::from_secs(20), fetch(kind, settings))
            .await
            .unwrap_or_else(|_| Err("官方模型列表请求超时，暂用静态建议。".into()))
    };
    match result {
        Ok(rows) => normalize(kind, &rows),
        Err(reason) => fallback(kind, &reason),
    }
}
async fn fetch(kind: &str, settings: &Value) -> Result<Vec<Value>, String> {
    let region = settings["region"].as_str().unwrap_or("global");
    let url = if kind == "qwen" {
        let workspace = settings["workspace_id"].as_str().unwrap_or_default();
        if !valid_workspace(workspace) {
            return Err("百炼官方列表需要业务空间 ID，请填写后读取；当前显示静态建议。".into());
        }
        let area = if region == "cn" {
            "cn-beijing"
        } else {
            "ap-southeast-1"
        };
        format!("https://{workspace}.{area}.maas.aliyuncs.com/api/v1/models")
    } else {
        format!("{}/models", provider_catalog::endpoint(kind, region))
    };
    fetch_pages(kind, settings, &url).await
}
async fn fetch_pages(kind: &str, settings: &Value, url: &str) -> Result<Vec<Value>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "模型列表连接失败")?;
    let mut output = Vec::new();
    let mut cursor = String::new();
    let mut bytes_read = 0;
    // Pagination is bounded and never follows a URL supplied by an upstream.
    for page in 1..=32 {
        let mut page_url = reqwest::Url::parse(url).map_err(|_| "模型列表地址无效")?;
        if kind == "anthropic" {
            page_url.query_pairs_mut().append_pair("limit", "1000");
            if !cursor.is_empty() {
                page_url.query_pairs_mut().append_pair("after_id", &cursor);
            }
        }
        if kind == "siliconflow" {
            page_url.query_pairs_mut().append_pair("sub_type", "chat");
        }
        if kind == "qwen" {
            page_url
                .query_pairs_mut()
                .append_pair("page_no", &page.to_string())
                .append_pair("page_size", "100")
                .append_pair("capabilities", "TG")
                .append_pair("providers", "qwen");
        }
        let mut req = client.get(page_url);
        if kind == "anthropic" {
            req = req
                .header(
                    "x-api-key",
                    settings["api_key"].as_str().unwrap_or_default(),
                )
                .header("anthropic-version", "2023-06-01");
        } else if let Some(key) = settings["api_key"].as_str().filter(|s| !s.is_empty() && kind != "openrouter") {
            req = req.bearer_auth(key);
        }
        let mut response = req
            .send()
            .await
            .map_err(|_| "官方模型列表连接失败，暂用静态建议。")?;
        if !response.status().is_success() {
            return Err(format!(
                "官方模型列表返回 HTTP {}，请检查密钥、区域与权限；当前使用静态建议。",
                response.status().as_u16()
            ));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| "模型列表读取中断")? {
            bytes_read += chunk.len();
            if bytes_read > 8 * 1024 * 1024 {
                return Err("模型列表超过大小限制，暂用静态建议。".into());
            }
            body.extend_from_slice(&chunk);
        }
        let data: Value =
            serde_json::from_slice(&body).map_err(|_| "模型列表格式无法识别，暂用静态建议。")?;
        let rows = data
            .get("data")
            .or_else(|| data.pointer("/output/models"))
            .and_then(Value::as_array)
            .ok_or("模型列表格式无法识别，暂用静态建议。")?;
        output.extend(rows.iter().cloned());
        if output.len() > 4096 {
            return Err("模型列表条目超过限制，暂用静态建议。".into());
        }
        if kind == "qwen"
            && data
                .pointer("/output/total")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > output.len() as u64
        {
            if rows.is_empty() {
                return Err("官方模型列表分页不完整，暂用静态建议。".into());
            }
            continue;
        }
        if kind == "anthropic" && data["has_more"] == true {
            let next = data["last_id"]
                .as_str()
                .filter(|id| valid_id(id) && *id != cursor)
                .ok_or("官方模型列表分页不完整")?;
            cursor = next.into();
            continue;
        }
        return Ok(output);
    }
    Err("官方模型列表分页超过限制，暂用静态建议。".into())
}
fn text_family(id: &str) -> bool {
    let blocked = [
        "audio",
        "realtime",
        "transcribe",
        "tts",
        "image",
        "embedding",
        "moderation",
        "search",
        "deep-research",
        "codex",
        "cyber",
        "rosalind",
        "computer-use",
        "-pro",
    ];
    if blocked.iter().any(|s| id.contains(s)) {
        return false;
    }
    [
        "gpt-6-luna",
        "gpt-6.1-terra",
        "gpt-6.1-sol",
        "gpt-6-astra",
        "gpt-5",
        "gpt-4",
        "gpt-3.5",
        "chatgpt-",
        "o1",
        "o3",
        "o4",
    ]
    .iter()
    .any(|s| id.starts_with(s))
}
fn category(kind: &str, row: &Value, id: &str) -> Option<&'static str> {
    let lower = id.to_ascii_lowercase();
    match kind {
        "openai" => {
            if super::openai_realtime::is_model(&lower) || lower.starts_with("gpt-4o-mini-tts") || lower == "tts-1" || lower == "tts-1-hd" {
                Some("tts")
            } else if lower.starts_with("gpt-transcribe")
                || lower == "whisper-1"
                || (lower.starts_with("gpt-4o-")
                    && lower.contains("transcribe")
                    && !lower.contains("diarize"))
            {
                Some("asr")
            } else if text_family(&lower) {
                Some("chat")
            } else {
                None
            }
        }
        "openrouter" => {
            let outputs = row.pointer("/architecture/output_modalities")?.as_array()?;
            if !outputs.contains(&json!("text")) || outputs.iter().any(|v| v != "text") {
                return None;
            }
            if let Some(endpoints) = row["endpoints"].as_array()
                && !endpoints.contains(&json!("/api/v1/chat/completions"))
            {
                return None;
            }
            Some("chat")
        }
        "mistral" => (row.pointer("/capabilities/completion_chat") == Some(&Value::Bool(true)))
            .then_some("chat"),
        "baidu" => {
            (row["type"] == "chat" || row["type"] == "text" || row["type"] == "text-generation")
                .then_some("chat")
        }
        "groq" => {
            if lower.starts_with("whisper-") {
                Some("asr")
            } else if ["llama-", "openai/gpt-oss-", "qwen/"]
                .iter()
                .any(|s| lower.starts_with(s))
            {
                Some("chat")
            } else {
                None
            }
        }
        "gemini" => (lower.starts_with("gemini-")
            && ![
                "image",
                "tts",
                "live",
                "transcribe",
                "robotics",
                "embedding",
            ]
            .iter()
            .any(|s| lower.contains(s)))
        .then_some("chat"),
        "xai" => (lower.starts_with("grok-")
            && !["image", "video", "voice", "audio", "imagine"]
                .iter()
                .any(|s| lower.contains(s)))
        .then_some("chat"),
        "qwen" => (!lower.starts_with("qwen4-max-preview")
            && ![
                "qwen-mt-",
                "qwen-doc-",
                "qwen-deep-research",
                "qwen-omni",
                "qwen-audio",
            ]
            .iter()
            .any(|s| lower.starts_with(s)))
        .then_some("chat"),
        "siliconflow" | "deepseek" | "kimi" | "anthropic" => Some("chat"),
        _ => None,
    }
}
pub fn normalize(kind: &str, rows: &[Value]) -> Value {
    let mut out = fallback(
        kind,
        "已读取官方模型列表；仅展示本站当前调用方式支持的模型。列表可见不代表账号已开通计费权限。",
    );
    let mut buckets = [BTreeSet::new(), BTreeSet::new(), BTreeSet::new()];
    for row in rows {
        if row["active"] == false
            || matches!(
                row["status"].as_str(),
                Some("retired" | "shutdown" | "inactive")
            )
            || row["lifecycle"] == "retired"
        {
            continue;
        }
        let Some(id) = row["id"]
            .as_str()
            .or_else(|| row["model"].as_str())
            .filter(|id| valid_id(id))
        else {
            continue;
        };
        let Some(cap) = category(kind, row, id) else {
            continue;
        };
        let i = match cap {
            "chat" => 0,
            "tts" => 1,
            _ => 2,
        };
        // Existing adapters may not implement a provider's other modalities.
        if out[format!("{cap}_models")]
            .as_array()
            .is_some_and(|a| !a.is_empty())
        {
            buckets[i].insert(id.to_owned());
        }
    }
    let mut any = false;
    for (i, cap) in ["chat", "tts", "asr"].into_iter().enumerate() {
        if buckets[i].is_empty() {
            continue;
        }
        let field = format!("{cap}_models");
        let mut ids = Vec::new();
        // Keep the current recommendations first, only when the official list contains them.
        for id in out[&field]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
        {
            if buckets[i].remove(id) {
                ids.push(id.to_owned());
            }
        }
        ids.extend(buckets[i].iter().cloned());
        out[&field] = json!(ids);
        out["capability_sources"][cap] = json!("official");
        any = true;
    }
    if any {
        out["source"] = json!("official");
        out["fetched_at"] = json!(chrono::Utc::now().to_rfc3339());
    } else {
        out["discovery_notice"] = json!("官方列表未返回当前接口支持的模型，显示已核对的静态建议。");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_models_separate_realtime_speech_from_text_and_images() {
        let v = normalize(
            "openai",
            &json!([
                {"id":"gpt-6-luna"},{"id":"gpt-6-luna-2026-11-12"},
                {"id":"gpt-realtime-2.1-mini"},{"id":"gpt-image-2.6-flare"},
                {"id":"gpt-4o-mini-tts"},{"id":"gpt-transcribe"},
                {"id":"gpt-6-luna","status":"retired"},{"id":"evil\r\n"}
            ])
            .as_array()
            .unwrap(),
        );
        assert_eq!(v["source"], "official");
        assert_eq!(
            v["chat_models"],
            json!(["gpt-6-luna", "gpt-6-luna-2026-11-12"])
        );
        assert_eq!(v["tts_models"], json!(["gpt-realtime-2.1-mini", "gpt-4o-mini-tts"]));
        assert_eq!(v["asr_models"], json!(["gpt-transcribe"]));
    }
    #[test]
    fn endpoint_metadata_filters_unsupported_output_and_preserves_static_fallback() {
        let v=normalize("openrouter", &json!([
            {"id":"new/text-model","architecture":{"output_modalities":["text"]},"endpoints":["/api/v1/chat/completions"]},
            {"id":"new/image-model","architecture":{"output_modalities":["text","image"]}},
            {"id":"new/batch-only","architecture":{"output_modalities":["text"]},"endpoints":["/api/v1/batches"]}
        ]).as_array().unwrap());
        assert_eq!(v["chat_models"], json!(["new/text-model"]));
        assert_eq!(normalize("openai", &[])["source"], "static");
        assert!(!valid_workspace("evil.example/path"));
        assert!(!valid_workspace("bad@localhost"));
        assert!(valid_workspace("llm-abc123"));
    }
    #[tokio::test]
    async fn official_pagination_uses_fixed_url_and_errors_hide_upstream_body() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for body in [
                json!({"data":[{"id":"claude-one"}],"has_more":true,"last_id":"claude-one"}),
                json!({"data":[{"id":"claude-two"}],"has_more":false}),
            ] {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut buf = Vec::new();
                while !buf.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    socket.read_exact(&mut byte).unwrap();
                    buf.extend(byte);
                }
                requests.push(String::from_utf8(buf).unwrap());
                let body = body.to_string();
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
            requests
        });
        let result = fetch_pages(
            "anthropic",
            &json!({"api_key":"private-test-key"}),
            &format!("http://127.0.0.1:{port}/models"),
        )
        .await
        .unwrap();
        assert_eq!(result.len(), 2);
        let requests = server.join().unwrap();
        assert!(
            requests[0]
                .to_lowercase()
                .contains("x-api-key: private-test-key")
        );
        assert!(requests[1].starts_with("GET /models?limit=1000&after_id=claude-one "));
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut b = [0; 2048];
            let _ = socket.read(&mut b);
            socket.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 14\r\nConnection: close\r\n\r\nsecret-api-key").unwrap();
        });
        let err = fetch_pages(
            "openai",
            &json!({"api_key":"test"}),
            &format!("http://127.0.0.1:{port}/models"),
        )
        .await
        .unwrap_err();
        assert!(err.contains("401"));
        assert!(!err.contains("secret-api-key"));
        server.join().unwrap();
    }
}
