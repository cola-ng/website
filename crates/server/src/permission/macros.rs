#[macro_export]
macro_rules! filter_for_steward {
    ($fragments:expr, $user:expr, $entity:expr, $action:expr, $edb:path, $conn:expr) => {{
        use diesel::prelude::*;
        use $crate::schema::*;
        use $edb as edb;
        if $user.in_kernel {
            let scopes = $crate::permission::allowed_scopes_for_steward($user, "org", $entity, $action);
            if !scopes.is_empty() {
                let realm_ids = realms::table
                    .filter(realms::kind.eq("org"))
                    .filter(
                        realms::id.eq_any(
                            realm_stewards::table
                                .filter(realm_stewards::user_id.eq($user.id))
                                .select(realm_stewards::realm_id),
                        ),
                    )
                    .select(realms::id);
                if scopes.contains("*") {
                    let query = edb::realm_id.eq_any(realm_ids);
                    // tracing::info!(code = $entity, action = $action, fragment = %debug_query!(&query), "filter_for_steward");
                    $fragments.push(Box::new(query));
                } else if scopes.contains("owned") {
                    let query = edb::realm_id.eq_any(realm_ids).and(edb::owner_id.eq($user.id));
                    // tracing::info!(code = $entity, action = $action, fragment = %debug_query!(&query), "filter_for_steward");
                    $fragments.push(Box::new(query));
                }
            }

            let scopes = $crate::permission::allowed_scopes_for_steward($user, "user", $entity, $action);
            if !scopes.is_empty() {
                let realm_ids = realms::table
                    .filter(realms::kind.eq("user"))
                    .filter(
                        realms::id.eq_any(
                            realm_stewards::table
                                .filter(realm_stewards::user_id.eq($user.id))
                                .select(realm_stewards::realm_id),
                        ),
                    )
                    .select(realms::id);
                if scopes.contains("*") {
                    let query = edb::realm_id.eq_any(realm_ids);
                    // tracing::info!(code = $entity, action = $action, fragment = %debug_query!(&query), "filter_for_steward");
                    $fragments.push(Box::new(query));
                } else if scopes.contains("owned") {
                    let query = edb::realm_id.eq_any(realm_ids).and(edb::owner_id.eq($user.id));
                    // tracing::info!(code = $entity, action = $action, fragment = %debug_query!(&query), "filter_for_steward");
                    $fragments.push(Box::new(query));
                }
            }
        }
    }};
}
#[macro_export]
macro_rules! filter_for_role {
    ($fragments:expr, $user:expr, $entity:expr, $action:expr, $edb:path, $conn:expr) => {{
        use diesel::prelude::*;
        use std::collections::HashSet;
        use $crate::models::*;
        use $crate::schema::*;
        use $edb as edb;

        let role_ids_query = role_users::table.filter(role_users::user_id.eq($user.id)).select(role_users::role_id);
        let permissions = permissions::table
            .filter(permissions::entity.eq($entity))
            .filter(permissions::action.eq($action))
            .filter(permissions::role_id.eq_any(role_ids_query))
            .get_results::<Permission>($conn)?;
        let record_ids = permissions
            .iter()
            .filter_map(|p| {
                if p.filter_name == format!("{}.id", $entity) {
                    p.filter_int_value
                } else {
                    None
                }
            })
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
            let query = edb::id.eq_any(record_ids);
            $fragments.push(Box::new(query));
        }
        if owned_allowed_realm_ids.len() > 0 {
            let query = edb::owner_id.eq($user.id).and(edb::realm_id.eq_any(owned_allowed_realm_ids));
            $fragments.push(Box::new(query));
        }
        if owned_allowed_realm_kinds.len() > 0 {
            let query = edb::owner_id.eq($user.id).and(diesel::dsl::exists(
                realms::table
                    .filter(realms::kind.eq_any(owned_allowed_realm_kinds))
                    .filter(realms::id.eq(edb::realm_id)),
            ));
            $fragments.push(Box::new(query));
        }

        if !realm_ids.is_empty() {
            let query = edb::realm_id.eq_any(realm_ids);
            // tracing::info!(
            //     entity = $entity,
            //     action = $action,
            //     fragment = %debug_query!(&query),
            //     "filter_for_role by realm ids"
            // );
            $fragments.push(Box::new(query));
        }
        if $user.in_kernel && !realm_kinds.is_empty() {
            let query = diesel::dsl::exists(
                realms::table
                    .filter(realms::kind.eq_any(realm_kinds))
                    .filter(realms::id.eq(edb::realm_id))
                    .select(realms::id),
            );
            // tracing::info!(
            //     entity = $entity,
            //     action = $action,
            //     fragment = %debug_query!(&query),
            //     "filter_for_role by realm kinds"
            // );
            $fragments.push(Box::new(query));
        }
    }};
}

