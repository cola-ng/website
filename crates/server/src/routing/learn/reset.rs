use diesel::prelude::*;
use salvo::prelude::*;
use serde::Serialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::{AppResult, DepotExt};

#[derive(Serialize)]
pub struct ResetResponse {
    deleted_count: usize,
    table: String,
}

#[derive(Serialize)]
pub struct ResetAllResponse {
    tables_reset: Vec<ResetResponse>,
    total_deleted: usize,
}

#[handler]
pub async fn reset_issue_words(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_issue_words::table.filter(learn_issue_words::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset issue words"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_issue_words".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_chats(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_chats::table.filter(learn_chats::user_id.eq(user_id))).execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset chats"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_chats".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_write_practices(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(
            learn_write_practices::table.filter(learn_write_practices::user_id.eq(user_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset write practices"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_write_practices".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_read_practices(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(
            learn_read_practices::table.filter(learn_read_practices::user_id.eq(user_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset read practices"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_read_practices".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_vocabulary(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_vocabularies::table.filter(learn_vocabularies::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset vocabulary"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_vocabularies".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_daily_stats(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_daily_stats::table.filter(learn_daily_stats::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset daily stats"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_daily_stats".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_achievements(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_achievements::table.filter(learn_achievements::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset achievements"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_achievements".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_suggestions(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_suggestions::table.filter(learn_suggestions::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset suggestions"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_suggestions".to_string(),
    }));
    Ok(())
}