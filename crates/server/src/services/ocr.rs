use base64::Engine;
use horme::prelude::*;
use serde::{Deserialize, Serialize};
use std::result::Result;

use crate::db::schema::user_ai_providers;
use crate::db::with_conn;
use crate::models::ai_provider::UserAiProvider;

pub(crate) const OCR_PROMPT: &str = "Extract all readable text from this screenshot in reading order. Return only JSON with: text (string), topics (at most 5 short lowercase English labels), and memories (at most 5 cautious long-term memory candidates). Each memory has memory_type (interest, project, learning, activity, or routine), subject (short label), summary (concise Chinese), confidence (0-1). Infer only what the screenshot directly supports; do not infer identity, health, politics, religion, sexuality, finances, or other sensitive traits. Replace passwords, access tokens, API keys, email addresses, phone numbers and payment numbers with [secret].";

pub(crate) fn camera_prompt() -> String {
    format!("{OCR_PROMPT} This is a physical camera image rather than a screenshot. Transcribe readable book/page text when present. If no text is legible, set text to a brief objective description of visible objects or state that the image is dark, blurry or unreadable. Never invent missing words. Do not leave text empty. Do not infer personal facts from surrounding objects; use an empty memories array unless directly supported by explicit readable text.")
}

#[derive(Debug, Clone)]
pub struct OcrClient {
    http: reqwest::Client,
    endpoint: String,
    api_key: String,
    model: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct OcrOutput {
    pub text: String,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub memories: Vec<MemoryCandidate>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MemoryCandidate {
    pub memory_type: String,
    pub subject: String,
    pub summary: String,
    pub confidence: f32,
}

#[derive(Deserialize)]
struct CompletionResponse {
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
struct Choice {
    message: CompletionMessage,
}
#[derive(Deserialize)]
struct CompletionMessage {
    content: String,
}

impl OcrClient {
    pub fn new(endpoint: String, api_key: String, model: String) -> Result<Self, String> {
        if endpoint.trim().is_empty() || api_key.trim().is_empty() || model.trim().is_empty() {
            return Err("OCR endpoint, API key and model are required".into());
        }
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(45))
                .build()
                .map_err(|error| error.to_string())?,
            endpoint,
            api_key,
            model,
        })
    }

    pub async fn extract_png(&self, png: &[u8]) -> Result<OcrOutput, String> {
        self.extract_image(png, "image/png").await
    }

    pub async fn extract_image(&self, png: &[u8], mime: &str) -> Result<OcrOutput, String> {
        self.request_image(png, mime, OCR_PROMPT).await
    }

    pub async fn observe_image(&self, image: &[u8], mime: &str) -> Result<OcrOutput, String> {
        self.request_image(image, mime, &camera_prompt()).await
    }

    async fn request_image(&self, png: &[u8], mime: &str, prompt: &str) -> Result<OcrOutput, String> {
        if !matches!(mime, "image/png" | "image/jpeg") {
            return Err("unsupported image format".into());
        }
        if png.is_empty() {
            return Err("screenshot is empty".into());
        }
        let data_url = zeroize::Zeroizing::new(format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        ));
        let response = self
            .http
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [{"role":"user","content":[
                    {"type":"text","text":prompt},
                    {"type":"image_url","image_url":{"url":data_url.as_str()}}
                ]}],
                "temperature": 0,
                // GLM-4V-Flash rejects requests above 1024 output tokens.
                "max_tokens": if self.model == "glm-4v-flash" { 1024 } else { 2000 },
                "stream": false
            }))
            .send()
            .await
            .map_err(|error| format!("OCR provider request failed: {error}"))?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!(
                "OCR provider returned {status}: {}",
                String::from_utf8_lossy(&bytes)
            ));
        }
        let completion: CompletionResponse = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid OCR response: {error}"))?;
        let content = completion
            .choices
            .first()
            .map(|choice| choice.message.content.trim())
            .filter(|content| !content.is_empty())
            .ok_or("OCR provider returned no text")?;
        parse_output(content)
    }
}

pub(crate) fn parse_output(content: &str) -> Result<OcrOutput, String> {
    let without_prefix = content
        .strip_prefix("```json")
        .or_else(|| content.strip_prefix("```"))
        .unwrap_or(content);
    let trimmed = without_prefix
        .strip_suffix("```")
        .unwrap_or(without_prefix)
        .trim();
    let mut output: OcrOutput = serde_json::from_str(trimmed).unwrap_or_else(|_| OcrOutput {
        text: content.trim().to_owned(),
        topics: Vec::new(),
        memories: Vec::new(),
    });
    output.text = output.text.trim().to_owned();
    output.topics = output
        .topics
        .into_iter()
        .map(|topic| topic.trim().to_lowercase())
        .filter(|topic| !topic.is_empty())
        .take(5)
        .collect();
    output.memories = output
        .memories
        .into_iter()
        .filter_map(|mut memory| {
            memory.memory_type = memory.memory_type.trim().to_lowercase();
            memory.subject = memory.subject.trim().to_lowercase();
            memory.summary = memory.summary.trim().to_owned();
            memory.confidence = memory.confidence.clamp(0.05, 0.95);
            let allowed = matches!(
                memory.memory_type.as_str(),
                "interest" | "project" | "learning" | "activity" | "routine"
            );
            (allowed && !memory.subject.is_empty() && !memory.summary.is_empty()).then_some(memory)
        })
        .take(5)
        .collect();
    if output.text.is_empty() {
        return Err("OCR provider returned empty text".into());
    }
    Ok(output)
}

