//! Study Time Tracking Service
//!
//! Uses in-memory cache to reduce database write frequency.
//! Periodically flushes accumulated study time to the database.

use std::collections::HashMap;
use std::sync::LazyLock;

use chrono::{DateTime, NaiveDate, Utc};
use tokio::sync::RwLock;

use crate::models::learn::{PageType, StudyTimeIncrement, UserActivityState};

/// Global study time tracker instance
pub static STUDY_TIME_TRACKER: LazyLock<StudyTimeTracker> = LazyLock::new(StudyTimeTracker::new);

/// Study time tracking service
///
/// Uses in-memory cache to reduce database write frequency.
pub struct StudyTimeTracker {
    /// User last activity state: user_id -> UserActivityState
    user_states: RwLock<HashMap<i64, UserActivityState>>,
    /// Pending increments to be persisted: (user_id, date) -> StudyTimeIncrement
    pending_increments: RwLock<HashMap<(i64, NaiveDate), StudyTimeIncrement>>,
    /// Maximum gap in seconds before considering a new session
    max_gap_seconds: i64,
    /// Seconds to add per heartbeat
    heartbeat_interval_seconds: i32,
}

impl StudyTimeTracker {
    /// Create a new tracker with default settings
    pub fn new() -> Self {
        Self {
            user_states: RwLock::new(HashMap::new()),
            pending_increments: RwLock::new(HashMap::new()),
            max_gap_seconds: 120,           // 2 minutes
            heartbeat_interval_seconds: 20, // heartbeat interval
        }
    }

    /// Process a heartbeat from the frontend
    ///
    /// Returns true if study time was recorded (i.e., conditions were met)
    pub async fn process_heartbeat(&self, user_id: i64, page_type: PageType) -> bool {
        let now = Utc::now();
        let today = now.date_naive();

        let mut states = self.user_states.write().await;
        let should_record = if let Some(prev_state) = states.get(&user_id) {
            // Check conditions: time gap < 2 minutes AND same page type
            let gap = now.signed_duration_since(prev_state.last_activity_time);
            gap.num_seconds() < self.max_gap_seconds && prev_state.last_page_type == page_type
        } else {
            false
        };

        // Update user state
        states.insert(
            user_id,
            UserActivityState {
                last_activity_time: now,
                last_page_type: page_type,
            },
        );

        if should_record {
            // Accumulate study time
            let mut increments = self.pending_increments.write().await;
            let key = (user_id, today);
            let increment = increments.entry(key).or_default();
            increment.add_seconds(page_type, self.heartbeat_interval_seconds);
        }

        should_record
    }

    /// Record a valid activity (e.g., sending a message, submitting an answer)
    ///
    /// This updates the last_activity_time for more accurate tracking.
    pub async fn record_valid_activity(&self, user_id: i64, page_type: PageType) {
        let now = Utc::now();
        let mut states = self.user_states.write().await;
        states.insert(
            user_id,
            UserActivityState {
                last_activity_time: now,
                last_page_type: page_type,
            },
        );
    }

    /// Take pending increments for persistence
    ///
    /// Returns all pending increments and clears the internal buffer.
    pub async fn take_pending_increments(&self) -> HashMap<(i64, NaiveDate), StudyTimeIncrement> {
        let mut increments = self.pending_increments.write().await;
        std::mem::take(&mut *increments)
    }

    /// Clean up stale user states (inactive for more than 10 minutes)
    pub async fn cleanup_stale_states(&self) {
        let threshold = Utc::now() - chrono::Duration::minutes(10);
        let mut states = self.user_states.write().await;
        states.retain(|_, state| state.last_activity_time > threshold);
    }

    /// Get today's study time summary for a user (from cache)
    pub async fn get_today_summary(&self, user_id: i64) -> Option<StudyTimeIncrement> {
        let today = Utc::now().date_naive();
        let increments = self.pending_increments.read().await;
        increments.get(&(user_id, today)).cloned()
    }

    /// Get user's current activity state
    pub async fn get_user_state(&self, user_id: i64) -> Option<UserActivityState> {
        let states = self.user_states.read().await;
        states.get(&user_id).cloned()
    }
}

impl Default for StudyTimeTracker {
    fn default() -> Self {
        Self::new()
    }
}
