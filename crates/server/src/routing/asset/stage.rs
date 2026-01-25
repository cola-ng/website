use diesel::prelude::*;
use salvo::prelude::*;

use crate::AppResult;
use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::asset::*;

#[handler]
pub async fn list_stages(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let difficulty = req.query::<i16>("difficulty");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let stages: Vec<Stage> = with_conn(move |conn| {
        let mut query = asset_stages::table
            .filter(asset_stages::is_active.eq(true))
            .order(asset_stages::display_order.asc())
            .limit(limit)
            .into_boxed();

        if let Some(diff) = difficulty {
            query = query.filter(asset_stages::difficulty.eq(diff));
        }

        query.load::<Stage>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list stages"))?;

    res.render(Json(stages));
    Ok(())
}

#[handler]
pub async fn get_stage(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let stage_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let stage: Stage = with_conn(move |conn| {
        asset_stages::table
            .filter(asset_stages::id.eq(stage_id))
            .first::<Stage>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("stage not found"))?;

    res.render(Json(stage));
    Ok(())
}

#[handler]
pub async fn get_scripts_by_stage(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let stage_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let scripts: Vec<Script> = with_conn(move |conn| {
        asset_scripts::table
            .filter(asset_scripts::stage_id.eq(stage_id))
            .load::<Script>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list scripts"))?;

    res.render(Json(scripts));
    Ok(())
}
