use chrono::NaiveDate;
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{AppResult, DepotExt};

#[derive(Deserialize, ToSchema)]
pub struct UpsertDailyStatRequest {
    stat_date: String,
    minutes_studied: Option<i32>,
    words_practiced: Option<i32>,
    sessions_completed: Option<i32>,
    errors_corrected: Option<i32>,
    new_words_learned: Option<i32>,
    review_words_count: Option<i32>,
}

#[handler]
pub async fn list_daily_stats(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let limit = req.query::<i64>("limit").unwrap_or(30).clamp(1, 365);

    let stats: Vec<DailyStat> = with_conn(move |conn| {
        learn_daily_stats::table
            .filter(learn_daily_stats::user_id.eq(user_id))
            .order(learn_daily_stats::stat_date.desc())
            .limit(limit)
            .load::<DailyStat>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list daily stats"))?;

    res.render(Json(stats));
    Ok(())
}

#[handler]
pub async fn upsert_daily_stat(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: UpsertDailyStatRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let date = NaiveDate::parse_from_str(&input.stat_date, "%Y-%m-%d")
        .map_err(|_| StatusError::bad_request().brief("invalid date format, use YYYY-MM-DD"))?;

    let new_stat = NewDailyStat {
        user_id,
        stat_date: date,
        minutes_studied: input.minutes_studied,
        words_practiced: input.words_practiced,
        sessions_completed: input.sessions_completed,
        errors_corrected: input.errors_corrected,
        new_words_learned: input.new_words_learned,
        review_words_count: input.review_words_count,
    };

    let stat: DailyStat = with_conn(move |conn| {
        diesel::insert_into(learn_daily_stats::table)
            .values(&new_stat)
            .on_conflict((learn_daily_stats::user_id, learn_daily_stats::stat_date))
            .do_update()
            .set(&UpdateDailyStat {
                minutes_studied: input.minutes_studied,
                words_practiced: input.words_practiced,
                sessions_completed: input.sessions_completed,
                errors_corrected: input.errors_corrected,
                new_words_learned: input.new_words_learned,
                review_words_count: input.review_words_count,
            })
            .get_result::<DailyStat>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to upsert daily stat"))?;

    res.render(Json(stat));
    Ok(())
}
