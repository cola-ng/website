//! Shared sentence audio: public approved files, private candidates, BYOK generation.
use std::result::Result;
use horme::prelude::*;
use horme::diesel_async::AsyncPgConnection;
use crate::{AppConfig, AppResult, DepotExt, JsonResult, db::with_conn, hoops, json_ok, services};
use chrono::{DateTime, Utc};
use diesel::{
    sql_types::{BigInt, Bool, Nullable, Text, Timestamptz},
};
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};

pub fn router() -> Router {
    Router::with_path("sentence-audio")
        .push(Router::with_path("candidates/{candidate_id}/audio").get(candidate_audio))
        .push(
            Router::with_path("{kind}/{id}")
                .get(metadata)
                .push(
                    Router::with_path("generate")
                        .hoop(hoops::require_auth)
                        .post(generate),
                )
                .push(Router::with_path("audio").get(official_audio)),
        )
}
pub fn admin_router() -> Router {
    Router::with_path("sentence-audio")
        .get(queue)
        .push(
            Router::with_path("whitelist")
                .get(whitelist)
                .put(update_whitelist),
        )
        .push(Router::with_path("{candidate_id}/review").post(review))
}
#[derive(QueryableByName)]
struct Source {
    #[diesel(sql_type=Text)]
    sentence: String,
    #[diesel(sql_type=Nullable<Text>)]
    audio_path: Option<String>,
}
#[derive(QueryableByName, Serialize, Clone)]
struct Candidate {
    #[diesel(sql_type=BigInt)]
    id: i64,
    #[diesel(sql_type=Text)]
    source_kind: String,
    #[diesel(sql_type=BigInt)]
    source_id: i64,
    #[diesel(sql_type=Text)]
    text_hash: String,
    #[diesel(sql_type=Text)]
    sentence: String,
    #[diesel(sql_type=Nullable<BigInt>)]
    user_id: Option<i64>,
    #[diesel(sql_type=Text)]
    provider_type: String,
    #[diesel(sql_type=Text)]
    model: String,
    #[diesel(sql_type=Text)]
    voice: String,
    #[diesel(sql_type=Nullable<Text>)]
    audio_path: Option<String>,
    #[diesel(sql_type=Text)]
    status: String,
    #[diesel(sql_type=Nullable<Text>)]
    review_note: Option<String>,
    #[diesel(sql_type=Nullable<BigInt>)]
    reviewed_by: Option<i64>,
    #[diesel(sql_type=Nullable<Timestamptz>)]
    reviewed_at: Option<DateTime<Utc>>,
    #[diesel(sql_type=Timestamptz)]
    created_at: DateTime<Utc>,
    #[diesel(sql_type=Timestamptz)]
    updated_at: DateTime<Utc>,
}
#[derive(QueryableByName, Serialize)]
struct Allowed {
    #[diesel(sql_type=Text)]
    provider_type: String,
    #[diesel(sql_type=Bool)]
    enabled: bool,
}
fn db_error() -> StatusError {
    StatusError::service_unavailable().brief("无法读取或保存句子音频，请重试")
}
fn source_error(error:String)->StatusError {
    if error.contains("Record not found"){StatusError::not_found().brief("句子不存在")}else{db_error()}
}

