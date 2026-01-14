use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::QueryFragment;
use diesel::result::QueryResult;

use crate::AppResult;
use crate::db::PermitFilter;
use crate::models::*;
use crate::permission::Accessible;
use crate::db::schema::*;

table_permit!(users::table);
impl Accessible for User {
    fn permitted(&self, visitor: &User, action: &str, conn: &mut PgConnection) -> AppResult<bool> {
        if self.is_root && !visitor.in_kernel {
            return Ok(false);
        }
        if self.id == visitor.id && ["view", "edit"].contains(&action) {
            return Ok(true);
        }
        if action == "view" {
            if visitor.in_kernel {
                return Ok(true);
            } else {
                let allowed = if self.in_kernel {
                    if visitor.is_root || self.id < 100 {
                        true
                    } else {
                        diesel_exists!(stewards::table.filter(stewards::user_id.eq(self.id)).select(stewards::id), conn)
                    }
                } else {
                    false
                };
                if allowed {
                    return Ok(allowed);
                }
                let domains = emails::table
                    .filter(emails::user_id.eq(visitor.id))
                    .filter(emails::is_verified.eq(true))
                    .select(emails::domain)
                    .distinct()
                    .get_results::<String>(conn)?;
                let allowed = if !domains.is_empty() {
                    let query = emails::table
                        .filter(emails::user_id.eq(self.id))
                        .filter(emails::is_verified.eq(true))
                        .filter(emails::domain.eq_any(&domains));
                    diesel_exists!(query, conn)
                } else {
                    false
                };
                if allowed {
                    return Ok(allowed);
                }
                let query = realms::table
                    .filter(diesel::dsl::exists(
                        realm_users::table
                            .filter(realm_users::realm_id.eq(realms::id))
                            .filter(realm_users::user_id.eq(self.id)),
                    ))
                    .filter(diesel::dsl::exists(
                        realm_users::table
                            .filter(realm_users::realm_id.eq(realms::id))
                            .filter(realm_users::user_id.eq(visitor.id)),
                    ));
                return Ok(diesel_exists!(query, conn));
            }
        }
        record_permitted!(self, visitor, action, users, conn)
    }
}
pub fn permit_filter(visitor: &User, action: &str, conn: &mut PgConnection) -> AppResult<PermitFilter> {
    if visitor.is_root {
        return Ok(PermitFilter::Allowed);
    }
    let mut fragments: Vec<Box<dyn QueryFragment<Pg>>> = vec![];
    if action == "view" {
        if visitor.in_kernel {
            return Ok(PermitFilter::Allowed);
        } else {
            let domains = emails::table
                .filter(emails::user_id.eq(visitor.id))
                .filter(emails::is_verified.eq(true))
                .select(emails::domain)
                .distinct()
                .get_results::<String>(conn)?;
            if !domains.is_empty() {
                let query = diesel::dsl::exists(
                    emails::table
                        .filter(emails::user_id.eq(users::id))
                        .filter(emails::domain.eq_any(domains))
                        .filter(emails::is_verified.eq(true))
                        .select(emails::id),
                );
                fragments.push(Box::new(query));
            }
            let query = users::in_kernel.eq(true).and(users::is_root.eq(false)).and(diesel::dsl::exists(
                stewards::table.filter(stewards::user_id.eq(users::id)).select(stewards::id),
            ));
            fragments.push(Box::new(query));

            let query = users::id.eq_any(
                realm_users::table
                    .filter(
                        realm_users::realm_id.eq_any(
                            realm_users::table
                                .filter(realm_users::user_id.eq(visitor.id))
                                .select(realm_users::realm_id),
                        ),
                    )
                    .select(realm_users::user_id)
                    .distinct(),
            );
            fragments.push(Box::new(query));
        }
    } else if action == "edit" {
        //user alway allow delete and edit himself
        fragments.push(Box::new(users::id.eq(visitor.id)));
    }

    if visitor.in_kernel {
        user_filter_for_role(&mut fragments, visitor, action, conn)?;
    }

    if !fragments.is_empty() {
        Ok(PermitFilter::Query(fragments))
    } else {
        Ok(PermitFilter::Denied)
    }
}

fn user_filter_for_role(fragments: &mut Vec<Box<dyn QueryFragment<Pg>>>, user: &User, action: &str, conn: &mut PgConnection) -> QueryResult<()> {
    use crate::models::*;
    use crate::db::schema::*;
    use diesel::prelude::*;
    if user.in_kernel {
        let role_ids_query = role_users::table.filter(role_users::user_id.eq(user.id)).select(role_users::role_id);
        let permissions = permissions::table
            .filter(permissions::entity.eq("user"))
            .filter(permissions::action.eq(action))
            .filter(permissions::role_id.eq_any(role_ids_query))
            .get_results::<Permission>(conn)?;

        let has_all_permission = permissions.iter().any(|p| {
            p.realm_id.eq(&crate::kernel_realm_id())
                && ((p.filter_name == "realm.kind" && p.filter_text_value == Some("kernel".into()))
                    || (p.filter_name == "realm.id" && p.filter_int_value == Some(crate::kernel_realm_id())))
        });
        if has_all_permission {
            let query = users::id.is_not_null();
            fragments.push(Box::new(query));
            return Ok(());
        }

        let record_ids = permissions
            .iter()
            .filter_map(|p| if p.filter_name == "user.id" { p.filter_int_value } else { None })
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

        if !record_ids.is_empty() {
            let query = users::id.eq_any(record_ids);
            // tracing::debug!(code = "user", action, fragment = %debug_query!(&query), "user_filter_for_role");
            fragments.push(Box::new(query));
        }

        //only view action can assign to no kernel realm.
        if !realm_ids.is_empty() && action == "view" {
            let query = users::id.eq_any(
                realm_users::table
                    .filter(realm_users::realm_id.eq_any(realm_ids))
                    .select(realm_users::user_id),
            );
            fragments.push(Box::new(query));
        }
    }
    Ok(())
}
