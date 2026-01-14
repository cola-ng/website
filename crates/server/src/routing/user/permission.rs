use diesel::prelude::*;
use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::db;
use crate::models::*;
use crate::permission::Accessible;
use crate::db::schema::*;
use crate::{DepotExt, JsonResult};

#[derive(Serialize, Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct HasPartialPermissionsItem {
    realm_id: i64,
    entity: String,
    actions: Vec<String>,
}
#[endpoint(tags("user"))]
pub async fn has_partial_permissions(
    user_id: PathParam<i64>,
    pdata: JsonBody<Vec<HasPartialPermissionsItem>>,
    depot: &mut Depot,
) -> JsonResult<Vec<HasPartialPermissionsItem>> {
    let mut pdata = pdata.into_inner();
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let user = users::table
        .filter(users::id.eq(user_id.into_inner()))
        .filter(users::is_disabled.eq(false))
        .first::<User>(conn)?
        .assign_to(cuser, "edit", conn)?;

    let realm_ids = pdata
        .iter()
        .map(|item| item.realm_id)
        .collect::<HashSet<i64>>()
        .into_iter()
        .collect::<Vec<i64>>();
    let realms = realms::table
        .filter(realms::id.eq_any(&realm_ids))
        .get_results::<Realm>(conn)?
        .into_iter()
        .map(|realm| (realm.id, realm))
        .collect::<HashMap<i64, Realm>>();

    for item in &mut pdata {
        if let Some(realm) = realms.get(&item.realm_id) {
            item.actions = crate::permission::has_partial_permissions(&user, realm, &item.entity, &item.actions, conn)?
        } else {
            item.actions = vec![];
        }
    }
    Ok(Json(pdata))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct HasRecordPermissionsReqItem {
    entity: String,
    actions: Vec<String>,
    record_ids: Vec<i64>,
}
#[derive(Serialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct HasRecordPermissionsOkItem {
    entity: String,
    action: String,
    record_ids: Vec<i64>,
}
#[endpoint(tags("user"))]
pub async fn has_record_permissions(
    user_id: PathParam<i64>,
    pdata: JsonBody<Vec<HasRecordPermissionsReqItem>>,
    depot: &mut Depot,
) -> JsonResult<Vec<HasRecordPermissionsOkItem>> {
    let pdata = pdata.into_inner();
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let user = users::table
        .filter(users::id.eq(user_id.into_inner()))
        .filter(users::is_disabled.eq(false))
        .first::<User>(conn)?
        .assign_to(cuser, "edit", conn)?;

    let mut responsed_data = vec![];
    for item in pdata {
        let permissions = crate::permission::has_record_permissions(&user, &item.entity, &item.actions, &item.record_ids, conn)?;
        for (action, record_ids) in permissions {
            responsed_data.push(HasRecordPermissionsOkItem {
                entity: item.entity.to_owned(),
                action: action.to_owned(),
                record_ids,
            });
        }
    }

    Ok(Json(responsed_data))
}
