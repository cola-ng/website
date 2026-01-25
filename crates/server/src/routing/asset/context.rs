use diesel::prelude::*;
use salvo::prelude::*;

use crate::AppResult;
use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::asset::*;

#[handler]
pub async fn list_contexts(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let difficulty = req.query::<i16>("difficulty");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let contexts: Vec<Context> = with_conn(move |conn| {
        let mut query = asset_contexts::table
            .filter(asset_contexts::is_active.eq(true))
            .order(asset_contexts::display_order.asc())
            .limit(limit)
            .into_boxed();

        if let Some(diff) = difficulty {
            query = query.filter(asset_contexts::difficulty.eq(diff));
        }

        query.load::<Context>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list contexts"))?;

    res.render(Json(contexts));
    Ok(())
}

#[handler]
pub async fn get_context(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let context_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let context: Context = with_conn(move |conn| {
        asset_contexts::table
            .filter(asset_contexts::id.eq(context_id))
            .first::<Context>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("context not found"))?;

    res.render(Json(context));
    Ok(())
}
