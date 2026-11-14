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
    pub checked_at: &'static str,
    pub docs_url: &'static str,
    pub notice: &'static str,
    pub model_notes: std::collections::BTreeMap<&'static str, &'static str>,
}

pub fn catalog() -> Vec<ProviderDefinition> {
    [
        (
            "zhipu",
            "智谱 Zhipu",
            vec!["glm-5.3-flash", "glm-5.3-flashx", "glm-5.3", "glm-5.2", "glm-5.1", "glm-4-flash"],
            vec!["glm-tts"],
            vec!["glm-asr-2512"],
            "tongtong",
            vec!["cn"],
        ),
        (
            "openai",
            "OpenAI",
            vec!["gpt-6-luna", "gpt-6.1-terra", "gpt-6.1-sol", "gpt-6-astra", "gpt-4.1-mini", "gpt-4.1"],
            vec!["gpt-realtime-2.1-mini", "gpt-realtime-2.1", "gpt-4o-mini-tts", "gpt-4o-mini-tts-2025-12-15", "tts-1", "tts-1-hd"],
            vec!["gpt-transcribe", "gpt-4o-mini-transcribe", "gpt-4o-transcribe", "whisper-1"],
            "marin",
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
            vec!["MiniMax-M3", "MiniMax-M3.1-Flash", "MiniMax-M2.7", "MiniMax-M2.7-highspeed"],
            vec!["speech-2.8-hd", "speech-2.8-turbo"],
            vec![],
            "English_expressive_narrator",
            vec!["cn", "global"],
        ),
        (
            "kimi",
            "Kimi 月之暗面 Moonshot",
            vec!["kimi-k3.1-pro-highspeed", "kimi-k3.1-pro", "kimi-k3"],
            vec![],
            vec![],
            "",
            vec!["cn", "global"],
        ),
        (
            "qwen",
            "通义千问 Qwen 阿里云百炼",
            vec!["qwen3.8-flash", "qwen3.7-plus", "qwen3.8-max", "qwen3.7-flash", "qwen-flash", "qwen-plus", "qwen-max"],
            vec![],
            vec![],
            "",
            vec!["cn", "global"],
        ),
        (
            "gemini",
            "Google Gemini",
            vec!["gemini-4-hydrogen", "gemini-4-argon", "gemini-4-carbon", "gemini-3.8-flash", "gemini-3.7-flash", "gemini-3.5-flash-lite", "gemini-3.1-flash-lite"],
            vec![],
            vec![],
            "",
            vec!["global"],
        ),
        (
            "anthropic",
            "Anthropic Claude",
            vec!["claude-haiku-5-5", "claude-sonnet-5-5", "claude-opus-5-5", "claude-fable-5-2", "claude-sonnet-4-6", "claude-haiku-4-5"],
            vec![],
            vec![],
            "",
            vec!["global"],
        ),
        (
            "siliconflow",
            "硅基流动 SiliconFlow",
            vec!["deepseek-ai/DeepSeek-V4.1-Flash", "deepseek-ai/DeepSeek-V4-Pro", "zai-org/GLM-5.3", "Qwen/Qwen3-8B", "deepseek-ai/DeepSeek-V3.2"],
            vec![],
            vec![],
            "",
            vec!["cn", "global"],
        ),
        (
            "openrouter",
            "OpenRouter",
            vec!["openai/gpt-6-luna", "openai/gpt-6.1-terra", "openai/gpt-6.1-sol", "google/gemini-4-hydrogen", "google/gemini-3.8-flash", "anthropic/claude-haiku-5.5", "anthropic/claude-sonnet-5.5", "deepseek/deepseek-v4.1-flash"],
            vec![],
            vec![],
            "",
            vec!["global"],
        ),
        (
            "xai",
            "xAI Grok",
            vec!["grok-4.7"],
            vec![],
            vec![],
            "",
            vec!["global"],
        ),
        (
            "mistral",
            "Mistral AI",
            vec!["mistral-small-latest", "mistral-small-2603", "mistral-large-4", "mistral-large-latest"],
            vec![],
            vec![],
            "",
            vec!["global"],
        ),
        (
            "groq",
            "Groq",
            vec!["openai/gpt-oss-20b", "openai/gpt-oss-120b", "llama-3.3-70b-versatile", "llama-3.1-8b-instant"],
            vec![],
            vec!["whisper-large-v3-turbo", "whisper-large-v3"],
            "",
            vec!["global"],
        ),
        (
            "baidu",
            "百度千帆 Baidu Qianfan",
            vec!["ernie-5.1", "ernie-5.0", "ernie-4.5-turbo-128k"],
            vec![],
            vec![],
            "",
            vec!["cn"],
        ),
        (
            "hunyuan",
            "腾讯混元 Tencent Hunyuan",
            vec!["hunyuan-a13b", "hunyuan-role-latest", "hunyuan-lite"],
            vec![],
            vec![],
            "",
            vec!["cn"],
        ),
    ]
    .into_iter()
    .map(
        |(id, label, chat_models, tts_models, asr_models, voice, regions)| {
            let model_notes = chat_models.iter().chain(&tts_models).chain(&asr_models).filter_map(|name| model_note(name).map(|note| (*name, note))).collect();
            ProviderDefinition {
            id,
            label,
            chat_models,
            tts_models,
            asr_models,
            voice,
            regions,
            checked_at: "2026-11-13",
            docs_url: documentation(id),
            notice: match id {
                "openai" => "文本建议从 GPT-6 Luna 开始；语音建议选择 Realtime 2.1 Mini，支持流式播放。Marin、Cedar 为官方推荐音色。旧 Speech TTS 模型将于 2027-01-06 停用。",
                "zhipu" => "GLM-5.3 系列始终启用思考，复杂任务可能更慢；ASR 与 TTS 仍使用各自的语音模型。",
                "kimi" => "K3.1 Pro 为当前系列；K2.5 已下线，K2.6 不再向新账号开放。高速版与标准版费用不同。",
                "qwen" => "按成本、均衡、质量提供 Flash、Plus、Max；所选区域与 API Key 必须一致。Token Plan 专属预览模型不在此列表。",
                "groq" => "Llama 3.1/3.3 现面向企业账号；新个人配置建议先选 GPT-OSS。",
                "hunyuan" => "此入口保留原混元 API 的兼容模型；新 Hy4 系列使用 TokenHub 的独立接口与密钥，尚未在此接入。",
                _ => "以下为当前接口的常用模型建议；实际可用范围、地区与价格以平台账号为准，可手填其他模型 ID。",
            },
            model_notes,
        }},
    )
    .collect()
}