fn params(req: &Request) -> Result<(String, i64), StatusError> {
    let kind = req
        .param::<String>("kind")
        .filter(|s| matches!(s.as_str(), "dict" | "read" | "script"))
        .ok_or_else(|| StatusError::bad_request().brief("句子类型无效"))?;
    let id = req
        .param::<i64>("id")
        .filter(|i| *i > 0)
        .ok_or_else(|| StatusError::bad_request().brief("句子编号无效"))?;
    Ok((kind, id))
}
async fn source(conn: &mut impl GetConnection<Connection = AsyncPgConnection>, kind: &str, id: i64, lock: bool) -> horme::Result<Source> {
    let query = match kind {
        "dict" => "SELECT sentence,NULL::text AS audio_path FROM dict_sentences WHERE id=$1",
        "read" => "SELECT content_en AS sentence,audio_path FROM asset_read_sentences WHERE id=$1",
        "script" => "SELECT content_en AS sentence,audio_path FROM asset_script_turns WHERE id=$1",
        _ => return Err(diesel::result::Error::NotFound.into()),
    };
    diesel::sql_query(format!("{query}{}", if lock { " FOR UPDATE" } else { "" }))
        .bind::<BigInt, _>(id)
        .get_result(conn).await
}
fn file(path: &str) -> Option<PathBuf> {
    let rel = Path::new(path);
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    let base = PathBuf::from(&AppConfig::get().space_path)
        .canonicalize()
        .ok()?;
    let path = base.join(rel).canonicalize().ok()?;
    (path.starts_with(&base) && path.is_file()).then_some(path)
}
async fn official(
    conn: &mut impl GetConnection<Connection = AsyncPgConnection>,
    kind: &str,
    id: i64,
    s: &Source,
) -> horme::Result<Option<String>> {
    // Generated files are tied to the exact current sentence, never a stale text edit.
    if let Some(path) = s
        .audio_path
        .as_ref()
        .filter(|p| !p.starts_with("sentence-audio/") && file(p).is_some())
    {
        return Ok(Some(path.clone()));
    }
    let hash = crate::auth::hash_code(&s.sentence);
    let candidate=diesel::sql_query("SELECT * FROM sentence_audio_candidates WHERE text_hash=$1 AND status='approved' ORDER BY id LIMIT 1").bind::<Text,_>(hash).get_result::<Candidate>(conn).await.optional()?;
    #[derive(QueryableByName)]
    struct Existing {
        #[diesel(sql_type=Text)]
        audio_path: String,
    }
    let legacy=diesel::sql_query("SELECT audio_path FROM asset_read_sentences WHERE content_en=$1 AND audio_path IS NOT NULL UNION SELECT audio_path FROM asset_script_turns WHERE content_en=$1 AND audio_path IS NOT NULL LIMIT 20").bind::<Text,_>(&s.sentence).load::<Existing>(conn).await?;
    if let Some(p) = legacy
        .into_iter()
        .find(|p| !p.audio_path.starts_with("sentence-audio/") && file(&p.audio_path).is_some())
    {
        return Ok(Some(p.audio_path));
    }
    let _ = (kind, id);
    Ok(candidate
        .and_then(|c| c.audio_path)
        .filter(|p| file(p).is_some()))
}
#[handler]
async fn metadata(req: &mut Request, depot: &mut Depot, res: &mut Response) -> JsonResult<Value> {
    let (kind, id) = params(req)?;
    let user = depot.user_id().ok();
    let returned_kind = kind.clone();
    let (platform,candidate,can_generate)=with_conn!(move|conn|{
        let s=source(conn,&kind,id,false).await?;let platform=official(conn,&kind,id,&s).await?;
        let candidate=if let Some(user)=user{diesel::sql_query("SELECT * FROM sentence_audio_candidates WHERE text_hash=$3 AND user_id=$4 AND $1 IN ('dict','read','script') AND $2>0 AND status IN ('pending','generating') AND (status<>'generating' OR updated_at>now()-interval '3 minutes') ORDER BY id DESC LIMIT 1").bind::<Text,_>(&kind).bind::<BigInt,_>(id).bind::<Text,_>(crate::auth::hash_code(&s.sentence)).bind::<BigInt,_>(user).get_result::<Candidate>(conn).await.optional()?}else{None};
        let can=if let Some(user)=user {use crate::db::schema::user_ai_providers as p;diesel::select(diesel::dsl::exists(p::table.filter(p::user_id.eq(user)).filter(p::tts_active.eq(true)))).get_result::<bool>(conn).await?}else{false};Ok((platform,candidate,can))
    }).await.map_err(|e|if e.contains("Record not found"){StatusError::not_found().brief("句子不存在")}else{db_error()})?;
    res.add_header("Cache-Control", "private, no-store", true)?;
    let pending = candidate.as_ref().filter(|c| {
        c.status == "pending" && c.audio_path.as_ref().is_some_and(|p| file(p).is_some())
    });
    json_ok(
        json!({"audio_url":if let Some(path)=platform.as_ref(){Some(format!("/api/sentence-audio/{returned_kind}/{id}/audio?v={}",crate::auth::hash_code(path)))}else{pending.map(|c|format!("/api/sentence-audio/candidates/{}/audio",c.id))},"official":platform.is_some(),"can_generate":can_generate,"status":if platform.is_some(){"approved"}else if pending.is_some(){"pending"}else if candidate.as_ref().is_some_and(|c|c.status=="generating"){"generating"}else{"missing"}}),
    )
}
#[handler]
async fn official_audio(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let (kind, id) = params(req)?;
    let path = with_conn!(move |conn| {
        let s = source(conn, &kind, id, false).await?;
        official(conn, &kind, id, &s).await
    })
    .await
    .map_err(source_error)?
    .and_then(|p| file(&p))
    .ok_or_else(|| StatusError::not_found().brief("暂无正式音频"))?;
    res.add_header("Cache-Control", "public, max-age=300", true)?;
    res.add_header("X-Content-Type-Options","nosniff",true)?;
    res.send_file(path, req.headers()).await;
    Ok(())
}
#[handler]
async fn candidate_audio(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let id = req
        .param::<i64>("candidate_id")
        .ok_or_else(|| StatusError::bad_request().brief("音频编号无效"))?;
    let c = with_conn!(move |conn| {
        diesel::sql_query("SELECT * FROM sentence_audio_candidates WHERE id=$1")
            .bind::<BigInt, _>(id)
            .get_result::<Candidate>(conn).await
    })
    .await
    .map_err(|_| StatusError::not_found().brief("音频不存在"))?;
    let user = depot.user_id().ok();
    if c.status != "approved"
        && (user.is_none() || (user != c.user_id && !hoops::is_user_admin(user.unwrap()).await))
    {
        return Err(StatusError::not_found().brief("音频不存在").into());
    }
    let path = c
        .audio_path
        .as_ref()
        .and_then(|p| file(p))
        .ok_or_else(|| StatusError::not_found().brief("音频文件不存在"))?;
    res.add_header("Cache-Control", "private, no-store", true)?;
    res.add_header("X-Content-Type-Options","nosniff",true)?;
    res.send_file(path, req.headers()).await;
    Ok(())
}
pub(super) fn audio_format(bytes: &[u8], format: &str) -> Option<&'static str> {
    if bytes.len() > 12 * 1024 * 1024 {
        return None;
    }
    if format == "wav"
        && bytes.len() >= 44
        && bytes.starts_with(b"RIFF")
        && &bytes[8..12] == b"WAVE"
    {
        Some("wav")
    } else if format == "mp3"
        && bytes.len() > 16
        && (bytes.starts_with(b"ID3") || (bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0))
    {
        Some("mp3")
    } else {
        None
    }
}
#[handler]
async fn generate(req: &mut Request, depot: &mut Depot) -> JsonResult<Value> {
    let user = depot.user_id()?;
    let (kind, id) = params(req)?;
    // Strictly personal credentials, even for a paid subscriber.
    let (provider, settings, tts) = services::personal_tts(user).await?;
    let model = settings["tts_model"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let voice = settings["tts_voice"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let k = kind.clone();
    let reserved=with_conn!(move|conn|horme::transaction(conn, async |conn| {
        use crate::db::schema::base_users;
        base_users::table.find(user).for_update().select(base_users::id).first::<i64>(conn).await?;
        let s=source(conn,&k,id,true).await?;
        diesel::sql_query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))").bind::<Text,_>(crate::auth::hash_code(&s.sentence)).execute(conn).await?;
        if official(conn,&k,id,&s).await?.is_some(){return Ok(Err(StatusError::conflict().brief("该句已有正式音频，请直接播放")));}
        if s.sentence.trim().is_empty()||s.sentence.chars().count()>1000{return Ok(Err(StatusError::bad_request().brief("仅支持 1 至 1000 字的句子")));}
        diesel::sql_query("UPDATE sentence_audio_candidates SET status='failed',updated_at=now() WHERE text_hash=$1 AND status='generating' AND updated_at<now()-interval '3 minutes'").bind::<Text,_>(crate::auth::hash_code(&s.sentence)).execute(conn).await?;
        let hash=crate::auth::hash_code(&s.sentence);
        let existing=diesel::sql_query("SELECT * FROM sentence_audio_candidates WHERE text_hash=$3 AND $1 IN ('dict','read','script') AND $2>0 AND (status='generating' OR (status='pending' AND user_id=$4)) ORDER BY id DESC LIMIT 1").bind::<Text,_>(&k).bind::<BigInt,_>(id).bind::<Text,_>(&hash).bind::<BigInt,_>(user).get_result::<Candidate>(conn).await.optional()?;
        if let Some(existing)=existing {
            if existing.status=="pending" && !existing.audio_path.as_ref().is_some_and(|p|file(p).is_some()) {
                diesel::sql_query("UPDATE sentence_audio_candidates SET status='failed',updated_at=now() WHERE id=$1").bind::<BigInt,_>(existing.id).execute(conn).await?;
            } else {return Ok(Err(StatusError::conflict().brief("该句音频正在生成或已提交审核，请稍后刷新播放")));}
        }
        #[derive(QueryableByName)]struct Total{#[diesel(sql_type=BigInt)] count:i64}
        let count=diesel::sql_query("SELECT count(*) AS count FROM sentence_audio_candidates WHERE user_id=$1 AND created_at>now()-interval '1 day'").bind::<BigInt,_>(user).get_result::<Total>(conn).await?.count;
        if count>=50{return Ok(Err(StatusError::too_many_requests().brief("今日例句生成次数已达 50 次，请明日再试")));}
        let c=diesel::sql_query("INSERT INTO sentence_audio_candidates(source_kind,source_id,text_hash,sentence,user_id,provider_type,model,voice,status) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'generating') RETURNING *").bind::<Text,_>(k).bind::<BigInt,_>(id).bind::<Text,_>(hash).bind::<Text,_>(s.sentence).bind::<BigInt,_>(user).bind::<Text,_>(provider).bind::<Text,_>(model).bind::<Text,_>(voice).get_result::<Candidate>(conn).await?;Ok(Ok(c))
    }).await).await.map_err(|e|if e.contains("Record not found"){StatusError::not_found().brief("句子不存在")}else{db_error()})??;
    let candidate_id = reserved.id;
    let generated = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        tts.synthesize(&reserved.sentence, None, None),
    )
    .await;
    let audio = match generated {
        Ok(Ok(audio)) => audio,
        _ => {
            fail(candidate_id).await;
            return Err(StatusError::bad_gateway()
                .brief("个人 TTS 生成失败，请检查密钥、模型、音色与余额后重试")
                .into());
        }
    };
    let Some(ext) = audio_format(&audio.audio_data, &audio.format) else {
        fail(candidate_id).await;
        return Err(StatusError::bad_gateway()
            .brief("TTS 返回的音频格式无效")
            .into());
    };
    let relative = format!("sentence-audio/{}.{ext}", uuid::Uuid::new_v4());
    let full = PathBuf::from(&AppConfig::get().space_path).join(&relative);
    if tokio::fs::create_dir_all(full.parent().unwrap())
        .await
        .is_err()
        || tokio::fs::write(&full, &audio.audio_data).await.is_err()
    {
        fail(candidate_id).await;
        return Err(db_error().into());
    }
    let finalized = finalize(candidate_id, relative, None).await;
    match finalized {
        Ok(status) => Ok(Json(
            json!({"id":candidate_id,"status":status,"official":status=="approved","audio_url":format!("/api/sentence-audio/candidates/{candidate_id}/audio")}),
        )),
        Err(error) => {
            let _ = tokio::fs::remove_file(full).await;
            fail(candidate_id).await;
            Err(error.into())
        }
    }
}
async fn fail(id: i64) {
    let _=with_conn!(move|conn|diesel::sql_query("UPDATE sentence_audio_candidates SET status='failed',updated_at=now() WHERE id=$1 AND status='generating'").bind::<BigInt,_>(id).execute(conn).await).await;
}

