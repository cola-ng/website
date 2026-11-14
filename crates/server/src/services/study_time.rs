//! Count elapsed active time once per account; retain unacknowledged flush batches.
use crate::models::learn::{PageType, StudyTimeIncrement, UserActivityState};
use chrono::{DateTime, NaiveDate, Utc};
use std::{
    collections::HashMap,
    sync::LazyLock,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

pub static STUDY_TIME_TRACKER: LazyLock<StudyTimeTracker> = LazyLock::new(StudyTimeTracker::new);
pub type Increments = HashMap<(i64, NaiveDate), StudyTimeIncrement>;
#[derive(Clone)]
pub struct Batch {
    pub id: uuid::Uuid,
    pub increments: Increments,
}
struct Activity {
    state: UserActivityState,
    tick: Instant,
    remainder: Duration,
}
#[derive(Default)]
struct State {
    users: HashMap<i64, Activity>,
    pending: Increments,
    inflight: Option<Batch>,
}
#[derive(Default)]
pub struct StudyTimeTracker {
    state: Mutex<State>,
}
impl StudyTimeTracker {
    pub fn new() -> Self {
        Self::default()
    }
    pub async fn process_heartbeat(&self, user_id: i64, page: PageType) -> bool {
        self.process_at(user_id, page, Utc::now(), Instant::now())
            .await
    }
    async fn process_at(
        &self,
        user_id: i64,
        page: PageType,
        wall: DateTime<Utc>,
        tick: Instant,
    ) -> bool {
        let mut state = self.state.lock().await;
        let mut recorded = false;
        if let Some(previous) = state.users.get_mut(&user_id) {
            let elapsed = tick.saturating_duration_since(previous.tick);
            let valid = elapsed < Duration::from_secs(120) && previous.state.last_page_type == page;
            let elapsed = if valid {
                elapsed + previous.remainder
            } else {
                Duration::ZERO
            };
            let seconds = elapsed.as_secs() as i32;
            previous.tick = tick;
            previous.remainder = elapsed - Duration::from_secs(seconds as u64);
            previous.state = UserActivityState {
                last_activity_time: wall,
                last_page_type: page,
            };
            if seconds > 0 {
                // Attribute the interval ending at this heartbeat across UTC midnight.
                let midnight = wall.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
                let today_seconds =
                    (wall - midnight).num_seconds().max(0).min(seconds as i64) as i32;
                if today_seconds > 0 {
                    state
                        .pending
                        .entry((user_id, wall.date_naive()))
                        .or_default()
                        .add_seconds(page, today_seconds);
                }
                if today_seconds < seconds {
                    state
                        .pending
                        .entry((user_id, wall.date_naive().pred_opt().unwrap()))
                        .or_default()
                        .add_seconds(page, seconds - today_seconds);
                }
                recorded = true;
            }
        } else {
            state.users.insert(
                user_id,
                Activity {
                    state: UserActivityState {
                        last_activity_time: wall,
                        last_page_type: page,
                    },
                    tick,
                    remainder: Duration::ZERO,
                },
            );
        }
        recorded
    }
    pub async fn record_valid_activity(&self, user_id: i64, page: PageType) {
        self.process_heartbeat(user_id, page).await;
    }
    pub async fn begin_flush(&self) -> Option<Batch> {
        let mut state = self.state.lock().await;
        if state.inflight.is_none() && !state.pending.is_empty() {
            state.inflight = Some(Batch {
                id: uuid::Uuid::new_v4(),
                increments: std::mem::take(&mut state.pending),
            });
        }
        state.inflight.clone()
    }
    pub async fn acknowledge(&self, id: uuid::Uuid) {
        let mut state = self.state.lock().await;
        if state.inflight.as_ref().is_some_and(|batch| batch.id == id) {
            state.inflight = None;
        }
    }
    pub async fn cleanup_stale_states(&self) {
        let now = Instant::now();
        self.state.lock().await.users.retain(|_, activity| {
            now.saturating_duration_since(activity.tick) < Duration::from_secs(600)
        });
    }
    pub async fn get_today_summary(&self, user_id: i64) -> Option<StudyTimeIncrement> {
        let state = self.state.lock().await;
        let key = (user_id, Utc::now().date_naive());
        let mut value = state.pending.get(&key).cloned().unwrap_or_default();
        if let Some(increment) = state.inflight.as_ref().and_then(|b| b.increments.get(&key)) {
            value.merge(increment);
        }
        (!value.is_empty()).then_some(value)
    }
    pub async fn get_user_state(&self, user_id: i64) -> Option<UserActivityState> {
        self.state
            .lock()
            .await
            .users
            .get(&user_id)
            .map(|v| v.state.clone())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn rapid_duplicate_heartbeats_and_retry_do_not_invent_or_lose_time() {
        let tracker = StudyTimeTracker::new();
        let t = Instant::now();
        let wall = Utc::now();
        tracker.process_at(1, PageType::Chat, wall, t).await;
        for _ in 0..100 {
            assert!(!tracker.process_at(1, PageType::Chat, wall, t).await);
        }
        tracker
            .process_at(
                1,
                PageType::Chat,
                wall + chrono::Duration::seconds(20),
                t + Duration::from_secs(20),
            )
            .await;
        let batch = tracker.begin_flush().await.unwrap();
        assert_eq!(
            batch
                .increments
                .values()
                .map(|v| v.total_seconds())
                .sum::<i32>(),
            20
        );
        tracker
            .process_at(
                1,
                PageType::Chat,
                wall + chrono::Duration::seconds(30),
                t + Duration::from_secs(30),
            )
            .await;
        assert_eq!(tracker.begin_flush().await.unwrap().id, batch.id);
        tracker.acknowledge(batch.id).await;
        let next = tracker.begin_flush().await.unwrap();
        assert_ne!(next.id, batch.id);
        assert_eq!(
            next.increments
                .values()
                .map(|v| v.total_seconds())
                .sum::<i32>(),
            10
        );
    }
    #[tokio::test]
    async fn subsecond_intervals_accumulate_and_midnight_splits() {
        let tracker = StudyTimeTracker::new();
        let t = Instant::now();
        let wall = DateTime::parse_from_rfc3339("2026-11-13T23:59:59Z")
            .unwrap()
            .with_timezone(&Utc);
        for n in 0..=20 {
            tracker
                .process_at(
                    1,
                    PageType::Review,
                    wall + chrono::Duration::milliseconds(n * 100),
                    t + Duration::from_millis(n as u64 * 100),
                )
                .await;
        }
        let batch = tracker.begin_flush().await.unwrap();
        assert_eq!(batch.increments.len(), 2);
        assert_eq!(
            batch
                .increments
                .values()
                .map(|v| v.total_seconds())
                .sum::<i32>(),
            2
        );
    }
}
