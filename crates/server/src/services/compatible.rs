//! Bounded HTTP adapters for fixed official endpoints. Never includes upstream bodies in errors.
use super::ai_provider::*;
use super::{chat_stream, provider_catalog};
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
pub struct CompatibleClient {
    kind: String,
    settings: Value,
    client: reqwest::Client,
    base: String,
}
impl CompatibleClient {
    pub fn new(kind: &str, settings: Value) -> Self {
        let base =
            provider_catalog::endpoint(kind, settings["region"].as_str().unwrap_or("global"))
                .to_owned();
        Self {
            kind: kind.into(),
            settings,
            base,
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(90))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("HTTP client"),
        }
    }
    pub async fn probe_asr(&self) -> Result<(), AiProviderError> {
        // Exercise the configured transcription endpoint with a short valid WAV.
        // Silence may legitimately produce an empty transcript; transport/auth/model success is enough here.
        let mut audio = b"RIFF".to_vec();
        audio.extend_from_slice(&32036u32.to_le_bytes());
        audio.extend_from_slice(b"WAVEfmt ");
        audio.extend_from_slice(&16u32.to_le_bytes());
        audio.extend_from_slice(&1u16.to_le_bytes());
        audio.extend_from_slice(&1u16.to_le_bytes());
        audio.extend_from_slice(&16000u32.to_le_bytes());
        audio.extend_from_slice(&32000u32.to_le_bytes());
        audio.extend_from_slice(&2u16.to_le_bytes());
        audio.extend_from_slice(&16u16.to_le_bytes());
        audio.extend_from_slice(b"data");
        audio.extend_from_slice(&32000u32.to_le_bytes());
        audio.resize(32044, 0);
        let part = reqwest::multipart::Part::bytes(audio)
            .file_name("silence.wav")
            .mime_str("audio/wav")
            .map_err(|_| AiProviderError::Config("音频格式无效".into()))?;
        let form = reqwest::multipart::Form::new()
            .text(
                "model",
                self.settings["asr_model"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            )
            .part("file", part);
        let v = self
            .json(self.request("/audio/transcriptions").multipart(form))
            .await?;
        if v.get("text").and_then(Value::as_str).is_none() {
            return Err(AiProviderError::Parse("ASR 响应格式无效".into()));
        }
        Ok(())
    }
    fn request(&self, path: &str) -> reqwest::RequestBuilder {
        let r = self.client.post(format!("{}{path}", self.base));
        if self.kind == "anthropic" {
            r.header(
                "x-api-key",
                self.settings["api_key"].as_str().unwrap_or_default(),
            )
            .header("anthropic-version", "2023-06-01")
        } else {
            r.bearer_auth(self.settings["api_key"].as_str().unwrap_or_default())
        }
    }
    async fn checked(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, AiProviderError> {
        let response = request
            .send()
            .await
            .map_err(|_| AiProviderError::Api("AI 服务连接失败或超时，请重试".into()))?;
        if !response.status().is_success() {
            return Err(AiProviderError::Api(format!(
                "AI 服务返回 HTTP {}，请检查密钥、模型权限、余额与服务区域",
                response.status().as_u16()
            )));
        }
        Ok(response)
    }
    async fn json(&self, request: reqwest::RequestBuilder) -> Result<Value, AiProviderError> {
        let r = self.checked(request).await?;
        let mut stream = r.bytes_stream();
        let mut raw = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| AiProviderError::Api("AI 连接中断".into()))?;
            if raw.len() + chunk.len() > 24 * 1024 * 1024 {
                return Err(AiProviderError::Parse("AI 响应超过限制".into()));
            }
            raw.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&raw).map_err(|_| AiProviderError::Parse("AI 响应格式无效".into()))
    }
    fn chat_body(
        &self,
        messages: Vec<ChatMessage>,
        limit: Option<u32>,
        stream: bool,
    ) -> (String, Value) {
        let model = self.settings["chat_model"].as_str().unwrap_or_default();
        if self.kind == "anthropic" {
            let system = messages
                .iter()
                .filter(|m| m.role == "system")
                .map(|m| m.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let messages: Vec<Value> = messages
                .into_iter()
                .filter(|m| m.role != "system")
                .map(|m| json!({"role":m.role,"content":m.content}))
                .collect();
            (
                "/messages".into(),
                json!({"model":model,"system":system,"messages":messages,"max_tokens":limit.unwrap_or(1600),"stream":stream}),
            )
        } else {
            let mut body = json!({"model":model,"messages":messages,"stream":stream});
            if self.kind == "openai" || self.kind == "minimax" {
                if let Some(n) = limit {
                    body["max_completion_tokens"] = json!(n);
                }
            } else if let Some(n) = limit {
                body["max_tokens"] = json!(n);
            }
            // Disable optional reasoning for conversational latency where documented.
            if self.kind == "deepseek" {
                body["thinking"] = json!({"type":"disabled"});
            }
            if self.kind == "qwen" {
                body["enable_thinking"] = json!(false);
            }
            if self.kind=="kimi" {
                if model.starts_with("kimi-k3") {body["reasoning_effort"]=json!("low");body["max_tokens"]=json!(limit.unwrap_or(4000).max(4000));}
                else if !model.contains("thinking") && !model.contains("k2.7-code"){body["thinking"]=json!({"type":"disabled"});}
            }
            if self.kind=="minimax" {
                body["reasoning_split"]=json!(true);
                if model.starts_with("MiniMax-M3.1") {body["reasoning_effort"]=json!("low");body["max_completion_tokens"]=json!(limit.unwrap_or(4000).max(4000));}
                else if model=="MiniMax-M3" {body["thinking"]=json!({"type":"disabled"});}
            }

            ("/chat/completions".into(), body)
        }
    }
}