#[macro_export]
macro_rules! kernel_entity_filter_for_role {
    ($fragments:expr, $user:expr, $entity:expr, $action:expr, $edb:path, $conn:expr) => {{
        use diesel::prelude::*;
        use $crate::models::*;
        use $crate::schema::*;
        use $edb as edb;

        if $user.in_kernel {
            let role_ids_query = role_users::table.filter(role_users::user_id.eq($user.id)).select(role_users::role_id);
            let permissions = permissions::table
                .filter(permissions::entity.eq($entity))
                .filter(permissions::action.eq($action))
                .filter(permissions::role_id.eq_any(role_ids_query))
                .get_results::<Permission>($conn)?;

            let all_allowed = permissions
                .iter()
                .any(|p| p.filter_name == "realm.kind" && p.scope == "*" && p.filter_text_value == Some("kernel".into()));
            if all_allowed {
                return Ok(PermitFilter::Allowed);
            }
            let record_ids = permissions
                .iter()
                .filter_map(|p| {
                    if p.filter_name == format!("{}.id", $entity) {
                        p.filter_int_value
                    } else {
                        None
                    }
                })
                .collect::<Vec<i64>>();

            if !record_ids.is_empty() {
                let query = edb::id.eq_any(record_ids);
                $fragments.push(Box::new(query));
            }
        }
    }};
}

#[macro_export]
macro_rules! filter_for_realm_owner {
    ($fragments:expr, $user:expr, $entity:expr, $action:expr, $edb:path, $conn:expr) => {{
        use diesel::prelude::*;
        use $crate::schema::*;
        use $edb as edb;

        let scopes = $crate::permission::allowed_scopes_for_realm_root_user($user, "kernel", $entity, $action);
        if !scopes.is_empty() {
            let query = realm_users::table
                .filter(realm_users::user_id.eq($user.id))
                .filter(realm_users::realm_id.eq($crate::kernel_realm_id()))
                .filter(realm_users::is_root.eq(true));
            if diesel_exists!(query, $conn) {
                if scopes.contains("*") || scopes.contains("kernel") {
                    let query = edb::realm_id.eq($crate::kernel_realm_id());
                    $fragments.push(Box::new(query));
                } else if scopes.contains("owned") {
                    let query = edb::realm_id.eq($crate::kernel_realm_id()).and(edb::owner_id.eq($user.id));
                    $fragments.push(Box::new(query));
                }
            }
        }
        let scopes = $crate::permission::allowed_scopes_for_realm_root_user($user, "user", $entity, $action);
        if !scopes.is_empty() {
            let realm_ids = realms::table
                .filter(realms::kind.eq("user"))
                .filter(
                    realms::id.eq_any(
                        realm_users::table
                            .filter(realm_users::user_id.eq($user.id))
                            .filter(realm_users::is_root.eq(true))
                            .select(realm_users::realm_id),
                    ),
                )
                .select(realms::id)
                .get_results::<i64>($conn)?;
            if !realm_ids.is_empty() {
                if scopes.contains("*") {
                    let query = edb::realm_id.eq_any(realm_ids);
                    $fragments.push(Box::new(query));
                } else if scopes.contains("owned") {
                    let query = edb::realm_id.eq_any(realm_ids).and(edb::owner_id.eq($user.id));
                    $fragments.push(Box::new(query));
                }
            }
        }
        let scopes = $crate::permission::allowed_scopes_for_realm_root_user($user, "org", $entity, $action);
        if !scopes.is_empty() {
            let realm_ids = realms::table
                .filter(realms::kind.eq("org"))
                .filter(
                    realms::id.eq_any(
                        realm_users::table
                            .filter(realm_users::user_id.eq($user.id))
                            .filter(realm_users::is_root.eq(true))
                            .select(realm_users::realm_id),
                    ),
                )
                .select(realms::id)
                .get_results::<i64>($conn)?;
            if !realm_ids.is_empty() {
                if scopes.contains("*") {
                    let query = edb::realm_id.eq_any(realm_ids);
                    $fragments.push(Box::new(query));
                } else if scopes.contains("owned") {
                    let query = edb::realm_id.eq_any(realm_ids).and(edb::owner_id.eq($user.id));
                    $fragments.push(Box::new(query));
                }
            }
        }
    }};
}

