use diesel::prelude::*;
use salvo::oapi::extract::*;
use salvo::prelude::*;

use crate::db;
use crate::models::*;
use crate::permission::Accessible;
use crate::db::schema::*;
use crate::{DepotExt, PagedResult};

#[endpoint(tags("user"))]
pub async fn list(user_id: PathParam<i64>, req: &mut Request, depot: &mut Depot) -> PagedResult<ShareLink> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let user = users::table
        .filter(users::id.eq(user_id.into_inner()))
        .filter(users::is_disabled.eq(false))
        .first::<User>(conn)?
        .assign_to(cuser, "view", conn)?;

    let query = share_links::table.filter(share_links::owner_id.eq(user.id));
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