async fn finalize(id: i64, path: String, reviewer: Option<i64>) -> Result<String, StatusError> {
    with_conn!(move|conn|horme::transaction(conn, async |conn| {
        let c=diesel::sql_query("SELECT * FROM sentence_audio_candidates WHERE id=$1").bind::<BigInt,_>(id).get_result::<Candidate>(conn).await?;
        let s=source(conn,&c.source_kind,c.source_id,true).await?;
        diesel::sql_query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))").bind::<Text,_>(&c.text_hash).execute(conn).await?;
        let current=diesel::sql_query("SELECT * FROM sentence_audio_candidates WHERE id=$1 FOR UPDATE").bind::<BigInt,_>(id).get_result::<Candidate>(conn).await?;
        if (reviewer.is_some()&&current.status!="pending") || (reviewer.is_none()&&current.status!="generating"){return Ok(Err(StatusError::conflict().brief("音频状态已改变，请刷新")));}
        let allowed=diesel::sql_query("SELECT provider_type,enabled FROM sentence_audio_whitelist WHERE provider_type=$1 FOR SHARE").bind::<Text,_>(&c.provider_type).get_result::<Allowed>(conn).await.optional()?.is_some_and(|p|p.enabled);
        let status=if crate::auth::hash_code(&s.sentence)!=c.text_hash || official(conn,&c.source_kind,c.source_id,&s).await?.is_some(){"superseded"}else if reviewer.is_some()||allowed{"approved"}else{"pending"};
        if status=="approved" {
            // All existing files were checked above. Retire a broken official reference before replacing it.
            diesel::sql_query("UPDATE sentence_audio_candidates SET status='superseded',review_note='正式音频文件已缺失，由新音频补全',updated_at=now() WHERE text_hash=$1 AND status='approved'").bind::<Text,_>(&c.text_hash).execute(conn).await?;
        }
        diesel::sql_query("UPDATE sentence_audio_candidates SET audio_path=$2,status=$3,reviewed_by=$4,reviewed_at=CASE WHEN $3='pending' THEN NULL ELSE now() END,review_note=CASE WHEN $3='superseded' THEN '句子已更新或已有正式音频' WHEN $3='approved' AND $4 IS NULL THEN '平台白名单自动通过' ELSE NULL END,updated_at=now() WHERE id=$1").bind::<BigInt,_>(id).bind::<Text,_>(&path).bind::<Text,_>(status).bind::<Nullable<BigInt>,_>(reviewer).execute(conn).await?;
        if status=="approved" {
            let table=match c.source_kind.as_str(){"read"=>Some("asset_read_sentences"),"script"=>Some("asset_script_turns"),_=>None};
            if let Some(table)=table {diesel::sql_query(format!("UPDATE {table} SET audio_path=$1 WHERE id=$2")).bind::<Text,_>(path).bind::<BigInt,_>(c.source_id).execute(conn).await?;}
        }
        Ok(Ok(status.to_owned()))
    }).await).await.map_err(|_|db_error())?
}
#[handler]
async fn queue(req: &mut Request) -> JsonResult<Value> {
    let status = req.query::<String>("status").unwrap_or("pending".into());
    if ![
        "pending",
        "approved",
        "rejected",
        "superseded",
        "failed",
        "generating",
    ]
    .contains(&status.as_str())
    {
        return Err(StatusError::bad_request().brief("审核状态无效").into());
    }
    let before = req.query::<i64>("before").unwrap_or(i64::MAX);
    let rows=with_conn!(move|conn|diesel::sql_query("SELECT * FROM sentence_audio_candidates WHERE status=$1 AND id<$2 ORDER BY id DESC LIMIT 50").bind::<Text,_>(status).bind::<BigInt,_>(before).load::<Candidate>(conn).await).await.map_err(|_|db_error())?;
    let next = if rows.len() == 50 {
        rows.last().map(|c| c.id)
    } else {
        None
    };
    json_ok(json!({"items":rows,"next_before":next}))
}
#[derive(Deserialize)]
struct Review {
    action: String,
    note: Option<String>,
}
#[handler]
async fn review(req: &mut Request, depot: &mut Depot) -> JsonResult<Value> {
    let admin = depot.user_id()?;
    let id = req
        .param::<i64>("candidate_id")
        .ok_or_else(|| StatusError::bad_request().brief("编号无效"))?;
    let input: Review = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("审核格式无效"))?;
    if input.action == "approve" {
        let c = with_conn!(move |conn| {
            diesel::sql_query("SELECT * FROM sentence_audio_candidates WHERE id=$1")
                .bind::<BigInt, _>(id)
                .get_result::<Candidate>(conn).await
        })
        .await
        .map_err(|_| StatusError::not_found().brief("音频不存在"))?;
        let path = c
            .audio_path
            .filter(|p| file(p).is_some())
            .ok_or_else(|| StatusError::bad_request().brief("音频文件不存在，不能通过审核"))?;
        let status = finalize(id, path, Some(admin)).await?;
        return json_ok(json!({"status":status}));
    }
    if input.action != "reject" {
        return Err(StatusError::bad_request().brief("审核操作无效").into());
    }
    let note = input.note.unwrap_or_default();
    if note.chars().count() > 500 {
        return Err(StatusError::bad_request()
            .brief("审核备注最多 500 字")
            .into());
    }
    let changed=with_conn!(move|conn|diesel::sql_query("UPDATE sentence_audio_candidates SET status='rejected',reviewed_by=$2,reviewed_at=now(),review_note=$3,updated_at=now() WHERE id=$1 AND status='pending'").bind::<BigInt,_>(id).bind::<BigInt,_>(admin).bind::<Text,_>(note).execute(conn).await).await.map_err(|_|db_error())?;
    if changed == 0 {
        return Err(StatusError::conflict()
            .brief("候选音频不在待审核状态，请刷新")
            .into());
    }
    json_ok(json!({"status":"rejected"}))
}
#[handler]
async fn whitelist() -> JsonResult<Vec<Allowed>> {
    json_ok(
        with_conn!(move |conn| {
            diesel::sql_query(
                "SELECT provider_type,enabled FROM sentence_audio_whitelist ORDER BY provider_type",
            )
            .load::<Allowed>(conn).await
        })
        .await
        .map_err(|_| db_error())?,
    )
}
#[derive(Deserialize)]
struct WhitelistUpdate {
    provider_type: String,
    enabled: bool,
}
#[handler]
async fn update_whitelist(req: &mut Request, depot: &mut Depot) -> JsonResult<Value> {
    let user = depot.user_id()?;
    let input: WhitelistUpdate = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("白名单格式无效"))?;
    if !services::provider_catalog::definition(&input.provider_type)
        .is_some_and(|p| !p.tts_models.is_empty())
    {
        return Err(StatusError::bad_request()
            .brief("该平台尚未接入 TTS")
            .into());
    }
    with_conn!(move|conn|diesel::sql_query("INSERT INTO sentence_audio_whitelist(provider_type,enabled,updated_by) VALUES($1,$2,$3) ON CONFLICT(provider_type) DO UPDATE SET enabled=EXCLUDED.enabled,updated_by=EXCLUDED.updated_by,updated_at=now()").bind::<Text,_>(input.provider_type).bind::<Bool,_>(input.enabled).bind::<BigInt,_>(user).execute(conn).await).await.map_err(|_|db_error())?;
    json_ok(json!({"saved":true}))
}