#[macro_export]
macro_rules! any_permitted_for_kernel_entity {
    ($user:expr, $entity:expr, $action:expr, $edb:path, $conn:expr) => {{
        use diesel::prelude::*;
        use $crate::schema::*;
        use $edb as edb;

        let mut permitted = $user.is_root;
        if !permitted && $user.in_kernel {
            let role_ids = role_users::table
                .filter(role_users::realm_id.eq($crate::kernel_realm_id()))
                .filter(role_users::user_id.eq($user.id))
                .select(role_users::role_id)
                .get_results::<i64>($conn)?;
            if !role_ids.is_empty() {
                let query = permissions::table
                    .filter(
                        permissions::filter_name
                            .eq("realm.kind")
                            .and(permissions::filter_text_value.eq("kernel"))
                            .or(permissions::filter_name
                                .eq("realm.id")
                                .and(permissions::filter_int_value.eq($crate::kernel_realm_id()))),
                    )
                    .filter(permissions::entity.eq($entity))
                    .filter(permissions::action.eq($action))
                    .filter(permissions::role_id.eq_any(&role_ids));
                permitted = diesel_exists!(query, $conn);
            }

            if !permitted {
                let record_ids = permissions::table
                    .filter(permissions::filter_name.eq(format!("{}.id", $entity)))
                    .filter(permissions::entity.eq($entity))
                    .filter(permissions::action.eq($action))
                    .filter(permissions::role_id.eq_any(&role_ids))
                    .select(permissions::filter_int_value)
                    .get_results::<Option<i64>>($conn)?
                    .into_iter()
                    .filter_map(|id| id)
                    .collect::<Vec<_>>();
                if !record_ids.is_empty() {
                    let query = edb::table.filter(edb::id.eq_any(&record_ids));
                    permitted = diesel_exists!(query, $conn);
                }
            }
        }
        Ok(permitted)
    }};
}

#[macro_export]
macro_rules! any_permitted_in_realm {
    ($user:expr, $realm:expr, $entity:expr, $action:expr, $edb:path, $conn:expr) => {{
        use diesel::prelude::*;
        use $crate::schema::*;
        use $edb as edb;

        if $crate::permission::allowed_scopes($user, &$realm.kind, $entity, $action)?.is_empty() {
            Ok(false)
        } else {
            let mut permitted = false;
            if $user.is_root || $realm.id == $user.realm_id || ($user.in_kernel && $user.is_realm_steward($realm.id, $conn)?) {
                permitted = true;
            }
            if !permitted {
                let query = realm_users::table
                    .filter(realm_users::realm_id.eq($realm.id))
                    .filter(realm_users::user_id.eq($user.id))
                    .filter(realm_users::is_root.eq(true));
                permitted = diesel_exists!(query, $conn);
            }
            if !permitted {
                let role_ids = role_users::table
                    .filter(role_users::realm_id.eq_any(vec![$realm.id, $crate::kernel_realm_id()]))
                    .filter(role_users::user_id.eq($user.id))
                    .select(role_users::role_id)
                    .get_results::<i64>($conn)?;
                if !role_ids.is_empty() {
                    let query = permissions::table
                        .filter(
                            permissions::filter_name
                                .eq("realm.kind")
                                .and(permissions::filter_text_value.eq(&$realm.kind))
                                .or(permissions::filter_name.eq("realm.id").and(permissions::filter_int_value.eq($realm.id))),
                        )
                        .filter(permissions::entity.eq($entity))
                        .filter(permissions::action.eq($action))
                        .filter(permissions::role_id.eq_any(&role_ids));
                    permitted = diesel_exists!(query, $conn);
                }
                //TODO: global, campaign...
                if !permitted {
                    let record_ids = permissions::table
                        .filter(permissions::filter_name.eq(format!("{}.id", $entity)))
                        .filter(permissions::entity.eq($entity))
                        .filter(permissions::action.eq($action))
                        .filter(permissions::role_id.eq_any(&role_ids))
                        .select(permissions::filter_int_value)
                        .get_results::<Option<i64>>($conn)?
                        .into_iter()
                        .filter_map(|id| id)
                        .collect::<Vec<_>>();
                    if !record_ids.is_empty() {
                        let query = edb::table.filter(edb::realm_id.eq($realm.id)).filter(edb::id.eq_any(&record_ids));
                        permitted = diesel_exists!(query, $conn);
                    }
                }
            }
            Ok(permitted)
        }
    }};
}

