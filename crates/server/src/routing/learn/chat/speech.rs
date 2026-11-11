//! User-selected TTS, ordered sentences and one segment of lookahead.
use super::*;
use crate::services::ai_provider::TtsService;
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::mpsc;

const MAX_SEGMENT: usize = 2 * 1024 * 1024;
const MAX_AUDIO: usize = 10 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Phrases {
    emitted: String,
    count: usize,
    failed: bool,
}
impl Phrases {
    pub fn take(&mut self, text: &str, done: bool) -> Result<Vec<String>, String> {
        let mut rest = text
            .strip_prefix(&self.emitted)
            .ok_or("回复文字已变更，已停止语音合成")?;
        let mut phrases = Vec::new();
        while !rest.is_empty() {
            let mut end = None;
            let mut last_space = None;
            for (count, (offset, ch)) in rest.char_indices().enumerate() {
                if ch.is_whitespace() {
                    last_space = Some(offset + ch.len_utf8());
                }
                let next = offset + ch.len_utf8();
                if ".!?。！？".contains(ch)
                    && (next == rest.len() || rest[next..].starts_with(char::is_whitespace))
                {
                    end = Some(next);
                    break;
                }
                if count >= 219 {
                    end = Some(last_space.unwrap_or(next));
                    break;
                }
            }
            let Some(end) = end.or_else(|| done.then_some(rest.len())) else {
                break;
            };
            let phrase = &rest[..end];
            self.emitted.push_str(phrase);
            if !phrase.trim().is_empty() {
                self.count += 1;
                if self.count > 64 {
                    return Err("回复语音超过分段限制".into());
                }
                phrases.push(phrase.to_owned());
            }
            rest = &rest[end..];
        }
        Ok(phrases)
    }
}

pub(super) async fn feed(
    phrases: &mut Phrases,
    text: &str,
    done: bool,
    sender: &Option<mpsc::Sender<String>>,
    job: &cancellation::Job,
    events: &mpsc::Sender<Value>,
) {
    if phrases.failed {
        return;
    }
    if let Some(sender) = sender {
        match phrases.take(text, done) {
            Ok(parts) => {
                for part in parts {
                    tokio::select! { biased; _ = job.wait() => return, _ = sender.send(part) => {} }
                }
            }
            Err(message) => {
                phrases.failed = true;
                let _ = frame(events, job, "voice_error", json!({"message":message})).await;
            }
        }
    }
}

async fn frame(
    events: &mpsc::Sender<Value>,
    job: &cancellation::Job,
    name: &str,
    data: Value,
) -> Result<(), String> {
    tokio::select! {
        biased;
        _ = job.wait() => Err("cancelled".into()),
        result = tokio::time::timeout(Duration::from_secs(5),events.send(json!({"event":name,"data":data}))) =>
            result.map_err(|_| "语音接收过慢，已停止合成")?.map_err(|_| "语音连接已关闭".into()),
    }
}