#[async_trait]
impl ChatService for CompatibleClient {
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        _temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<String, AiProviderError> {
        let (path, body) = self.chat_body(messages, max_tokens, false);
        let v = self.json(self.request(&path).json(&body)).await?;
        let text = if self.kind == "anthropic" {
            v["content"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v["text"].as_str())
                        .collect::<String>()
                })
                .unwrap_or_default()
        } else {
            v["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or_default()
                .into()
        };
        if text.trim().is_empty() {
            return Err(AiProviderError::Parse("AI 未返回有效回复".into()));
        }
        Ok(text)
    }
    async fn chat_structured(
        &self,
        history: Vec<ChatMessage>,
        text: &str,
        prompt: &str,
    ) -> Result<StructuredChatResponse, AiProviderError> {
        let raw = self
            .chat(
                chat_stream::messages(history, text, prompt),
                None,
                Some(2000),
            )
            .await?;
        chat_stream::finish(&raw)
    }
    async fn chat_structured_stream(
        &self,
        history: Vec<ChatMessage>,
        text: &str,
        prompt: &str,
        deltas: tokio::sync::mpsc::UnboundedSender<String>,
    ) -> Result<StructuredChatResponse, AiProviderError> {
        let (path, body) = self.chat_body(
            chat_stream::messages(history, text, prompt),
            Some(2000),
            true,
        );
        let response = self.checked(self.request(&path).json(&body)).await?;
        let mut chunks = response.bytes_stream();
        let mut decoder = StreamDecoder::default();
        while let Some(chunk) = chunks.next().await {
            let chunk =
                chunk.map_err(|_| AiProviderError::Api("AI 流式连接中断，请重试".into()))?;
            decoder.push(&chunk, self.kind == "anthropic", &deltas)?;
            if decoder.done {
                break;
            }
        }
        if !decoder.done {
            return Err(AiProviderError::Parse("AI 流式回答未完成，请重试".into()));
        }
        chat_stream::finish(&decoder.text)
    }
}

