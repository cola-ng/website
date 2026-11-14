use argon2::Argon2;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use base64::Engine;
use chrono::{Duration as ChronoDuration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use salvo::http::StatusError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::LazyLock;
use tokio::sync::Semaphore;

use crate::models::*;
use crate::{AppResult, user};

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub version: i64,
}

static PASSWORD_WORK: LazyLock<std::sync::Arc<Semaphore>> =
    LazyLock::new(|| std::sync::Arc::new(Semaphore::new(4)));

pub async fn hash_password(password: &str) -> Result<String, String> {
    if !(8..=1024).contains(&password.len()) {
        return Err("password must contain 8 to 1024 bytes".into());
    }
    let permit = PASSWORD_WORK
        .clone()
        .try_acquire_owned()
        .map_err(|_| "password service busy; try again".to_string())?;
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| "failed to hash password".to_string())
            .map(|h| h.to_string())
    })
    .await
    .map_err(|_| "password worker failed".to_string())?
}

pub async fn verify_hash(hash: String, password: String) -> AppResult<()> {
    if password.len() > 1024 {
        return Err(StatusError::unauthorized()
            .brief("invalid credentials")
            .into());
    }
    let permit = PASSWORD_WORK
        .clone()
        .try_acquire_owned()
        .map_err(|_| StatusError::too_many_requests().brief("password service busy; try again"))?;
    let matches = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        PasswordHash::new(&hash).ok().is_some_and(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
    })
    .await
    .map_err(|_| StatusError::service_unavailable().brief("password worker failed"))?;
    if matches {
        Ok(())
    } else {
        Err(StatusError::unauthorized()
            .brief("invalid credentials")
            .into())
    }
}

pub async fn verify_password(user: &User, password: &str) -> AppResult<()> {
    if user.disabled_at.is_some() || user.locked_at.is_some() {
        return Err(StatusError::unauthorized()
            .brief("the user has been deactivated")
            .into());
    }

    let password_hash = user::get_password_hash(user.id).await?;
    if password_hash.is_empty() {
        return Err(StatusError::unauthorized()
            .brief("the user has been deactivated")
            .into());
    }

    verify_hash(password_hash, password.to_owned()).await
}

pub fn issue_access_token(
    user_id: i64,
    version: i64,
    jwt_secret: &str,
    ttl_seconds: u64,
) -> Result<String, String> {
    let now = Utc::now();
    let exp = now + ChronoDuration::seconds(ttl_seconds as i64);
    let claims = AccessClaims {
        sub: user_id.to_string(),
        iat: now.timestamp() as usize,
        exp: exp.timestamp() as usize,
        version,
    };
    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|_| "failed to issue token".to_string())
}

pub fn decode_access_token(token: &str, jwt_secret: &str) -> Result<AccessClaims, String> {
    let data = jsonwebtoken::decode::<AccessClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| "invalid token".to_string())?;
    Ok(data.claims)
}

pub fn random_code() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    hex::encode(hasher.finalize())
}

// Keys use socket peer addresses only: untrusted forwarded headers cannot bypass limits.
pub fn limit_password_attempt(peer: &str, account: &str) -> AppResult<()> {
    use std::{
        collections::HashMap,
        sync::Mutex,
        time::{Duration, Instant},
    };
    static ATTEMPTS: LazyLock<Mutex<HashMap<String, (Instant, u32)>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    let now = Instant::now();
    let mut attempts = ATTEMPTS.lock().unwrap_or_else(|e| e.into_inner());
    attempts.retain(|_, (since, _)| now.duration_since(*since) < Duration::from_secs(60));
    for (key, limit) in [
        (format!("ip:{peer}"), 120),
        (format!("account:{}", hash_code(account)), 10),
    ] {
        if attempts.len() >= 10_000 && !attempts.contains_key(&key) {
            return Err(StatusError::too_many_requests()
                .brief("authentication busy; try later")
                .into());
        }
        let entry = attempts.entry(key).or_insert((now, 0));
        entry.1 += 1;
        if entry.1 > limit {
            return Err(StatusError::too_many_requests()
                .brief("too many authentication attempts; try later")
                .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn password_attempt_budget_is_per_account() {
        let account = super::random_code();
        for _ in 0..10 {
            assert!(super::limit_password_attempt("test-peer", &account).is_ok());
        }
        assert!(super::limit_password_attempt("test-peer", &account).is_err());
        assert!(super::limit_password_attempt("test-peer", &super::random_code()).is_ok());
    }
}
