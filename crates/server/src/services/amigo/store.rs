use chrono::{DateTime, Utc};
use diesel::sql_types::{BigInt, Jsonb, Nullable, Text, Timestamptz, Uuid as SqlUuid};
use horme::diesel_async::AsyncPgConnection;
use horme::prelude::*;
use serde::Serialize;
use std::result::Result;
use uuid::Uuid;

use super::model::{Context, Profile};
use crate::db::schema::{learn_issue_words, learn_observations, learn_person_memories};
use crate::db::with_conn;
use crate::models::companion::PersonMemory;

#[derive(Clone, QueryableByName, Serialize)]
pub struct Device {
    #[diesel(sql_type = SqlUuid)]
    pub id: Uuid,
    #[diesel(sql_type = BigInt)]
    pub user_id: i64,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub revoked_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(QueryableByName)]
struct StateRow {
    #[diesel(sql_type = Jsonb)]
    state: serde_json::Value,
}

pub async fn provision(user_id: i64, name: String) -> Result<(Device, String), String> {
    let token = format!("am_{}", crate::auth::random_code());
    let hash = crate::auth::hash_code(&token);
    let device = with_conn!(move |conn| diesel::sql_query(
        "INSERT INTO amigo_devices(id,user_id,name,token_hash) VALUES($1,$2,$3,$4) RETURNING id,user_id,name,created_at,revoked_at,last_seen_at")
        .bind::<SqlUuid,_>(Uuid::new_v4()).bind::<BigInt,_>(user_id)
        .bind::<Text,_>(name).bind::<Text,_>(hash).get_result::<Device>(conn).await).await?;
    Ok((device, token))
}

pub async fn devices(user_id: i64) -> Result<Vec<Device>, String> {
    with_conn!(move |conn| diesel::sql_query("SELECT id,user_id,name,created_at,revoked_at,last_seen_at FROM amigo_devices WHERE user_id=$1 ORDER BY created_at DESC")
        .bind::<BigInt,_>(user_id).load(conn).await).await
}

pub async fn revoke(user_id: i64, device: Uuid) -> Result<bool, String> {
    with_conn!(move |conn| async {
        diesel::sql_query("UPDATE amigo_devices SET revoked_at=NOW(),lease_id=NULL,lease_until=NULL WHERE user_id=$1 AND id=$2")
            .bind::<BigInt,_>(user_id).bind::<SqlUuid,_>(device).execute(conn).await.map(|n| n == 1)
    }).await
}

pub async fn ticket(user: i64, device: Uuid) -> Result<Option<String>, String> {
    let ticket = format!("amws_{}", crate::auth::random_code());
    let hash = crate::auth::hash_code(&ticket);
    let issued = with_conn!(move |conn| horme::transaction(conn, async |conn| {
        diesel::sql_query("DELETE FROM amigo_ws_tickets WHERE expires_at<NOW()").execute(conn).await?;
        diesel::sql_query("INSERT INTO amigo_ws_tickets(token_hash,device_id) SELECT $1,d.id FROM amigo_devices d JOIN base_users u ON u.id=d.user_id WHERE d.id=$2 AND d.user_id=$3 AND d.revoked_at IS NULL AND u.disabled_at IS NULL")
            .bind::<Text,_>(hash).bind::<SqlUuid,_>(device).bind::<BigInt,_>(user).execute(conn).await.map(|n|n==1)
    })).await?;
    Ok(issued.then_some(ticket))
}

pub async fn consume_ticket(ticket: &str) -> Result<Option<Device>, String> {
    let hash = crate::auth::hash_code(ticket);
    with_conn!(move |conn| diesel::sql_query("WITH consumed AS (DELETE FROM amigo_ws_tickets WHERE token_hash=$1 AND expires_at>NOW() RETURNING device_id) SELECT d.id,d.user_id,d.name,d.created_at,d.revoked_at,d.last_seen_at FROM amigo_devices d JOIN consumed c ON c.device_id=d.id JOIN base_users u ON u.id=d.user_id WHERE d.revoked_at IS NULL AND u.disabled_at IS NULL")
        .bind::<Text,_>(hash).get_result::<Device>(conn).await.optional()).await
}

