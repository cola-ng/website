use diesel::prelude::*;
use salvo::prelude::*;
use serde::Serialize;

use crate::AppResult;
use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::asset::*;

#[derive(Serialize, ToSchema)]
pub struct PaginatedSubjects {
    pub items: Vec<ReadSubject>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Serialize, ToSchema)]
pub struct PaginatedSentences {
    pub items: Vec<ReadSentence>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[handler]
pub async fn list_read_subjects(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let difficulty = req.query::<i16>("difficulty");
    let subject_type = req.query::<String>("type");
    let page = req.query::<i64>("page").unwrap_or(1).max(1);
    let per_page = req.query::<i64>("per_page").unwrap_or(20).clamp(1, 100);

    let offset = (page - 1) * per_page;

    let (subjects, total): (Vec<ReadSubject>, i64) = with_conn(move |conn| {
        let mut query = asset_read_subjects::table.into_boxed();
        let mut count_query = asset_read_subjects::table.into_boxed();

        if let Some(diff) = difficulty {
            query = query.filter(asset_read_subjects::difficulty.eq(diff));
            count_query = count_query.filter(asset_read_subjects::difficulty.eq(diff));
        }
        if let Some(ref st) = subject_type {
            query = query.filter(asset_read_subjects::subject_type.eq(st));
            count_query = count_query.filter(asset_read_subjects::subject_type.eq(st.clone()));
        }

        let total: i64 = count_query.count().get_result(conn)?;

        let items = query
            .order(asset_read_subjects::id.asc())
            .offset(offset)
            .limit(per_page)
            .load::<ReadSubject>(conn)?;

        Ok((items, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list subjects"))?;

    res.render(Json(PaginatedSubjects {
        items: subjects,
        total,
        page,
        per_page,
    }));
    Ok(())
}

#[handler]
pub async fn get_read_sentences(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let subject_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let page = req.query::<i64>("page").unwrap_or(1).max(1);
    let per_page = req.query::<i64>("per_page").unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let (sentences, total): (Vec<ReadSentence>, i64) = with_conn(move |conn| {
        let total: i64 = asset_read_sentences::table
            .filter(asset_read_sentences::subject_id.eq(subject_id))
            .count()
            .get_result(conn)?;

        let items = asset_read_sentences::table
            .filter(asset_read_sentences::subject_id.eq(subject_id))
            .order(asset_read_sentences::sentence_order.asc())
            .offset(offset)
            .limit(per_page)
            .load::<ReadSentence>(conn)?;

        Ok((items, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list sentences"))?;

    res.render(Json(PaginatedSentences {
        items: sentences,
        total,
        page,
        per_page,
    }));
    Ok(())
}

#[handler]
pub async fn list_read_sentences(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let subject_id = req.query::<i64>("subject");
    let page = req.query::<i64>("page").unwrap_or(1).max(1);
    let per_page = req.query::<i64>("per_page").unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let (sentences, total): (Vec<ReadSentence>, i64) = with_conn(move |conn| {
        let mut query = asset_read_sentences::table.into_boxed();
        let mut count_query = asset_read_sentences::table.into_boxed();

        if let Some(sid) = subject_id {
            query = query.filter(asset_read_sentences::subject_id.eq(sid));
            count_query = count_query.filter(asset_read_sentences::subject_id.eq(sid));
        }

        let total: i64 = count_query.count().get_result(conn)?;

        let items = query
            .order(asset_read_sentences::sentence_order.asc())
            .offset(offset)
            .limit(per_page)
            .load::<ReadSentence>(conn)?;

        Ok((items, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list sentences"))?;

    res.render(Json(PaginatedSentences {
        items: sentences,
        total,
        page,
        per_page,
    }));
    Ok(())
}