pub async fn user_chat_config(user_id:i64)->Result<Option<(String,serde_json::Value)>,String> {
    let row=with_conn!(move |conn| user_ai_providers::table.filter(user_ai_providers::user_id.eq(user_id)).filter(user_ai_providers::is_active.eq(true)).first::<UserAiProvider>(conn).await.optional()).await?;
    Ok(row.map(|r|(r.provider_type,r.settings)))
}
pub async fn client_for_user(user_id:i64)->Result<OcrClient,String> {
    // Enforce the website's personal-provider / paid-platform policy for vision too.
    crate::services::get_user_provider(user_id).await.map_err(|e|e.to_string())?;
    match user_chat_config(user_id).await? {
        Some((kind,settings)) => client_for_config(&kind,&settings),
        None=>system_client(),
    }
}
pub fn client_for_config(kind:&str,settings:&serde_json::Value)->Result<OcrClient,String>{
    let base=crate::services::provider_catalog::endpoint(kind,settings["region"].as_str().unwrap_or("global"));
    let model=if kind=="zhipu" {"glm-4v-plus"} else {settings["chat_model"].as_str().ok_or("chat model missing")?};
    OcrClient::new(format!("{base}/chat/completions"),settings["api_key"].as_str().unwrap_or_default().into(),model.into())
}

/// Match the system-default ASR/chat/TTS fallback when no user override exists.
pub fn system_client() -> Result<OcrClient, String> {
    let provider = std::env::var("AI_PROVIDER_DEFAULT")
        .unwrap_or_default()
        .to_lowercase();
    system_client_for(&provider)
}

pub fn system_client_for(provider: &str) -> Result<OcrClient, String> {
    if let (Ok(endpoint), Ok(api_key), Ok(model)) = (
        std::env::var("OCR_API_URL"),
        std::env::var("OCR_API_KEY"),
        std::env::var("OCR_MODEL"),
    ) {
        return OcrClient::new(endpoint, api_key, model);
    }
    match provider {
        "zhipu" => OcrClient::new(
            "https://open.bigmodel.cn/api/paas/v4/chat/completions".into(),
            std::env::var("ZHIPU_API_KEY")
                .or_else(|_| std::env::var("ZHIPUAI_API_KEY"))
                .map_err(|_| "missing system vision key")?,
            std::env::var("ZHIPU_OCR_MODEL").unwrap_or_else(|_| "glm-4v-flash".into()),
        ),
        _ => Err("no active user or system vision provider configured".into()),
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    use super::*;

    #[test]
    fn parses_json_and_limits_topics() {
        let output = parse_output("```json\n{\"text\":\" Hello world \",\"topics\":[\" Work \",\"RUST\",\"one\",\"two\",\"three\",\"ignored\"],\"memories\":[{\"memory_type\":\"Learning\",\"subject\":\"Rust\",\"summary\":\"正在学习 Rust\",\"confidence\":1.2}]}\n```").unwrap();
        assert_eq!(output.text, "Hello world");
        assert_eq!(output.topics, ["work", "rust", "one", "two", "three"]);
        assert_eq!(output.memories[0].memory_type, "learning");
        assert_eq!(output.memories[0].confidence, 0.95);
    }

    #[test]
    fn plain_text_is_a_safe_fallback() {
        let output = parse_output("visible text").unwrap();
        assert_eq!(output.text, "visible text");
        assert!(output.topics.is_empty());
    }

    #[tokio::test]
    async fn sends_authenticated_multimodal_request_and_parses_response() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut request_bytes = Vec::new();
            loop {
                let mut chunk = [0_u8; 4096];
                let size = stream.read(&mut chunk).unwrap();
                if size == 0 {
                    break;
                }
                request_bytes.extend_from_slice(&chunk[..size]);
                let Some(header_end) = request_bytes
                    .windows(4)
                    .position(|part| part == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request_bytes[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                if request_bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&request_bytes).into_owned();
            let body = r#"{"choices":[{"message":{"content":"{\"text\":\"hello\",\"topics\":[\"work\"]}"}}]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            request
        });
        let output = OcrClient::new(endpoint, "secret-key".into(), "glm-4v-flash".into())
            .unwrap()
            .extract_png(b"png")
            .await
            .unwrap();
        assert_eq!(output.text, "hello");
        assert_eq!(output.topics, ["work"]);
        let request = server.join().unwrap().to_ascii_lowercase();
        assert!(request.contains("authorization: bearer secret-key"));
        assert!(request.contains("data:image/png;base64,"));
        let body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body["model"], "glm-4v-flash");
        assert_eq!(body["max_tokens"], 1024);
    }
}
