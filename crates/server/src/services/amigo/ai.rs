//! Per-device selections and credentials. Never place secrets in the public profile.
use super::store::Device;
use crate::db::with_conn;
use crate::services::{self, AiProvider};
use diesel::sql_types::{BigInt, Jsonb, Text, Uuid as SqlUuid};
use horme::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, result::Result, sync::Arc};

pub const CAPS: [&str; 4] = ["chat", "asr", "tts", "realtime"];
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub provider_type: String,
    #[serde(default)]
    pub provider_id: Option<i64>,
    pub settings: Value,
}
pub type Bindings = BTreeMap<String, Option<Binding>>;
#[derive(Clone)]
pub struct Resolved {
    pub kind: String,
    pub settings: Value,
}
impl Resolved {
    pub fn model(&self, cap: &str) -> String {
        self.settings[format!("{}_model", if cap == "realtime" { "tts" } else { cap })]
            .as_str()
            .unwrap_or_default()
            .into()
    }
    pub fn provider(&self) -> Result<Arc<dyn AiProvider>, String> {
        services::create_provider_from_user_config(&self.kind, &self.settings)
            .ok_or("设备 AI 配置无效".into())
    }
}
pub async fn load(device: &Device) -> Result<Bindings, String> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type=Jsonb)]
        bindings: Value,
    }
    let device = device.clone();
    let row=with_conn!(move|conn|diesel::sql_query("SELECT a.bindings FROM amigo_device_ai a JOIN amigo_devices d ON d.id=a.device_id WHERE a.device_id=$1 AND a.user_id=$2 AND d.user_id=$2 AND d.revoked_at IS NULL")
        .bind::<SqlUuid,_>(device.id).bind::<BigInt,_>(device.user_id).get_result::<Row>(conn).await.optional()).await?;
    serde_json::from_value(row.map(|r| r.bindings).unwrap_or(json!({})))
        .map_err(|_| "设备 AI 配置格式无效".into())
}
pub async fn account(user: i64, cap: &str, id: Option<i64>) -> Result<Option<Resolved>, String> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type=Text)]
        provider_type: String,
        #[diesel(sql_type=Jsonb)]
        settings: Value,
    }
    let cap = cap.to_owned();
    let row=with_conn!(move|conn|diesel::sql_query("SELECT provider_type,settings FROM user_ai_providers WHERE user_id=$1 AND ($2::bigint IS NULL OR id=$2) AND ($2 IS NOT NULL OR CASE $3 WHEN 'chat' THEN is_active WHEN 'asr' THEN asr_active WHEN 'tts' THEN tts_active ELSE provider_type='openai' END) ORDER BY CASE WHEN $3='realtime' THEN (tts_active OR is_active)::int ELSE 0 END DESC,id DESC LIMIT 1")
        .bind::<BigInt,_>(user).bind::<diesel::sql_types::Nullable<BigInt>,_>(id).bind::<Text,_>(cap).get_result::<Row>(conn).await.optional()).await?;
    row.map(|r| {
        services::provider_catalog::validate(&r.provider_type, r.settings).map(|settings| {
            Resolved {
                kind: r.provider_type,
                settings,
            }
        })
    })
    .transpose()
}
pub async fn resolve(
    user: i64,
    cap: &str,
    binding: Option<&Binding>,
) -> Result<Option<Resolved>, String> {
    let Some(binding) = binding else {
        return account(user, cap, None).await;
    };
    if !CAPS.contains(&cap) {
        return Err("未知 AI 能力".into());
    }
    let mut settings = if let Some(id) = binding.provider_id {
        let row = account(user, cap, Some(id))
            .await?
            .ok_or("所选账号 AI 配置不存在或无权访问")?;
        if row.kind != binding.provider_type {
            return Err("提供商与所选账号配置不匹配".into());
        }
        row.settings
    } else {
        json!({"chat_model":"","asr_model":"","tts_model":""})
    };
    let fields = binding.settings.as_object().ok_or("设备 AI 参数格式无效")?;
    for (key, value) in fields {
        if ![
            "api_key",
            "chat_model",
            "asr_model",
            "tts_model",
            "tts_voice",
            "region",
            "workspace_id",
        ]
        .contains(&key.as_str())
        {
            return Err("不支持的设备 AI 参数".into());
        }
        if key == "api_key" && binding.provider_id.is_some() {
            continue;
        }
        settings[key] = value.clone();
    }
    let settings = services::provider_catalog::validate(&binding.provider_type, settings)?;
    let resolved = Resolved {
        kind: binding.provider_type.clone(),
        settings,
    };
    if cap == "realtime" {
        if resolved.kind != "openai" || !services::openai_realtime::is_model(&resolved.model(cap)) {
            return Err("实时语音请选择 OpenAI Realtime 模型".into());
        }
    } else if resolved.model(cap).is_empty() {
        return Err(format!("所选提供商尚未配置 {cap} 模型"));
    }
    Ok(Some(resolved))
}
fn merge_keys(input: &mut Bindings, old: &Bindings) -> Result<(), String> {
    for (cap, binding) in input.iter_mut() {
        if !CAPS.contains(&cap.as_str()) {
            return Err("未知 AI 能力".into());
        }
        let Some(b) = binding else { continue };
        if !b.settings.is_object(){return Err("设备 AI 参数必须是对象".into())}
        if b.provider_id.is_some() {
            b.settings
                .as_object_mut()
                .ok_or("参数格式无效")?
                .remove("api_key");
            continue;
        }
        let key = b.settings["api_key"].as_str().unwrap_or_default();
        if key.is_empty() {
            if let Some(previous) = old
                .get(cap)
                .and_then(Option::as_ref)
                .filter(|p| p.provider_id.is_none() && p.provider_type == b.provider_type)
            {
                b.settings["api_key"] = previous.settings["api_key"].clone();
            }
        }
    }
    Ok(())
}
pub async fn save(device: Device, mut input: Bindings) -> Result<Bindings, String> {
    let old = load(&device).await?;
    merge_keys(&mut input, &old)?;
    for cap in CAPS {
        if let Some(binding) = input.get(cap).and_then(Option::as_ref) {
            resolve(device.user_id, cap, Some(binding)).await?;
        }
    }
    let data = serde_json::to_value(&input).map_err(|_| "设备 AI 配置格式无效")?;
    with_conn!(move|conn|diesel::sql_query("INSERT INTO amigo_device_ai(device_id,user_id,bindings) SELECT id,user_id,$3 FROM amigo_devices WHERE id=$1 AND user_id=$2 AND revoked_at IS NULL ON CONFLICT(device_id) DO UPDATE SET bindings=EXCLUDED.bindings,updated_at=now() WHERE amigo_device_ai.user_id=$2")
        .bind::<SqlUuid,_>(device.id).bind::<BigInt,_>(device.user_id).bind::<Jsonb,_>(data).execute(conn).await).await?;
    Ok(input)
}
pub fn public(bindings: &Bindings) -> Value {
    let mut value = serde_json::to_value(bindings).unwrap_or(json!({}));
    for (_, binding) in value.as_object_mut().unwrap() {
        if let Some(settings) = binding.get_mut("settings").and_then(Value::as_object_mut) {
            let present = settings
                .remove("api_key")
                .and_then(|v| v.as_str().map(|s| !s.is_empty()))
                .unwrap_or(false);
            settings.insert("api_key".into(), json!(""));
            binding["has_api_key"] = json!(present);
        }
    }
    value
}
pub async fn provider(
    device: &Device,
) -> Result<(Arc<dyn AiProvider>, Option<Resolved>, Option<String>), String> {
    let bindings = load(device).await?;
    let mut global: Option<Arc<dyn AiProvider>> = None;
    let mut selected = services::SelectedProvider {
        name: "device",
        chat: None,
        tts: None,
        asr: None,
    };
    let mut chat_config = None;
    let mut tts_voice = None;
    for cap in ["chat", "tts", "asr"] {
        let binding = bindings.get(cap).and_then(Option::as_ref);
        let row = resolve(device.user_id, cap, binding).await?;
        let p = if let Some(row) = &row {
            row.provider()?
        } else {
            if global.is_none() {global=Some(services::get_user_provider(device.user_id).await.map_err(|e|e.to_string())?);}
            global.clone().ok_or("全局 AI 服务不可用")?
        };
        match cap {
            "chat" => {
                selected.chat = p.chat_service();
                chat_config = row
            }
            "tts" => {
                selected.tts = p.tts();
                if binding.is_some() {
                    tts_voice =
                        row.and_then(|r| r.settings["tts_voice"].as_str().map(str::to_owned));
                }
            }
            _ => selected.asr = p.asr(),
        }
    }
    Ok((Arc::new(selected), chat_config, tts_voice))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_keys_survive_blank_edits_but_never_escape_public_view() {
        let old = Bindings::from([(
            "chat".into(),
            Some(Binding {
                provider_type: "openai".into(),
                provider_id: None,
                settings: json!({"api_key":"private-test-key","chat_model":"gpt-4.1-mini"}),
            }),
        )]);
        let mut draft = old.clone();
        draft.get_mut("chat").unwrap().as_mut().unwrap().settings["api_key"] = json!("");
        merge_keys(&mut draft, &old).unwrap();
        assert_eq!(
            draft["chat"].as_ref().unwrap().settings["api_key"],
            "private-test-key"
        );
        let public = public(&draft);
        assert!(!public.to_string().contains("private-test-key"));
        assert_eq!(public["chat"]["has_api_key"], true);
        draft
            .get_mut("chat")
            .unwrap()
            .as_mut()
            .unwrap()
            .provider_type = "zhipu".into();
        draft.get_mut("chat").unwrap().as_mut().unwrap().settings["api_key"] = json!("");
        merge_keys(&mut draft, &old).unwrap();
        assert_eq!(draft["chat"].as_ref().unwrap().settings["api_key"], "");
    }
    #[test]
    fn account_bindings_do_not_store_supplied_keys_and_bad_shapes_are_rejected(){
        let mut data=Bindings::from([("tts".into(),Some(Binding{provider_type:"openai".into(),provider_id:Some(7),settings:json!({"api_key":"must-not-store","tts_model":"gpt-realtime-2.1-mini"})}))]);
        merge_keys(&mut data,&Bindings::new()).unwrap();assert!(data["tts"].as_ref().unwrap().settings.get("api_key").is_none());
        data.get_mut("tts").unwrap().as_mut().unwrap().settings=json!([]);assert!(merge_keys(&mut data,&Bindings::new()).is_err());
        assert!(public(&Bindings::new()).as_object().unwrap().is_empty());
    }
}