#[derive(Default)]
struct StreamDecoder {
    pending: Vec<u8>,
    text: String,
    done: bool,
    finished: bool,
}
impl StreamDecoder {
    fn push(
        &mut self,
        chunk: &[u8],
        anthropic: bool,
        deltas: &tokio::sync::mpsc::UnboundedSender<String>,
    ) -> Result<(), AiProviderError> {
        self.pending.extend_from_slice(chunk);
        while let Some(end) = self.pending.iter().position(|c| *c == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();
            let line = std::str::from_utf8(&line)
                .map_err(|_| AiProviderError::Parse("流式编码无效".into()))?
                .trim();
            let Some(data) = line.strip_prefix("data:").map(str::trim) else {
                continue;
            };
            if data == "[DONE]" {
                self.done = self.finished;
                break;
            }
            if data.is_empty() {
                continue;
            }
            let v: Value = serde_json::from_str(data)
                .map_err(|_| AiProviderError::Parse("流式响应格式无效".into()))?;
            if v.get("error").is_some() || v["type"] == "error" {
                return Err(AiProviderError::Api("AI 流式服务返回错误".into()));
            }
            let delta = if anthropic {
                if v["type"] == "message_delta" {
                    self.finished = matches!(
                        v["delta"]["stop_reason"].as_str(),
                        Some("end_turn" | "stop_sequence")
                    );
                    if !self.finished {
                        return Err(AiProviderError::Parse("AI 回答被截断".into()));
                    }
                }
                if v["type"] == "message_stop" {
                    self.done = self.finished;
                }
                if v["delta"]["type"] == "text_delta" {
                    v["delta"]["text"].as_str()
                } else {
                    None
                }
            } else {
                if let Some(reason) = v["choices"][0]["finish_reason"].as_str() {
                    if reason != "stop" {
                        return Err(AiProviderError::Parse("AI 回答被截断，请重试".into()));
                    }
                    self.finished = true;
                    self.done = true;
                }
                v["choices"][0]["delta"]["content"].as_str()
            };
            if let Some(delta) = delta {
                if self.text.len() + delta.len() > chat_stream::MAX_OUTPUT_BYTES {
                    return Err(AiProviderError::Parse("AI 回答超过限制".into()));
                }
                self.text.push_str(delta);
                let _ = deltas.send(delta.into());
            }
        }
        if self.pending.len() > chat_stream::MAX_OUTPUT_BYTES {
            return Err(AiProviderError::Parse("AI 流式事件超过限制".into()));
        }
        Ok(())
    }
}