pub async fn authenticate(token: &str) -> Result<Option<Device>, String> {
    let hash = crate::auth::hash_code(token);
    with_conn!(move |conn| diesel::sql_query("SELECT d.id,d.user_id,d.name,d.created_at,d.revoked_at,d.last_seen_at FROM amigo_devices d JOIN base_users u ON u.id=d.user_id WHERE d.token_hash=$1 AND d.revoked_at IS NULL AND u.disabled_at IS NULL")
        .bind::<Text,_>(hash).get_result(conn).await.optional()).await
}

/// The newest authenticated connection owns the device lease. A reset may leave
/// a half-open TCP socket refreshing its lease until the heartbeat timeout;
/// refusing the new connection would strand the physical device for a minute.
/// Old refreshes/commits/releases are fenced by their original lease UUID.
pub async fn reconnect(device: Uuid, lease: Uuid) -> Result<bool, String> {
    with_conn!(move |conn| async {
        diesel::sql_query("UPDATE amigo_devices SET lease_id=$2,lease_until=NOW()+INTERVAL '20 seconds',last_seen_at=NOW() WHERE id=$1 AND revoked_at IS NULL")
            .bind::<SqlUuid,_>(device).bind::<SqlUuid,_>(lease).execute(conn).await.map(|n| n == 1)
    }).await
}

/// Browser tickets can inspect an idle device but cannot displace hardware.
pub async fn claim(device: Uuid, lease: Uuid) -> Result<bool, String> {
    with_conn!(move |conn| async {
        diesel::sql_query("UPDATE amigo_devices SET lease_id=$2,lease_until=NOW()+INTERVAL '20 seconds',last_seen_at=NOW() WHERE id=$1 AND revoked_at IS NULL AND (lease_until IS NULL OR lease_until<NOW())")
            .bind::<SqlUuid,_>(device).bind::<SqlUuid,_>(lease).execute(conn).await.map(|n|n==1)
    }).await
}

pub async fn refresh(device: Uuid, lease: Uuid) -> Result<bool, String> {
    with_conn!(move |conn| async {
        diesel::sql_query("UPDATE amigo_devices d SET lease_until=NOW()+INTERVAL '20 seconds',last_seen_at=NOW() FROM base_users u WHERE d.id=$1 AND d.lease_id=$2 AND d.lease_until>NOW() AND d.revoked_at IS NULL AND u.id=d.user_id AND u.disabled_at IS NULL")
            .bind::<SqlUuid,_>(device).bind::<SqlUuid,_>(lease).execute(conn).await.map(|n| n == 1)
    }).await
}

pub async fn release(device: Uuid, lease: Uuid) -> Result<(), String> {
    with_conn!(move |conn| async {
        diesel::sql_query(
            "UPDATE amigo_devices SET lease_id=NULL,lease_until=NULL WHERE id=$1 AND lease_id=$2",
        )
        .bind::<SqlUuid, _>(device)
        .bind::<SqlUuid, _>(lease)
        .execute(conn)
        .await
        .map(|_| ())
    })
    .await
}

pub async fn load(user_id: i64) -> Result<Profile, String> {
    with_conn!(move |conn| {
        let row = diesel::sql_query("SELECT state FROM amigo_profiles WHERE user_id=$1")
            .bind::<BigInt, _>(user_id)
            .get_result::<StateRow>(conn)
            .await
            .optional()?;
        let mut profile: Profile = row
            .map(|row| serde_json::from_value(row.state))
            .transpose()
            .map_err(|e| diesel::result::Error::DeserializationError(Box::new(e)))?
            .unwrap_or_default();
        profile.ensure_agents();
        merge_shared(conn, user_id, &mut profile).await?;
        Ok(profile)
    })
    .await
}