#[macro_export]
macro_rules! all_permitted_in_realm {
    ($user:expr, $realm:expr, $entity:expr, $action:expr, $conn:expr) => {{
        use diesel::prelude::*;
        use $crate::schema::*;

        if $crate::permission::allowed_scopes($user, &$realm.kind, $entity, $action)?.is_empty() {
            Ok(false)
        } else {
            let mut permitted = false;
            if $user.is_root
                || ($realm.id == $user.realm_id && $realm.kind == "user")
                || ($user.in_kernel && $user.is_realm_steward($realm.id, $conn)?)
            {
                permitted = true
            }
            if !permitted {
                let query = realm_users::table
                    .filter(realm_users::realm_id.eq($realm.id))
                    .filter(realm_users::user_id.eq($user.id))
                    .filter(realm_users::is_root.eq(true));
                permitted = diesel_exists!(query, $conn);
            }
            if !permitted {
                let query = role_users::table.filter(role_users::user_id.eq($user.id)).filter(
                    role_users::role_id.eq_any(
                        permissions::table
                            .filter(
                                permissions::filter_name
                                    .eq("realm.id")
                                    .and(permissions::filter_int_value.eq($realm.id))
                                    .or(permissions::filter_name
                                        .eq("realm.kind")
                                        .and(permissions::filter_text_value.eq(&$realm.kind))),
                            )
                            .filter(permissions::entity.eq($entity))
                            .filter(permissions::action.eq($action))
                            .select(permissions::role_id),
                    ),
                );
                permitted = diesel_exists!(query, $conn);
            }
            Ok(permitted)
        }
    }};
}

#[macro_export]
macro_rules! all_permitted_in_kernel {
    ($user:expr, $entity:expr, $action:expr, $conn:expr) => {{
        use diesel::prelude::*;
        use $crate::schema::*;
        if !$user.in_kernel {
            return Ok(false);
        }
        if $user.is_root {
            return Ok(true);
        }
        let query = realm_users::table
            .filter(realm_users::is_root.eq(true))
            .filter(realm_users::user_id.eq($user.id))
            .filter(realm_users::realm_id.eq($crate::kernel_realm_id()));
        if diesel_exists!(query, $conn) {
            return Ok(true);
        }
        let query = role_users::table.filter(role_users::user_id.eq($user.id)).filter(
            role_users::role_id.eq_any(
                permissions::table
                    .filter(permissions::filter_name.eq("realm.kind").and(permissions::filter_text_value.eq("kernel")))
                    .filter(permissions::entity.eq($entity))
                    .filter(permissions::action.eq($action))
                    .select(permissions::role_id),
            ),
        );
        Ok(diesel_exists!(query, $conn))
    }};
}

