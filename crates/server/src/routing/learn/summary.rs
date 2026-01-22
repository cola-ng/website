use chrono::{Datelike, Duration, Utc};
use diesel::prelude::*;
use salvo::prelude::*;
use serde::Serialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{AppResult, DepotExt};

#[derive(Serialize)]
pub struct WeeklyMinutes {
    /// Day of week: 0 = Monday, 6 = Sunday
    pub day: i32,
    /// Date in YYYY-MM-DD format
    pub date: String,
    /// Minutes studied on this day
    pub minutes: i32,
}

#[derive(Serialize)]
pub struct LearnSummary {
    /// Whether the user has any learning data
    pub has_data: bool,
    /// Total conversation minutes this week
    pub weekly_conversation_minutes: i32,
    /// Number of mastered vocabulary words (mastery_level >= 4)
    pub mastered_vocabulary_count: i64,
    /// Number of words due for review
    pub pending_review_count: i64,
    /// Minutes studied per day this week (Monday to Sunday)
    pub weekly_minutes: Vec<WeeklyMinutes>,
}

#[handler]
pub async fn get_learn_summary(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let summary = with_conn(move |conn| {
        let now = Utc::now();
        let today = now.date_naive();

        // Calculate the start of the current week (Monday)
        let days_since_monday = today.weekday().num_days_from_monday() as i64;
        let week_start = today - Duration::days(days_since_monday);
        let week_end = week_start + Duration::days(6);

        // 1. Get weekly conversation minutes from learn_daily_stats
        let weekly_stats: Vec<DailyStat> = learn_daily_stats::table
            .filter(learn_daily_stats::user_id.eq(user_id))
            .filter(learn_daily_stats::stat_date.ge(week_start))
            .filter(learn_daily_stats::stat_date.le(week_end))
            .load::<DailyStat>(conn)?;

        let weekly_conversation_minutes: i32 = weekly_stats
            .iter()
            .map(|s| s.minutes_studied.unwrap_or(0))
            .sum();

        // Build weekly minutes array (Monday to Sunday)
        let mut weekly_minutes = Vec::new();
        for i in 0..7 {
            let date = week_start + Duration::days(i);
            let minutes = weekly_stats
                .iter()
                .find(|s| s.stat_date == date)
                .map(|s| s.minutes_studied.unwrap_or(0))
                .unwrap_or(0);
            weekly_minutes.push(WeeklyMinutes {
                day: i as i32,
                date: date.format("%Y-%m-%d").to_string(),
                minutes,
            });
        }

        // 2. Count mastered vocabulary (mastery_level >= 4)
        let mastered_vocabulary_count: i64 = learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .filter(learn_vocabularies::mastery_level.ge(4))
            .count()
            .get_result(conn)?;

        // 3. Count pending review words
        let pending_review_count: i64 = learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .filter(
                learn_vocabularies::next_review_at
                    .is_null()
                    .or(learn_vocabularies::next_review_at.le(now)),
            )
            .count()
            .get_result(conn)?;

        // 4. Check if user has any learning data
        let has_chats: i64 = learn_chats::table
            .filter(learn_chats::user_id.eq(user_id))
            .count()
            .get_result(conn)?;

        let has_vocab: i64 = learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .count()
            .get_result(conn)?;

        let has_data = has_chats > 0 || has_vocab > 0 || weekly_conversation_minutes > 0;

        Ok::<_, diesel::result::Error>(LearnSummary {
            has_data,
            weekly_conversation_minutes,
            mastered_vocabulary_count,
            pending_review_count,
            weekly_minutes,
        })
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get learn summary"))?;

    res.render(Json(summary));
    Ok(())
}