pub(super) async fn merge_shared(
    conn: &mut impl GetConnection<Connection = AsyncPgConnection>,
    user_id: i64,
    profile: &mut Profile,
) -> horme::Result<()> {
    // Desktop observations and memories are also useful context for the robot.
    let recent = learn_observations::table
        .filter(learn_observations::user_id.eq(user_id))
        .order(learn_observations::observed_at.desc())
        .limit(5)
        .select((
            learn_observations::source,
            learn_observations::redacted_text,
            learn_observations::topics,
            learn_observations::observed_at,
        ))
        .load::<(String, String, serde_json::Value, DateTime<Utc>)>(conn)
        .await?;
    for (source, text, topics, observed_at) in recent.into_iter().rev() {
        let text = super::model::clean(&text, 2000);
        if !profile.observations.iter().any(|o| o.text == text) {
            profile.observations.push(Context {
                source,
                text,
                topics: serde_json::from_value(topics).unwrap_or_default(),
                observed_at,
            });
        }
    }
    profile.observations.sort_by_key(|o| o.observed_at);
    let shared = learn_person_memories::table
        .filter(learn_person_memories::user_id.eq(user_id))
        .filter(learn_person_memories::is_active.eq(true))
        .order(learn_person_memories::last_seen_at.desc())
        .limit(20)
        .load::<PersonMemory>(conn)
        .await?;
    for memory in shared {
        if !profile.forgotten.contains(&memory.subject)
            && !profile.memories.iter().any(|m| m.subject == memory.subject)
        {
            profile.memories.push(super::model::Memory {
                user_confirmed: false,
                subject: memory.subject,
                summary: super::model::clean(&memory.summary, 240),
                evidence_count: memory.evidence_count.max(0) as u32,
                confidence: memory.confidence,
                updated_at: memory.last_seen_at,
            });
        }
    }
    let issue_words = learn_issue_words::table
        .filter(learn_issue_words::user_id.eq(user_id))
        .order(learn_issue_words::created_at.desc())
        .limit(30)
        .load::<crate::models::learn::IssueWord>(conn)
        .await?;
    for word in issue_words {
        let key = super::model::clean(&word.word, 80).to_lowercase();
        if key.is_empty()
            || profile.forgotten.contains(&key)
            || profile.weaknesses.iter().any(|w| w.key == key)
        {
            continue;
        }
        profile.weaknesses.push(super::model::Weakness {
            user_corrected: false,
            key,
            kind: "vocabulary".into(),
            original: super::model::clean(word.context.as_deref().unwrap_or(&word.word), 240),
            corrected: super::model::clean(&word.word, 240),
            failures: 1,
            successful_recalls: 0,
            interval_days: word.review_interval_days.unwrap_or(0).max(0) as u32,
            next_review_at: word.next_review_at.unwrap_or(word.created_at),
            updated_at: word.created_at,
        });
    }
    if profile.observations.len() > 20 {
        profile
            .observations
            .drain(..profile.observations.len() - 20);
    }
    if profile.memories.len() > 100 {
        profile.memories.truncate(100);
    }
    if profile.weaknesses.len() > 200 {
        profile.weaknesses.truncate(200);
    }
    Ok(())
}

pub async fn update<F>(user_id: i64, mutate: F) -> Result<Profile, String>
where
    F: FnOnce(&mut Profile) + Send + 'static,
{
    with_conn!(move |conn| horme::transaction(conn, async |conn| {
        mutate_locked(conn, user_id, mutate).await
    }))
    .await
}

async fn mutate_locked<F>(
    conn: &mut AsyncPgConnection,
    user_id: i64,
    mutate: F,
) -> horme::Result<Profile>
where
    F: FnOnce(&mut Profile),
{
    diesel::sql_query(
        "INSERT INTO amigo_profiles(user_id,state) VALUES($1,$2) ON CONFLICT DO NOTHING",
    )
    .bind::<BigInt, _>(user_id)
    .bind::<Jsonb, _>(serde_json::to_value(Profile::default()).unwrap())
    .execute(conn)
    .await?;
    let row = diesel::sql_query("SELECT state FROM amigo_profiles WHERE user_id=$1 FOR UPDATE")
        .bind::<BigInt, _>(user_id)
        .get_result::<StateRow>(conn)
        .await?;
    let mut profile: Profile = serde_json::from_value(row.state)
        .map_err(|e| diesel::result::Error::DeserializationError(Box::new(e)))?;
    profile.ensure_agents();
    merge_shared(conn, user_id, &mut profile).await?;
    mutate(&mut profile);
    diesel::sql_query("UPDATE amigo_profiles SET state=$2,updated_at=NOW() WHERE user_id=$1")
        .bind::<BigInt, _>(user_id)
        .bind::<Jsonb, _>(serde_json::to_value(&profile).unwrap())
        .execute(conn)
        .await?;
    Ok(profile)
}

