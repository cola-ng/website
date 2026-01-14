use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::QueryFragment;
use diesel::result::QueryResult;
use std::collections::HashSet;

use crate::AppResult;
use crate::db::PermitFilter;
use crate::models::*;
use crate::db::schema::*;

table_permit!(trade_orders::table);

pub fn all_permitted_in_realm(user: &User, realm: &Realm, action: &str, conn: &mut PgConnection) -> AppResult<bool> {
    all_permitted_in_realm!(user, realm, "trade_order", action, conn)
}

pub fn permit_filter(user: &User, action: &str, conn: &mut PgConnection) -> AppResult<PermitFilter> {
    if user.is_root {
        return Ok(PermitFilter::Allowed);
    }

    let mut fragments: Vec<Box<dyn diesel::query_builder::QueryFragment<diesel::pg::Pg>>> = vec![];
    filter_for_steward!(fragments, user, "trade_order", action, trade_orders, conn);
    order_filter_for_realm_owner(&mut fragments, user, action, conn)?;

    order_filter_for_role(&mut fragments, user, action, conn)?;

    if !fragments.is_empty() {
        Ok(PermitFilter::Query(fragments))
    } else {
        Ok(PermitFilter::Denied)
    }
}

fn order_filter_for_realm_owner(
    fragments: &mut Vec<Box<dyn QueryFragment<Pg>>>,
    user: &User,
    action: &str,
    conn: &mut PgConnection,
) -> QueryResult<()> {
    let scopes = crate::permission::allowed_scopes_for_realm_root_user(user, "user", "trade_order", action);
    if !scopes.is_empty() {
        let realm_ids = realms::table
            .filter(realms::kind.eq("user"))
            .filter(
                realms::id.eq_any(
                    realm_users::table
                        .filter(realm_users::user_id.eq(user.id))
                        .filter(realm_users::is_root.eq(true))
                        .select(realm_users::realm_id),
                ),
            )
            .select(realms::id)
            .get_results::<i64>(conn)?;
        if !realm_ids.is_empty() {
            if scopes.contains("*") || scopes.contains("realm") {
                if !user.in_kernel {
                    if action == "view" || action == "edit" {
                        let query = trade_orders::realm_id.eq_any(realm_ids);
                        fragments.push(Box::new(query));
                    } else {
                        let query = trade_orders::realm_id
                            .eq_any(realm_ids)
                            .and(trade_orders::flow_status.eq_any(&["created", "invalided", "pending_pay"]))
                            .and(trade_orders::controlled_by.ne("realm.kernel"));
                        fragments.push(Box::new(query));
                    }
                } else {
                    let query = trade_orders::realm_id.eq_any(realm_ids);
                    fragments.push(Box::new(query));
                }
            } else if scopes.contains("owned") {
                if !user.in_kernel {
                    if action == "view" || action == "edit" {
                        let query = trade_orders::realm_id.eq_any(realm_ids).and(trade_orders::owner_id.eq(user.id));
                        fragments.push(Box::new(query));
                    } else {
                        let query = trade_orders::realm_id
                            .eq_any(realm_ids)
                            .and(trade_orders::flow_status.eq_any(&["created", "invalided", "pending_pay"]))
                            .and(trade_orders::owner_id.eq(user.id))
                            .and(trade_orders::controlled_by.ne("realm.kernel"));
                        fragments.push(Box::new(query));
                    }
                } else {
                    let query = trade_orders::realm_id.eq_any(realm_ids).and(trade_orders::owner_id.eq(user.id));
                    fragments.push(Box::new(query));
                }
            }
        }
    }
    let scopes = crate::permission::allowed_scopes_for_realm_root_user(user, "org", "trade_order", action);
    if !scopes.is_empty() {
        let realm_ids = realms::table
            .filter(realms::kind.eq("org"))
            .filter(
                realms::id.eq_any(
                    realm_users::table
                        .filter(realm_users::user_id.eq(user.id))
                        .filter(realm_users::is_root.eq(true))
                        .select(realm_users::realm_id),
                ),
            )
            .select(realms::id)
            .get_results::<i64>(conn)?;
        if !realm_ids.is_empty() {
            if scopes.contains("*") || scopes.contains("realm") {
                if !user.in_kernel {
                    if action == "view" || action == "edit" {
                        let query = trade_orders::realm_id.eq_any(realm_ids);
                        fragments.push(Box::new(query));
                    } else {
                        let query = trade_orders::realm_id
                            .eq_any(realm_ids)
                            .and(trade_orders::flow_status.eq_any(&["created", "invalided", "pending_pay"]))
                            .and(trade_orders::controlled_by.ne("realm.kernel"));
                        fragments.push(Box::new(query));
                    }
                } else {
                    let query = trade_orders::realm_id.eq_any(realm_ids);
                    fragments.push(Box::new(query));
                }
            } else if scopes.contains("owned") {
                if !user.in_kernel {
                    if action == "view" || action == "edit" {
                        let query = trade_orders::realm_id.eq_any(realm_ids).and(trade_orders::owner_id.eq(user.id));
                        fragments.push(Box::new(query));
                    } else {
                        let query = trade_orders::realm_id
                            .eq_any(realm_ids)
                            .and(trade_orders::flow_status.eq_any(&["created", "invalided", "pending_pay"]))
                            .and(trade_orders::owner_id.eq(user.id))
                            .and(trade_orders::controlled_by.ne("realm.kernel"));
                        fragments.push(Box::new(query));
                    }
                } else {
                    let query = trade_orders::realm_id.eq_any(realm_ids).and(trade_orders::owner_id.eq(user.id));
                    fragments.push(Box::new(query));
                }
            }
        }
    }
    Ok(())
}

