use chrono::{DateTime, Utc};
use diesel::dsl;
use diesel::prelude::*;
use salvo::http::StatusError;
use serde::Deserialize;

// use crate::config::JwtConfig;
use crate::db::conn;
use crate::db::schema::*;
use crate::models::{NewPassword, NewUser, User};
use crate::{AppResult, diesel_exists};

#[derive(Debug, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
}

// pub fn validate_jwt_token(config: &JwtConfig, token: &str) -> AppResult<JwtClaims> {
//     let verifier = init_jwt_verifier(config)?;
//     let validator = init_jwt_validator(config)?;
//     jsonwebtoken::decode::<JwtClaims>(token, &verifier, &validator)
//         .map(|decoded| (decoded.header, decoded.claims))
//         .inspect(|(head, claim)| debug!(?head, ?claim, "jwt token decoded"))
//         .map_err(|e| {
//             StatusError::not_found()
//                 .brief(format!("invalid jwt token: {e}"))
//                 .into()
//         })
//         .map(|(_, claims)| claims)
// }

// fn init_jwt_verifier(config: &JwtConfig) -> AppResult<DecodingKey> {
//     let secret = &config.secret;
//     let format = config.format.as_str();

//     Ok(match format {
//         "HMAC" => DecodingKey::from_secret(secret.as_bytes()),

//         "HMACB64" => DecodingKey::from_base64_secret(secret.as_str())
//             .map_err(|_e| AppError::public("jwt secret is not valid base64"))?,

//         "ECDSA" => DecodingKey::from_ec_pem(secret.as_bytes())
//             .map_err(|_e| AppError::public("jwt key is not valid PEM"))?,

//         _ => return Err(AppError::public("jwt secret format is not supported")),
//     })
// }

// fn init_jwt_validator(config: &JwtConfig) -> AppResult<Validation> {
//     let alg = config.algorithm.as_str();
//     let alg = Algorithm::from_str(alg)
//         .map_err(|_e| AppError::public("jwt algorithm is not recognized or configured"))?;

//     let mut validator = Validation::new(alg);
//     let mut required_spec_claims: Vec<_> = ["sub"].into();

//     validator.validate_exp = config.validate_exp;
//     if config.require_exp {
//         required_spec_claims.push("exp");
//     }

//     validator.validate_nbf = config.validate_nbf;
//     if config.require_nbf {
//         required_spec_claims.push("nbf");
//     }

//     if !config.audience.is_empty() {
//         required_spec_claims.push("aud");
//         validator.set_audience(&config.audience);
//     }

//     if !config.issuer.is_empty() {
//         required_spec_claims.push("iss");
//         validator.set_issuer(&config.issuer);
//     }

//     if cfg!(debug_assertions) && !config.validate_signature {
//         warn!("jwt signature validation is disabled!");
//         validator.insecure_disable_signature_validation();
//     }

//     validator.set_required_spec_claims(&required_spec_claims);
//     debug!(?validator, "jwt configured");

//     Ok(validator)
// }

pub fn is_username_available(name: &str) -> AppResult<bool> {
    let available = !user_exists(name)?;
    Ok(available)
}

pub fn create_user(name: impl Into<String>, password: Option<&str>) -> AppResult<User> {
    let new_user = NewUser {
        name: name.into(),
        email: None,
        phone: None,
        display_name: None,
        inviter_id: None,
        created_by: None,
        updated_by: None,
    };
    let user = diesel::insert_into(base_users::table)
        .values(&new_user)
        .on_conflict(base_users::id)
        .do_update()
        .set(&new_user)
        .get_result::<User>(&mut conn()?)?;
    if let Some(password) = password {
        let hash = crate::auth::hash_password(password)?;
        diesel::insert_into(base_passwords::table)
            .values(NewPassword {
                user_id: user.id,
                hash,
                created_at: Utc::now(),
            })
            .execute(&mut conn()?)?;
    }
    Ok(user)
}