#[macro_export]
macro_rules! permit_filter {
    ($user:expr, $model:ty, $entity:expr, $action:expr, $edb:path, $conn:expr) => {{
        use $crate::db::PermitFilter;
        use $edb as edb;

        if $user.is_root {
            return Ok(PermitFilter::Allowed);
        }

        let mut fragments: Vec<Box<dyn diesel::query_builder::QueryFragment<diesel::pg::Pg>>> = vec![];
        filter_for_steward!(fragments, $user, $entity, $action, $edb, $conn);
        filter_for_realm_owner!(fragments, $user, $entity, $action, $edb, $conn);
        filter_for_role!(fragments, $user, $entity, $action, edb, $conn);

        if !fragments.is_empty() {
            Ok(PermitFilter::Query(fragments))
        } else {
            Ok(PermitFilter::Denied)
        }
    }};
}
#[macro_export]
macro_rules! kernel_entity_permit_filter {
    ($user:expr, $model:ty, $entity:expr, $action:expr, $edb:path, $conn:expr) => {{
        use diesel::prelude::*;
        use $crate::db::PermitFilter;
        use $crate::schema::*;
        use $edb as edb;

        if $user.is_root {
            return Ok(PermitFilter::Allowed);
        }
        let query = realm_users::table
            .filter(realm_users::is_root.eq(true))
            .filter(realm_users::user_id.eq($user.id))
            .filter(realm_users::realm_id.eq($crate::kernel_realm_id()));
        if diesel_exists!(query, $conn) {
            return Ok(PermitFilter::Allowed);
        }

        let mut fragments: Vec<Box<dyn diesel::query_builder::QueryFragment<diesel::pg::Pg>>> = vec![];
        kernel_entity_filter_for_role!(fragments, $user, $entity, $action, edb, $conn);

        if !fragments.is_empty() {
            Ok(PermitFilter::Query(fragments))
        } else {
            Ok(PermitFilter::Denied)
        }
    }};
}

#[macro_export]
macro_rules! record_permitted {
    ($record:expr, $user:expr, $action:expr, $edb:path, $conn:expr) => {{
        use diesel::prelude::*;
        use $edb as edb;
        if $user.is_root {
            return Ok(true);
        }
        let query = edb::table.permit($user, $action, $conn)?.filter(edb::id.eq($record.id)).select(edb::id);
        // print_query!(&query);
        Ok(diesel_exists!(query, $conn))
    }};
}
#[macro_export]
macro_rules! entity_accessible {
    ($model:ty, $edb:path) => {
        impl $crate::permission::Accessible for $model {
            fn permitted(&self, user: &$crate::models::User, action: &str, conn: &mut diesel::pg::PgConnection) -> $crate::AppResult<bool> {
                record_permitted!(self, user, action, $edb, conn)
            }
        }
    };
}

#[macro_export]
macro_rules! owner_entity_accessible {
    ($model:ty) => {
        impl $crate::permission::Accessible for $model {
            fn permitted(&self, user: &$crate::models::User, _action: &str, _conn: &mut diesel::pg::PgConnection) -> $crate::AppResult<bool> {
                Ok(self.owner_id == user.id)
            }
        }
    };
}