/// Sources are reviewed deliberately; publication of a model does not imply this
/// adapter supports another endpoint (Realtime, image generation, embeddings...).
fn documentation(id: &str) -> &'static str {
    match id {
        "zhipu" => "https://docs.bigmodel.cn/cn/guide/start/model-overview",
        "openai" => "https://developers.openai.com/api/docs/models",
        "deepseek" => "https://api-docs.deepseek.com/api/list-models/",
        "minimax" => "https://platform.minimax.io/docs/guides/models-intro",
        "kimi" => "https://platform.kimi.com/docs/models",
        "qwen" => "https://help.aliyun.com/zh/model-studio/text-generation-model",
        "gemini" => "https://aistudio.google.com/docs/models",
        "anthropic" => "https://platform.claude.com/docs/en/models/overview",
        "siliconflow" => "https://www.siliconflow.cn/models",
        "openrouter" => "https://openrouter.ai/models",
        "xai" => "https://docs.x.ai/developers/models",
        "mistral" => "https://docs.mistral.ai/models",
        "groq" => "https://console.groq.com/docs/models",
        "baidu" => "https://cloud.baidu.com/doc/qianfan-api/s/wmwbl48n4",
        "hunyuan" => "https://cloud.tencent.com/document/product/1729/104753",
        _ => unreachable!("catalogue provider"),
    }
}
fn model_note(id: &str) -> Option<&'static str> {
    Some(match id {
        "gpt-realtime-2.1-mini" => "Realtime 流式语音，低延迟与低成本",
        "gpt-realtime-2.1" => "Realtime 流式语音，较高能力",
        "gpt-6-luna" => "低成本日常对话",
        "gpt-6.1-terra" => "速度与能力均衡",
        "gpt-6.1-sol" => "高能力通用模型",
        "gpt-6-astra" => "旗舰复杂推理，费用较高",
        "gpt-4.1-mini" | "gpt-4.1" => "旧版：2027-01-14 停用",
        "gpt-4o-mini-tts" | "gpt-4o-mini-tts-2025-12-15" | "tts-1" | "tts-1-hd" => {
            "兼容语音接口：2027-01-06 停用"
        }
        "gpt-transcribe" => "当前录音转写模型",
        "gpt-4o-mini-transcribe" | "gpt-4o-transcribe" | "whisper-1" => "旧版：2027-02-26 停用",
        "glm-5.3-flash" => "当前高性价比系列，始终思考",
        "glm-5.3-flashx" => "当前高速系列，始终思考",
        "glm-5.3" => "当前旗舰，始终思考",
        "glm-4-flash" => "旧版兼容选择",
        "MiniMax-M3" => "支持关闭思考，适合快速对话",
        "MiniMax-M3.1-Flash" => "当前系列，保留思考",
        "qwen3.8-flash" => "成本优先",
        "qwen3.7-plus" => "能力与成本均衡",
        "qwen3.8-max" => "质量优先",
        "kimi-k3.1-pro-highspeed" => "当前旗舰高速版",
        "kimi-k3.1-pro" => "当前旗舰标准版",
        "gemini-4-hydrogen" => "当前高效率通用模型",
        "gemini-4-argon" => "当前均衡推理模型",
        "gemini-4-carbon" => "当前旗舰推理模型",
        "claude-haiku-5-5" => "低延迟日常任务",
        "claude-sonnet-5-5" => "能力与速度均衡",
        "claude-opus-5-5" => "复杂任务与长程推理",
        "claude-fable-5-2" => "旗舰长程任务，始终思考",
        "llama-3.3-70b-versatile" | "llama-3.1-8b-instant" => "需要 Groq 企业账号权限",
        _ => return None,
    })
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
    let default_voice=if kind=="openai"{super::voices::openai_voice_ids(output["tts_model"].as_str().unwrap_or_default()).first().copied().unwrap_or(p.voice)}else{p.voice};
    let voice = obj
        .get("tts_voice")
        .and_then(Value::as_str)
        .unwrap_or(default_voice)
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
    let voice=if voice.is_empty(){default_voice}else{voice};
    if kind=="openai" && model(&output,"tts").is_some() && !super::voices::openai_voice_supported(output["tts_model"].as_str().unwrap_or_default(),voice) {return Err("所选模型不支持此音色，请从音色列表重新选择".into());}
    output["tts_voice"] = json!(voice);
    let region = obj
        .get("region")
        .and_then(Value::as_str)
        .unwrap_or(p.regions[0]);
    if !p.regions.contains(&region) {
        return Err("服务区域无效".into());
    }
    output["region"] = json!(region);
    if let Some(workspace) = obj.get("workspace_id").and_then(Value::as_str) {
        let workspace = workspace.trim();
        if !workspace.is_empty() && !super::model_catalog::valid_workspace(workspace) {
            return Err("业务空间 ID 格式无效".into());
        }
        output["workspace_id"] = json!(workspace);
    }
    Ok(output)
}

pub fn endpoint(kind: &str, region: &str) -> &'static str {
    match (kind, region) {
        ("zhipu", _) => "https://open.bigmodel.cn/api/paas/v4",
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
