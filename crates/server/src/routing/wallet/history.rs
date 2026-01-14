use salvo::prelude::*;

use crate::models::wallet::*;
use crate::models::*;
use crate::db::schema::*;
use crate::{DepotExt, PagedResult, db};

#[endpoint(tags("wallet"))]
pub async fn list(req: &mut Request, depot: &mut Depot) -> PagedResult<History> {
    let _cuser = depot.current_user()?.must_in_kernel()?;
    let conn = &mut db::conn()?;
    let data = query_pagation_data!(
        req,
        res,
        History,
        wallet_histories::table, //.permit(cuser, "view")?,
        "created_at desc",
        HISTORY_FILTER_FIELDS.clone(),
        HISTORY_JOINED_OPTIONS.clone(),
        ID_SEARCH_TMPL,
        conn
    );
    Ok(Json(data))
}
