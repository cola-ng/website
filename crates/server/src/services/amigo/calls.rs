//! Owner-scoped diagnostic metadata. Never persist prompts, responses or credentials.
use super::{
    agent::Agent,
    model::{Analysis, Plan, Profile},
    realtime::{PreparedReply, PreparedTurn},
    store::Device,
    tutor::Backend,
};
use crate::db::with_conn;
use amigo::protocol::Source;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::sql_types::{BigInt, Nullable, Text, Timestamptz, Uuid as SqlUuid};
use horme::prelude::*;
use serde::Serialize;
use std::result::Result;
use std::{future::Future, sync::Arc, time::Instant};
use tokio::sync::mpsc;
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Serialize, QueryableByName)]
pub struct Call {
    #[diesel(sql_type=SqlUuid)]
    pub id: Uuid,
    #[diesel(sql_type=Text)]
    pub turn_id: String,
    #[diesel(sql_type=Text)]
    pub stage: String,
    #[diesel(sql_type=Text)]
    pub provider: String,
    #[diesel(sql_type=Text)]
    pub model: String,
    #[diesel(sql_type=Text)]
    pub state: String,
    #[diesel(sql_type=Nullable<Text>)]
    pub error: Option<String>,
    #[diesel(sql_type=BigInt)]
    pub duration_ms: i64,
    #[diesel(sql_type=Timestamptz)]
    pub started_at: DateTime<Utc>,
}
pub async fn list(device: Device) -> Result<Vec<Call>, String> {
    with_conn!(move|conn|diesel::sql_query("SELECT id,turn_id,stage,provider,model,state,error,duration_ms,started_at FROM amigo_calls WHERE device_id=$1 AND user_id=$2 ORDER BY started_at DESC LIMIT 100")
 .bind::<SqlUuid,_>(device.id).bind::<BigInt,_>(device.user_id).load(conn).await).await
}
struct Guard {
    id: Uuid,
    device: Uuid,
    start: Instant,
    secret: Zeroizing<String>,
    armed: bool,
}
impl Guard {
    async fn start(device: Device, turn: u64, stage: &str) -> Result<Self, String> {
        Self::start_with_model(device, turn, stage, None).await
    }
    async fn start_with_model(device: Device, turn: u64, stage: &str, realtime_model: Option<String>) -> Result<Self, String> {
        #[derive(QueryableByName)]
        struct Meta {
            #[diesel(sql_type=Text)]
            provider: String,
            #[diesel(sql_type=Text)]
            model: String,
            #[diesel(sql_type=Text)]
            secret: String,
        }
        let cap = match stage {
            "asr" => "asr",
            "tts" => "tts",
            _ => "chat",
        };
        let stage = stage.to_string();
        let device_meta=if realtime_model.is_none(){
            let bindings=super::ai::load(&device).await?;
            if let Some(binding)=bindings.get(cap).and_then(Option::as_ref){
                super::ai::resolve(device.user_id,cap,Some(binding)).await?.map(|row|(row.kind,row.settings))
            }else{None}
        }else{None};
        let id = Uuid::new_v4();
        let start = Instant::now();
        let device_id = device.id;
        let secret=with_conn!(move|conn| async {
   let meta=if let Some(model)=realtime_model { Meta {provider:"openai".into(),model,secret:String::new()} } else if let Some((kind,settings))=device_meta {Meta{provider:kind,model:settings[format!("{cap}_model")].as_str().unwrap_or_default().into(),secret:settings["api_key"].as_str().unwrap_or_default().into()}} else {diesel::sql_query("SELECT provider_type AS provider,COALESCE(settings->>($2||'_model'),'') AS model,COALESCE(settings->>'api_key','') AS secret FROM user_ai_providers WHERE user_id=$1 AND CASE $2 WHEN 'asr' THEN asr_active WHEN 'tts' THEN tts_active ELSE is_active END ORDER BY id DESC LIMIT 1")
    .bind::<BigInt,_>(device.user_id).bind::<Text,_>(cap).get_result::<Meta>(conn).await.optional()?.unwrap_or(Meta{provider:"platform".into(),model:"account default".into(),secret:String::new()})};
   diesel::sql_query("UPDATE amigo_calls SET state='cancelled',error='调用超时或服务已重启' WHERE device_id=$1 AND state='running' AND started_at<NOW()-INTERVAL '3 minutes'").bind::<SqlUuid,_>(device_id).execute(conn).await?;
   diesel::sql_query("INSERT INTO amigo_calls(id,device_id,user_id,turn_id,stage,provider,model,state) VALUES($1,$2,$3,$4,$5,$6,$7,'running')")
    .bind::<SqlUuid,_>(id).bind::<SqlUuid,_>(device_id).bind::<BigInt,_>(device.user_id).bind::<Text,_>(turn.to_string()).bind::<Text,_>(stage).bind::<Text,_>(meta.provider).bind::<Text,_>(meta.model).execute(conn).await?;
   Ok(meta.secret)
  }).await?;
        Ok(Self {
            id,
            device: device_id,
            start,
            secret: Zeroizing::new(secret),
            armed: true,
        })
    }
    async fn finish(mut self, result: &Result<impl Sized, String>) {
        let error = result.as_ref().err().map(|e| {
            let safe = if self.secret.is_empty() {
                e.clone()
            } else {
                e.replace(self.secret.as_str(), "[redacted]")
            };
            super::model::clean(&safe, 512)
        });
        self.armed = false;
        let _ = finish(
            self.id,
            self.device,
            self.start.elapsed().as_millis().min(i64::MAX as u128) as i64,
            if error.is_some() { "error" } else { "success" },
            error,
        )
        .await;
    }
}
pub async fn live<T>(device: Device, turn: u64, model: &str, operation: impl Future<Output=Result<T,String>>) -> Result<T,String> {
    let log=Guard::start_with_model(device,turn,"realtime",Some(model.to_owned())).await.ok();
    let result=operation.await;
    if let Some(log)=log {log.finish(&result).await;}
    result
}
async fn finish(
    id: Uuid,
    device: Uuid,
    ms: i64,
    state: &'static str,
    error: Option<String>,
) -> Result<(), String> {
    with_conn!(move|conn|async{
  diesel::sql_query("UPDATE amigo_calls SET state=$2,error=$3,duration_ms=$4 WHERE id=$1 AND state='running'").bind::<SqlUuid,_>(id).bind::<Text,_>(state).bind::<Nullable<Text>,_>(error).bind::<BigInt,_>(ms).execute(conn).await?;
  diesel::sql_query("DELETE FROM amigo_calls WHERE device_id=$1 AND id NOT IN (SELECT id FROM amigo_calls WHERE device_id=$1 ORDER BY started_at DESC LIMIT 200)").bind::<SqlUuid,_>(device).execute(conn).await?;Ok(())
 }).await
}
impl Drop for Guard {
    fn drop(&mut self) {
        if self.armed {
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                let id = self.id;
                let device = self.device;
                let ms = self.start.elapsed().as_millis().min(i64::MAX as u128) as i64;
                runtime.spawn(async move {
                    let _ =
                        finish(id, device, ms, "cancelled", Some("调用被取消或超时".into())).await;
                });
            }
        }
    }
}
pub fn traced(inner: Arc<dyn Backend>, device: Device, turn: u64) -> Arc<dyn Backend> {
    Arc::new(Traced {
        inner,
        device,
        turn,
    })
}
struct Traced {
    inner: Arc<dyn Backend>,
    device: Device,
    turn: u64,
}
impl Traced {
    async fn run<T>(
        &self,
        stage: &str,
        future: impl Future<Output = Result<T, String>>,
    ) -> Result<T, String> {
        let log = Guard::start(self.device.clone(), self.turn, stage)
            .await
            .ok();
        let result = future.await;
        if let Some(log) = log {
            log.finish(&result).await;
        }
        result
    }
}
#[async_trait]
impl Backend for Traced {
    fn ready(&self) -> bool {
        self.inner.ready()
    }
    fn test_mode(&self) -> bool {
        self.inner.test_mode()
    }
    fn start_asr(
        &self,
        p: mpsc::Sender<crate::services::asr_stream::AsrPartial>,
    ) -> Option<crate::services::asr_stream::PcmAsrStream> {
        self.inner.start_asr(p)
    }
    async fn assess_pronunciation(
        &self,
        pcm: &[u8],
        text: &str,
        target: Option<&str>,
    ) -> Option<super::pronunciation::Assessment> {
        self.inner.assess_pronunciation(pcm, text, target).await
    }
    async fn prepare_turn(
        &self,
        p: &Profile,
        text: &str,
        proactive: bool,
        camera: Option<bool>,
    ) -> Result<Option<PreparedTurn>, String> {
        self.run("route", self.inner.prepare_turn(p, text, proactive, camera))
            .await
    }
    async fn plan_turn(
        &self,
        p: &Profile,
        text: &str,
        proactive: bool,
        camera: Option<bool>,
    ) -> Result<Option<super::workflow::Decision>, String> {
        self.run("plan", self.inner.plan_turn(p, text, proactive, camera))
            .await
    }
    async fn reply_prepared(
        &self,
        p: &Profile,
        plan: &Plan,
        text: &str,
        prepared: Option<PreparedReply>,
        sentences: mpsc::Sender<String>,
    ) -> Result<Analysis, String> {
        self.run(
            "chat",
            self.inner
                .reply_prepared(p, plan, text, prepared, sentences),
        )
        .await
    }
    async fn reply_stream(
        &self,
        p: &Profile,
        plan: &Plan,
        text: &str,
        sentences: mpsc::Sender<String>,
    ) -> Result<Analysis, String> {
        self.run("chat", self.inner.reply_stream(p, plan, text, sentences))
            .await
    }
    async fn transcribe(&self, pcm: &[u8]) -> Result<String, String> {
        self.run("asr", self.inner.transcribe(pcm)).await
    }
    async fn observe(&self, s: Source, media: &[u8]) -> Result<(String, Analysis), String> {
        self.run("vision", self.inner.observe(s, media)).await
    }
    async fn reply(&self, p: &Profile, plan: &Plan, text: &str) -> Result<Analysis, String> {
        self.run("chat", self.inner.reply(p, plan, text)).await
    }
    async fn synthesize(&self, text: &str, a: &Agent) -> Result<Vec<u8>, String> {
        self.run("tts", self.inner.synthesize(text, a)).await
    }
    async fn synthesize_stream(
        &self,
        text: &str,
        a: &Agent,
        audio: mpsc::Sender<Vec<u8>>,
    ) -> Result<(), String> {
        self.run("tts", self.inner.synthesize_stream(text, a, audio))
            .await
    }
}
