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
    realtime: Arc<super::openai_realtime::Pool>,
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
            realtime: Arc::new(super::openai_realtime::Pool::default()),
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
        let audio = super::ai_provider::probe_audio();
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
        let mut response = request
            .send()
            .await
            .map_err(|_| AiProviderError::Api("AI 服务连接失败或超时，请重试".into()))?;
        if !response.status().is_success() {
            let status=response.status().as_u16();
            let request_id=response.headers().get("x-request-id").or_else(||response.headers().get("request-id")).and_then(|v|v.to_str().ok()).and_then(diagnostic_field);
            // Only structured identifiers are retained. Provider error messages
            // can echo submitted audio/text or credentials and are never logged.
            let fields=tokio::time::timeout(Duration::from_secs(2),async {
                let mut bytes=Vec::new();
                while let Ok(Some(chunk))=response.chunk().await {
                    if bytes.len()+chunk.len()>16*1024{return Value::Null;}
                    bytes.extend_from_slice(&chunk);
                }
                serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null)
            }).await.unwrap_or(Value::Null);
            let secret=self.settings["api_key"].as_str().unwrap_or_default();
            let mut details=Vec::new();
            for (name,field) in [("code",fields.pointer("/error/code")),("param",fields.pointer("/error/param"))] {
                if let Some(value)=field.and_then(Value::as_str).and_then(diagnostic_field).filter(|v|secret.is_empty()||!v.contains(secret)){details.push(format!("{name}={value}"));}
            }
            if let Some(id)=request_id.filter(|v|secret.is_empty()||!v.contains(secret)){details.push(format!("request_id={id}"));}
            let details=if details.is_empty(){String::new()}else{format!(" · {}",details.join(" · "))};
            return Err(AiProviderError::Api(format!("AI 服务返回 HTTP {status}{details}，请检查请求参数、模型权限、密钥、余额与服务区域")));
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
            // Only the newly listed GPT-6 text families receive these fields.
            if self.kind == "openai" && model.starts_with("gpt-6") {
                let no_reasoning = model.starts_with("gpt-6-luna") || model.starts_with("gpt-6.1-terra");
                body["reasoning_effort"] = json!(if no_reasoning { "none" } else { "low" });
                if !no_reasoning { body["max_completion_tokens"] = json!(limit.unwrap_or(8192).max(8192)); }
            }
            // Disable optional reasoning for conversational latency where documented.
            if self.kind == "zhipu" && model.starts_with("glm-5") {
                body["thinking"] = json!({"type":"enabled"});
                body["reasoning_effort"] = json!("low");
                body["max_tokens"] = json!(limit.unwrap_or(8192).max(8192));
            }
            if self.kind == "deepseek" {
                body["thinking"] = json!({"type":"disabled"});
            }
            if self.kind == "qwen" {
                body["enable_thinking"] = json!(false);
            }
            if self.kind == "kimi" {
                if model.starts_with("kimi-k3") {
                    body["reasoning_effort"] = json!("low");
                    body["max_tokens"] = json!(limit.unwrap_or(4000).max(4000));
                } else if !model.contains("thinking") && !model.contains("k2.7-code") {
                    body["thinking"] = json!({"type":"disabled"});
                }
            }
            if self.kind == "minimax" {
                body["reasoning_split"] = json!(true);
                if model.starts_with("MiniMax-M3.1") {
                    body["reasoning_effort"] = json!("low");
                    body["max_completion_tokens"] = json!(limit.unwrap_or(4000).max(4000));
                } else if model == "MiniMax-M3" {
                    body["thinking"] = json!({"type":"disabled"});
                }
            }

            ("/chat/completions".into(), body)
        }
    }
}

