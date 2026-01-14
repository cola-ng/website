#[macro_export]
macro_rules! get_id_query {
    ($req:expr, $res:expr, $name:expr) => {{
        let value = $req.query($name).unwrap_or(0i64);
        if value <= 0 {
            return Err(salvo::http::StatusError::bad_request()
                .brief("Error happened when parse url param.")
                .into());
        }
        value
    }};
}

#[macro_export]
macro_rules! bulk_delete_records {
    ($req:expr, $depot:expr, $res:expr, $edb:path, $model:ty, $del:expr, $conn:expr) => {{
        use $crate::permission::Accessible;
        use $edb as edb;
        let ids = $crate::parse_ids_from_request($req, "id", "ids").await;
        let cuser = $depot.current_user()?;

        let records = edb::table.filter(edb::id.eq_any(&ids)).get_results::<$model>($conn)?;
        let mut done_ids = vec![];
        let mut denied_ids = vec![];
        let mut nerr_ids = vec![];
        for record in &records {
            if record.permitted(cuser, "delete", $conn)? {
                if $del(record.id, cuser.id, $conn).is_err() {
                    nerr_ids.push(record.id);
                } else {
                    done_ids.push(record.id);
                }
            } else {
                denied_ids.push(record.id);
            }
        }
        let mut info = $crate::StatusInfo::done();
        if !denied_ids.is_empty() {
            info.items.push($crate::ErrorItem::new("_", denied_ids, "Denied access."));
        }
        if !nerr_ids.is_empty() {
            info.items.push($crate::ErrorItem::new("_", nerr_ids, "Unknown error."));
        }
        if !info.items.is_empty() {
            info.brief = "Some records failed to be processed.".into();
        }
        info
    }};
    ($req:expr, $depot:expr, $res:expr, $edb:path, $model:ty, $del:expr, $dep_edb:path, $dep_model:ty, $cfield:tt, $action:expr, $conn:expr) => {{
        use $crate::permission::Accessible;
        use $dep_edb as dep_edb;
        use $edb as edb;
        let ids = $crate::parse_ids_from_request($req, "id", "ids").await;
        let cuser = $depot.current_user()?;

        let records = edb::table.filter(edb::id.eq_any(&ids)).get_results::<$model>($conn);
        if records.is_err() {
            return Err(salvo::http::StatusError::internal_server_error()
                .brief("Unkown db error happened.")
                .into());
        }
        let records = records.unwrap();
        let mut deps: HashMap<i64, $dep_model> = std::collections::HashMap::new();
        let mut done_ids = vec![];
        let mut denied_ids = vec![];
        let mut nerr_ids = vec![];
        for record in &records {
            if deps.get(&record.$cfield).is_none() {
                let dep = dep_edb::table.find(record.$cfield).get_result::<$dep_model>($conn);
                if dep.is_err() {
                    nerr_ids.push(record.id);
                    continue;
                }
                deps.insert(record.$cfield.clone(), dep.unwrap());
            }
            if deps[&record.$cfield].permitted(cuser, $action, $conn)? {
                if $del(record.id, cuser.id, $conn).is_err() {
                    nerr_ids.push(record.id);
                } else {
                    done_ids.push(record.id);
                }
            } else {
                denied_ids.push(record.id);
            }
        }
        let mut info = StatusInfo::done();
        if !denied_ids.is_empty() {
            info.items.push(ErrorItem::new("_", denied_ids, "Denied access."));
        }
        if !nerr_ids.is_empty() {
            info.items.push(ErrorItem::new("_", nerr_ids, "Unknown error."));
        }
        if !info.items.is_empty() {
            info.brief = "Some records failed to be processed.".into();
        }
        info
    }};
}
#[macro_export]
macro_rules! url_filter_joined_options {
    ($($outer_table:expr, $inner_key:expr=>$outer_key:expr, $($url_field:expr=>$o_field:expr),+;)*) => {
        {
            let mut options = vec![];
            $(
                let mut map = std::collections::HashMap::new();
                $(
                    map.insert($url_field.into(), $o_field.into());
                )+
                options.push($crate::db::url_filter::JoinedOption {
                    outer_table: $outer_table.into(),
                    outer_key: $outer_key.into(),
                    inner_key: $inner_key.into(),
                    url_name_map: map,
                });
            )*
            options
        }
    };
}

