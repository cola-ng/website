use diesel::dsl;
use diesel::helper_types::Filter;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::QueryFragment;
use diesel::result::QueryResult;
use std::collections::HashSet;

use crate::AppResult;
use crate::db::PermitFilter;
use crate::models::*;
use crate::db::schema::*;

macro_rules! all_entity_permit_realms_filter {
    ($user:expr, $model:ty, $entity:expr, $action:expr, $conn:expr) => {{
        use crate::db::PermitFilter;
        use crate::models::*;
        use crate::db::schema::*;
        use diesel::prelude::*;
        use std::collections::HashSet;

        let filter = if $user.is_root {
            PermitFilter::Allowed
        } else if crate::permission::allowed_scopes($user, "kernel", $entity, $action)?.is_empty()
            && crate::permission::allowed_scopes($user, "org", $entity, $action)?.is_empty()
            && crate::permission::allowed_scopes($user, "user", $entity, $action)?.is_empty()
        {
            PermitFilter::Denied
        } else {
            let mut fragments: Vec<Box<dyn diesel::query_builder::QueryFragment<diesel::pg::Pg>>> = vec![];
            let query = realms::id.eq($user.realm_id);
            fragments.push(Box::new(query));

            if $user.in_kernel {
                let query = realm_stewards::table
                    .filter(realm_stewards::user_id.eq($user.id))
                    .select(realm_stewards::realm_id)
                    .get_results::<i64>($conn)?;
                fragments.push(Box::new(realms::id.eq_any(query)));
            }
            let query = realm_users::table
                .filter(realm_users::user_id.eq($user.id))
                .filter(realm_users::is_root.eq(true))
                .select(realm_users::realm_id);
            fragments.push(Box::new(realms::id.eq_any(query)));

            let role_ids_query = role_users::table.filter(role_users::user_id.eq($user.id)).select(role_users::role_id);
            let realm_ids_query = realms::table
                .filter(realms::id.eq_any(realm_users::table.filter(realm_users::user_id.eq($user.id)).select(realm_users::realm_id)))
                .select(realms::id);
            let permissions = permissions::table
                .filter(permissions::entity.eq($entity))
                .filter(permissions::action.eq($action))
                .filter(permissions::role_id.eq_any(role_ids_query))
                .filter(permissions::realm_id.eq_any(realm_ids_query))
                .get_results::<Permission>($conn)?;
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
            if !realm_kinds.is_empty() && !realm_ids.is_empty() {
                let query = realms::id.eq_any(realm_ids).or(realms::kind.eq_any(realm_kinds));
                fragments.push(Box::new(query));
            } else if !realm_ids.is_empty() {
                let query = realms::id.eq_any(realm_ids);
                fragments.push(Box::new(query));
            } else if !realm_kinds.is_empty() {
                let query = realms::kind.eq_any(realm_kinds);
                fragments.push(Box::new(query));
            }
            PermitFilter::Query(fragments)
        };
        Ok(realms::table.permit($user, "view", $conn)?.filter(filter))
    }};
}

table_permit!(realms::table);
impl realms::table {
    pub fn for_stock_blueprint_permit(
        self,
        user: &User,
        action: &str,
        conn: &mut PgConnection,
    ) -> AppResult<Filter<Self, dsl::And<PermitFilter, PermitFilter>>> {
        all_entity_permit_realms_filter!(user, Blueprint, "stock_blueprint", action, conn)
    }
    pub fn for_stock_mold_permit(
        self,
        user: &User,
        action: &str,
        conn: &mut PgConnection,
    ) -> AppResult<Filter<Self, dsl::And<PermitFilter, PermitFilter>>> {
        all_entity_permit_realms_filter!(user, Mold, "stock_mold", action, conn)
    }
    pub fn for_campaign_permit(
        self,
        user: &User,
        action: &str,
        conn: &mut PgConnection,
    ) -> AppResult<Filter<Self, dsl::And<PermitFilter, PermitFilter>>> {
        all_entity_permit_realms_filter!(user, Campaign, "campaign", action, conn)
    }
    pub fn for_creative_permit(
        self,
        user: &User,
        action: &str,
        conn: &mut PgConnection,
    ) -> AppResult<Filter<Self, dsl::And<PermitFilter, PermitFilter>>> {
        all_entity_permit_realms_filter!(user, Creative, "creative", action, conn)
    }
    pub fn for_interflow_stream_permit(
        self,
        user: &User,
        action: &str,
        conn: &mut PgConnection,
    ) -> AppResult<Filter<Self, dsl::And<PermitFilter, PermitFilter>>> {
        all_entity_permit_realms_filter!(user, Stream, "interflow_stream", action, conn)
    }
    pub fn for_interflow_thread_permit(
        self,
        user: &User,
        action: &str,
        conn: &mut PgConnection,
    ) -> AppResult<Filter<Self, dsl::And<PermitFilter, PermitFilter>>> {
        all_entity_permit_realms_filter!(user, Thread, "interflow_thread", action, conn)
    }
    pub fn for_deploy_deployment_permit(
        self,
        user: &User,
        action: &str,
        conn: &mut PgConnection,
    ) -> AppResult<Filter<Self, dsl::And<PermitFilter, PermitFilter>>> {
        all_entity_permit_realms_filter!(user, Deployment, "deploy_deployment", action, conn)
    }
    pub fn for_stock_font_permit(
        self,
        user: &User,
        action: &str,
        conn: &mut PgConnection,
    ) -> AppResult<Filter<Self, dsl::And<PermitFilter, PermitFilter>>> {
        all_entity_permit_realms_filter!(user, Font, "stock_font", action, conn)
    }
    pub fn for_user_permit(
        self,
        user: &User,
        action: &str,
        conn: &mut PgConnection,
    ) -> AppResult<Filter<Self, dsl::And<PermitFilter, PermitFilter>>> {
        all_entity_permit_realms_filter!(user, User, "user", action, conn)
    }
}

