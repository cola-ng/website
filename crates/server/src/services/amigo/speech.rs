//! Ordered PCM streams with one bounded segment of lookahead. Dropping the
//! coordinator aborts both current and prefetched provider requests.
use super::{agent::Agent, tutor::Backend};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot};
use tokio::task::JoinSet;

pub struct Segment {
    pub text: String,
    pub audio: mpsc::Receiver<Vec<u8>>,
    pub complete: oneshot::Receiver<Result<(), String>>,
    _slot: OwnedSemaphorePermit,
}
pub async fn synthesize(
    backend: Arc<dyn Backend>,
    agent: Agent,
    mut sentences: mpsc::Receiver<String>,
    segments: mpsc::Sender<Segment>,
) -> Result<(), String> {
    let slots = Arc::new(Semaphore::new(2));
    let mut workers = JoinSet::new();
    while let Some(text) = sentences.recv().await {
        if text.trim().is_empty() {
            continue;
        }
        let slot = slots
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| "speech cancelled")?;
        let (audio, receive) = mpsc::channel(4);
        let (done, complete) = oneshot::channel();
        let service = backend.clone();
        let voice = agent.clone();
        let phrase = text.clone();
        workers.spawn(async move {
            let result = service.synthesize_stream(&phrase, &voice, audio).await;
            let _ = done.send(result);
        });
        segments
            .send(Segment {
                text,
                audio: receive,
                complete,
                _slot: slot,
            })
            .await
            .map_err(|_| "speech cancelled")?;
        while let Some(result) = workers.try_join_next() {
            result.map_err(|_| "TTS worker failed")?;
        }
    }
    drop(segments);
    while let Some(result) = workers.join_next().await {
        result.map_err(|_| "TTS worker failed")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::model::{Analysis, Plan, Profile};
    use super::*;
    use async_trait::async_trait;
    use amigo::protocol::Source;
    #[derive(Default)]
    struct Mock {
        second: tokio::sync::Notify,
        release: tokio::sync::Notify,
        finished: Arc<tokio::sync::Notify>,
    }
    struct Guard(Arc<tokio::sync::Notify>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.notify_one();
        }
    }
    #[async_trait]
    impl Backend for Mock {
        async fn transcribe(&self, _: &[u8]) -> Result<String, String> {
            unreachable!()
        }
        async fn observe(&self, _: Source, _: &[u8]) -> Result<(String, Analysis), String> {
            unreachable!()
        }
        async fn reply(&self, _: &Profile, _: &Plan, _: &str) -> Result<Analysis, String> {
            unreachable!()
        }
        async fn synthesize(&self, _: &str, _: &Agent) -> Result<Vec<u8>, String> {
            unreachable!()
        }
        async fn synthesize_stream(
            &self,
            text: &str,
            _: &Agent,
            audio: mpsc::Sender<Vec<u8>>,
        ) -> Result<(), String> {
            if text == "one" {
                let _guard = Guard(self.finished.clone());
                audio.send(vec![1, 0]).await.map_err(|_| "closed")?;
                self.release.notified().await;
                audio.send(vec![3, 0]).await.map_err(|_| "closed")?;
            } else {
                self.second.notify_one();
                audio.send(vec![2, 0]).await.map_err(|_| "closed")?;
            }
            Ok(())
        }
    }
    #[tokio::test]
    async fn lookahead_starts_early_but_pcm_remains_in_sentence_order() {
        let backend = Arc::new(Mock::default());
        let (input, sentences) = mpsc::channel(2);
        input.send("one".into()).await.unwrap();
        input.send("two".into()).await.unwrap();
        drop(input);
        let (output, mut segments) = mpsc::channel(1);
        let worker = tokio::spawn(synthesize(
            backend.clone(),
            Agent::default(),
            sentences,
            output,
        ));
        {
            let mut first = segments.recv().await.unwrap();
            assert_eq!(first.text, "one");
            assert_eq!(first.audio.recv().await.unwrap(), [1, 0]);
            tokio::time::timeout(std::time::Duration::from_secs(1), backend.second.notified())
                .await
                .unwrap();
            backend.release.notify_one();
            assert_eq!(first.audio.recv().await.unwrap(), [3, 0]);
            assert!(first.audio.recv().await.is_none());
            first.complete.await.unwrap().unwrap();
        }
        let mut second = segments.recv().await.unwrap();
        assert_eq!(second.text, "two");
        assert_eq!(second.audio.recv().await.unwrap(), [2, 0]);
        second.complete.await.unwrap().unwrap();
        assert!(segments.recv().await.is_none());
        worker.await.unwrap().unwrap();
    }
    #[tokio::test]
    async fn dropping_the_coordinator_cancels_a_synthesizer_waiting_for_more_audio() {
        let backend = Arc::new(Mock::default());
        let (input, sentences) = mpsc::channel(1);
        input.send("one".into()).await.unwrap();
        let (output, mut segments) = mpsc::channel(1);
        let worker = tokio::spawn(synthesize(
            backend.clone(),
            Agent::default(),
            sentences,
            output,
        ));
        let mut first = segments.recv().await.unwrap();
        first.audio.recv().await.unwrap();
        worker.abort();
        assert!(worker.await.unwrap_err().is_cancelled());
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            backend.finished.notified(),
        )
        .await
        .unwrap();
    }
}
