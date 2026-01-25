use diesel::prelude::*;
use salvo::prelude::*;

use crate::AppResult;
use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::asset::*;

#[handler]
pub async fn list_scripts(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let stage_id = req.query::<i64>("stage_id");
    let difficulty = req.query::<i16>("difficulty");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let scripts: Vec<Script> = with_conn(move |conn| {
        let mut query = asset_scripts::table.limit(limit).into_boxed();

        if let Some(sid) = stage_id {
            query = query.filter(asset_scripts::stage_id.eq(sid));
        }
        if let Some(diff) = difficulty {
            query = query.filter(asset_scripts::difficulty.eq(diff));
        }

        query.load::<Script>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list scripts"))?;

    res.render(Json(scripts));
    Ok(())
}

#[handler]
pub async fn get_script_turns(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let script_id: i64 = req
        .param::<i64>("script_id")
        .ok_or_else(|| StatusError::bad_request().brief("missing script_id"))?;

    let turns: Vec<ScriptTurn> = with_conn(move |conn| {
        asset_script_turns::table
            .filter(asset_script_turns::script_id.eq(script_id))
            .order(asset_script_turns::turn_number.asc())
            .load::<ScriptTurn>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list turns"))?;

    res.render(Json(turns));
    Ok(())
}
