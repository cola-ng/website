use cruet::Inflector;
use diesel::prelude::*;
use diesel::sql_types::*;
use salvo::oapi::extract::*;
use salvo::prelude::*;
use salvo::routing::FlowCtrl;
use serde::Deserialize;

use crate::models::*;
use crate::permission::Accessible;
use crate::schema::*;
use crate::{AppError, AppResult, DepotExt, data, db};

#[endpoint(tags("role"))]
pub fn list(role_id: PathParam<i64>, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let role = roles::table
        .find(role_id.into_inner())
        .first::<Role>(conn)?
        .assign_to(cuser, "permissions.view", conn)?;

    let permissions = permissions::table
        .filter(permissions::role_id.eq(role.id))
        .distinct()
        .get_results::<Permission>(conn)?;
    res.render(salvo::writing::Json(permissions));
    Ok(())
}

#[derive(Deserialize, ToSchema, Debug)]
struct UpdateInItem {
    entity: String,
    action: String,
    scope: String,
    filter_name: String,
    filter_int_value: Option<i64>,
    filter_text_value: Option<String>,
}
#[endpoint(tags("role"))]
pub async fn update(
    role_id: PathParam<i64>,
    pdata: JsonBody<Vec<UpdateInItem>>,
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) -> AppResult<()> {
    #[derive(QueryableByName, Serialize, Clone, Debug)]
    pub struct RecordExists {
        #[diesel(sql_type = Bool)]
        pub exists: bool,
    }

    let pdata = pdata.into_inner();
    let cuser = depot.current_user()?;
    let mut conn = db::connect()?;
    let role = roles::table
        .find(role_id.into_inner())
        .first::<Role>(&mut conn)?
        .assign_to(cuser, "permissions.assign", &mut conn)?;

    // let realm = realms::table.find(role.realm_id).first::<Realm>(conn)?;
    let is_kernel_role = crate::is_kernel_realm_id(role.realm_id);
    conn.transaction::<_, AppError, _>(|conn| {
        diesel::delete(permissions::table.filter(permissions::role_id.eq(role.id))).execute(conn)?;
        for cperm in &pdata {
            if cperm.filter_int_value.is_some() && cperm.filter_text_value.is_some() {
                tracing::error!(permission = ?cperm, "permission filter_int_value and filter_text_value not set at the same time");
                continue;
            } else if cperm.filter_int_value.is_none() && cperm.filter_text_value.is_none() {
                tracing::error!(permission = ?cperm, "permission both filter_int_value and filter_text_value not set");
                continue;
            }
            let opts: Vec<&Operation> = if is_kernel_role {
                data::operations::all_in_db()
                    .iter()
                    .filter(|(_, o)| o.entity == cperm.entity && o.action == cperm.action && o.scope == cperm.scope)
                    .map(|(_, v)| v)
                    .collect()
            } else {
                data::operations::all_in_db()
                    .iter()
                    .filter(|(_, o)| o.entity == cperm.entity && o.action == cperm.action && o.scope == cperm.scope && o.scope != "kernel")
                    .map(|(_, v)| v)
                    .collect()
            };
            if opts.is_empty() {
                tracing::error!(realm_id = role.realm_id, "role/permission, operation not found");
                continue;
            };
            let filter_valid = if cperm.filter_name == "realm.kind" {
                if is_kernel_role {
                    cperm
                        .filter_text_value
                        .as_ref()
                        .map(|txt| ["kernel", "user", "org"].contains(&&**txt))
                        .unwrap_or(false)
                } else {
                    false
                }
            } else if cperm.filter_name == "realm.id" {
                if is_kernel_role {
                    if let Some(realm_id) = cperm.filter_int_value {
                        diesel_exists!(realms::table.find(realm_id), conn)
                    } else {
                        false
                    }
                } else if cperm.filter_int_value != Some(role.realm_id) {
                    false
                } else {
                    diesel_exists!(realms::table.find(role.realm_id), conn)
                }
            } else if cperm.filter_name == format!("{}.id", &cperm.entity) {
                if is_kernel_role {
                    if let Some(record_id) = cperm.filter_int_value {
                        diesel::sql_query(format!(
                            "select exists(select id from {} where id={})",
                            cperm.entity.to_plural(),
                            record_id
                        ))
                        .get_result::<RecordExists>(conn)?
                        .exists
                    } else {
                        false
                    }
                } else if let Some(record_id) = cperm.filter_int_value {
                    diesel::sql_query(format!(
                        "select exists(select id from {} where id={} and realm_id={})",
                        cperm.entity.to_plural(),
                        record_id,
                        role.realm_id
                    ))
                    .get_result::<RecordExists>(conn)?
                    .exists
                } else {
                    false
                }
            } else if cperm.entity == "creative" {
                if cperm.filter_name == "campaign.id" {
                    if is_kernel_role {
                        if let Some(campaign_id) = cperm.filter_int_value {
                            diesel_exists!(campaigns::table.find(campaign_id), conn)
                        } else {
                            false
                        }
                    } else if let Some(campaign_id) = cperm.filter_int_value {
                        diesel_exists!(
                            campaigns::table
                                .filter(campaigns::id.eq(campaign_id))
                                .filter(campaigns::realm_id.eq(role.realm_id)),
                            conn
                        )
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if !filter_valid {
                tracing::error!(permission = ?cperm, "permission's filter is invalid");
                continue;
            }

            let new_permission = NewPermission {
                realm_id: role.realm_id,
                role_id: role.id,
                entity: &cperm.entity,
                action: &cperm.action,
                scope: &cperm.scope,
                filter_name: &cperm.filter_name,
                filter_int_value: cperm.filter_int_value,
                filter_text_value: cperm.filter_text_value.as_deref(),
            };
            diesel::insert_into(permissions::table).values(&new_permission).execute(conn)?;
        }
        Ok(())
    })?;
    drop(conn);

    list.handle(req, depot, res, ctrl).await;
    Ok(())
}