fn order_filter_for_role(fragments: &mut Vec<Box<dyn QueryFragment<Pg>>>, user: &User, action: &str, conn: &mut PgConnection) -> QueryResult<()> {
    let role_ids_query = role_users::table.filter(role_users::user_id.eq(user.id)).select(role_users::role_id);
    let permissions = permissions::table
        .filter(permissions::entity.eq("trade_order"))
        .filter(permissions::action.eq(action))
        .filter(permissions::role_id.eq_any(role_ids_query))
        .get_results::<Permission>(conn)?;

    let record_ids = permissions
        .iter()
        .filter_map(|p| if p.filter_name == "trade_order.id" { p.filter_int_value } else { None })
        .collect::<Vec<i64>>();
    let owned_allowed_realm_ids = permissions
        .iter()
        .filter_map(|p| {
            if p.filter_name == "realm.id" && p.scope == "owned" {
                p.filter_int_value
            } else {
                None
            }
        })
        .collect::<Vec<i64>>();
    let owned_allowed_realm_kinds = permissions
        .iter()
        .filter_map(|p| {
            if p.filter_name == "realm.kind" && p.scope == "owned" {
                p.filter_text_value.clone()
            } else {
                None
            }
        })
        .collect::<Vec<String>>();
    let realm_ids = permissions
        .iter()
        .filter_map(|p| {
            if p.filter_name == "realm.id" && p.scope != "owned" {
                p.filter_int_value
            } else {
                None
            }
        })
        .collect::<Vec<i64>>();
    let realm_kinds = permissions
        .iter()
        .filter_map(|p| {
            if p.filter_name == "realm.kind" && p.scope != "owned" {
                p.filter_text_value.to_owned()
            } else {
                None
            }
        })
        .collect::<HashSet<String>>();
    if !record_ids.is_empty() {
        if !user.in_kernel {
            if action == "view" || action == "edit" {
                let query = trade_orders::id.eq_any(record_ids);
                fragments.push(Box::new(query));
            } else {
                let query = trade_orders::id
                    .eq_any(record_ids)
                    .and(trade_orders::flow_status.eq_any(&["created", "invalided", "pending_pay"]))
                    .and(trade_orders::controlled_by.ne("realm.kernel"));
                fragments.push(Box::new(query));
            }
        } else {
            let query = trade_orders::id.eq_any(record_ids);
            fragments.push(Box::new(query));
        }
    }
    if !owned_allowed_realm_ids.is_empty() {
        if !user.in_kernel {
            if action == "view" || action == "edit" {
                let query = trade_orders::owner_id
                    .eq(user.id)
                    .and(trade_orders::realm_id.eq_any(owned_allowed_realm_ids));
                fragments.push(Box::new(query));
            } else {
                let query = trade_orders::owner_id
                    .eq(user.id)
                    .and(trade_orders::realm_id.eq_any(owned_allowed_realm_ids))
                    .and(trade_orders::flow_status.eq_any(&["created", "invalided", "pending_pay"]))
                    .and(trade_orders::controlled_by.ne("realm.kernel"));
                fragments.push(Box::new(query));
            }
        } else {
            let query = trade_orders::owner_id
                .eq(user.id)
                .and(trade_orders::realm_id.eq_any(owned_allowed_realm_ids));
            fragments.push(Box::new(query));
        }
    }
    if !owned_allowed_realm_kinds.is_empty() {
        if !user.in_kernel {
            if action == "view" || action == "edit" {
                let query = trade_orders::owner_id.eq(user.id).and(diesel::dsl::exists(
                    realms::table
                        .filter(realms::kind.eq_any(owned_allowed_realm_kinds))
                        .filter(realms::id.eq(trade_orders::realm_id)),
                ));
                fragments.push(Box::new(query));
            } else {
                let query = trade_orders::owner_id
                    .eq(user.id)
                    .and(diesel::dsl::exists(
                        realms::table
                            .filter(realms::kind.eq_any(owned_allowed_realm_kinds))
                            .filter(realms::id.eq(trade_orders::realm_id)),
                    ))
                    .and(trade_orders::flow_status.eq_any(&["created", "invalided", "pending_pay"]))
                    .and(trade_orders::controlled_by.ne("realm.kernel"));
                fragments.push(Box::new(query));
            }
        } else {
            let query = trade_orders::owner_id.eq(user.id).and(diesel::dsl::exists(
                realms::table
                    .filter(realms::kind.eq_any(owned_allowed_realm_kinds))
                    .filter(realms::id.eq(trade_orders::realm_id)),
            ));
            fragments.push(Box::new(query));
        }
    }
    if !realm_ids.is_empty() {
        if !user.in_kernel {
            if action == "view" || action == "edit" {
                let query = trade_orders::realm_id.eq_any(realm_ids);
                // tracing::debug!(
                //     entity = "trade_order",
                //     action,
                //     fragment = %debug_query!(&query),
                //     "filter_for_role by realm ids"
                // );
                fragments.push(Box::new(query));
            } else {
                let query = trade_orders::realm_id
                    .eq_any(realm_ids)
                    .and(trade_orders::flow_status.eq_any(&["created", "invalided", "pending_pay"]));
                // tracing::debug!(
                //     entity = "trade_order",
                //     action,
                //     fragment = %debug_query!(&query),
                //     "filter_for_role by realm ids"
                // );
                fragments.push(Box::new(query));
            }
        } else {
            let query = trade_orders::realm_id.eq_any(realm_ids);
            // tracing::debug!(
            //     entity = "trade_order",
            //     action,
            //     fragment = %debug_query!(&query),
            //     "filter_for_role by realm ids"
            // );
            fragments.push(Box::new(query));
        }
    }
    if user.in_kernel && !realm_kinds.is_empty() {
        let query = diesel::dsl::exists(
            realms::table
                .filter(realms::kind.eq_any(realm_kinds))
                .filter(realms::id.eq(trade_orders::realm_id))
                .select(realms::id),
        );
        // tracing::debug!(
        //     entity = "trade_order",
        //     action,
        //     fragment = %debug_query!(&query),
        //     "filter_for_role by realm kinds"
        // );
        fragments.push(Box::new(query));
    }
    Ok(())
}
