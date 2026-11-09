//! Public, credential-free catalogue. Endpoints are fixed; users cannot proxy arbitrary URLs.
use salvo::oapi::ToSchema;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Serialize, ToSchema)]
pub struct ProviderDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub chat_models: Vec<&'static str>,
    pub tts_models: Vec<&'static str>,
    pub asr_models: Vec<&'static str>,
    pub voice: &'static str,
    pub regions: Vec<&'static str>,
}

pub fn catalog() -> Vec<ProviderDefinition> {
    [
        (
            "zhipu",
            "智谱 Zhipu",
            vec!["glm-4-flash", "glm-4.6"],
            vec!["glm-tts"],
            vec!["glm-asr-2512"],
            "tongtong",
            vec!["cn"],
        ),
        (
            "openai",
            "OpenAI",
            vec!["gpt-4.1-mini", "gpt-4.1"],
            vec!["gpt-4o-mini-tts", "tts-1"],
            vec!["gpt-transcribe", "whisper-1"],
            "alloy",
            vec!["global"],
        ),
        (
            "deepseek",
            "DeepSeek 深度求索",
            vec!["deepseek-flash", "deepseek-v4-pro"],
            vec![],
            vec![],
            "",
            vec!["global"],
        ),
        (
            "minimax",
            "MiniMax 稀宇科技",
            vec!["MiniMax-M3", "MiniMax-M3.1-Flash"],
            vec!["speech-2.8-hd", "speech-2.8-turbo"],
            vec![],
            "English_expressive_narrator",
            vec!["cn", "global"],
        ),
        (
            "kimi",
            "Kimi 月之暗面 Moonshot",
            vec!["kimi-k2.6", "kimi-k3.1-pro", "kimi-k2.5"],
            vec![],
            vec![],
            "",
            vec!["cn", "global"],
        ),
        (
            "qwen",
            "通义千问 Qwen 阿里云百炼",
            vec!["qwen-plus", "qwen-turbo"],
            vec![],
            vec![],
            "",
            vec!["cn", "global"],
        ),
        (
            "gemini",
            "Google Gemini",
            vec!["gemini-3.8-flash", "gemini-2.5-flash"],
            vec![],
            vec![],
            "",
            vec!["global"],
        ),
        (
            "anthropic",
            "Anthropic Claude",
            vec!["claude-sonnet-4-5", "claude-haiku-4-5"],
            vec![],
            vec![],
            "",
            vec!["global"],
        ),
        (
            "siliconflow",
            "硅基流动 SiliconFlow",
            vec!["Qwen/Qwen3-8B", "deepseek-ai/DeepSeek-V3.2"],
            vec![],
            vec![],
            "",
            vec!["cn", "global"],
        ),
        (
            "openrouter",
            "OpenRouter",
            vec!["openai/gpt-4.1-mini", "google/gemini-2.5-flash"],
            vec![],
            vec![],
            "",
            vec!["global"],
        ),
        (
            "xai",
            "xAI Grok",
            vec!["grok-4.7", "grok-4-fast-non-reasoning"],
            vec![],
            vec![],
            "",
            vec!["global"],
        ),
        (
            "mistral",
            "Mistral AI",
            vec!["mistral-small-latest", "mistral-large-latest"],
            vec![],
            vec![],
            "",
            vec!["global"],
        ),
        (
            "groq",
            "Groq",
            vec!["llama-3.3-70b-versatile", "llama-3.1-8b-instant"],
            vec![],
            vec!["whisper-large-v3-turbo", "whisper-large-v3"],
            "",
            vec!["global"],
        ),
        (
            "baidu",
            "百度千帆 Baidu Qianfan",
            vec!["ernie-4.5-turbo-128k", "ernie-4.5-8k-preview"],
            vec![],
            vec![],
            "",
            vec!["cn"],
        ),
        (
            "hunyuan",
            "腾讯混元 Tencent Hunyuan",
            vec!["hunyuan-turbos-latest", "hunyuan-lite"],
            vec![],
            vec![],
            "",
            vec!["cn"],
        ),
    ]
    .into_iter()
    .map(
        |(id, label, chat_models, tts_models, asr_models, voice, regions)| ProviderDefinition {
            id,
            label,
            chat_models,
            tts_models,
            asr_models,
            voice,
            regions,
        },
    )
    .collect()
}

pub fn definition(id: &str) -> Option<ProviderDefinition> {
    catalog().into_iter().find(|p| p.id == id)
}