// Existing devices keep the account profile until their first explicit per-device save.
// This preserves ongoing learning/conversation state during an in-place upgrade.
pub(super) async fn mutate_scoped_locked<F>(
    conn: &mut AsyncPgConnection,
    device: &Device,
    mutate: F,
) -> horme::Result<Profile>
where
    F: FnOnce(&mut Profile),
{
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type=diesel::sql_types::Bool)]
        configured: bool,
    }
    let row=diesel::sql_query("SELECT configuration IS NOT NULL AS configured FROM amigo_devices WHERE id=$1 AND user_id=$2 AND revoked_at IS NULL FOR UPDATE")
        .bind::<SqlUuid,_>(device.id).bind::<BigInt,_>(device.user_id).get_result::<Row>(conn).await?;
    if row.configured {
        mutate_device_locked(conn, device.user_id, device.id, mutate).await
    } else {
        mutate_locked(conn, device.user_id, mutate).await
    }
}

pub(super) async fn mutate_device_locked<F>(
    conn: &mut AsyncPgConnection,
    user_id: i64,
    device_id: Uuid,
    mutate: F,
) -> horme::Result<Profile>
where
    F: FnOnce(&mut Profile),
{
    diesel::sql_query(
        "INSERT INTO amigo_device_profiles(device_id,state) VALUES($1,COALESCE((SELECT state FROM amigo_profiles WHERE user_id=$3),$2)) ON CONFLICT DO NOTHING",
    )
    .bind::<SqlUuid, _>(device_id)
    .bind::<Jsonb, _>(serde_json::to_value(Profile::default()).unwrap())
    .bind::<BigInt, _>(user_id)
    .execute(conn)
    .await?;
    let row =
        diesel::sql_query("SELECT state FROM amigo_device_profiles WHERE device_id=$1 FOR UPDATE")
            .bind::<SqlUuid, _>(device_id)
            .get_result::<StateRow>(conn)
            .await?;
    let mut profile: Profile = serde_json::from_value(row.state)
        .map_err(|e| diesel::result::Error::DeserializationError(Box::new(e)))?;
    if let Some(shared) = diesel::sql_query("SELECT state FROM amigo_profiles WHERE user_id=$1")
        .bind::<BigInt, _>(user_id)
        .get_result::<StateRow>(conn)
        .await
        .optional()?
    {
        let account: Profile = serde_json::from_value(shared.state)
            .map_err(|e| diesel::result::Error::DeserializationError(Box::new(e)))?;
        profile.agents = account.agents;
    }
    profile.ensure_agents();
    merge_shared(conn, user_id, &mut profile).await?;
    mutate(&mut profile);
    diesel::sql_query(
        "UPDATE amigo_device_profiles SET state=$2,updated_at=NOW() WHERE device_id=$1",
    )
    .bind::<SqlUuid, _>(device_id)
    .bind::<Jsonb, _>(serde_json::to_value(&profile).unwrap())
    .execute(conn)
    .await?;
    Ok(profile)
}

