//! Short-lived physical pairing codes. Permanent credentials stay on the device.
use chrono::{DateTime, Utc};
use diesel::sql_types::{BigInt, Bool, Integer, Nullable, Text, Timestamptz, Uuid as SqlUuid};
use horme::prelude::*;
use serde_json::{Value, json};
use std::result::Result;
use uuid::Uuid;

use super::store::Device;
use crate::db::with_conn;

#[derive(QueryableByName)]
struct Pending {
    #[diesel(sql_type = Text)]
    code: String,
    #[diesel(sql_type = Text)]
    token_hash: String,
    #[diesel(sql_type = Text)]
    board: String,
    #[diesel(sql_type = Timestamptz)]
    expires_at: DateTime<Utc>,
    #[diesel(sql_type = Nullable<SqlUuid>)]
    device_id: Option<Uuid>,
}

pub async fn limit(key: String, maximum: i32) -> Result<bool, String> {
    #[derive(QueryableByName)]
    struct Count {
        #[diesel(sql_type=Integer)]
        attempts: i32,
    }
    let row=with_conn!(move |conn| horme::transaction(conn, async |conn| {
        diesel::sql_query("DELETE FROM amigo_pairing_limits WHERE window_at<NOW()-INTERVAL '1 hour'").execute(conn).await?;
        diesel::sql_query("INSERT INTO amigo_pairing_limits(key) VALUES($1) ON CONFLICT(key) DO UPDATE SET attempts=CASE WHEN amigo_pairing_limits.window_at<NOW()-INTERVAL '1 minute' THEN 1 ELSE amigo_pairing_limits.attempts+1 END,window_at=CASE WHEN amigo_pairing_limits.window_at<NOW()-INTERVAL '1 minute' THEN NOW() ELSE amigo_pairing_limits.window_at END RETURNING attempts")
            .bind::<Text,_>(key).get_result::<Count>(conn).await.map_err(Into::into)
    })).await?;
    Ok(row.attempts <= maximum)
}

/// Existing credentials never generate a rebind code. Rebinding requires the
/// device to generate a fresh secret after an explicit physical action.
async fn linked(hash: String) -> Result<Option<Value>, String> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type=SqlUuid)]
        id: Uuid,
        #[diesel(sql_type=BigInt)]
        user_id: i64,
        #[diesel(sql_type=Bool)]
        revoked: bool,
        #[diesel(sql_type=Bool)]
        disabled: bool,
    }
    let row=with_conn!(move |conn| diesel::sql_query("SELECT d.id,d.user_id,d.revoked_at IS NOT NULL AS revoked,(u.disabled_at IS NOT NULL OR u.locked_at IS NOT NULL) AS disabled FROM amigo_devices d JOIN base_users u ON u.id=d.user_id WHERE d.token_hash=$1").bind::<Text,_>(hash).get_result::<Row>(conn).await.optional()).await?;
    let Some(row) = row else { return Ok(None) };
    if row.revoked {
        return Ok(Some(json!({"state":"revoked"})));
    }
    if row.disabled {
        return Ok(Some(json!({"state":"blocked"})));
    }
    let ready = super::tutor::ProductionBackend::for_user(row.user_id)
        .await
        .is_ok();
    Ok(Some(
        json!({"state":"paired","device_id":row.id,"ai_ready":ready}),
    ))
}

pub async fn start(token: &str, board: String) -> Result<Value, String> {
    let hash = crate::auth::hash_code(token);
    if let Some(state) = linked(hash.clone()).await? {
        return Ok(state);
    }
    let row=with_conn!(move |conn| horme::transaction(conn, async |conn| {
        // Serialize code allocation across backend processes; claims remain row locked.
        diesel::sql_query("SELECT pg_advisory_xact_lock(718632169)").execute(conn).await?;
        diesel::sql_query("DELETE FROM amigo_pairings WHERE expires_at<=NOW()").execute(conn).await?;
        if let Some(row)=diesel::sql_query("SELECT code::text,token_hash,board,expires_at,device_id FROM amigo_pairings WHERE token_hash=$1").bind::<Text,_>(&hash).get_result::<Pending>(conn).await.optional()? {return Ok(row);}
        loop {
            let code=format!("{:06}",rand::random_range(0..1_000_000_u32));
            if let Some(row)=diesel::sql_query("INSERT INTO amigo_pairings(token_hash,code,board) VALUES($1,$2,$3) ON CONFLICT(code) DO NOTHING RETURNING code::text,token_hash,board,expires_at,device_id")
                .bind::<Text,_>(&hash).bind::<Text,_>(code).bind::<Text,_>(&board).get_result::<Pending>(conn).await.optional()? {return Ok(row);}
        }
    })).await?;
    Ok(
        json!({"state":"waiting","code":row.code,"expires_in":(row.expires_at-Utc::now()).num_seconds().max(0)}),
    )
}

pub async fn status(token: &str) -> Result<Value, String> {
    let hash = crate::auth::hash_code(token);
    if let Some(state) = linked(hash.clone()).await? {
        return Ok(state);
    }
    let row=with_conn!(move |conn| diesel::sql_query("SELECT code::text,token_hash,board,expires_at,device_id FROM amigo_pairings WHERE token_hash=$1 AND expires_at>NOW()")
        .bind::<Text,_>(hash).get_result::<Pending>(conn).await.optional()).await?;
    Ok(match row {
        Some(row) => {
            json!({"state":"waiting","expires_in":(row.expires_at-Utc::now()).num_seconds().max(0)})
        }
        None => json!({"state":"expired"}),
    })
}

pub async fn claim(user: i64, code: String, name: String) -> Result<Option<Device>, String> {
    with_conn!(move |conn| horme::transaction(conn, async |conn| {
        let row=diesel::sql_query("SELECT code::text,token_hash,board,expires_at,device_id FROM amigo_pairings WHERE code=$1 AND expires_at>NOW() FOR UPDATE")
            .bind::<Text,_>(&code).get_result::<Pending>(conn).await.optional()?;
        let Some(row)=row else{return Ok(None)};
        if let Some(id)=row.device_id {
            // A lost claim response can be retried by the same owner only.
            return diesel::sql_query("SELECT id,user_id,name,created_at,revoked_at,last_seen_at FROM amigo_devices WHERE id=$1 AND user_id=$2 AND revoked_at IS NULL")
                .bind::<SqlUuid,_>(id).bind::<BigInt,_>(user).get_result::<Device>(conn).await.optional().map_err(Into::into);
        }
        let id=Uuid::new_v4();
        let device=diesel::sql_query("INSERT INTO amigo_devices(id,user_id,name,token_hash,board) VALUES($1,$2,$3,$4,$5) RETURNING id,user_id,name,created_at,revoked_at,last_seen_at")
            .bind::<SqlUuid,_>(id).bind::<BigInt,_>(user).bind::<Text,_>(name).bind::<Text,_>(row.token_hash).bind::<Text,_>(row.board).get_result::<Device>(conn).await?;
        diesel::sql_query("UPDATE amigo_pairings SET device_id=$2 WHERE code=$1 AND device_id IS NULL")
            .bind::<Text,_>(code).bind::<SqlUuid,_>(id).execute(conn).await?;
        Ok(Some(device))
    })).await
}
