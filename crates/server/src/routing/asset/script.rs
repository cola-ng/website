use std::path::PathBuf;

use diesel::prelude::*;
use salvo::prelude::*;

use crate::AppResult;
use crate::config::AppConfig;
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

/// Serve audio file for a script turn
///
/// Path: /asset/scripts/turns/{id}/audio
#[handler]
pub async fn serve_turn_audio(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let turn_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing turn id"))?;

    // Get the turn from database to find audio_path
    let turn: ScriptTurn = with_conn(move |conn| {
        asset_script_turns::table
            .find(turn_id)
            .first::<ScriptTurn>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("turn not found"))?;

    // Check if audio_path exists
    let audio_path = turn
        .audio_path
        .ok_or_else(|| StatusError::not_found().brief("no audio available for this turn"))?;

    // Build the full file path
    let file_path = PathBuf::from(&AppConfig::get().space_path).join(&audio_path);

    // Check if file exists
    if !file_path.exists() {
        tracing::warn!("Script turn audio file not found: {:?}", file_path);
        return Err(StatusError::not_found()
            .brief("audio file not found")
            .into());
    }

    res.send_file(file_path, req.headers()).await;

    Ok(())
}