pub(super) async fn run(
    tts: Arc<dyn TtsService>,
    sentences: mpsc::Receiver<String>,
    events: &mpsc::Sender<Value>,
    job: &cancellation::Job,
) -> usize {
    let result: Result<usize,String> = async {
        let input = futures_util::stream::unfold(sentences, |mut receiver| async move {
            receiver.recv().await.map(|text| (text, receiver))
        });
        let results = input.map(|text| {
            let tts = tts.clone();
            async move {
                tokio::time::timeout(Duration::from_secs(45),tts.synthesize(&text,None,None)).await
                    .map_err(|_| "用户 TTS 服务超时，请稍后重试".to_string())?
                    .map_err(|_| "用户 TTS 服务合成失败，请检查语音配置或稍后重试".to_string())
            }
        }).buffered(2);
        tokio::pin!(results);
        let mut sequence = 0;
        let mut total = 0;
        loop {
            let audio = tokio::select! { biased; _ = job.wait() => return Err("cancelled".into()), result = results.next() => result };
            let Some(audio) = audio else { return Ok(sequence); };
            let audio = audio?;
            total += audio.audio_data.len();
            if audio.audio_data.is_empty() || audio.audio_data.len() > MAX_SEGMENT || total > MAX_AUDIO {
                return Err("用户 TTS 服务返回的音频为空或超过大小限制".into());
            }
            frame(events,job,"audio_start",json!({"sequence":sequence,"bytes":audio.audio_data.len(),"format":audio.format})).await?;
            for (index,chunk) in audio.audio_data.chunks(24*1024).enumerate() {
                frame(events,job,"audio_chunk",json!({"sequence":sequence,"offset":index*24*1024,"audio_base64":BASE64.encode(chunk)})).await?;
            }
            frame(events,job,"audio_end",json!({"sequence":sequence})).await?;
            sequence += 1;
        }
    }.await;
    match result {
        Ok(count) => count,
        Err(message) => {
            if !job.cancelled() {
                let _ = frame(events, job, "voice_error", json!({"message":message})).await;
            }
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ai_provider::TtsResponse;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Active(Arc<AtomicUsize>);
    impl Drop for Active {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }
    struct Fake {
        active: Arc<AtomicUsize>,
        mode: u8,
    }
    #[async_trait]
    impl TtsService for Fake {
        async fn synthesize(
            &self,
            text: &str,
            voice: Option<&str>,
            speed: Option<f32>,
        ) -> Result<TtsResponse, AiProviderError> {
            assert!(voice.is_none() && speed.is_none()); // The service retains user configuration.
            self.active.fetch_add(1, Ordering::SeqCst);
            let _active = Active(self.active.clone());
            if self.mode == 1 {
                futures_util::future::pending::<()>().await;
            }
            if self.mode == 2 {
                return Err(AiProviderError::Request("provider denied".into()));
            }
            tokio::time::sleep(Duration::from_millis(if text == "slow" { 80 } else { 5 })).await;
            Ok(TtsResponse {
                audio_data: vec![if text == "slow" { 1 } else { 2 }; 100],
                format: "wav".into(),
                duration_ms: None,
            })
        }
    }
    #[tokio::test]
    async fn provider_speech_order_failure_and_cancellation() {
        let active = Arc::new(AtomicUsize::new(0));
        let (sentences, receive) = mpsc::channel(2);
        sentences.send("slow".into()).await.unwrap();
        sentences.send("fast".into()).await.unwrap();
        drop(sentences);
        let (events, mut frames) = mpsc::channel(16);
        let job = cancellation::Job::new();
        assert_eq!(
            run(
                Arc::new(Fake {
                    active: active.clone(),
                    mode: 0
                }),
                receive,
                &events,
                &job
            )
            .await,
            2
        );
        let mut bytes = Vec::new();
        while let Ok(frame) = frames.try_recv() {
            if frame["event"] == "audio_chunk" {
                bytes.push(
                    BASE64
                        .decode(frame["data"]["audio_base64"].as_str().unwrap())
                        .unwrap()[0],
                );
            }
        }
        assert_eq!(bytes, vec![1, 2]);
        assert_eq!(active.load(Ordering::SeqCst), 0);

        let (sentences, receive) = mpsc::channel(2);
        sentences.send("fail".into()).await.unwrap();
        drop(sentences);
        assert_eq!(
            run(
                Arc::new(Fake {
                    active: active.clone(),
                    mode: 2
                }),
                receive,
                &events,
                &job
            )
            .await,
            0
        );
        assert_eq!(frames.recv().await.unwrap()["event"], "voice_error");

        let (sentences, receive) = mpsc::channel(2);
        sentences.send("one".into()).await.unwrap();
        sentences.send("two".into()).await.unwrap();
        drop(sentences);
        let worker_job = job.clone();
        let counter = active.clone();
        let task = tokio::spawn(async move {
            run(
                Arc::new(Fake {
                    active: counter,
                    mode: 1,
                }),
                receive,
                &events,
                &worker_job,
            )
            .await
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            while active.load(Ordering::SeqCst) < 2 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        job.request_cancel();
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            active.load(Ordering::SeqCst),
            0,
            "cancel must drop both synthesis futures"
        );
        assert!(frames.try_recv().is_err());
    }
    #[test]
    fn stable_sentences_unicode_tail_and_limits() {
        let mut phrases = Phrases::default();
        assert!(phrases.take("Hello", false).unwrap().is_empty());
        assert_eq!(
            phrases.take("Hello there! 你好。 Tail", false).unwrap(),
            vec!["Hello there!", " 你好。"]
        );
        assert_eq!(
            phrases.take("Hello there! 你好。 Tail", true).unwrap(),
            vec![" Tail"]
        );
        assert!(phrases.take("Rewritten", true).is_err());
        assert!(
            Phrases::default()
                .take(&"中".repeat(500), true)
                .unwrap()
                .iter()
                .all(|s| s.chars().count() <= 220)
        );
    }
}
