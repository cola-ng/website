use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::Engine;
use chrono::{Duration as ChronoDuration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| "failed to hash password".to_string())
        .map(|h| h.to_string())
}

pub async fn verify_password(password: &str, password_hash: &str) -> AppResult<()> {
    if user.deactivated_at.is_some() {
        return Err(StatusError::unauthorized()
            .brief("the user has been deactivated")
            .into());
    }
    let hash = user::get_password_hash(user.id)
        .map_err(|_| StatusError::unauthorized().brief("wrong username or password"))?;
    if hash.is_empty() {
        return Err(StatusError::unauthorized()
            .brief("the user has been deactivated")
            .into());
    }
    
    let parsed =
        PasswordHash::new(password_hash).map_err(|_| StatusError::unauthorized().brief("invalid password hash"))?;
     let hash_matches = Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok();

     if !hash_matches {
        Err(StatusError::unauthorized()
            .brief("wrong username or password")
            .into())
    } else {
        Ok(())
    }
}

pub fn issue_access_token(
    user_id: i64,
    jwt_secret: &str,
    ttl_seconds: u64,
) -> Result<String, String> {
    let now = Utc::now();
    let exp = now + ChronoDuration::seconds(ttl_seconds as i64);
    let claims = AccessClaims {
        sub: user_id.to_string(),
        iat: now.timestamp() as usize,
        exp: exp.timestamp() as usize,
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

pub fn hash_desktop_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    hex::encode(hasher.finalize())
}
