use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::QueryFragment;
use diesel::result::QueryResult;
use std::collections::HashSet;

use crate::AppResult;
use crate::db::PermitFilter;
use crate::models::*;
use crate::schema::*;

table_permit!(creatives::table);
pub fn all_permitted_in_realm(user: &User, realm: &Realm, action: &str, conn: &mut PgConnection) -> AppResult<bool> {
    all_permitted_in_realm!(user, realm, "creative", action, conn)
}
pub fn all_permitted_in_campaign(user: &User, campaign: &Campaign, action: &str, conn: &mut PgConnection) -> AppResult<bool> {
    let realm = realms::table.find(campaign.realm_id).get_result::<Realm>(conn)?;
    if user.is_root || campaign.realm_id == user.realm_id {
        return Ok(true);
    }
    let query = role_users::table.filter(role_users::user_id.eq(user.id)).filter(
        role_users::role_id.eq_any(
            permissions::table
                .filter(
                    permissions::filter_name
                        .eq("realm.kind")
                        .and(permissions::filter_text_value.eq(&realm.kind))
                        .or(permissions::filter_name.eq("realm.id").and(permissions::filter_int_value.eq(realm.id)))
                        .or(permissions::filter_name
                            .eq("campaign.id")
                            .and(permissions::filter_int_value.eq(campaign.id))),
                )
                .filter(permissions::entity.eq("creative"))
                .filter(permissions::action.eq(action))
                .select(permissions::role_id),
        ),
    );
    // print_query!(&query);
    Ok(diesel_exists!(query, conn))
}

pub fn permit_filter(user: &User, action: &str, conn: &mut PgConnection) -> AppResult<PermitFilter> {
    if user.is_root {
        return Ok(PermitFilter::Allowed);
    }
    if crate::permission::allowed_scopes(user, "org", "creative", action)?.is_empty()
        && crate::permission::allowed_scopes(user, "user", "creative", action)?.is_empty()
    {
        return Ok(PermitFilter::Denied);
    }
    let mut fragments: Vec<Box<dyn QueryFragment<Pg>>> = vec![];
    filter_for_steward!(fragments, user, "creative", action, creatives, conn);
    creative_filter_for_realm_owner(&mut fragments, user, action, conn)?;

    creative_filter_for_role(&mut fragments, user, action, conn)?;

    if !fragments.is_empty() {
        Ok(PermitFilter::Query(fragments))
    } else {
        Ok(PermitFilter::Denied)
    }
}
fn creative_filter_for_realm_owner(
    fragments: &mut Vec<Box<dyn QueryFragment<Pg>>>,
    user: &User,
    action: &str,
    conn: &mut PgConnection,
) -> QueryResult<()> {
    let scopes = crate::permission::allowed_scopes_for_realm_root_user(user, "user", "creative", action);
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
            if scopes.contains("*") {
                if !user.in_kernel && (action == "edit" || action == "delete") {
                    let query = creatives::realm_id.eq_any(realm_ids).and(creatives::controlled_by.ne("realm.kernel"));
                    fragments.push(Box::new(query));
                } else {
                    let query = creatives::realm_id.eq_any(realm_ids).and(
                        creatives::controlled_by
                            .ne("realm.kernel")
                            .or(creatives::controlled_by.eq("realm.kernel").and(creatives::flow_status.ne("developing"))),
                    );
                    fragments.push(Box::new(query));
                }
            } else if scopes.contains("owned") {
                if !user.in_kernel && (action == "edit" || action == "delete") {
                    let query = creatives::realm_id
                        .eq_any(realm_ids)
                        .and(creatives::controlled_by.ne("realm.kernel"))
                        .and(creatives::owner_id.eq(user.id));
                    fragments.push(Box::new(query));
                } else {
                    let query = creatives::realm_id.eq_any(realm_ids).and(creatives::owner_id.eq(user.id));
                    fragments.push(Box::new(query));
                }
            }
        }
    }
    let scopes = crate::permission::allowed_scopes_for_realm_root_user(user, "org", "creative", action);
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
            if scopes.contains("*") {
                if !user.in_kernel && (action == "edit" || action == "delete") {
                    let query = creatives::realm_id.eq_any(realm_ids).and(creatives::controlled_by.ne("realm.kernel"));
                    fragments.push(Box::new(query));
                } else {
                    let query = creatives::realm_id.eq_any(realm_ids).and(
                        creatives::controlled_by
                            .ne("realm.kernel")
                            .or(creatives::controlled_by.eq("realm.kernel").and(creatives::flow_status.ne("developing"))),
                    );
                    fragments.push(Box::new(query));
                }
            } else if scopes.contains("owned") {
                if !user.in_kernel && (action == "edit" || action == "delete") {
                    let query = creatives::realm_id
                        .eq_any(realm_ids)
                        .and(creatives::controlled_by.ne("realm.kernel"))
                        .and(creatives::owner_id.eq(user.id));
                    fragments.push(Box::new(query));
                } else {
                    let query = creatives::realm_id.eq_any(realm_ids).and(creatives::owner_id.eq(user.id));
                    fragments.push(Box::new(query));
                }
            }
        }
    }
    Ok(())
}