pub async fn record<F>(
    device: Device,
    lease: Uuid,
    event: Uuid,
    kind: &'static str,
    content: serde_json::Value,
    mutate: F,
) -> Result<Profile, String>
where
    F: FnOnce(&mut Profile) + Send + 'static,
{
    with_conn!(move |conn| horme::transaction(conn, async |conn| {
        // Lock the device as well: revocation and commits have an explicit order.
        let valid = diesel::sql_query("SELECT id,user_id,name,created_at,revoked_at,last_seen_at FROM amigo_devices WHERE id=$1 AND lease_id=$2 AND lease_until>NOW() AND revoked_at IS NULL AND EXISTS (SELECT 1 FROM base_users u WHERE u.id=amigo_devices.user_id AND u.disabled_at IS NULL) FOR UPDATE")
            .bind::<SqlUuid,_>(device.id).bind::<SqlUuid,_>(lease).get_result::<Device>(conn).await.optional()?;
        if valid.is_none() { return Err(diesel::result::Error::NotFound.into()); }
        let inserted = diesel::sql_query("INSERT INTO amigo_events(id,user_id,device_id,kind,content) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
            .bind::<SqlUuid,_>(event).bind::<BigInt,_>(device.user_id).bind::<SqlUuid,_>(device.id)
            .bind::<Text,_>(kind).bind::<Jsonb,_>(content).execute(conn).await?;
        if inserted == 1 { mutate_scoped_locked(conn, &device, mutate).await }
        else { mutate_scoped_locked(conn, &device, |_| {}).await }
    })).await
}

/// Claim a proactive budget before calling the model. Multiple devices cannot
/// race through the user's cooldown. Failed attempts still consume that slot.
pub async fn reserve_outreach(user_id: i64, now: DateTime<Utc>) -> Result<bool, String> {
    with_conn!(move |conn| horme::transaction(conn, async |conn| {
        let mut reserved = false;
        mutate_locked(conn, user_id, |profile| {
            if profile.may_outreach(now, true) {
                profile.mark_outreach(now);
                reserved = true;
            }
        })
        .await?;
        Ok(reserved)
    }))
    .await
}

pub async fn reserve_observation(
    user_id: i64,
    now: DateTime<Utc>,
    last_source: amigo::protocol::Source,
) -> Result<Option<amigo::protocol::Source>, String> {
    use amigo::protocol::Source;
    with_conn!(move |conn| horme::transaction(conn, async |conn| {
        let mut source = None;
        mutate_locked(conn, user_id, |profile| {
            let settings = &profile.settings;
            let due = profile.last_observation_at.is_none_or(|last| {
                now.signed_duration_since(last).num_seconds()
                    >= settings.observation_interval_secs as i64
            });
            if !due
                || settings.quiet(now)
                || (!settings.camera_enabled && !settings.ambient_audio_enabled)
            {
                return;
            }
            source = Some(
                if settings.ambient_audio_enabled
                    && (!settings.camera_enabled || last_source != Source::Speech)
                {
                    Source::Speech
                } else {
                    settings.camera_target
                },
            );
            profile.last_observation_at = Some(now);
        })
        .await?;
        Ok(source)
    }))
    .await
}

pub async fn reserve_outreach_device(device: Device, now: DateTime<Utc>) -> Result<bool, String> {
    with_conn!(move |conn| horme::transaction(conn, async |conn| {
        let mut reserved = false;
        mutate_scoped_locked(conn, &device, |profile| {
            if profile.may_outreach(now, true) {
                profile.mark_outreach(now);
                reserved = true;
            }
        })
        .await?;
        Ok(reserved)
    }))
    .await
}

pub async fn reserve_observation_device(
    device: Device,
    now: DateTime<Utc>,
    last_source: amigo::protocol::Source,
) -> Result<Option<amigo::protocol::Source>, String> {
    use amigo::protocol::Source;
    with_conn!(move |conn| horme::transaction(conn, async |conn| {
        let mut source = None;
        mutate_scoped_locked(conn, &device, |profile| {
            let settings = &profile.settings;
            let due = profile.last_observation_at.is_none_or(|last| {
                now.signed_duration_since(last).num_seconds()
                    >= settings.observation_interval_secs as i64
            });
            if !due
                || settings.quiet(now)
                || (!settings.camera_enabled && !settings.ambient_audio_enabled)
            {
                return;
            }
            source = Some(
                if settings.ambient_audio_enabled
                    && (!settings.camera_enabled || last_source != Source::Speech)
                {
                    Source::Speech
                } else {
                    settings.camera_target
                },
            );
            profile.last_observation_at = Some(now);
        })
        .await?;
        Ok(source)
    }))
    .await
}
