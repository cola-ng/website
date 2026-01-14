macro_rules! stock_entity_impl {
    ($model:ty, $table:ty, $edb:path, $entity:expr) => {
        use diesel::pg::Pg;
        use diesel::prelude::*;
        use diesel::query_builder::QueryFragment;

        use crate::AppResult;
        use crate::db::PermitFilter;
        use crate::models::*;

        use $edb as edb;
        table_permit!($table);
        pub fn all_permitted_in_realm(user: &User, realm: &Realm, action: &str, conn: &mut diesel::pg::PgConnection) -> AppResult<bool> {
            all_permitted_in_realm!(user, realm, $entity, action, conn)
        }
        pub fn permit_filter(user: &User, action: &str, conn: &mut PgConnection) -> crate::AppResult<PermitFilter> {
            if user.is_root {
                return Ok(PermitFilter::Allowed);
            }
            if crate::permission::allowed_scopes(user, "kernel", $entity, action)?.is_empty()
                && crate::permission::allowed_scopes(user, "org", $entity, action)?.is_empty()
                && crate::permission::allowed_scopes(user, "user", $entity, action)?.is_empty()
            {
                return Ok(PermitFilter::Denied);
            }
            let mut fragments: Vec<Box<dyn QueryFragment<Pg>>> = vec![];
            filter_for_steward!(fragments, user, $entity, action, edb, conn);
            filter_for_realm_owner!(fragments, user, $entity, action, edb, conn);

            filter_for_role!(&mut fragments, user, $entity, action, edb, conn);
            if action == "view" {
                fragments.push(Box::new(edb::is_public.eq(true)));
            }
            if !fragments.is_empty() {
                Ok(PermitFilter::Query(fragments))
            } else {
                Ok(PermitFilter::Denied)
            }
        }
    };
}
// macro_rules! stock_entity_impl_without_public {
//     ($model:ty, $table:ty, $edb:path, $entity:expr) => {
//         use diesel::pg::Pg;
//         use diesel::prelude::*;
//         use diesel::query_builder::QueryFragment;

//         use crate::db::PermitFilter;
//         use crate::models::*;
//         use crate::AppResult;

//         use $edb as edb;
//         table_permit!($table);
//         pub fn all_permitted_in_realm(user: &User, realm: &Realm, action: &str, conn: &mut diesel::pg::PgConnection) -> AppResult<bool> {
//             all_permitted_in_realm!(user, realm, $entity, action, conn)
//         }
//         pub fn permit_filter(user: &User, action: &str, conn: &mut PgConnection) -> crate::AppResult<PermitFilter> {
//             if user.is_root {
//                 return Ok(PermitFilter::Allowed);
//             }
//             if crate::permission::allowed_scopes(user, "kernel", $entity, action)?.is_empty()
//                 && crate::permission::allowed_scopes(user, "org", $entity, action)?.is_empty()
//                 && crate::permission::allowed_scopes(user, "user", $entity, action)?.is_empty()
//             {
//                 return Ok(PermitFilter::Denied);
//             }
//             let mut fragments: Vec<Box<dyn QueryFragment<Pg>>> = vec![];
//             filter_for_steward!(fragments, user, $entity, action, edb, conn);
//             filter_for_realm_owner!(fragments, user, $entity, action, edb, conn);

//             filter_for_role!(&mut fragments, user, $entity, action, edb, conn);
//             if !fragments.is_empty() {
//                 Ok(PermitFilter::Query(fragments))
//             } else {
//                 Ok(PermitFilter::Denied)
//             }
//         }
//     };
// }

