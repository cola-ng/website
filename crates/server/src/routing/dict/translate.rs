//! Translation endpoint using AI services

use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::services::{ChatMessage, get_user_provider};
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
    /// Translation options
    #[serde(default)]
    options: TranslateOptions,
}

/// Translation options for customizing the output
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct TranslateOptions {
    /// Auto-correct grammar and spelling errors in the source text
    #[serde(default)]
    pub auto_correct: bool,
    /// Tone/style for the translation
    #[serde(default)]
    pub tone: TranslateTone,
    /// Whether to preserve original formatting (line breaks, etc.) or reformat
    #[serde(default = "default_true")]
    pub preserve_formatting: bool,
}

/// Tone/style options for translation
#[derive(Debug, Default, Clone, Copy, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TranslateTone {
    /// Keep original tone, no modification
    #[default]
    Original,
    /// Casual, everyday language
    Casual,
    /// Business/professional tone
    Business,
    /// Academic/formal tone
    Academic,
    /// Creative/literary style
    Literary,
    /// Simple and easy to understand
    Simple,
}

fn default_source() -> String {
    "auto".to_string()
}

fn default_true() -> bool {
    true
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
pub async fn translate(req: &mut Request, depot: &mut Depot) -> JsonResult<TranslateResponse> {
    let input: TranslateRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("Invalid JSON request"))?;

    if input.text.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("Text cannot be empty")
            .into());
    }

    if input.text.len() > 5000 {
        return Err(StatusError::bad_request()
            .brief("Text too long (max 5000 characters)")
            .into());
    }

    let target = input.target.to_lowercase();
    if target != "en" && target != "zh" {
        return Err(StatusError::bad_request()
            .brief("Target must be 'en' or 'zh'")
            .into());
    }

    let provider = get_user_provider(depot.user_id()?).await?;

    let chat_service = provider
        .chat_service()
        .ok_or_else(|| StatusError::service_unavailable().brief("Chat service not available"))?;

    // Build translation prompt
    let (target_lang, source_hint) = match target.as_str() {
        "en" => ("English", "Chinese"),
        "zh" => ("Chinese (Simplified)", "English"),
        _ => unreachable!(),
    };

    let system_prompt = build_system_prompt(target_lang, source_hint, &input.options);

    let user_prompt = build_user_prompt(&input.text, &input.options);

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
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
            let has_chinese = input
                .text
                .chars()
                .any(|c| matches!(c, '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}'));
            Some(if has_chinese {
                "zh".to_string()
            } else {
                "en".to_string()
            })
        } else {
            None
        },
    })
}

/// Build the system prompt based on translation options
fn build_system_prompt(target_lang: &str, source_lang: &str, options: &TranslateOptions) -> String {
    let mut rules = Vec::new();

    // Base rule about output format
    rules.push("Provide ONLY the translation, no explanations or additional text".to_string());

    // Tone-specific rules
    let tone_instruction = match options.tone {
        TranslateTone::Original => "Maintain the original meaning and tone accurately".to_string(),
        TranslateTone::Casual => {
            "Use casual, everyday language that sounds natural and friendly".to_string()
        }
        TranslateTone::Business => {
            "Use professional, formal business language appropriate for workplace communication"
                .to_string()
        }
        TranslateTone::Academic => {
            "Use formal academic language with precise terminology".to_string()
        }
        TranslateTone::Literary => "Use elegant, creative language with literary flair".to_string(),
        TranslateTone::Simple => {
            "Use simple, easy-to-understand language avoiding complex vocabulary".to_string()
        }
    };
    rules.push(tone_instruction);

    // Auto-correct rule
    if options.auto_correct {
        rules.push("Correct any grammar, spelling, or punctuation errors in the source text before translating".to_string());
    }

    // Formatting rule
    if options.preserve_formatting {
        rules.push(
            "Preserve the original formatting including line breaks and paragraph structure"
                .to_string(),
        );
    } else {
        rules.push(
            "Reformat the text for better readability with proper paragraphs and punctuation"
                .to_string(),
        );
    }

    // General rules
    rules.push("Use natural, idiomatic expressions in the target language".to_string());
    rules.push("Keep proper nouns as appropriate for the target language".to_string());

    let rules_text = rules
        .iter()
        .enumerate()
        .map(|(i, r)| format!("{}. {}", i + 1, r))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are a professional translator specializing in {source_lang} to {target_lang} translation.

Rules:
{rules_text}"#
    )
}

/// Build the user prompt based on translation options
fn build_user_prompt(text: &str, options: &TranslateOptions) -> String {
    let mut prompt = String::from("Translate the following text:\n\n");
    prompt.push_str(text);

    if options.auto_correct {
        prompt.push_str(
            "\n\n(Note: Please correct any errors in the source text before translating)",
        );
    }

    prompt
}
