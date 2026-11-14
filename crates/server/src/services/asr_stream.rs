//! Provider-independent streaming ASR result and cancellation handle.
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsrPartial {
    pub text: String,
    pub utterance_complete: bool,
}

pub struct PcmAsrStream {
    input: Option<mpsc::Sender<Option<Vec<u8>>>>,
    task: Option<JoinHandle<Result<String, String>>>,
}
impl Drop for PcmAsrStream {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
impl PcmAsrStream {
    pub fn audio(&self, pcm: &[u8]) -> Result<(), String> {
        self.input
            .as_ref()
            .ok_or("ASR closed")?
            .try_send(Some(pcm.to_vec()))
            .map_err(|_| "ASR input backlog or closed".into())
    }
    pub async fn finish(mut self) -> Result<String, String> {
        self.input
            .take()
            .ok_or("ASR closed")?
            .send(None)
            .await
            .map_err(|_| "ASR input closed")?;
        // Keep the task in self while awaiting, so parent cancellation aborts it.
        let result = self
            .task
            .as_mut()
            .ok_or("ASR task missing")?
            .await
            .map_err(|_| "ASR task failed")?;
        self.task.take();
        result
    }
}