pub fn permit_filter(user: &User, action: &str, conn: &mut PgConnection) -> AppResult<PermitFilter> {
    if user.is_root {
        return Ok(PermitFilter::Allowed);
    }
    let mut fragments: Vec<Box<dyn QueryFragment<Pg>>> = vec![];
    if action == "view" {
        let realm_ids = realm_users::table
            .filter(realm_users::user_id.eq(user.id))
            .select(realm_users::realm_id)
            .get_results::<i64>(conn)?;
        if !realm_ids.is_empty() {
            let query = realms::id.eq_any(realm_ids);
            fragments.push(Box::new(query));
        }
    }

    realm_filter_for_steward(&mut fragments, user, action, conn)?;
    realm_filter_for_realm_owner(&mut fragments, user, action, conn)?;

    realm_filter_for_role(&mut fragments, user, action, conn)?;

    if !fragments.is_empty() {
        Ok(PermitFilter::Query(fragments))
    } else {
        Ok(PermitFilter::Denied)
    }
}
fn realm_filter_for_role(fragments: &mut Vec<Box<dyn QueryFragment<Pg>>>, user: &User, action: &str, conn: &mut PgConnection) -> QueryResult<()> {
    let role_ids_query = role_users::table.filter(role_users::user_id.eq(user.id)).select(role_users::role_id);
    let permissions = permissions::table
        .filter(permissions::entity.eq("realm"))
        .filter(permissions::action.eq(action))
        .filter(permissions::role_id.eq_any(role_ids_query))
        .get_results::<Permission>(conn)?;

    let realm_kinds = permissions
        .iter()
        .filter_map(|p| {
            if p.filter_name == "realm.kind" {
                p.filter_text_value.to_owned()
            } else {
                None
            }
        })
        .collect::<HashSet<String>>();

    let record_ids = permissions
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
        let query = realms::id.eq_any(record_ids);
        fragments.push(Box::new(query));
    }
    if !realm_kinds.is_empty() {
        let query = realms::kind.eq_any(realm_kinds);
        fragments.push(Box::new(query));
    }
    Ok(())
}

fn realm_filter_for_steward(fragments: &mut Vec<Box<dyn QueryFragment<Pg>>>, user: &User, action: &str, conn: &mut PgConnection) -> QueryResult<()> {
    if user.in_kernel {
        if !super::allowed_scopes_for_steward(user, "user", "realm", action).is_empty() {
            let realm_ids = realms::table
                .filter(realms::kind.eq("user"))
                .filter(
                    realms::id.eq_any(
                        realm_stewards::table
                            .filter(realm_stewards::user_id.eq(user.id))
                            .select(realm_stewards::realm_id),
                    ),
                )
                .select(realms::id)
                .get_results::<i64>(conn)?;

            if !realm_ids.is_empty() {
                let query = realms::id.eq_any(realm_ids);
                // tracing::info!(entity = "realm", action, fragment = %debug_query!(&query), "realm_filter_for_steward");
                fragments.push(Box::new(query));
            }
        }
        if !super::allowed_scopes_for_steward(user, "org", "realm", action).is_empty() {
            let realm_ids = realms::table
                .filter(realms::kind.eq("org"))
                .filter(
                    realms::id.eq_any(
                        realm_stewards::table
                            .filter(realm_stewards::user_id.eq(user.id))
                            .select(realm_stewards::realm_id),
                    ),
                )
                .select(realms::id)
                .get_results::<i64>(conn)?;
            if !realm_ids.is_empty() {
                let query = realms::id.eq_any(realm_ids);
                // tracing::info!(entity = "realm", action, fragment = %debug_query!(&query), "realm_filter_for_steward");
                fragments.push(Box::new(query));
            }
        }
    }
    Ok(())
}

fn realm_filter_for_realm_owner(
    fragments: &mut Vec<Box<dyn QueryFragment<Pg>>>,
    user: &User,
    action: &str,
    conn: &mut PgConnection,
) -> QueryResult<()> {
    if !super::allowed_scopes_for_realm_root_user(user, "kernel", "realm", action).is_empty() {
        let query = realm_users::table
            .filter(realm_users::user_id.eq(user.id))
            .filter(realm_users::realm_id.eq(crate::kernel_realm_id()))
            .filter(realm_users::is_root.eq(true));
        if diesel_exists!(query, conn) {
            let query = realms::kind.eq("kernel");
            fragments.push(Box::new(query));
        }
    }
    if !super::allowed_scopes_for_realm_root_user(user, "user", "realm", action).is_empty() {
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
            let query = realms::id.eq_any(realm_ids);
            fragments.push(Box::new(query));
        }
    }
    if !super::allowed_scopes_for_realm_root_user(user, "org", "realm", action).is_empty() {
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
            let query = realms::id.eq_any(realm_ids);
            fragments.push(Box::new(query));
        }
    }
    Ok(())
}