// pub fn valid_refresh_token(user_id: i64, device_id: i64, token: &str) -> AppResult<()> {
//     let Ok(expires_at) = user_refresh_tokens::table
//         .filter(user_refresh_tokens::user_id.eq(user_id))
//         .filter(user_refresh_tokens::device_id.eq(device_id))
//         .filter(user_refresh_tokens::token.eq(token))
//         .select(user_refresh_tokens::expires_at)
//         .first::<DateTime<Utc>>(&mut conn()?)
//     else {
//         return Err(StatusError::unauthorized()
//             .brief("invalid refresh token")
//             .into());
//     };
//     if expires_at < Utc::now() {
//         return Err(StatusError::unauthorized()
//             .brief("refresh token expired")
//             .into());
//     }
//     Ok(())
// }

// pub fn make_user_admin(user_id: i64) -> AppResult<()> {
//     let user_id = user_id.to_owned();
//     diesel::update(users::table.filter(users::id.eq(&user_id)))
//         .set(users::is_admin.eq(true))
//         .execute(&mut conn()?)?;
//     Ok(())
// }

// pub async fn deactivate_account(user_id: i64) -> AppResult<()> {
//     diesel::update(users::table.find(user_id))
//         .set(users::deactivated_at.eq(Utc::now()))
//         .execute(&mut conn()?)?;
//     Ok(())
// }

// pub fn is_admin(user_id: i64) -> AppResult<bool> {
//     users::table
//         .filter(users::id.eq(user_id))
//         .select(users::is_admin)
//         .first::<bool>(&mut conn()?)
//         .map_err(Into::into)
// }

/// Check if a user has an account on this homeserver.
pub fn user_exists(username: &str) -> AppResult<bool> {
    let query = base_users::table.filter(base_users::name.eq(username));
    diesel_exists!(query, &mut conn()?).map_err(Into::into)
}

pub fn get_user(user_id: i64) -> AppResult<User> {
    base_users::table
        .find(user_id)
        .first::<User>(&mut conn()?)
        .map_err(Into::into)
}

/// Returns the number of users registered on this server.
pub fn count() -> AppResult<u64> {
    let count = base_passwords::table
        .select(dsl::count(base_passwords::user_id).aggregate_distinct())
        .first::<i64>(&mut conn()?)?;
    Ok(count as u64)
}

/// Returns the display_name of a user on this homeserver.
pub fn display_name(user_id: i64) -> AppResult<Option<String>> {
    base_users::table
        .filter(base_users::id.eq(user_id))
        .select(base_users::display_name)
        .first::<Option<String>>(&mut conn()?)
        .optional()
        .map(Option::flatten)
        .map_err(Into::into)
}
pub fn set_display_name(user_id: i64, display_name: &str) -> AppResult<()> {
    diesel::update(base_users::table.filter(base_users::id.eq(user_id)))
        .set(base_users::display_name.eq(display_name))
        .execute(&mut conn()?)
        .map(|_| ())
        .map_err(Into::into)
}
pub fn remove_display_name(user_id: i64) -> AppResult<()> {
    diesel::update(base_users::table.filter(base_users::id.eq(user_id)))
        .set(base_users::display_name.eq::<Option<String>>(None))
        .execute(&mut conn()?)
        .map(|_| ())
        .map_err(Into::into)
}

/// Get the avatar_url of a user.
pub fn avatar_url(user_id: i64) -> AppResult<Option<String>> {
    base_users::table
        .filter(base_users::id.eq(user_id))
        .select(base_users::avatar)
        .first::<Option<String>>(&mut conn()?)
        .optional()
        .map(Option::flatten)
        .map_err(Into::into)
}
pub fn set_avatar_url(user_id: i64, avatar_url: &str) -> AppResult<()> {
    diesel::update(base_users::table.filter(base_users::id.eq(user_id)))
        .set(base_users::avatar.eq(avatar_url))
        .execute(&mut conn()?)?;
    Ok(())
}

// pub fn is_deactivated(user_id: i64) -> AppResult<bool> {
//     let deactivated_at = users::table
//         .filter(users::id.eq(user_id))
//         .select(users::deactivated_at)
//         .first::<Option<DateTime<Utc>>>(&mut conn()?)
//         .optional()?
//         .flatten();
//     Ok(deactivated_at.is_some())
// }

// pub fn all_device_ids(user_id: i64) -> AppResult<Vec<i64>> {
//     user_devices::table
//         .filter(user_devices::user_id.eq(user_id))
//         .select(user_devices::id)
//         .load::<i64>(&mut conn()?)
//         .map_err(Into::into)
// }

