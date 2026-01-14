use salvo::oapi::extract::*;
use salvo::prelude::*;

use crate::models::track::*;
use crate::models::*;
use crate::db::schema::*;
use crate::{DepotExt, PagedResult, db};

#[endpoint(tags("account"))]
pub fn list_actions(req: &mut Request, realm_id: QueryParam<i64, true>, depot: &mut Depot) -> PagedResult<Action> {
    let cuser = depot.current_user()?;
    let realm_id = realm_id.into_inner();
    let conn = &mut db::connect()?;
    let data = query_pagation_data!(
        req,
        res,
        Action,
        track_actions::table
            .filter(track_actions::realm_id.eq(realm_id))
            .filter(track_actions::user_id.eq(cuser.id)),
        "created_at desc",
        ACTION_FILTER_FIELDS.clone(),
        ACTION_JOINED_OPTIONS.clone(),
        ID_NAME_SEARCH_TMPL,
        conn
    );
    Ok(Json(data))
}