macro_rules! filter_for_realm_owner {
    ($fragments:expr, $user:expr, $entity:expr, $action:expr, $edb:path, $conn:expr) => {{
        use crate::schema::*;
        use diesel::prelude::*;
        use $edb as edb;

        let scopes = crate::permission::allowed_scopes_for_realm_root_user($user, "kernel", $entity, $action);
        if !scopes.is_empty() {
            let query = realm_users::table
                .filter(realm_users::user_id.eq($user.id))
                .filter(realm_users::realm_id.eq(crate::kernel_realm_id()))
                .filter(realm_users::is_root.eq(true));
            if diesel_exists!(query, $conn) {
                if scopes.contains("*") || scopes.contains("kernel") {
                    let query = edb::realm_id.eq(crate::kernel_realm_id());
                    $fragments.push(Box::new(query));
                } else if scopes.contains("owned") {
                    let query = edb::realm_id.eq(crate::kernel_realm_id()).and(edb::owner_id.eq($user.id));
                    $fragments.push(Box::new(query));
                }
            }
        }
        let scopes = crate::permission::allowed_scopes_for_realm_root_user($user, "user", $entity, $action);
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
                .select(realms::id);
            if scopes.contains("*") {
                if !$user.in_kernel && ($action == "edit" || $action == "delete") {
                    let query = edb::realm_id.eq_any(realm_ids).and(edb::controlled_by.ne("realm.kernel"));
                    $fragments.push(Box::new(query));
                } else {
                    let query = edb::realm_id.eq_any(realm_ids);
                    // .and(edb::controlled_by.ne("realm.kernel"));
                    $fragments.push(Box::new(query));
                }
            } else if scopes.contains("owned") {
                if !$user.in_kernel && ($action == "edit" || $action == "delete") {
                    let query = edb::realm_id
                        .eq_any(realm_ids)
                        // .and(edb::controlled_by.ne("realm.kernel"))
                        .and(edb::owner_id.eq($user.id));
                    $fragments.push(Box::new(query));
                } else {
                    let query = edb::realm_id.eq_any(realm_ids).and(edb::owner_id.eq($user.id));
                    $fragments.push(Box::new(query));
                }
            }
        }
        let scopes = crate::permission::allowed_scopes_for_realm_root_user($user, "org", $entity, $action);
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
                .select(realms::id);
            if scopes.contains("*") {
                if !$user.in_kernel && ($action == "edit" || $action == "delete") {
                    let query = edb::realm_id.eq_any(realm_ids).and(edb::controlled_by.ne("realm.kernel"));
                    $fragments.push(Box::new(query));
                } else {
                    let query = edb::realm_id.eq_any(realm_ids).and(
                        edb::controlled_by
                            .ne("realm.kernel")
                            .or(edb::controlled_by.eq("realm.kernel").and(edb::flow_status.ne("developing"))),
                    );
                    $fragments.push(Box::new(query));
                }
            } else if scopes.contains("owned") {
                if !$user.in_kernel && ($action == "edit" || $action == "delete") {
                    let query = edb::realm_id
                        .eq_any(realm_ids)
                        // .and(edb::controlled_by.ne("realm.kernel"))
                        .and(edb::owner_id.eq($user.id));
                    $fragments.push(Box::new(query));
                } else {
                    let query = edb::realm_id.eq_any(realm_ids).and(edb::owner_id.eq($user.id));
                    $fragments.push(Box::new(query));
                }
            }
        }
    }};
}

macro_rules! stock_entity_impls {
    ($($mname:ident, $model:ty, $table:ty, $edb:path, $entity:expr;)+) => {
        $(
        pub mod $mname {
            stock_entity_impl!($model, $table, $edb, $entity);
        })+
    };
}

stock_entity_impls! {
    stock_blueprints, crate::models::stock::Blueprint, crate::schema::stock_blueprints::table, crate::schema::stock_blueprints, "stock_blueprint";
    stock_fonts, crate::models::stock::Font, crate::schema::stock_fonts::table, crate::schema::stock_fonts, "stock_font";
    stock_molds, crate::models::stock::Mold, crate::schema::stock_molds::table, crate::schema::stock_molds, "stock_mold";
    stock_images, crate::models::stock::Image, crate::schema::stock_images::table, crate::schema::stock_images, "stock_image";
    stock_videos, crate::models::stock::Video, crate::schema::stock_videos::table, crate::schema::stock_videos, "stock_video";
    stock_audios, crate::models::stock::Audio, crate::schema::stock_audios::table, crate::schema::stock_audios, "stock_audio";
    stock_stylekits, crate::models::stock::Stylekit, crate::schema::stock_stylekits::table, crate::schema::stock_stylekits, "stock_stylekit";
}

pub mod stock_exhibits {
    in_realm_entity_impl!(
        crate::models::stock::Exhibit,
        crate::schema::stock_exhibits::table,
        crate::schema::stock_exhibits,
        "stock_exhibit"
    );
}