#[macro_export]
macro_rules! query_pagation_data {
    ($req:expr, $res:expr, $model:ty, $query:expr, $default_sort:expr, $filter_fields:expr, $joined_options:expr, $search_tmpl:expr, $conn:expr) => {
        {
            use diesel::prelude::*;
            use $crate::db::Paginate;

            let offset = $req.query::<i64>("offset").map(|l|{
                if l < 0 {
                    0
                } else {
                    l
                }
            }).unwrap_or(0);
            let limit = $req.query::<i64>("limit").map(|l|{
                if l > 200 || l <= 0 {
                    200
                } else {
                    l
                }
            }).unwrap_or(200);


            let sort = match $req.query::<String>("sort") {
                Some(sort) => {
                    if &*sort == "" || $crate::utils::validator::validate_db_sort(&sort).is_err() {
                        $default_sort.to_string()
                    } else {
                        sort
                    }
                },
                None => $default_sort.to_string(),
            };
            let filter = $req.query::<String>("filter").unwrap_or_default();
            let mut search = $req.query::<String>("search").unwrap_or_default();
            search.retain(|c|!['\'', '\"','>', '=', '<', '!', '?', '/', ';', ':'].contains(&c));
            let search =search.replace("_", "\\_");

            let mut parser = $crate::db::url_filter::Parser::new(filter, $filter_fields, $joined_options);
            let filter = match parser.parse() {
                Ok(filter) => filter,
                Err(msg) => {
                    tracing::info!( error = %msg, "parse url filter error");
                    "".into()
                },
            };
            // tracing::info!( filter = %filter, "url data filter");
            let filter = if search.is_empty() || $search_tmpl.is_empty() {
                filter
            } else {
                let hb = handlebars::Handlebars::new();
                let mut data = std::collections::HashMap::new();
                data.insert("data", &search);
                match hb.render_template($search_tmpl, &data) {
                    Ok(search) => if !filter.is_empty() {
                        format!("({}) and ({})", &filter, &search)
                    } else {
                        search
                    },
                    Err(e) => {
                        tracing::error!(error = ?e, tmpl = %$search_tmpl, data = %search, "search template error");
                        filter
                    }
                }
            };

            let (records, total) = if !filter.is_empty() {
                let query = $query.filter(::diesel::dsl::sql::<::diesel::sql_types::Bool>(&format!("({})", filter))).order(::diesel::dsl::sql::<::diesel::sql_types::Text>(&sort)).paginate(offset).limit(limit);
                //  print_query!(&query);
                query.load_and_total::<$model>($conn)?
            } else {
                let query = $query.order(::diesel::dsl::sql::<::diesel::sql_types::Text>(&sort)).paginate(offset).limit(limit);
                // print_query!(&query);
                query.load_and_total::<$model>($conn)?
            };
            $crate::data::PagedData{
                records,
                limit,
                offset,
                total,
                sort: Some(sort.to_string()),
            }
        }
    };
}

#[macro_export]
macro_rules! join_path {
    ($($part:expr),+) => {
        {
            let mut p = std::path::PathBuf::new();
            $(
                p.push($part);
            )*
            path_slash::PathBufExt::to_slash_lossy(&p).to_string()
        }
    }
}

#[macro_export]
macro_rules! create_bulk_action_result_data {
    ($(($ids:expr, $name:expr, $brief:expr, $detail:expr)),+) => {
        {
            let mut result = $crate::StatusInfo::done{done_ids: $done_ids, errors: vec![]};
            $(
                if !$ids.is_empty() {
                    result.brief = "Some records failed to be processed.".into();
                    result.item($ErrorItem{
                        record_ids: $ids,
                        name: $name.into(),
                        brief: $brief.into(),
                        detail: $detail.into(),
                    });
                }
            )+
            result
        }
    };
}

#[macro_export]
macro_rules! check_ident_name_preserved {
    ($name:expr) => {
        if $crate::is_ident_name_preserved($name) {
            return Err(salvo::http::StatusError::conflict()
                .brief("Oops! That username is already taken. Try adding numbers, more characters, or choosing a different one.")
                .into());
        }
    };
}

#[macro_export]
macro_rules! check_ident_name_other_taken {
    ($user_id:expr, $ident_name:expr, $conn:expr) => {{
        match $crate::utils::validator::is_ident_name_other_taken($user_id, $ident_name, $conn) {
            Ok(true) => {
                return Err(salvo::http::StatusError::conflict()
                    .brief("Oops! That username is already taken. Try adding numbers, more characters, or choosing a different one.")
                    .into());
            }
            Err(_) => {
                return Err(salvo::http::StatusError::internal_server_error()
                    .brief("DB error when check username conflict")
                    .into());
            }
            _ => {}
        }
    }};
}
#[macro_export]
macro_rules! check_email_other_taken {
    ($user_id:expr, $email:expr, $conn:expr) => {{
        match $crate::utils::validator::is_email_other_taken($user_id, $email, $conn) {
            Ok(true) => {
                return Err(salvo::http::StatusError::conflict()
                    .brief("This email is already taken, please try another.")
                    .into());
            }
            Err(_) => {
                return Err(salvo::http::StatusError::internal_server_error()
                    .brief("DB error when check email conflict")
                    .into());
            }
            _ => {}
        }
    }};
}

#[macro_export]
macro_rules! diesel_exists {
    ($query:expr, $conn:expr) => {{
        // tracing::info!( sql = %debug_query!(&$query), "diesel_exists");
        diesel::select(diesel::dsl::exists($query)).get_result::<bool>($conn)?
    }};
    ($query:expr, $default:expr, $conn:expr) => {{
        // tracing::info!( sql = debug_query!(&$query), "diesel_exists");
        diesel::select(diesel::dsl::exists($query)).get_result::<bool>($conn).unwrap_or($default)
    }};
}
#[macro_export]
macro_rules! diesel_exists_result {
    ($query:expr, $conn:expr) => {{
        // tracing::info!( sql = %debug_query!(&$query), "diesel_exists");
        diesel::select(diesel::dsl::exists($query)).get_result::<bool>($conn)
    }};
}

#[macro_export]
macro_rules! print_query {
    ($query:expr) => {
        println!("{}", diesel::debug_query::<diesel::pg::Pg, _>($query));
    };
}

#[macro_export]
macro_rules! debug_query {
    ($query:expr) => {{ format!("{}", diesel::debug_query::<diesel::pg::Pg, _>($query)) }};
}
