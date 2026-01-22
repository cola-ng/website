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
pub async fn reset_chat_turns(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_chat_turns::table.filter(learn_chat_turns::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset chat turns"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_chat_turns".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_chat_annotations(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(
            learn_chat_annotations::table.filter(learn_chat_annotations::user_id.eq(user_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset chat annotations"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_chat_annotations".to_string(),
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

#[handler]
pub async fn reset_all_learn_data(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let results = with_conn(move |conn| {
        let mut tables_reset = Vec::new();

        // Delete in order to respect foreign key constraints
        // First delete dependent tables, then parent tables

        // Chat annotations (depends on chat turns)
        let count = diesel::delete(
            learn_chat_annotations::table.filter(learn_chat_annotations::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_chat_annotations".to_string(),
        });

        // Chat turns
        let count = diesel::delete(
            learn_chat_turns::table.filter(learn_chat_turns::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_chat_turns".to_string(),
        });

        // Chats
        let count =
            diesel::delete(learn_chats::table.filter(learn_chats::user_id.eq(user_id)))
                .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_chats".to_string(),
        });

        // Write practices
        let count = diesel::delete(
            learn_write_practices::table.filter(learn_write_practices::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_write_practices".to_string(),
        });

        // Read practices
        let count = diesel::delete(
            learn_read_practices::table.filter(learn_read_practices::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_read_practices".to_string(),
        });

        // Issue words
        let count =
            diesel::delete(learn_issue_words::table.filter(learn_issue_words::user_id.eq(user_id)))
                .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_issue_words".to_string(),
        });

        // Vocabulary
        let count = diesel::delete(
            learn_vocabularies::table.filter(learn_vocabularies::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_vocabularies".to_string(),
        });

        // Daily stats
        let count =
            diesel::delete(learn_daily_stats::table.filter(learn_daily_stats::user_id.eq(user_id)))
                .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_daily_stats".to_string(),
        });

        // Achievements
        let count = diesel::delete(
            learn_achievements::table.filter(learn_achievements::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_achievements".to_string(),
        });

        // Suggestions
        let count =
            diesel::delete(learn_suggestions::table.filter(learn_suggestions::user_id.eq(user_id)))
                .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_suggestions".to_string(),
        });

        Ok::<_, diesel::result::Error>(tables_reset)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset all learn data"))?;

    let total_deleted: usize = results.iter().map(|r| r.deleted_count).sum();

    res.render(Json(ResetAllResponse {
        tables_reset: results,
        total_deleted,
    }));
    Ok(())
}
