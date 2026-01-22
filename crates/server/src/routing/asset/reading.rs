use diesel::prelude::*;
use salvo::prelude::*;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::asset::*;
use crate::AppResult;

#[handler]
pub async fn list_read_subjects(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let difficulty = req.query::<i16>("difficulty");
    let subject_type = req.query::<String>("type");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let subjects: Vec<ReadSubject> = with_conn(move |conn| {
        let mut query = asset_read_subjects::table.limit(limit).into_boxed();

        if let Some(diff) = difficulty {
            query = query.filter(asset_read_subjects::difficulty.eq(diff));
        }
        if let Some(st) = subject_type {
            query = query.filter(asset_read_subjects::subject_type.eq(st));
        }

        query.load::<ReadSubject>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list subjects"))?;

    res.render(Json(subjects));
    Ok(())
}

#[handler]
pub async fn get_read_sentences(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let subject_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let sentences: Vec<ReadSentence> = with_conn(move |conn| {
        asset_read_sentences::table
            .filter(asset_read_sentences::subject_id.eq(subject_id))
            .order(asset_read_sentences::sentence_order.asc())
            .load::<ReadSentence>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list sentences"))?;

    res.render(Json(sentences));
    Ok(())
}
