//! MiniMax's OpenAI-compatible API. Credentials stay on the server.
use super::ai_provider::*;
use async_trait::async_trait;
use base64::Engine;
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Clone)]
pub struct MiniMaxClient {
    http: reqwest::Client,
    endpoint: String,
    api_key: String,
    pub model: String,
}

pub fn setting(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
            dotenvy::from_path_iter(path)
                .ok()?
                .filter_map(Result::ok)
                .find(|(key, _)| key == name)
                .map(|(_, v)| v)
                .filter(|s| !s.trim().is_empty())
        })
}
impl MiniMaxClient {
    pub fn from_settings(settings: &Value) -> Result<Self, String> {
        let base = super::provider_catalog::endpoint(
            "minimax",
            settings["region"].as_str().unwrap_or("global"),
        );
        Self::new(
            format!("{base}/chat/completions"),
            settings["api_key"].as_str().unwrap_or_default().into(),
            settings["chat_model"]
                .as_str()
                .unwrap_or("MiniMax-M3.1-Flash")
                .into(),
        )
    }

    pub fn from_env() -> Result<Self, String> {
        let file = setting("MINIMAX_CONFIG_PATH")
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str::<Value>(&text).ok());
        let file_value = |keys: &[&str]| -> Option<String> {
            keys.iter()
                .find_map(|key| file.as_ref()?.pointer(key)?.as_str().map(str::to_owned))
        };
        let key = setting("MINIMAX_API_KEY")
            .or_else(|| setting("MINMAX_API_KEY"))
            .or_else(|| {
                file_value(&[
                    "/api_key",
                    "/env/MINIMAX_API_KEY",
                    "/env/ANTHROPIC_AUTH_TOKEN",
                    "/env/ANTHROPIC_API_KEY",
                    "/env/OPENAI_API_KEY",
                ])
            })
            .filter(|s| !s.trim().is_empty())
            .ok_or("尚未配置 MINIMAX_API_KEY")?;
        let base = setting("MINIMAX_BASE_URL")
            .or_else(|| {
                file_value(&[
                    "/base_url",
                    "/env/ANTHROPIC_BASE_URL",
                    "/env/OPENAI_BASE_URL",
                ])
            })
            .unwrap_or_else(|| "https://api.minimax.io/v1".into());
        let base = base
            .trim_end_matches('/')
            .trim_end_matches("/anthropic")
            .to_owned();
        let endpoint = if base.ends_with("/chat/completions") {
            base
        } else if base.ends_with("/v1") {
            format!("{base}/chat/completions")
        } else {
            format!("{base}/v1/chat/completions")
        };
        Self::new(
            endpoint,
            key,
            setting("MINIMAX_MODEL")
                .or_else(|| file_value(&["/model", "/env/ANTHROPIC_MODEL"]))
                .unwrap_or_else(|| "MiniMax-M3.1-Flash".into()),
        )
    }
    pub fn new(endpoint: String, api_key: String, model: String) -> Result<Self, String> {
        let url = url::Url::parse(&endpoint).map_err(|_| "MiniMax 地址无效")?;
        if !["http", "https"].contains(&url.scheme())
            || api_key.trim().is_empty()
            || model.trim().is_empty()
        {
            return Err("MiniMax 配置无效".into());
        }
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(65))
                .build()
                .map_err(|_| "无法初始化 MiniMax 客户端")?,
            endpoint,
            api_key,
            model,
        })
    }
    fn stream_body(&self, messages: Vec<ChatMessage>, fast: bool) -> Value {
        let mut body = json!({"model":self.model,"messages":messages,"stream":true,"max_completion_tokens":8192,"reasoning_split":true,"reasoning_effort":"low"});
        // M3 supports direct answers; M3.1 always thinks and uses effort=low.
        if fast && self.model == "MiniMax-M3" {
            body["thinking"] = json!({"type":"disabled"});
        }
        body
    }
    async fn stream_chat(
        &self,
        messages: Vec<ChatMessage>,
        chunks: tokio::sync::mpsc::Sender<String>,
        fast: bool,
    ) -> Result<String, AiProviderError> {
        let mut response = self
            .http
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&self.stream_body(messages, fast))
            .send()
            .await
            .map_err(|_| AiProviderError::Request("MiniMax stream connection failed".into()))?;
        if !response.status().is_success() {
            return Err(AiProviderError::Api(format!(
                "MiniMax stream HTTP {}",
                response.status().as_u16()
            )));
        }
        let mut parser = super::amigo::streaming::Sse::default();
        let mut raw = String::new();
        let mut sent = 0;
        let mut total = 0;
        let mut stopped = false;
        let mut done = false;
        while let Some(bytes) = response
            .chunk()
            .await
            .map_err(|_| AiProviderError::Request("MiniMax stream interrupted".into()))?
        {
            total += bytes.len();
            if total > 2 * 1024 * 1024 {
                return Err(AiProviderError::Parse(
                    "MiniMax stream exceeds limit".into(),
                ));
            }
            for event in parser.push(&bytes).map_err(AiProviderError::Parse)? {
                if event == "[DONE]" {
                    done = true;
                    break;
                }
                let value: Value = serde_json::from_str(&event)
                    .map_err(|_| AiProviderError::Parse("invalid MiniMax SSE JSON".into()))?;
                if value
                    .pointer("/base_resp/status_code")
                    .and_then(Value::as_i64)
                    .is_some_and(|c| c != 0)
                    || value.get("error").is_some()
                {
                    return Err(AiProviderError::Api("MiniMax stream rejected".into()));
                }
                if let Some(reason) = value
                    .pointer("/choices/0/finish_reason")
                    .and_then(Value::as_str)
                {
                    if reason != "stop" {
                        return Err(AiProviderError::Parse(
                            "MiniMax stream did not finish normally".into(),
                        ));
                    }
                    stopped = true;
                }
                // Reasoning fields are intentionally never forwarded or spoken.
                if let Some(delta) = value
                    .pointer("/choices/0/delta/content")
                    .and_then(Value::as_str)
                {
                    raw.push_str(delta);
                    if raw.len() > 16000 {
                        return Err(AiProviderError::Parse(
                            "MiniMax tutor JSON exceeds limit".into(),
                        ));
                    }
                    let mut visible = raw.trim_start();
                    loop {
                        if "<think>".starts_with(visible) {
                            visible = "";
                            break;
                        }
                        if visible.starts_with("<think>") {
                            if let Some((_, rest)) = visible.split_once("</think>") {
                                visible = rest.trim_start();
                            } else {
                                visible = "";
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    if !visible.is_empty() && visible.len() > sent {
                        chunks.send(visible[sent..].to_owned()).await.map_err(|_| {
                            AiProviderError::Request("MiniMax consumer cancelled".into())
                        })?;
                        sent = visible.len();
                    }
                }
            }
            if done {
                break;
            }
        }
        // MiniMax can close a successful stream after finish_reason=stop without
        // an OpenAI [DONE] sentinel. EOF without a terminal finish is truncation.
        if !stopped {
            return Err(AiProviderError::Parse(
                "MiniMax SSE ended without completion".into(),
            ));
        }
        final_answer(&raw)
            .ok_or_else(|| AiProviderError::Parse("MiniMax stream has no answer".into()))
    }
    async fn complete(&self, messages: Value) -> Result<String, AiProviderError> {
        let mut response=self.http.post(&self.endpoint).bearer_auth(&self.api_key).json(&json!({"model":self.model,"messages":messages,"stream":false,"max_tokens":8192,"reasoning_split":true,"reasoning_effort":"low"}))
            .send().await.map_err(|_|AiProviderError::Request("MiniMax connection failed".into()))?;
        if !response.status().is_success() {
            return Err(AiProviderError::Api(format!(
                "MiniMax HTTP {}",
                response.status().as_u16()
            )));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| AiProviderError::Request("MiniMax response interrupted".into()))?
        {
            if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
                return Err(AiProviderError::Parse(
                    "MiniMax response exceeds limit".into(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        let body: Value = serde_json::from_slice(&bytes)
            .map_err(|_| AiProviderError::Parse("invalid MiniMax response".into()))?;
        if body
            .pointer("/base_resp/status_code")
            .and_then(Value::as_i64)
            .is_some_and(|code| code != 0)
        {
            return Err(AiProviderError::Api(
                "MiniMax rejected the request; check endpoint and key permissions".into(),
            ));
        }
        let choice = body
            .pointer("/choices/0")
            .ok_or_else(|| AiProviderError::Parse("MiniMax returned no answer".into()))?;
        if choice["finish_reason"] == "length" {
            return Err(AiProviderError::Parse(
                "MiniMax output reached its token limit".into(),
            ));
        }
        let text = choice
            .pointer("/message/content")
            .and_then(Value::as_str)
            .ok_or_else(|| AiProviderError::Parse("MiniMax returned empty content".into()))?;
        final_answer(text).ok_or_else(|| {
            AiProviderError::Parse("MiniMax returned thinking without an answer".into())
        })
    }
    pub async fn vision(
        &self,
        media: &[u8],
        mime: &str,
        camera: bool,
    ) -> Result<super::ocr::OcrOutput, String> {
        if self.model.starts_with("MiniMax-M2") {
            return Err("此 MiniMax 模型不支持图片，请配置视觉模型".into());
        }
        let url = format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(media)
        );
        let prompt = if camera {
            super::ocr::camera_prompt()
        } else {
            super::ocr::OCR_PROMPT.into()
        };
        let text=self.complete(json!([{"role":"user","content":[{"type":"text","text":prompt},{"type":"image_url","image_url":{"url":url}}]}])).await.map_err(|e|e.to_string())?;
        super::ocr::parse_output(&text)
    }
}
fn final_answer(text: &str) -> Option<String> {
    let mut text = text.trim();
    while text.starts_with("<think>") {
        text = text.split_once("</think>")?.1.trim();
    }
    (!text.is_empty()).then(|| text.to_owned())
}
#[async_trait]
impl ChatService for MiniMaxClient {
    async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        chunks: tokio::sync::mpsc::Sender<String>,
    ) -> Result<String, AiProviderError> {
        self.stream_chat(messages, chunks, false).await
    }
    async fn chat_stream_fast(
        &self,
        messages: Vec<ChatMessage>,
        chunks: tokio::sync::mpsc::Sender<String>,
    ) -> Result<String, AiProviderError> {
        self.stream_chat(messages, chunks, true).await
    }
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        _temperature: Option<f32>,
        _max_tokens: Option<u32>,
    ) -> Result<String, AiProviderError> {
        self.complete(
            serde_json::to_value(messages)
                .map_err(|_| AiProviderError::Parse("invalid chat messages".into()))?,
        )
        .await
    }
    async fn chat_structured(
        &self,
        mut messages: Vec<ChatMessage>,
        user_text: &str,
        system_prompt: &str,
    ) -> Result<StructuredChatResponse, AiProviderError> {
        messages.insert(
            0,
            ChatMessage {
                role: "system".into(),
                content: system_prompt.into(),
            },
        );
        messages.push(ChatMessage {
            role: "user".into(),
            content: user_text.into(),
        });
        let text = self.chat(messages, None, None).await?;
        serde_json::from_str(
            text.trim()
                .trim_start_matches("```json")
                .trim_end_matches("```")
                .trim(),
        )
        .map_err(|_| AiProviderError::Parse("invalid MiniMax teaching JSON".into()))
    }
}
pub struct MiniMaxRobotProvider {
    chat: Arc<MiniMaxClient>,
    speech: Arc<dyn AiProvider>,
}
impl MiniMaxRobotProvider {
    pub fn selected(chat: MiniMaxClient, speech: Arc<dyn AiProvider>) -> Self {
        Self {
            chat: Arc::new(chat),
            speech,
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let chat = Arc::new(MiniMaxClient::from_env()?);
        let speech = super::ZhipuClient::from_env()
            .map(|p| Arc::new(p) as Arc<dyn AiProvider>)
            .ok_or("尚未配置语音识别和语音合成服务")?;
        Ok(Self { chat, speech })
    }
}
#[async_trait]
impl AiProvider for MiniMaxRobotProvider {
    fn name(&self) -> &'static str {
        "minimax"
    }
    fn asr(&self) -> Option<Arc<dyn AsrService>> {
        self.speech.asr()
    }
    fn tts(&self) -> Option<Arc<dyn TtsService>> {
        self.speech.tts()
    }
    fn chat_service(&self) -> Option<Arc<dyn ChatService>> {
        Some(self.chat.clone())
    }
}
pub fn runtime_status() -> Value {
    let config = MiniMaxClient::from_env();
    let model = config
        .as_ref()
        .map(|c| c.model.clone())
        .unwrap_or_else(|_| {
            setting("MINIMAX_MODEL").unwrap_or_else(|| "MiniMax-M3.1-Flash".into())
        });
    let configured = config.is_ok();
    let voice_ready = super::ZhipuClient::from_env().is_some();
    json!({"provider":"MiniMax","model":model,"configured":configured,"voice_ready":voice_ready,"ready":configured&&voice_ready,"message":config.err(),"asr":"已配置的语音服务","vision":if model.starts_with("MiniMax-M2") {"智谱视觉"}else{"MiniMax"},"voice_studio":super::amigo::voice_studio::runtime_status()})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_reasoning_never_becomes_spoken_text() {
        assert_eq!(
            final_answer("<think>private</think>\n{\"reply_en\":\"Hello\"}"),
            Some("{\"reply_en\":\"Hello\"}".into())
        );
        assert_eq!(final_answer("<think>unfinished"), None);
    }
    #[tokio::test]
    async fn actual_sse_transport_hides_reasoning_and_rejects_truncated_completion() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for (terminal, finish) in [(true, true), (false, true), (false, false)] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut block = [0; 4096];
                loop {
                    let size = socket.read(&mut block).await.unwrap();
                    assert!(size > 0);
                    request.extend_from_slice(&block[..size]);
                    if let Some(at) = request.windows(4).position(|b| b == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request[..at]).to_lowercase();
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                line.strip_prefix("content-length:")
                                    .and_then(|v| v.trim().parse::<usize>().ok())
                            })
                            .unwrap();
                        if request.len() >= at + 4 + length {
                            assert_eq!(
                                serde_json::from_slice::<Value>(&request[at + 4..at + 4 + length])
                                    .unwrap()["stream"],
                                true
                            );
                            break;
                        }
                    }
                }
                let mut body = String::new();
                for value in [
                    json!({"choices":[{"delta":{"reasoning_content":"PRIVATE"}}]}),
                    json!({"choices":[{"delta":{"content":"<thi"}}]}),
                    json!({"choices":[{"delta":{"content":"nk>HIDDEN</think>{\"reply_en\":\"Hi 世界.\"}"}}]}),
                ] {
                    body.push_str(&format!("data: {value}\r\n\r\n"));
                }
                if finish {
                    body.push_str(&format!(
                        "data: {}\r\n\r\n",
                        json!({"choices":[{"delta":{},"finish_reason":"stop"}]})
                    ));
                }
                if terminal {
                    body.push_str("data: [DONE]\n\n");
                }
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).as_bytes()).await.unwrap();
                for byte in body.as_bytes() {
                    socket.write_all(&[*byte]).await.unwrap();
                }
            });
            let client = MiniMaxClient::new(
                format!("http://{address}/v1/chat/completions"),
                "synthetic-test-key".into(),
                "test-model".into(),
            )
            .unwrap();
            let (tx, mut rx) = tokio::sync::mpsc::channel(8);
            let result = client.chat_stream(vec![], tx).await;
            let mut spoken = String::new();
            while let Some(part) = rx.recv().await {
                spoken.push_str(&part);
            }
            assert!(!spoken.contains("PRIVATE") && !spoken.contains("HIDDEN"));
            if finish {
                assert_eq!(result.unwrap(), "{\"reply_en\":\"Hi 世界.\"}");
                assert_eq!(spoken, "{\"reply_en\":\"Hi 世界.\"}");
            } else {
                assert!(result.is_err());
            }
            server.await.unwrap();
        }
    }
    #[test]
    fn direct_answer_is_only_used_for_m3_fast_continuation() {
        let mut client = MiniMaxClient::new(
            "http://localhost/chat".into(),
            "test-key".into(),
            "MiniMax-M3".into(),
        )
        .unwrap();
        assert_eq!(
            client.stream_body(vec![], true)["thinking"]["type"],
            "disabled"
        );
        assert!(client.stream_body(vec![], false).get("thinking").is_none());
        client.model = "MiniMax-M3.1-Flash".into();
        assert!(client.stream_body(vec![], true).get("thinking").is_none());
        assert_eq!(client.stream_body(vec![], true)["reasoning_effort"], "low");
    }
}
