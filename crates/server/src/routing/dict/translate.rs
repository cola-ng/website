//! Translation endpoint using AI services

use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::services::{ChatMessage, get_user_provider, create_provider_from_env};
use crate::{AppResult, DepotExt, JsonResult, json_ok};

#[derive(Debug, Deserialize, ToSchema)]
pub struct TranslateRequest {
    /// Text to translate
    text: String,
    /// Source language: "en" for English, "zh" for Chinese, "auto" for auto-detect
    #[serde(default = "default_source")]
    source: String,
    /// Target language: "en" for English, "zh" for Chinese
    target: String,
}

fn default_source() -> String {
    "auto".to_string()
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TranslateResponse {
    /// Translated text
    pub translated: String,
    /// Detected source language (if source was "auto")
    pub detected_source: Option<String>,
}

/// Translate text between English and Chinese
#[endpoint(tags("Dictionary"))]
pub async fn translate(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<TranslateResponse> {
    let input: TranslateRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("Invalid JSON request"))?;

    if input.text.trim().is_empty() {
        return Err(StatusError::bad_request().brief("Text cannot be empty").into());
    }

    if input.text.len() > 5000 {
        return Err(StatusError::bad_request().brief("Text too long (max 5000 characters)").into());
    }

    let target = input.target.to_lowercase();
    if target != "en" && target != "zh" {
        return Err(StatusError::bad_request().brief("Target must be 'en' or 'zh'").into());
    }

    // Try to get user's provider, fall back to system default
    let provider = if let Ok(user_id) = depot.user_id() {
        get_user_provider(user_id).await
    } else {
        create_provider_from_env()
    };

    let provider = provider.ok_or_else(|| {
        StatusError::service_unavailable().brief("AI service not available")
    })?;

    let chat_service = provider.chat_service().ok_or_else(|| {
        StatusError::service_unavailable().brief("Chat service not available")
    })?;

    // Build translation prompt
    let (target_lang, source_hint) = match target.as_str() {
        "en" => ("English", "Chinese"),
        "zh" => ("Chinese (Simplified)", "English"),
        _ => unreachable!(),
    };

    let system_prompt = format!(
        r#"You are a professional translator. Translate the given text to {}.
Rules:
1. Provide ONLY the translation, no explanations or additional text
2. Maintain the original meaning and tone
3. Use natural, idiomatic expressions in the target language
4. If the text contains proper nouns, keep them as appropriate
5. For {source_lang} to {target_lang} translation, ensure accuracy and fluency"#,
        target_lang,
        source_lang = source_hint,
        target_lang = target_lang
    );

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("Translate the following text:\n\n{}", input.text),
        },
    ];

    let translated = chat_service
        .chat(messages, Some(0.3), Some(2000))
        .await
        .map_err(|e| {
            tracing::error!("Translation error: {:?}", e);
            StatusError::internal_server_error().brief("Translation failed")
        })?;

    // Clean up the response (remove quotes if present)
    let translated = translated.trim().trim_matches('"').to_string();

    json_ok(TranslateResponse {
        translated,
        detected_source: if input.source == "auto" {
            // Simple heuristic for language detection
            let has_chinese = input.text.chars().any(|c| {
                matches!(c, '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}')
            });
            Some(if has_chinese { "zh".to_string() } else { "en".to_string() })
        } else {
            None
        },
    })
}
