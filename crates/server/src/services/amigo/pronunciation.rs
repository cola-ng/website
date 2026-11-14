use std::time::Duration;

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::audio;
use crate::services::minimax::setting;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    pub reference: String,
    pub accuracy: f32,
    pub weak_word: Option<String>,
}

pub struct AzureSpeech {
    http: reqwest::Client,
    url: String,
    key: String,
}

impl AzureSpeech {
    pub fn from_env() -> Option<Self> {
        let key = setting("AMIGO_AZURE_SPEECH_KEY")?;
        let resource = setting("AMIGO_AZURE_SPEECH_RESOURCE")?;
        if resource.is_empty()
            || resource.len() > 63
            || !resource
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return None;
        }
        Some(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(3))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .ok()?,
            url: format!(
                "https://{resource}.cognitiveservices.azure.com/stt/speech/recognition/conversation/cognitiveservices/v1?language=en-US&format=detailed"
            ),
            key,
        })
    }

    pub async fn assess(
        &self,
        pcm: &[u8],
        transcript: &str,
        target: Option<&str>,
    ) -> Option<Assessment> {
        let reference = target.unwrap_or(transcript);
        if reference.len() > 180
            || words(reference).next().is_none()
            || reference
                .chars()
                .any(|c| c.is_alphabetic() && !c.is_ascii())
            || (target.is_none() && words(reference).count() < 3)
        {
            return None;
        }
        let params = serde_json::json!({"ReferenceText": reference, "GradingSystem": "HundredMark",
            "Granularity": "Word", "Dimension": "Comprehensive", "EnableMiscue": "True"});
        let header = base64::engine::general_purpose::STANDARD.encode(params.to_string());
        let mut response = self
            .http
            .post(&self.url)
            .header("Ocp-Apim-Subscription-Key", &self.key)
            .header("Pronunciation-Assessment", header)
            .header(
                "Content-Type",
                "audio/wav; codecs=audio/pcm; samplerate=16000",
            )
            .body(audio::wav(pcm).ok()?)
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.ok()? {
            if bytes.len() + chunk.len() > 64 * 1024 {
                return None;
            }
            bytes.extend_from_slice(&chunk);
        }
        parse(
            &serde_json::from_slice(&bytes).ok()?,
            reference,
            target.is_some(),
        )
    }
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_alphabetic() && c != '\'')
        .filter(|part| part.bytes().any(|b| b.is_ascii_alphabetic()))
}

pub fn contains_words(text: &str, reference: &str) -> bool {
    let wanted = words(reference).collect::<Vec<_>>();
    let actual = words(text).collect::<Vec<_>>();
    !wanted.is_empty()
        && actual.windows(wanted.len()).any(|window| {
            window
                .iter()
                .zip(&wanted)
                .all(|(a, b)| a.eq_ignore_ascii_case(b))
        })
}

fn score(value: &Value) -> Option<f32> {
    value
        .as_f64()
        .filter(|n| (0.0..=100.0).contains(n))
        .map(|n| n as f32)
}

fn parse(value: &Value, reference: &str, scripted: bool) -> Option<Assessment> {
    if value["RecognitionStatus"].as_str()? != "Success" {
        return None;
    }
    let best = &value["NBest"][0];
    if best["Confidence"].as_f64()? < 0.65 || score(&best["CompletenessScore"])? < 70.0 {
        return None;
    }
    let weak_word = best["Words"]
        .as_array()?
        .iter()
        .filter_map(|word| {
            let text = word["Word"].as_str()?;
            let accuracy = score(&word["AccuracyScore"])?;
            (word["ErrorType"].as_str() == Some("Mispronunciation")
                && accuracy < 60.0
                && text.len() <= 40
                && words(text).count() == 1
                && contains_words(reference, text))
            .then(|| (accuracy, text.to_ascii_lowercase()))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, word)| word);
    Some(Assessment {
        reference: if scripted {
            reference.to_owned()
        } else {
            String::new()
        },
        accuracy: score(&best["AccuracyScore"])?,
        weak_word,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assessment_requires_acoustic_error_and_reliable_alignment() {
        let mut result = serde_json::json!({"RecognitionStatus":"Success","NBest":[{
            "Confidence":0.9,"AccuracyScore":76.0,"CompletenessScore":100.0,
            "Words":[{"Word":"three","AccuracyScore":46,"ErrorType":"Mispronunciation"},
            {"Word":"tea","AccuracyScore":0,"ErrorType":"Omission"}]
        }]});
        assert_eq!(
            parse(&result, "I need three cups", false)
                .unwrap()
                .weak_word
                .as_deref(),
            Some("three")
        );
        assert!(
            parse(&result, "I need tea", false)
                .unwrap()
                .weak_word
                .is_none()
        );
        result["NBest"][0]["Confidence"] = 0.2.into();
        assert!(parse(&result, "I need three cups", false).is_none());
    }
}