#[macro_export]
macro_rules! in_realm_entity_impl {
    ($model:ty, $table:ty, $edb:path, $entity:expr) => {
        pub fn all_permitted_in_realm(
            user: &$crate::models::User,
            realm: &$crate::models::Realm,
            action: &str,
            conn: &mut diesel::pg::PgConnection,
        ) -> $crate::AppResult<bool> {
            all_permitted_in_realm!(user, realm, $entity, action, conn)
        }
        pub fn permit_filter(
            user: &$crate::models::User,
            action: &str,
            conn: &mut diesel::pg::PgConnection,
        ) -> $crate::AppResult<$crate::db::PermitFilter> {
            permit_filter!(user, $model, $entity, action, $edb, conn)
        }
        table_permit!($table);
    };
}
#[macro_export]
macro_rules! kernel_entity_impl {
    ($model:ty, $table:ty, $edb:path, $entity:expr) => {
        pub fn all_permitted_in_kernel(user: &$crate::models::User, action: &str, conn: &mut diesel::pg::PgConnection) -> $crate::AppResult<bool> {
            all_permitted_in_kernel!(user, $entity, action, conn)
        }
        pub fn permit_filter(
            user: &$crate::models::User,
            action: &str,
            conn: &mut diesel::pg::PgConnection,
        ) -> $crate::AppResult<$crate::db::PermitFilter> {
            kernel_entity_permit_filter!(user, $model, $entity, action, $edb, conn)
        }
        table_permit!($table);
    };
}
#[macro_export]
macro_rules! table_permit {
    ($table:ty) => {
        impl $table {
            pub fn permit(
                self,
                user: &$crate::models::User,
                action: &str,
                conn: &mut diesel::pg::PgConnection,
            ) -> $crate::AppResult<diesel::helper_types::Filter<Self, $crate::db::PermitFilter>> {
                use diesel::prelude::*;
                let filter = permit_filter(user, action, conn)?;
                Ok(self.filter(filter))
            }
        }
    };
}
#[macro_export]
macro_rules! in_realm_entity_impls {
    ($($mname:ident, $model:ty, $table:ty, $edb:path, $entity:expr;)+) => {
        $(
        pub mod $mname {
            in_realm_entity_impl!($model, $table, $edb, $entity);
        })+
    };
}
#[macro_export]
macro_rules! kernel_entity_impls {
    ($($mname:ident, $model:ty, $table:ty, $edb:path, $entity:expr;)+) => {
        $(
        pub mod $mname {
            kernel_entity_impl!($model, $table, $edb, $entity);
        })+
    };
}
#[macro_export]
macro_rules! require_all_permitted_in_kernel {
    ($res:expr, $user:expr, $action:expr, $pimp:path, $conn:expr) => {{
        use $pimp as pimp;
        if !pimp::all_permitted_in_kernel($user, $action, $conn)? {
            return Err(salvo::http::StatusError::forbidden().into());
        }
    }};
}

#[macro_export]
macro_rules! require_all_permitted_in_realm {
    ($res:expr, $user:expr, $realm:expr, $action:expr, $pimp:path, $conn:expr) => {{
        use $pimp as pimp;
        if !pimp::all_permitted_in_realm($user, $realm, $action, $conn)? {
            return Err(salvo::http::StatusError::forbidden().into());
        }
    }};
}
#[macro_export]
macro_rules! require_all_permitted_in_campaign {
    ($res:expr, $user:expr, $campaign:expr, $action:expr, $pimp:path, $conn:expr) => {{
        use $pimp as pimp;
        if !pimp::all_permitted_in_campaign($user, $campaign, $action, $conn)? {
            return Err(salvo::http::StatusError::forbidden().into());
        }
    }};
}

#[macro_export]
macro_rules! filter_entity_record_ids {
    ($user:expr, $action:expr, $record_ids:expr, $edb:path, $conn:expr) => {{
        use diesel::prelude::*;
        use $edb as edb;
        // print_query!(&edb::table.permit($user, $action)?.filter(edb::id.eq_any($record_ids)).select(edb::id));
        edb::table
            .permit($user, $action, $conn)?
            .filter(edb::id.eq_any($record_ids))
            .select(edb::id)
            .get_results::<i64>($conn)?
    }};
}
#[macro_export]
macro_rules! filter_entity_actions {
    ($user:expr, $realm:expr, $entity:expr, $actions:expr, $edb:path, $conn:expr) => {{
        let mut filtered = Vec::with_capacity($actions.len());
        for action in $actions {
            let result: AppResult<bool> = any_permitted_in_realm!($user, $realm, $entity, action, $edb, $conn);
            if result.unwrap_or(false) {
                filtered.push(action.to_owned());
            }
        }
        filtered
    }};
}

#[macro_export]
macro_rules! filter_kernel_entity_actions {
    ($user:expr, $entity:expr, $actions:expr, $edb:path, $conn:expr) => {{
        let mut filtered = Vec::with_capacity($actions.len());
        for action in $actions {
            let result: AppResult<bool> = any_permitted_for_kernel_entity!($user, $entity, action, $edb, $conn);
            if result.unwrap_or(false) {
                filtered.push(action.to_owned());
            }
        }
        filtered
    }};
}
