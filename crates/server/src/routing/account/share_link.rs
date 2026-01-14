use diesel::prelude::*;
use salvo::prelude::*;

use crate::db;
use crate::models::*;
use crate::schema::*;
use crate::{DepotExt, PagedResult};

#[endpoint(tags("account"))]
pub async fn list(req: &mut Request, depot: &mut Depot) -> PagedResult<ShareLink> {
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let query = share_links::table.filter(share_links::owner_id.eq(cuser.id));
    let data = query_pagation_data!(
        req,
        res,
        ShareLink,
        query,
        "updated_at desc",
        SHARE_LINK_FILTER_FIELDS.clone(),
        SHARE_LINK_JOINED_OPTIONS.clone(),
        ID_NAME_SEARCH_TMPL,
        conn
    );
    Ok(Json(data))
}
