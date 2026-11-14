//! Voice lists use the owner's saved provider. Preview is a short fixed phrase.
use super::{ai_provider::TtsResponse, provider_catalog};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::result::Result;
#[derive(Clone, Serialize, Deserialize)]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub gender: String,
    pub language: String,
    pub description: String,
}
pub const OPENAI_REALTIME_VOICES: &[&str] = &[
    "marin", "cedar", "alloy", "ash", "ballad", "coral", "echo", "sage", "shimmer", "verse",
];
const OPENAI_SPEECH_VOICES: &[&str] = &[
    "marin", "cedar", "alloy", "ash", "ballad", "coral", "echo", "fable", "nova", "onyx", "sage",
    "shimmer", "verse",
];
const OPENAI_TTS1_VOICES: &[&str] = &[
    "alloy", "ash", "coral", "echo", "fable", "onyx", "nova", "sage", "shimmer",
];
pub fn openai_voice_supported(model: &str, id: &str) -> bool {
    valid(id)
        && (id.starts_with("voice_") && !matches!(model, "tts-1" | "tts-1-hd")
            || openai_voice_ids(model).contains(&id))
}
pub(super) fn openai_voice_ids(model: &str) -> &'static [&'static str] {
    if super::openai_realtime::is_model(model) {
        OPENAI_REALTIME_VOICES
    } else if matches!(model, "tts-1" | "tts-1-hd") {
        OPENAI_TTS1_VOICES
    } else {
        OPENAI_SPEECH_VOICES
    }
}
pub fn presets_for_model(kind: &str, model: &str) -> Vec<Voice> {
    if kind != "openai" {
        return presets(kind);
    }
    openai_voice_ids(model)
        .iter()
        .map(|id| Voice {
            id: (*id).into(),
            name: if matches!(*id, "marin" | "cedar") {
                format!("{} · 官方推荐", title(id))
            } else {
                title(id)
            },
            gender: "unknown".into(),
            language: "multi".into(),
            description: "支持中文、英语等多种语言。官方未提供性别标签，请通过试听选择。".into(),
        })
        .collect()
}
fn title(id: &str) -> String {
    let mut chars = id.chars();
    format!(
        "{}{}",
        chars.next().unwrap_or_default().to_ascii_uppercase(),
        chars.as_str()
    )
}
fn voice(id: &str, name: &str, description: &str) -> Voice {
    let search = format!("{id} {name} {description}").to_lowercase();
    let words = search
        .split(|c: char| !c.is_ascii_alphabetic())
        .collect::<Vec<_>>();
    let gender = if search.contains('女')
        || words
            .iter()
            .any(|s| matches!(*s, "female" | "woman" | "girl" | "lady"))
    {
        "female"
    } else if search.contains('男') || words.iter().any(|s| matches!(*s, "male" | "man" | "boy")) {
        "male"
    } else {
        "unknown"
    };
    let language =
        if search.contains("english") || search.contains("英文") || search.contains("英语") {
            "en"
        } else if search.contains("chinese") || search.contains("中文") || search.contains("普通话")
        {
            "zh"
        } else {
            "other"
        };
    Voice {
        id: id.into(),
        name: name.into(),
        gender: gender.into(),
        language: language.into(),
        description: description.into(),
    }
}
pub fn presets(kind: &str) -> Vec<Voice> {
    match kind {
        "openai" => presets_for_model("openai", "gpt-realtime-2.1-mini"),
        "zhipu" => [
            ("tongtong", "彤彤"),
            ("chuichui", "锤锤"),
            ("xiaochen", "小陈"),
            ("jam", "jam"),
            ("kazi", "kazi"),
            ("douji", "douji"),
            ("luodo", "luodo"),
        ]
        .into_iter()
        .map(|(id, name)| voice(id, name, "官方预置音色，可试听选择；平台未提供性别标签"))
        .collect(),
        "minimax" => [
            (
                "English_expressive_narrator",
                "Expressive Narrator",
                "英文叙述",
            ),
            (
                "English_Trustworthy_Man",
                "Trustworthy Man",
                "美式英语，青年男声",
            ),
            (
                "English_Graceful_Lady",
                "Graceful Lady",
                "英式英语，成熟女声",
            ),
            (
                "English_Gentle-voiced_man",
                "Gentle-voiced Man",
                "美式英语，温柔男声",
            ),
            (
                "English_Whispering_girl",
                "Whispering Girl",
                "美式英语，轻柔女声",
            ),
            ("male-qn-qingse", "青涩青年", "中文男声"),
            ("female-shaonv", "少女", "中文女声"),
        ]
        .into_iter()
        .map(|(id, name, description)| voice(id, name, description))
        .collect(),
        _ => Vec::new(),
    }
}
pub async fn list(kind: &str, settings: &Value) -> Result<Vec<Voice>, String> {
    if kind != "minimax" {
        return Ok(presets_for_model(
            kind,
            settings["tts_model"].as_str().unwrap_or_default(),
        ));
    }
    let base = provider_catalog::endpoint(kind, settings["region"].as_str().unwrap_or("cn"));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "无法读取音色")?;
    let mut response = client
        .post(format!("{base}/get_voice"))
        .bearer_auth(settings["api_key"].as_str().unwrap_or_default())
        .json(&serde_json::json!({"voice_type":"all"}))
        .send()
        .await
        .map_err(|_| "音色列表连接失败")?;
    if !response.status().is_success() {
        return Err(format!("读取音色失败 HTTP {}", response.status().as_u16()));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "音色列表读取中断")? {
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
            return Err("音色列表过大".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "音色列表格式无效")?;
    if value
        .pointer("/base_resp/status_code")
        .and_then(Value::as_i64)
        != Some(0)
    {
        return Err("音色列表不可用，请检查密钥及区域".into());
    }
    let mut output = Vec::new();
    for key in ["system_voice", "voice_cloning", "voice_generation"] {
        if let Some(rows) = value[key].as_array() {
            for row in rows.iter().take(1024) {
                if let Some(id) = row["voice_id"]
                    .as_str()
                    .filter(|s| !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control))
                {
                    let description = row["description"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join(" · ")
                        })
                        .unwrap_or_default();
                    output.push(voice(
                        id,
                        row["voice_name"].as_str().unwrap_or(id),
                        &super::amigo::model::clean(&description, 500),
                    ));
                }
            }
        }
    }
    Ok(output)
}
pub fn valid(id: &str) -> bool {
    !id.is_empty() && id.len() <= 128 && !id.chars().any(char::is_control)
}
pub async fn preview(kind: &str, settings: &Value, id: &str) -> Result<TtsResponse, String> {
    if !valid(id) {
        return Err("请选择音色".into());
    }
    if kind == "openai"
        && !openai_voice_supported(settings["tts_model"].as_str().unwrap_or_default(), id)
    {
        return Err("此模型不支持所选音色，请从列表重新选择".into());
    }
    let provider = super::create_provider_from_user_config(kind, settings).ok_or("TTS 配置无效")?;
    let service = provider.tts().ok_or("此配置没有 TTS 模型")?;
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        service.synthesize("Hello! I'm your Amigo. 很高兴认识你。", Some(id), Some(1.0)),
    )
    .await
    .map_err(|_| "试听超时")?
    .map_err(|_| "试听失败，请检查音色、模型、区域及余额")?;
    if response.audio_data.is_empty() || response.audio_data.len() > 2 * 1024 * 1024 {
        return Err("试听音频无效".into());
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_metadata_filters_do_not_infer_man_from_mandarin() {
        assert_eq!(
            voice("Chinese (Mandarin)_Narrator", "Narrator", "").gender,
            "unknown"
        );
        assert_eq!(voice("English_Trustworthy_Man", "Man", "").gender, "male");
        assert_eq!(voice("English_Graceful_Lady", "Lady", "").gender, "female");
        assert_eq!(presets("zhipu").len(), 7);
        assert!(!valid("voice\r\nInjected"));
    }
    #[test]
    fn openai_lists_match_each_model_and_do_not_invent_gender() {
        let realtime = presets_for_model("openai", "gpt-realtime-2.1-mini");
        assert_eq!(realtime.len(), 10);
        assert_eq!(realtime[0].id, "marin");
        assert!(
            realtime
                .iter()
                .all(|v| v.gender == "unknown" && v.language == "multi")
        );
        assert_eq!(presets_for_model("openai", "gpt-4o-mini-tts").len(), 13);
        assert_eq!(presets_for_model("openai", "tts-1").len(), 9);
        assert!(!openai_voice_supported("gpt-realtime-2.1-mini", "fable"));
        assert!(!openai_voice_supported("tts-1", "marin"));
        assert!(openai_voice_supported(
            "gpt-realtime-2.1-mini",
            "voice_custom"
        ));
        assert_eq!(
            provider_catalog::validate(
                "openai",
                serde_json::json!({"api_key":"test","tts_model":"tts-1"})
            )
            .unwrap()["tts_voice"],
            "alloy"
        );
        assert!(provider_catalog::validate("openai",serde_json::json!({"api_key":"test","tts_model":"gpt-realtime-2.1-mini","tts_voice":"nova"})).is_err());
    }
}