#[async_trait]
impl AsrService for CompatibleClient {
    async fn transcribe(
        &self,
        audio: Vec<u8>,
        language: Option<&str>,
    ) -> Result<AsrResponse, AiProviderError> {
        let model = provider_catalog::model(&self.settings, "asr")
            .ok_or_else(|| AiProviderError::NotSupported("未配置 ASR 模型".into()))?;
        let part = reqwest::multipart::Part::bytes(audio)
            .file_name("recording.wav")
            .mime_str("audio/wav")
            .map_err(|_| AiProviderError::Config("音频格式无效".into()))?;
        let mut form = reqwest::multipart::Form::new()
            .text("model", model.clone())
            .part("file", part);
        if let Some(lang) = language {
            form = if model == "gpt-transcribe" {
                form.text("languages[]", lang.to_owned())
            } else {
                form.text("language", lang.to_owned())
            };
        }
        let v = self
            .json(self.request("/audio/transcriptions").multipart(form))
            .await?;
        let text = v["text"]
            .as_str()
            .filter(|t| !t.trim().is_empty())
            .ok_or_else(|| AiProviderError::Parse("未识别到有效语音".into()))?;
        Ok(AsrResponse {
            text: text.into(),
            confidence: None,
            words: None,
        })
    }
    fn supported_formats(&self) -> Vec<&'static str> {
        vec!["wav", "mp3", "webm", "m4a"]
    }
}
#[async_trait]
impl TtsService for CompatibleClient {
    async fn synthesize(
        &self,
        text: &str,
        _voice: Option<&str>,
        speed: Option<f32>,
    ) -> Result<TtsResponse, AiProviderError> {
        let model = provider_catalog::model(&self.settings, "tts")
            .ok_or_else(|| AiProviderError::NotSupported("未配置 TTS 模型".into()))?;
        let voice = self.settings["tts_voice"].as_str().unwrap_or("alloy");
        let audio = if self.kind == "minimax" {
            let v=self.json(self.request("/t2a_v2").json(&json!({"model":model,"text":text,"stream":false,"output_format":"hex","voice_setting":{"voice_id":voice,"speed":speed.unwrap_or(1.0).clamp(0.5,2.0),"vol":1,"pitch":0},"audio_setting":{"sample_rate":32000,"bitrate":128000,"format":"mp3","channel":1}}))).await?;
            if v["base_resp"]["status_code"].as_i64() != Some(0) {
                return Err(AiProviderError::Api(
                    "MiniMax 语音合成失败，请检查密钥、模型与音色权限".into(),
                ));
            }
            hex::decode(v["data"]["audio"].as_str().unwrap_or_default())
                .map_err(|_| AiProviderError::Parse("语音数据格式无效".into()))?
        } else {
            let response=self.checked(self.request("/audio/speech").json(&json!({"model":model,"input":text,"voice":voice,"speed":speed.unwrap_or(1.0).clamp(0.25,4.0),"response_format":"mp3"}))).await?;
            let mut chunks = response.bytes_stream();
            let mut bytes = Vec::new();
            while let Some(chunk) = chunks.next().await {
                let chunk = chunk.map_err(|_| AiProviderError::Api("语音传输中断".into()))?;
                if bytes.len() + chunk.len() > 12 * 1024 * 1024 {
                    return Err(AiProviderError::Parse("语音超过限制".into()));
                }
                bytes.extend_from_slice(&chunk);
            }
            bytes
        };
        if audio.is_empty() {
            return Err(AiProviderError::Parse("未生成有效语音".into()));
        }
        Ok(TtsResponse {
            audio_data: audio,
            format: "mp3".into(),
            duration_ms: None,
        })
    }
    fn available_voices(&self) -> Vec<&'static str> {
        vec![]
    }
}
impl AiProvider for CompatibleClient {
    fn name(&self) -> &'static str {
        match self.kind.as_str() {
            "openai" => "openai",
            "deepseek" => "deepseek",
            "minimax" => "minimax",
            "kimi" => "kimi",
            "qwen" => "qwen",
            "gemini" => "gemini",
            "anthropic" => "anthropic",
            "siliconflow" => "siliconflow",
            "openrouter" => "openrouter",
            "xai" => "xai",
            "mistral" => "mistral",
            "groq" => "groq",
            "baidu" => "baidu",
            "hunyuan" => "hunyuan",
            _ => "unknown",
        }
    }
    fn chat_service(&self) -> Option<Arc<dyn ChatService>> {
        provider_catalog::model(&self.settings, "chat").map(|_| Arc::new(self.clone()) as _)
    }
    fn tts(&self) -> Option<Arc<dyn TtsService>> {
        provider_catalog::model(&self.settings, "tts").map(|_| Arc::new(self.clone()) as _)
    }
    fn asr(&self) -> Option<Arc<dyn AsrService>> {
        provider_catalog::model(&self.settings, "asr").map(|_| Arc::new(self.clone()) as _)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    fn mock(
        kind: &str,
        response: &[u8],
        content_type: &str,
    ) -> (CompatibleClient, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let response = response.to_vec();
        let content_type = content_type.to_owned();
        let thread = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 4096];
            loop {
                let n = socket.read(&mut chunk).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&chunk[..n]);
                if let Some(pos) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..pos]);
                    let len = headers
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|s| s.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= pos + 4 + len {
                        break;
                    }
                }
                assert!(request.len() < 1024 * 1024);
            }
            write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
            for chunk in response.chunks(7) {
                write!(socket, "{:x}\r\n", chunk.len()).unwrap();
                socket.write_all(chunk).unwrap();
                socket.write_all(b"\r\n").unwrap();
                socket.flush().unwrap();
            }
            socket.write_all(b"0\r\n\r\n").unwrap();
            String::from_utf8_lossy(&request).into_owned()
        });
        let settings =
            provider_catalog::validate(kind, json!({"api_key":"mock-private-key"})).unwrap();
        let mut client = CompatibleClient::new(kind, settings);
        client.base = format!("http://127.0.0.1:{port}");
        (client, thread)
    }
    #[tokio::test]
    async fn every_text_adapter_uses_selected_model_and_auth() {
        for p in provider_catalog::catalog()
            .into_iter()
            .filter(|p| p.id != "zhipu")
        {
            let response = if p.id == "anthropic" {
                json!({"content":[{"type":"text","text":"OK"}]})
            } else {
                json!({"choices":[{"message":{"content":"OK"}}]})
            };
            let (client, request) = mock(p.id, response.to_string().as_bytes(), "application/json");
            assert_eq!(
                client
                    .chat(
                        vec![
                            ChatMessage {
                                role: "system".into(),
                                content: "test prompt".into()
                            },
                            ChatMessage {
                                role: "user".into(),
                                content: "Hi".into()
                            }
                        ],
                        None,
                        Some(16)
                    )
                    .await
                    .unwrap(),
                "OK"
            );
            let request = request.join().unwrap();
            let body: Value =
                serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
            assert_eq!(body["model"], p.chat_models[0]);
            if p.id == "anthropic" {
                assert!(request.contains("/messages "));
                assert!(request.to_ascii_lowercase().contains("x-api-key: mock-private-key"));
                assert_eq!(body["system"], "test prompt");
                assert_eq!(body["messages"].as_array().unwrap().len(), 1);
            } else {
                assert!(request.contains("/chat/completions "));
                assert!(request.to_ascii_lowercase().contains("authorization: bearer mock-private-key"));
            }
        }
    }
    #[tokio::test]
    async fn real_http_stream_handles_fragmented_unicode() {
        for kind in ["openai", "anthropic"] {
            let raw=json!({"reply_en":"Hello 你好","reply_zh":"你好","use_lang":"en","original_en":"Hi","original_zh":"你好","issues":[]}).to_string();
            let event = if kind == "anthropic" {
                format!(
                    "data: {}\n\ndata: {}\n\ndata: {}\n\n",
                    json!({"type":"content_block_delta","delta":{"type":"text_delta","text":raw}}),
                    json!({"type":"message_delta","delta":{"stop_reason":"end_turn"}}),
                    json!({"type":"message_stop"})
                )
            } else {
                format!(
                    "data: {}\r\n\r\ndata: {}\r\n\r\ndata: [DONE]\r\n\r\n",
                    json!({"choices":[{"delta":{"content":raw,"reasoning_content":"hidden"},"finish_reason":null}]}),
                    json!({"choices":[{"delta":{},"finish_reason":"stop"}]})
                )
            };
            let (client, thread) = mock(kind, event.as_bytes(), "text/event-stream");
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let result = client
                .chat_structured_stream(vec![], "Hi", chat_stream::SYSTEM_PROMPT, tx)
                .await
                .unwrap();
            assert_eq!(result.reply_en, "Hello 你好");
            assert!(!rx.recv().await.unwrap().contains("hidden"));
            thread.join().unwrap();
        }
    }
    #[tokio::test]
    async fn speech_uses_separate_models_multipart_and_binary() {
        let (client, thread) = mock("openai", br#"{"text":"Hello"}"#, "application/json");
        assert_eq!(
            client
                .transcribe(b"RIFFtest-audio".to_vec(), Some("en"))
                .await
                .unwrap()
                .text,
            "Hello"
        );
        let request = thread.join().unwrap();
        assert!(request.contains("/audio/transcriptions "));
        assert!(request.contains("multipart/form-data; boundary="));
        assert!(request.contains("name=\"model\"\r\n\r\ngpt-transcribe"));
        assert!(request.contains("name=\"languages[]\""));
        assert!(request.contains("RIFFtest-audio"));
        let (client, thread) = mock("openai", b"ID3mock-mp3", "audio/mpeg");
        assert_eq!(
            client
                .synthesize("Hello", None, None)
                .await
                .unwrap()
                .audio_data,
            b"ID3mock-mp3"
        );
        let request = thread.join().unwrap();
        assert!(request.contains("gpt-4o-mini-tts"));
        assert!(!request.contains("gpt-transcribe"));
        let (client, thread) = mock(
            "minimax",
            br#"{"base_resp":{"status_code":0},"data":{"audio":"494433"}}"#,
            "application/json",
        );
        assert_eq!(
            client
                .synthesize("Hello", None, None)
                .await
                .unwrap()
                .audio_data,
            b"ID3"
        );
        let request = thread.join().unwrap();
        assert!(request.contains("/t2a_v2 "));
        assert!(request.contains("speech-2.8-hd"));
        assert!(request.contains("English_expressive_narrator"));
    }
    #[test]
    fn truncated_and_overlong_streams_rejected() {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let mut d = StreamDecoder::default();
        assert!(
            d.push(
                b"data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n",
                false,
                &tx
            )
            .is_err()
        );
        assert!(
            StreamDecoder::default()
                .push(&vec![b'x'; chat_stream::MAX_OUTPUT_BYTES + 1], false, &tx)
                .is_err()
        );
        let mut d = StreamDecoder::default();
        d.push(b"data: [DONE]\n", false, &tx).unwrap();
        assert!(!d.done);
    }
}