fn creative_filter_for_role(fragments: &mut Vec<Box<dyn QueryFragment<Pg>>>, user: &User, action: &str, conn: &mut PgConnection) -> QueryResult<()> {
    let role_ids_query = role_users::table.filter(role_users::user_id.eq(user.id)).select(role_users::role_id);
    let permissions = permissions::table
        .filter(permissions::entity.eq("creative"))
        .filter(permissions::action.eq(action))
        .filter(permissions::role_id.eq_any(role_ids_query))
        .get_results::<Permission>(conn)?;

    // print_query!(&role_ids_query);

    let record_ids = permissions
        .iter()
        .filter_map(|p| if p.filter_name == "creative.id" { p.filter_int_value } else { None })
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
    let campaign_ids = permissions
        .iter()
        .filter_map(|p| {
            if p.filter_name == "campaign.id" && p.scope != "owned" {
                p.filter_int_value
            } else {
                None
            }
        })
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
            if p.filter_name == "realm.kind" {
                p.filter_text_value.to_owned()
            } else {
                None
            }
        })
        .collect::<HashSet<String>>();

    if !record_ids.is_empty() {
        if !user.in_kernel && (action == "edit" || action == "delete") {
            let query = creatives::id.eq_any(record_ids).and(creatives::controlled_by.ne("realm.kernel"));
            // tracing::debug!(entity = "creative", action, fragment = %debug_query!(&query), "creative_filter_for_role");
            fragments.push(Box::new(query));
        } else {
            let query = creatives::id.eq_any(record_ids).and(
                creatives::controlled_by
                    .ne("realm.kernel")
                    .or(creatives::controlled_by.eq("realm.kernel").and(creatives::flow_status.ne("developing"))),
            );
            // tracing::debug!(entity = "creative", action, fragment = %debug_query!(&query), "creative_filter_for_role");
            fragments.push(Box::new(query));
        }
    }
    if !owned_allowed_realm_ids.is_empty() {
        if !user.in_kernel && (action == "edit" || action == "delete") {
            let query = creatives::owner_id
                .eq(user.id)
                .and(creatives::realm_id.eq_any(owned_allowed_realm_ids))
                .and(creatives::controlled_by.ne("realm.kernel"));
            fragments.push(Box::new(query));
        } else {
            let query = creatives::owner_id.eq(user.id).and(creatives::realm_id.eq_any(owned_allowed_realm_ids));
            fragments.push(Box::new(query));
        }
    }
    if !owned_allowed_realm_kinds.is_empty() {
        if !user.in_kernel && (action == "edit" || action == "delete") {
            let query = creatives::owner_id
                .eq(user.id)
                .and(diesel::dsl::exists(
                    realms::table
                        .filter(realms::kind.eq_any(owned_allowed_realm_kinds))
                        .filter(realms::id.eq(creatives::realm_id)),
                ))
                .and(creatives::controlled_by.ne("realm.kernel"));
            fragments.push(Box::new(query));
        } else {
            let query = creatives::owner_id.eq(user.id).and(diesel::dsl::exists(
                realms::table
                    .filter(realms::kind.eq_any(owned_allowed_realm_kinds))
                    .filter(realms::id.eq(creatives::realm_id)),
            ));
            fragments.push(Box::new(query));
        }
    }

    if !campaign_ids.is_empty() {
        if !user.in_kernel && (action == "edit" || action == "delete") {
            let query = creatives::campaign_id
                .eq_any(campaign_ids)
                .and(creatives::controlled_by.ne("realm.kernel"));
            // tracing::debug!(
            //     entity = "creative",
            //     action,
            //     fragment = %debug_query!(&query),
            //     "creative_filter_for_role campaign_ids"
            // );
            fragments.push(Box::new(query));
        } else {
            let query = creatives::campaign_id.eq_any(campaign_ids).and(
                creatives::controlled_by
                    .ne("realm.kernel")
                    .or(creatives::controlled_by.eq("realm.kernel").and(creatives::flow_status.ne("developing"))),
            );
            // tracing::debug!(
            //     entity = "creative",
            //     action,
            //     fragment = %debug_query!(&query),
            //     "creative_filter_for_role campaign_ids"
            // );
            fragments.push(Box::new(query));
        }
    }

    if !realm_ids.is_empty() {
        if !user.in_kernel && (action == "edit" || action == "delete") {
            let query = creatives::realm_id.eq_any(realm_ids).and(creatives::controlled_by.ne("realm.kernel"));
            // tracing::debug!(
            //     entity = "creative",
            //     action,
            //     fragment = %debug_query!(&query),
            //     "creative_filter_for_role realm_ids"
            // );
            fragments.push(Box::new(query));
        } else {
            let query = creatives::realm_id.eq_any(realm_ids).and(
                creatives::controlled_by
                    .ne("realm.kernel")
                    .or(creatives::controlled_by.eq("realm.kernel").and(creatives::flow_status.ne("developing"))),
            );
            // tracing::debug!(
            //     entity = "creative",
            //     action,
            //     fragment = %debug_query!(&query),
            //     "creative_filter_for_role realm_ids"
            // );
            fragments.push(Box::new(query));
        }
    }
    if user.in_kernel && !realm_kinds.is_empty() {
        let query = diesel::dsl::exists(
            realms::table
                .filter(realms::kind.eq_any(realm_kinds))
                .filter(realms::id.eq(creatives::realm_id))
                .select(realms::id),
        );
        // tracing::debug!(
        //     entity = "creative",
        //     action,
        //     fragment = %debug_query!(&query),
        //     "creative_filter_for_role realm_kinds"
        // );
        fragments.push(Box::new(query));
    }
    Ok(())
}
