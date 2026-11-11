//! Explicit, request-scoped cancellation. Disconnecting an SSE reader is not a cancel.
use super::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use tokio::sync::watch;

type Key = (i64, i64, String);
static JOBS: OnceLock<Mutex<HashMap<Key, Arc<Job>>>> = OnceLock::new();

pub(super) struct Job {
    cancel: watch::Sender<bool>,
    done: watch::Sender<bool>,
    created: Instant,
}
impl Job {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            cancel: watch::channel(false).0,
            done: watch::channel(false).0,
            created: Instant::now(),
        })
    }
    pub fn request_cancel(&self) {
        self.cancel.send_replace(true);
    }
    pub fn cancelled(&self) -> bool {
        *self.cancel.borrow()
    }
    pub async fn wait(&self) {
        let _ = self.cancel.subscribe().wait_for(|v| *v).await;
    }
    pub fn finish(&self) {
        self.done.send_replace(true);
    }
    async fn drained(&self) {
        let _ = self.done.subscribe().wait_for(|v| *v).await;
    }
}
pub(super) fn valid_id(id: &str) -> bool {
    (16..=96).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
fn job(user: i64, chat: i64, id: String, cancel: bool) -> Result<Arc<Job>, StatusError> {
    let mut jobs = JOBS.get_or_init(Default::default).lock().unwrap();
    // Keep completed IDs for longer than the client request deadline. This also
    // handles a cancel arriving before the corresponding POST reaches us.
    jobs.retain(|_, job| !*job.done.borrow() || job.created.elapsed() < Duration::from_secs(180));
    let key = (user, chat, id);
    if let Some(job) = jobs.get(&key) {
        if !cancel {
            return Err(StatusError::conflict().brief("此轮请求已处理或取消"));
        }
        job.request_cancel();
        return Ok(job.clone());
    }
    if jobs.keys().filter(|(owner, _, _)| *owner == user).count() >= 128 {
        return Err(StatusError::too_many_requests().brief("语音操作过于频繁，请稍候"));
    }
    let job = Job::new();
    if cancel {
        job.request_cancel();
        job.finish();
    }
    jobs.insert(key, job.clone());
    Ok(job)
}
pub(super) fn register(user: i64, chat: i64, id: String) -> Result<Arc<Job>, StatusError> {
    job(user, chat, id, false)
}

#[handler]
pub async fn cancel_turn(req: &mut Request, depot: &mut Depot) -> JsonResult<OkResponse> {
    let user = depot.user_id()?;
    let chat = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("对话编号无效"))?;
    streaming::check_owner(user, chat).await?;
    #[derive(Deserialize)]
    struct Input {
        request_id: String,
    }
    let input: Input = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("取消请求无效"))?;
    if !valid_id(&input.request_id) {
        return Err(StatusError::bad_request().brief("轮次编号无效").into());
    }
    let job = job(user, chat, input.request_id, true)?;
    tokio::time::timeout(Duration::from_secs(15), job.drained())
        .await
        .map_err(|_| StatusError::gateway_timeout().brief("正在停止旧回复，请稍后重试"))?;
    json_ok(OkResponse { ok: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn cancellation_is_scoped_and_handles_reordered_requests() {
        let id = crate::auth::random_code();
        let a = register(-1, 1, id.clone()).unwrap();
        let b = register(-1, 2, id.clone()).unwrap();
        let c = register(-2, 1, id.clone()).unwrap();
        let next = register(-1, 1, format!("{id}-next")).unwrap();
        job(-1, 1, id.clone(), true).unwrap();
        a.wait().await;
        assert!(!b.cancelled() && !c.cancelled() && !next.cancelled());
        a.finish();
        b.finish();
        c.finish();
        next.finish();
        job(-1, 1, id, true).unwrap().drained().await;
        let early = format!("{}-early", crate::auth::random_code());
        job(-1, 1, early.clone(), true).unwrap().drained().await;
        assert!(register(-1, 1, early).is_err());
    }
}
