use chrono::{DateTime, Utc};
use diesel::sql_types::{BigInt, Jsonb, Nullable, Text, Timestamptz, Uuid as SqlUuid};
use horme::prelude::*;
use serde::{Deserialize, Serialize};
use std::result::Result;
use uuid::Uuid;

use super::{
    model::{Profile, Settings},
    store::{self, Device},
};
use crate::db::with_conn;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Configuration {
    pub revision: i64,
    pub volume: u8,
    pub continuous: bool,
    pub voice_interrupt: bool,
    pub playback_wake: bool,
}
impl Default for Configuration {
    fn default() -> Self {
        Self {
            revision: 0,
            volume: 70,
            continuous: true,
            voice_interrupt: true,
            playback_wake: true,
        }
    }
}
#[derive(Serialize, QueryableByName)]
pub struct Details {
    #[diesel(sql_type = SqlUuid)]
    pub id: Uuid,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Text)]
    pub conversation_mode: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub board: Option<String>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub last_seen_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub online: bool,
    #[diesel(sql_type = Nullable<Jsonb>)]
    pub configuration: Option<serde_json::Value>,
    #[diesel(sql_type = Nullable<BigInt>)]
    pub applied_revision: Option<i64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    #[serde(default)]
    pub companion: Option<super::agent::Agent>,
    pub name: String,
    pub agent_id: String,
    pub settings: Settings,
    pub configuration: Configuration,
}
#[derive(Serialize, QueryableByName)]
pub struct Event {
    #[diesel(sql_type = SqlUuid)]
    pub id: Uuid,
    #[diesel(sql_type = Text)]
    pub kind: String,
    #[diesel(sql_type = Jsonb)]
    pub content: serde_json::Value,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,
}
pub async fn list(user: i64) -> Result<Vec<Details>, String> {
    with_conn!(move |conn| diesel::sql_query("SELECT id,name,COALESCE((SELECT state#>>'{settings,conversation_mode}' FROM amigo_device_profiles p WHERE p.device_id=amigo_devices.id),'standard') AS conversation_mode,board,last_seen_at,(lease_until>NOW() AND lease_id IS NOT NULL) IS TRUE AS online,configuration,applied_revision FROM amigo_devices WHERE user_id=$1 AND revoked_at IS NULL ORDER BY created_at DESC").bind::<BigInt,_>(user).load(conn).await).await
}
pub async fn owned(user: i64, id: Uuid) -> Result<Option<Device>, String> {
    Ok(store::devices(user)
        .await?
        .into_iter()
        .find(|d| d.id == id && d.revoked_at.is_none()))
}
pub async fn load(device: Device) -> Result<Profile, String> {
    let account = store::load(device.user_id).await?;
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type=Jsonb)]
        state: serde_json::Value,
    }
    let id = device.id;
    let row=with_conn!(move |conn| diesel::sql_query("SELECT p.state FROM amigo_device_profiles p JOIN amigo_devices d ON d.id=p.device_id WHERE p.device_id=$1 AND d.configuration IS NOT NULL AND d.revoked_at IS NULL").bind::<SqlUuid,_>(id).get_result::<Row>(conn).await.optional()).await?;
    let Some(row) = row else { return Ok(account) };
    let mut profile: Profile = serde_json::from_value(row.state).map_err(|e| e.to_string())?;
    profile.agents = account.agents;
    profile.ensure_agents();
    with_conn!(move |conn| {
        store::merge_shared(conn, device.user_id, &mut profile).await?;
        Ok(profile)
    })
    .await
}
pub async fn update<F>(device: Device, mutate: F) -> Result<Profile, String>
where
    F: FnOnce(&mut Profile) + Send + 'static,
{
    with_conn!(move |conn| horme::transaction(conn, async |conn| {
        store::mutate_scoped_locked(conn, &device, mutate).await
    }))
    .await
}
pub async fn save(device: Device, mut input: Update) -> Result<Profile, String> {
    input.configuration.revision = Utc::now().timestamp_micros();
    with_conn!(move |conn| horme::transaction(conn, async |conn| {
        let n=diesel::sql_query("UPDATE amigo_devices SET name=$3,configuration=$4 WHERE id=$1 AND user_id=$2 AND revoked_at IS NULL").bind::<SqlUuid,_>(device.id).bind::<BigInt,_>(device.user_id).bind::<Text,_>(input.name.trim()).bind::<Jsonb,_>(serde_json::to_value(&input.configuration).unwrap()).execute(conn).await?;
        if n!=1 {return Err(diesel::result::Error::NotFound.into());}
        store::mutate_device_locked(conn,device.user_id,device.id,move |p| {p.settings=input.settings;p.active_agent_id=input.agent_id;p.device_agent=input.companion;p.ensure_agents();}).await
    })).await
}
pub async fn configuration(id: Uuid) -> Result<Option<Configuration>, String> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type=Nullable<Jsonb>)]
        configuration: Option<serde_json::Value>,
    }
    let row = with_conn!(move |conn| diesel::sql_query(
        "SELECT configuration FROM amigo_devices WHERE id=$1 AND revoked_at IS NULL"
    )
    .bind::<SqlUuid, _>(id)
    .get_result::<Row>(conn)
    .await)
    .await?;
    row.configuration
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())
}
pub async fn applied(id: Uuid, revision: i64) -> Result<(), String> {
    with_conn!(move |conn| diesel::sql_query("UPDATE amigo_devices SET applied_revision=$2 WHERE id=$1 AND (configuration->>'revision')::bigint=$2").bind::<SqlUuid,_>(id).bind::<BigInt,_>(revision).execute(conn).await).await.map(|_|())
}
pub async fn board(id: Uuid, board: String) -> Result<(), String> {
    with_conn!(
        move |conn| diesel::sql_query("UPDATE amigo_devices SET board=$2 WHERE id=$1")
            .bind::<SqlUuid, _>(id)
            .bind::<Text, _>(board)
            .execute(conn)
            .await
    )
    .await
    .map(|_| ())
}
pub async fn history(device: Device) -> Result<Vec<Event>, String> {
    with_conn!(move |conn| diesel::sql_query("SELECT id,kind,content,created_at FROM amigo_events WHERE user_id=$1 AND device_id=$2 ORDER BY created_at DESC LIMIT 50").bind::<BigInt,_>(device.user_id).bind::<SqlUuid,_>(device.id).load(conn).await).await
}