// pub fn delete_access_tokens(user_id: i64) -> AppResult<()> {
//     diesel::delete(user_access_tokens::table.filter(user_access_tokens::user_id.eq(user_id)))
//         .execute(&mut conn()?)?;
//     Ok(())
// }

// pub fn delete_refresh_tokens(user_id: i64) -> AppResult<()> {
//     diesel::delete(user_refresh_tokens::table.filter(user_refresh_tokens::user_id.eq(user_id)))
//         .execute(&mut conn()?)?;
//     Ok(())
// }

// pub fn remove_all_devices(user_id: i64) -> AppResult<()> {
//     delete_access_tokens(user_id)?;
//     delete_refresh_tokens(user_id)?;
//     Ok(())
// }

// pub fn deactivate(user_id: i64) -> AppResult<()> {
//     diesel::update(users::table.find(user_id))
//         .set((users::deactivated_at.eq(Utc::now()),))
//         .execute(&mut conn()?)?;

//     Ok(())
// }

/// Set locked status for a user
pub fn set_locked(user_id: i64, locked: bool, locker_id: Option<i64>) -> AppResult<()> {
    if locked {
        diesel::update(base_users::table.find(user_id))
            .set((
                base_users::locked_at.eq(Some(Utc::now())),
                base_users::locked_by.eq(locker_id.map(|u| u.to_owned())),
            ))
            .execute(&mut conn()?)?;
    } else {
        diesel::update(base_users::table.find(user_id))
            .set((
                base_users::locked_at.eq::<Option<DateTime<Utc>>>(None),
                base_users::locked_by.eq::<Option<i64>>(None),
            ))
            .execute(&mut conn()?)?;
    }
    Ok(())
}

/// List users with pagination and filtering
#[derive(Debug, Clone, Default)]
pub struct ListUsersFilter {
    pub from: Option<i64>,
    pub limit: Option<i64>,
    pub name: Option<String>,
    pub guests: Option<bool>,
    pub deactivated: Option<bool>,
    pub admins: Option<bool>,
    pub user_types: Option<Vec<String>>,
    pub order_by: Option<String>,
    pub dir: Option<String>,
}

pub fn list_users(filter: &ListUsersFilter) -> AppResult<Vec<User>> {
    let mut query = base_users::table.into_boxed();

    // // Filter by deactivated
    // if let Some(deactivated) = filter.deactivated {
    //     if deactivated {
    //         query = query.filter(users::deactivated_at.is_not_null());
    //     } else {
    //         query = query.filter(users::deactivated_at.is_null());
    //     }
    // }

    // Get total count before pagination
    let _total: i64 = base_users::table.count().get_result(&mut conn()?)?;

    // Apply ordering
    let dir_asc = filter.dir.as_ref().map(|d| d == "f").unwrap_or(true);
    query = match filter.order_by.as_deref() {
        Some("name") => {
            if dir_asc {
                query.order(base_users::name.asc())
            } else {
                query.order(base_users::name.desc())
            }
        }
        Some("disabled") => {
            if dir_asc {
                query.order(base_users::disabled_at.asc())
            } else {
                query.order(base_users::disabled_at.desc())
            }
        }
        _ => {
            if dir_asc {
                query.order(base_users::created_at.asc())
            } else {
                query.order(base_users::created_at.desc())
            }
        }
    };

    // Apply pagination
    if let Some(from) = filter.from {
        query = query.offset(from);
    }

    let limit = filter.limit.unwrap_or(100).min(1000);
    query = query.limit(limit);

    let users = query.load::<User>(&mut conn()?)?;

    Ok(users)
}

pub async fn get_password_hash(user_id: i64) -> AppResult<String> {
    base_passwords::table
        .filter(base_passwords::user_id.eq(user_id))
        .order_by(base_passwords::id.desc())
        .select(base_passwords::hash)
        .first::<String>(&mut conn()?)
        .map_err(Into::into)
}

pub fn set_password(user_id: i64, password: &str) -> AppResult<()> {
    if let Ok(hash) = crate::auth::hash_password(password) {
        diesel::insert_into(base_passwords::table)
            .values(NewPassword {
                user_id: user_id.to_owned(),
                hash,
                created_at: Utc::now(),
            })
            .execute(&mut conn()?)?;
        Ok(())
    } else {
        Err(StatusError::internal_server_error()
            .brief("password does not meet the requirements")
            .into())
    }
}