fn diagnostic_field(value:&str)->Option<String>{
    (!value.is_empty()&&value.len()<=128&&!value.to_ascii_lowercase().starts_with("sk-")&&value.bytes().all(|c|c.is_ascii_alphanumeric()||b"-_.:/[]".contains(&c))).then(||value.to_owned())
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
        // The shared ASR contract accepts "auto". OpenAI/Groq expect an
        // omitted hint for detection, never the literal ISO language "auto".
        if let Some(lang) = language.map(str::trim).filter(|lang|!lang.is_empty()&&!lang.eq_ignore_ascii_case("auto")) {
            form = if model == "gpt-transcribe" || model.starts_with("gpt-transcribe-") {
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
            .ok_or_else(|| AiProviderError::Parse("语音识别响应缺少 text 字段".into()))?;
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
/// PCM chunks are increments. An optional final aggregate must never replay them.
#[derive(Default)]
struct MiniMaxPcm {
    bytes: usize,
    complete: bool,
}
impl MiniMaxPcm {
    fn push(&mut self, value: &Value) -> Result<Vec<Vec<u8>>, AiProviderError> {
        if value
            .pointer("/base_resp/status_code")
            .and_then(Value::as_i64)
            .is_some_and(|c| c != 0)
        {
            return Err(AiProviderError::Api(
                "MiniMax TTS rejected the request; check voice/model/key".into(),
            ));
        }
        let status = value.pointer("/data/status").and_then(Value::as_i64);
        let encoded = value
            .pointer("/data/audio")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let mut pcm =
            hex::decode(encoded).map_err(|_| AiProviderError::Parse("invalid PCM hex".into()))?;
        if !pcm.len().is_multiple_of(2) {
            return Err(AiProviderError::Parse("unaligned PCM chunk".into()));
        }
        if self.complete && !pcm.is_empty() {
            return Err(AiProviderError::Parse("audio after TTS completion".into()));
        }
        if status == Some(2) {
            // exclude_aggregated_audio=true guarantees the final packet contains
            // only any remaining increment, never a replay of earlier chunks.
            self.complete = true;
        }
        if self.bytes + pcm.len() > amigo::protocol::MAX_REPLY_BYTES {
            return Err(AiProviderError::Parse("TTS audio exceeds limit".into()));
        }
        self.bytes += pcm.len();
        Ok(pcm.chunks(6400).map(<[u8]>::to_vec).collect())
    }
    fn finish(&self) -> Result<(), AiProviderError> {
        if !self.complete || self.bytes == 0 {
            Err(AiProviderError::Parse(
                "incomplete MiniMax TTS stream".into(),
            ))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl TtsService for CompatibleClient {
    fn supports_pcm_stream(&self) -> bool {
        self.kind == "minimax" || self.kind == "openai" && super::openai_realtime::is_model(self.settings["tts_model"].as_str().unwrap_or_default())
    }
    async fn synthesize_pcm_stream(
        &self,
        text: &str,
        voice: Option<&str>,
        speed: Option<f32>,
        audio: tokio::sync::mpsc::Sender<Vec<u8>>,
    ) -> Result<(), AiProviderError> {
        if self.kind == "openai" && super::openai_realtime::is_model(self.settings["tts_model"].as_str().unwrap_or_default()) {
            return self.realtime.stream(&self.settings,text,voice,speed,audio).await;
        }
        if self.kind != "minimax" {
            return Err(AiProviderError::NotSupported("PCM streaming TTS".into()));
        }
        let model = provider_catalog::model(&self.settings, "tts")
            .ok_or_else(|| AiProviderError::Config("未配置 TTS 模型".into()))?;
        let voice = voice
            .filter(|v| !v.is_empty())
            .or_else(|| self.settings["tts_voice"].as_str())
            .unwrap_or("English_expressive_narrator");
        let mut response=self.checked(self.request("/t2a_v2").json(&json!({"model":model,"text":text,"stream":true,"stream_options":{"exclude_aggregated_audio":true},"output_format":"hex","voice_setting":{"voice_id":voice,"speed":speed.unwrap_or(1.0).clamp(0.5,2.0),"vol":1,"pitch":0},"audio_setting":{"sample_rate":16000,"format":"pcm","channel":1}}))).await?;
        let mut parser = super::amigo::streaming::Sse::audio();
        let mut decoder = MiniMaxPcm::default();
        let mut network_bytes = 0;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| AiProviderError::Request("MiniMax TTS stream interrupted".into()))?
        {
            network_bytes += chunk.len();
            if network_bytes > 8 * 1024 * 1024 {
                return Err(AiProviderError::Parse("TTS stream exceeds limit".into()));
            }
            for event in parser.push(&chunk).map_err(AiProviderError::Parse)? {
                if event == "[DONE]" {
                    continue;
                }
                let value: Value = serde_json::from_str(&event)
                    .map_err(|_| AiProviderError::Parse("invalid TTS event".into()))?;
                for part in decoder.push(&value)? {
                    audio
                        .send(part)
                        .await
                        .map_err(|_| AiProviderError::Request("TTS cancelled".into()))?;
                }
            }
        }
        decoder.finish()
    }

    async fn synthesize(
        &self,
        text: &str,
        voice: Option<&str>,
        speed: Option<f32>,
    ) -> Result<TtsResponse, AiProviderError> {
        if self.kind == "openai" && super::openai_realtime::is_model(self.settings["tts_model"].as_str().unwrap_or_default()) {
            return self.realtime.synthesize(&self.settings,text,voice,speed).await;
        }
        let model = provider_catalog::model(&self.settings, "tts")
            .ok_or_else(|| AiProviderError::NotSupported("未配置 TTS 模型".into()))?;
        let voice = voice
            .filter(|v| !v.is_empty())
            .or_else(|| self.settings["tts_voice"].as_str())
            .unwrap_or("alloy");
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
            let speech_voice=if self.kind=="openai"&&voice.starts_with("voice_"){json!({"id":voice})}else{json!(voice)};
            let response=self.checked(self.request("/audio/speech").json(&json!({"model":model,"input":text,"voice":speech_voice,"speed":speed.unwrap_or(1.0).clamp(0.25,4.0),"response_format":"mp3"}))).await?;
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
        if self.kind=="openai" {super::voices::openai_voice_ids(self.settings["tts_model"].as_str().unwrap_or_default()).to_vec()} else {vec![]}
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
    #[tokio::test]
    async fn empty_recognition_is_silence_but_missing_text_is_malformed() {
        for (body, valid) in [(r#"{"text":""}"#,true),(r#"{"text":"   "}"#,true),(r#"{}"#,false)] {
            let (client,request)=mock("openai",body.as_bytes(),"application/json");
            let result=client.transcribe(b"RIFFtest-audio".to_vec(),Some("auto")).await;
            if valid {assert!(result.unwrap().text.trim().is_empty());}else{assert!(result.is_err());}
            request.join().unwrap();
        }
    }
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
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains("x-api-key: mock-private-key")
                );
                assert_eq!(body["system"], "test prompt");
                assert_eq!(body["messages"].as_array().unwrap().len(), 1);
            } else {
                assert!(request.contains("/chat/completions "));
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains("authorization: bearer mock-private-key")
                );
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
    #[ignore="requires explicitly supplied saved OpenAI settings and a fixed test WAV"]
    async fn live_automatic_asr_probe() {
        let settings:Value=serde_json::from_str(&std::env::var("COLANG_OPENAI_ASR_TEST_SETTINGS").expect("supply test settings privately")).unwrap();
        let audio=tokio::fs::read(std::env::var("COLANG_OPENAI_ASR_TEST_AUDIO").unwrap()).await.unwrap();
        let model=settings["asr_model"].as_str().unwrap_or_default().to_owned();
        let response=CompatibleClient::new("openai",settings).transcribe(audio,Some("auto")).await.expect("automatic ASR failed");
        assert!(!response.text.trim().is_empty());
        println!("{}",json!({"model":model,"automaticLanguage":true,"recognized":true,"textLength":response.text.len()}));
    }
    #[tokio::test]
    async fn automatic_asr_language_is_omitted_in_official_http_requests() {
        for (kind,model,hint) in [("openai","gpt-transcribe","auto"),("openai","gpt-transcribe"," Auto "),("openai","whisper-1","auto"),("groq","whisper-large-v3-turbo","auto")] {
            let(mut client,thread)=mock(kind,br#"{"text":"Hello"}"#,"application/json");
            client.settings["asr_model"]=json!(model);
            assert_eq!(client.transcribe(b"RIFFtest-audio".to_vec(),Some(hint)).await.unwrap().text,"Hello");
            let request=thread.join().unwrap();
            assert!(!request.contains("name=\"languages[]\""));
            assert!(!request.contains("name=\"language\""));
            assert!(request.contains(model));
        }
    }
    #[tokio::test]
    async fn provider_errors_keep_identifiers_but_never_echo_private_input() {
        use std::io::{Read,Write};
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let port=listener.local_addr().unwrap().port();
        let server=std::thread::spawn(move||{
            let(mut socket,_)=listener.accept().unwrap();socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut request=Vec::new();let mut buf=[0;4096];
            loop {let count=socket.read(&mut buf).unwrap();assert!(count>0);request.extend_from_slice(&buf[..count]);if let Some(pos)=request.windows(4).position(|p|p==b"\r\n\r\n"){let head=String::from_utf8_lossy(&request[..pos]);let len=head.lines().find_map(|l|l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|s|s.trim().parse::<usize>().ok())).unwrap_or(0);if request.len()>=pos+4+len{break;}}}
            let body=json!({"error":{"code":"invalid_value","param":"languages[0]","message":"mock-private-key PRIVATE_AUDIO_TRANSCRIPT"}}).to_string();
            write!(socket,"HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nx-request-id: req_safe_123\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        });
        let mut client=CompatibleClient::new("openai",json!({"api_key":"mock-private-key","asr_model":"gpt-transcribe"}));client.base=format!("http://127.0.0.1:{port}");
        let error=client.transcribe(b"RIFFtest-audio".to_vec(),None).await.unwrap_err().to_string();
        assert!(error.contains("HTTP 400")&&error.contains("code=invalid_value")&&error.contains("param=languages[0]")&&error.contains("request_id=req_safe_123"),"{error}");
        assert!(!error.contains("mock-private-key")&&!error.contains("PRIVATE_AUDIO_TRANSCRIPT"));server.join().unwrap();
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
        let (mut client, thread) = mock("openai", b"ID3mock-mp3", "audio/mpeg");
        client.settings["tts_model"] = json!("gpt-4o-mini-tts");
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
    fn current_glm_series_keeps_required_thinking_with_bounded_reply_budget() {
        for model in ["glm-5.3-flash", "glm-5.3", "glm-5.2"] {
            let client=CompatibleClient::new("zhipu",json!({"api_key":"test","chat_model":model}));
            let (path,body)=client.chat_body(vec![],Some(1600),true);
            assert_eq!(path,"/chat/completions");
            assert_eq!(body["thinking"]["type"],"enabled");
            assert_eq!(body["reasoning_effort"],"low");
            assert_eq!(body["max_tokens"],8192);
        }
    }
    #[test]
    fn current_openai_families_use_supported_reasoning_and_legacy_requests_stay_unchanged() {
        for (model, effort) in [("gpt-6-luna", "none"), ("gpt-6.1-terra", "none"), ("gpt-6.1-sol", "low"), ("gpt-6-astra", "low")] {
            let client = CompatibleClient::new("openai", json!({"api_key":"test", "chat_model":model}));
            let (_, body)=client.chat_body(vec![], Some(1600), true);
            assert_eq!(body["reasoning_effort"],effort);
            assert_eq!(body["max_completion_tokens"],if effort=="none"{1600}else{8192});
        }
        let client=CompatibleClient::new("openai",json!({"api_key":"test","chat_model":"gpt-4.1-mini"}));
        assert!(client.chat_body(vec![],Some(1600),true).1.get("reasoning_effort").is_none());
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
    #[tokio::test]
    async fn minimax_pcm_stream_preserves_increments_and_uses_device_voice() {
        let event = |audio: &str, status: i32| {
            format!(
                "data: {}\r\n\r\n",
                json!({"data":{"audio":audio,"status":status},"base_resp":{"status_code":0}})
            )
        };
        for complete in [true, false] {
            let mut body = event("01000200", 1);
            body += &event("0300040005000600", 1);
            if complete {
                body += &event("", 2);
            }
            let (client, thread) = mock("minimax", body.as_bytes(), "text/event-stream");
            let (tx, mut rx) = tokio::sync::mpsc::channel(8);
            let result = client
                .synthesize_pcm_stream("Hello", Some("English_Trustworthy_Man"), Some(1.0), tx)
                .await;
            let mut pcm = Vec::new();
            while let Some(chunk) = rx.recv().await {
                pcm.extend(chunk);
            }
            assert_eq!(pcm, hex::decode("010002000300040005000600").unwrap());
            assert_eq!(result.is_ok(), complete);
            let request = thread.join().unwrap();
            assert!(request.contains("English_Trustworthy_Man"));
            assert!(request.contains("exclude_aggregated_audio"));
            assert!(request.contains("16000"));
            assert!(request.contains("\"format\":\"pcm\""));
        }
        let mut decoder = MiniMaxPcm::default();
        assert!(
            decoder
                .push(&json!({"data":{"audio":"01","status":1}}))
                .is_err()
        );
        assert!(
            MiniMaxPcm::default()
                .push(&json!({"base_resp":{"status_code":1004}}))
                .is_err()
        );
        let (client, thread) = mock(
            "minimax",
            br#"{"base_resp":{"status_code":0},"data":{"audio":"494433"}}"#,
            "application/json",
        );
        client
            .synthesize("Hello", Some("English_Graceful_Lady"), None)
            .await
            .unwrap();
        assert!(thread.join().unwrap().contains("English_Graceful_Lady"));
    }
}
