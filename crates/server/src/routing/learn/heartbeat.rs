//! Heartbeat-based study time tracking endpoints

use chrono::Utc;
use horme::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::learn_study_time_daily;
use crate::db::with_conn;
use crate::models::learn::{PageType, StudyTimeDaily};
use crate::services::study_time::STUDY_TIME_TRACKER;
use crate::{AppResult, DepotExt};

/// Heartbeat request from frontend
#[derive(Deserialize, ToSchema)]
pub struct HeartbeatRequest {
    /// Current page type (chat, review, stage, read, dictionary, translate)
    pub page_type: String,
}

/// Heartbeat response
#[derive(Serialize, ToSchema)]
pub struct HeartbeatResponse {
    /// Whether study time was recorded
    pub recorded: bool,
    /// Today's total study seconds (from cache)
    pub today_total_seconds: Option<i32>,
}

/// Process heartbeat from frontend
///
/// POST /api/learn/heartbeat
#[handler]
pub async fn heartbeat(req: &mut Request, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: HeartbeatRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let page_type = PageType::from_str(&input.page_type)
        .ok_or_else(|| StatusError::bad_request().brief("invalid page_type"))?;

    // Process heartbeat
    let recorded = STUDY_TIME_TRACKER
        .process_heartbeat(user_id, page_type)
        .await;

    // Get today's summary from cache
    let today_total_seconds = STUDY_TIME_TRACKER
        .get_today_summary(user_id)
        .await
        .map(|s| s.total_seconds());

    res.render(Json(HeartbeatResponse {
        recorded,
        today_total_seconds,
    }));
    Ok(())
}

/// Study time stats response
#[derive(Serialize, ToSchema)]
pub struct StudyTimeStatsResponse {
    /// Daily stats from database
    pub daily_stats: Vec<StudyTimeDaily>,
    /// Total study seconds in the period
    pub total_seconds: i64,
    /// Today's study seconds (database + cache)
    pub today_seconds: i32,
}

/// Get study time statistics
///
/// GET /api/learn/study-time?days=7
#[handler]
pub async fn get_study_time_stats(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let days = req.query::<i64>("days").unwrap_or(7).clamp(1, 365);

    // Get historical stats from database
    let daily_stats: Vec<StudyTimeDaily> = with_conn!(move |conn| {
        learn_study_time_daily::table
            .filter(learn_study_time_daily::user_id.eq(user_id))
            .order(learn_study_time_daily::stat_date.desc())
            .limit(days)
            .load::<StudyTimeDaily>(conn)
            .await
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch study time stats"))?;

    // Calculate total seconds
    let total_seconds: i64 = daily_stats.iter().map(|s| s.total_seconds as i64).sum();

    // Get today's cached data
    let today_cached = STUDY_TIME_TRACKER.get_today_summary(user_id).await;
    let today = Utc::now().date_naive();

    // Merge today's data (database + cache)
    let today_db_seconds = daily_stats
        .iter()
        .find(|s| s.stat_date == today)
        .map(|s| s.total_seconds)
        .unwrap_or(0);
    let today_cached_seconds = today_cached.map(|s| s.total_seconds()).unwrap_or(0);
    let today_seconds = today_db_seconds + today_cached_seconds;

    res.render(Json(StudyTimeStatsResponse {
        daily_stats,
        total_seconds,
        today_seconds,
    }));
    Ok(())
}