pub fn model(settings: &Value, capability: &str) -> Option<String> {
    settings
        .get(format!("{capability}_model"))?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

pub fn validate(kind: &str, settings: Value) -> Result<Value, String> {
    let p = definition(kind).ok_or("不支持的 AI 平台")?;
    let obj = settings.as_object().ok_or("配置格式无效")?;
    let key = obj
        .get("api_key")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if key.is_empty()
        || key.len() > 4096
        || key.chars().all(|c| c == '*')
        || key.chars().any(char::is_control)
    {
        return Err("请填写有效的 API Key".into());
    }
    let mut output = json!({"api_key":key});
    let mut count = 0;
    for (cap, models) in [
        ("chat", &p.chat_models),
        ("tts", &p.tts_models),
        ("asr", &p.asr_models),
    ] {
        let field = format!("{cap}_model");
        // Legacy records omitted speech models; keep their existing capabilities.
        let value = match obj.get(&field) {
            None => models.first().copied().unwrap_or(""),
            Some(Value::Null) => "",
            Some(Value::String(s)) => s.trim(),
            _ => return Err("模型名称格式无效".into()),
        };
        if !value.is_empty() {
            if models.is_empty() {
                return Err(format!("该平台暂未接入 {cap} 功能"));
            }
            if value.len() > 128
                || !value
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_.:/".contains(&c))
            {
                return Err("模型名称只能包含英文、数字及 -_.:/，最多 128 字符".into());
            }
            if catalog().iter().any(|d| {
                [
                    ("chat", &d.chat_models),
                    ("tts", &d.tts_models),
                    ("asr", &d.asr_models),
                ]
                .iter()
                .any(|(other, names)| *other != cap && names.contains(&value))
            }) {
                return Err("模型类型不匹配，请区分文本、TTS 和 ASR 模型".into());
            }
            if kind == "zhipu" && cap != "chat" && !models.contains(&value) {
                return Err("该智谱语音模型尚未接入".into());
            }
            count += 1;
        }
        output[&field] = json!(value);
    }
    if count == 0 {
        return Err("请至少配置一种模型".into());
    }
    let voice = obj
        .get("tts_voice")
        .and_then(Value::as_str)
        .unwrap_or(p.voice)
        .trim();
    if voice.len() > 128 || voice.chars().any(char::is_control) {
        return Err("音色名称无效".into());
    }
    if kind == "zhipu"
        && ![
            "tongtong", "chuichui", "xiaochen", "jam", "kazi", "douji", "luodo",
        ]
        .contains(&voice)
    {
        return Err("智谱音色无效".into());
    }
    output["tts_voice"] = json!(if voice.is_empty() { p.voice } else { voice });
    let region = obj
        .get("region")
        .and_then(Value::as_str)
        .unwrap_or(p.regions[0]);
    if !p.regions.contains(&region) {
        return Err("服务区域无效".into());
    }
    output["region"] = json!(region);
    Ok(output)
}

pub fn endpoint(kind: &str, region: &str) -> &'static str {
    match (kind, region) {
        ("openai", _) => "https://api.openai.com/v1",
        ("deepseek", _) => "https://api.deepseek.com",
        ("minimax", "cn") => "https://api.minimaxi.com/v1",
        ("minimax", _) => "https://api.minimax.io/v1",
        ("kimi", "cn") => "https://api.moonshot.cn/v1",
        ("kimi", _) => "https://api.moonshot.ai/v1",
        ("qwen", "cn") => "https://dashscope.aliyuncs.com/compatible-mode/v1",
        ("qwen", _) => "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        ("gemini", _) => "https://generativelanguage.googleapis.com/v1beta/openai",
        ("anthropic", _) => "https://api.anthropic.com/v1",
        ("siliconflow", "cn") => "https://api.siliconflow.cn/v1",
        ("siliconflow", _) => "https://api.siliconflow.com/v1",
        ("openrouter", _) => "https://openrouter.ai/api/v1",
        ("xai", _) => "https://api.x.ai/v1",
        ("mistral", _) => "https://api.mistral.ai/v1",
        ("groq", _) => "https://api.groq.com/openai/v1",
        ("baidu", _) => "https://qianfan.baidubce.com/v2",
        ("hunyuan", _) => "https://api.hunyuan.cloud.tencent.com/v1",
        _ => unreachable!("validated provider"),
    }
}
