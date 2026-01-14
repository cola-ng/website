use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::QueryFragment;
use diesel::result::QueryResult;
use std::collections::HashSet;

use crate::AppResult;
use crate::db::PermitFilter;
use crate::models::*;
use crate::db::schema::*;

table_permit!(trade_refunds::table);
pub fn permit_filter(user: &User, action: &str, conn: &mut PgConnection) -> AppResult<PermitFilter> {
    if user.is_root {
        return Ok(PermitFilter::Allowed);
    }

    let mut fragments: Vec<Box<dyn diesel::query_builder::QueryFragment<diesel::pg::Pg>>> = vec![];
    // filter_for_steward!(fragments, user, "trade_refund", action, trade_refunds, conn);
    // filter_for_realm_owner!(fragments, user, "trade_refund", action, trade_refunds, conn);

    refund_filter_for_role(&mut fragments, user, action, conn)?;

    if !fragments.is_empty() {
        Ok(PermitFilter::Query(fragments))
    } else {
        Ok(PermitFilter::Denied)
    }
}
fn refund_filter_for_role(fragments: &mut Vec<Box<dyn QueryFragment<Pg>>>, user: &User, action: &str, conn: &mut PgConnection) -> QueryResult<()> {
    let role_ids_query = role_users::table.filter(role_users::user_id.eq(user.id)).select(role_users::role_id);
    let permissions = permissions::table
        .filter(permissions::entity.eq("trade_refund"))
        .filter(permissions::action.eq(action))
        .filter(permissions::role_id.eq_any(role_ids_query))
        .get_results::<Permission>(conn)?;

    let has_all_permission = permissions.iter().any(|p| {
        p.realm_id.eq(&crate::kernel_realm_id())
            && ((p.filter_name == "realm.kind" && p.filter_text_value == Some("kernel".into()))
                || (p.filter_name == "realm.id" && p.filter_int_value == Some(crate::kernel_realm_id())))
    });
    if has_all_permission {
        let query = trade_refunds::id.is_not_null();
        fragments.push(Box::new(query));
        return Ok(());
    }

    let record_ids = permissions
        .iter()
        .filter_map(|p| if p.filter_name == "trade_refund.id" { p.filter_int_value } else { None })
        .collect::<Vec<i64>>();
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
        let query = trade_refunds::id.eq_any(record_ids);
        fragments.push(Box::new(query));
    }

    if !realm_ids.is_empty() {
        let query = trade_refunds::realm_id.eq_any(realm_ids);
        fragments.push(Box::new(query));
    }
    if !realm_kinds.is_empty() {
        let query = trade_refunds::realm_id.eq_any(realms::table.filter(realms::kind.eq_any(realm_kinds)).select(realms::id));
        fragments.push(Box::new(query));
    }
    Ok(())
}
