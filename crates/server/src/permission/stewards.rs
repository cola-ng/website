use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::QueryFragment;
use diesel::result::QueryResult;

use crate::AppResult;
use crate::db::PermitFilter;
use crate::models::*;
use crate::permission::Accessible;
use crate::db::schema::*;

table_permit!(stewards::table);
pub fn all_permitted_in_kernel(user: &User, action: &str, conn: &mut PgConnection) -> AppResult<bool> {
    all_permitted_in_kernel!(user, "steward", action, conn)
}
impl Accessible for Steward {
    fn permitted(&self, user: &User, action: &str, conn: &mut PgConnection) -> AppResult<bool> {
        if !user.in_kernel {
            return Ok(false);
        }
        if user.is_root || (action == "view" && user.in_kernel) {
            return Ok(true);
        }
        record_permitted!(self, user, action, stewards, conn)
    }
}
pub fn permit_filter(user: &User, action: &str, conn: &mut PgConnection) -> AppResult<PermitFilter> {
    if !user.in_kernel {
        return Ok(PermitFilter::Denied);
    }
    if user.is_root || (action == "view" && user.in_kernel) {
        return Ok(PermitFilter::Allowed);
    }

    let mut fragments: Vec<Box<dyn QueryFragment<Pg>>> = vec![];

    if user.in_kernel {
        steward_filter_for_role(&mut fragments, user, action, conn)?;
    }

    if !fragments.is_empty() {
        Ok(PermitFilter::Query(fragments))
    } else {
        Ok(PermitFilter::Denied)
    }
}

fn steward_filter_for_role(fragments: &mut Vec<Box<dyn QueryFragment<Pg>>>, user: &User, action: &str, conn: &mut PgConnection) -> QueryResult<()> {
    use crate::models::*;
    use crate::db::schema::*;
    use diesel::prelude::*;
    if user.in_kernel {
        let role_ids_query = role_users::table.filter(role_users::user_id.eq(user.id)).select(role_users::role_id);

        let permissions = permissions::table
            .filter(permissions::entity.eq("steward"))
            .filter(permissions::action.eq(action))
            .filter(permissions::role_id.eq_any(role_ids_query))
            .get_results::<Permission>(conn)?;

        let has_all_permission = permissions.iter().any(|p| {
            p.realm_id.eq(&crate::kernel_realm_id())
                && ((p.filter_name == "realm.kind" && p.filter_text_value == Some("kernel".into()))
                    || (p.filter_name == "realm.id" && p.filter_int_value == Some(crate::kernel_realm_id())))
        });
        if has_all_permission {
            let query = stewards::id.is_not_null();
            fragments.push(Box::new(query));
            return Ok(());
        }

        let record_ids = permissions
            .iter()
            .filter_map(|p| if p.filter_name == "steward.id" { p.filter_int_value } else { None })
            .collect::<Vec<i64>>();

        let query = realms::id.eq_any(record_ids);
        fragments.push(Box::new(query));
    }
    Ok(())
}
