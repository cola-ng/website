use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};

const DEFAULT_WORDS_FILE: &str = "words-all.txt";
const DEFAULT_OUTPUT_DIR: &str = "../words";
const BIGMODEL_API_URL: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";
const DEFAULT_MODEL: &str = "GLM-4-Flash";
const DEFAULT_CONCURRENCY: usize = 200;
const DEFAULT_RETRY_COUNT: usize = 3;
const DEFAULT_RETRY_DELAY_MS: u64 = 1000;

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    response_format: ResponseFormat,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    content: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let mut words_file = PathBuf::from(DEFAULT_WORDS_FILE);
    let mut output_dir = PathBuf::from(DEFAULT_OUTPUT_DIR);
    let mut concurrency = DEFAULT_CONCURRENCY;
    let mut start_from: Option<String> = None;
    let mut limit: Option<usize> = None;
    let mut model = DEFAULT_MODEL.to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--words-file" => {
                if i + 1 < args.len() {
                    words_file = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --words-file requires a path argument");
                    std::process::exit(1);
                }
            }
            "--output-dir" => {
                if i + 1 < args.len() {
                    output_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --output-dir requires a path argument");
                    std::process::exit(1);
                }
            }
            "--concurrency" => {
                if i + 1 < args.len() {
                    concurrency = args[i + 1].parse().unwrap_or(DEFAULT_CONCURRENCY);
                    i += 2;
                } else {
                    eprintln!("Error: --concurrency requires a number argument");
                    std::process::exit(1);
                }
            }
            "--start-from" => {
                if i + 1 < args.len() {
                    start_from = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --start-from requires a word argument");
                    std::process::exit(1);
                }
            }
            "--limit" => {
                if i + 1 < args.len() {
                    limit = Some(args[i + 1].parse().unwrap_or(100));
                    i += 2;
                } else {
                    eprintln!("Error: --limit requires a number argument");
                    std::process::exit(1);
                }
            }
            "--model" => {
                if i + 1 < args.len() {
                    model = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --model requires a model name argument");
                    std::process::exit(1);
                }
            }
            "--help" => {
                print_usage();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_usage();
                std::process::exit(1);
            }
        }
    }

    let api_key = std::env::var("BIGMODEL_API_KEY").expect("BIGMODEL_API_KEY must be set");

    println!("===============================================");
    println!("Dictionary Generator using BigModel AI");
    println!("===============================================");
    println!("Words file: {}", words_file.display());
    println!("Output directory: {}", output_dir.display());
    println!("Model: {}", model);
    println!("Concurrency: {}", concurrency);
    if let Some(ref start) = start_from {
        println!("Starting from: {}", start);
    }
    if let Some(l) = limit {
        println!("Limit: {} words", l);
    }

    fs::create_dir_all(&output_dir)?;

    let existing_words = load_existing_valid_words(&output_dir)?;
    println!("Found {} existing valid word files", existing_words.len());

    let words = load_words(&words_file, &existing_words, start_from.as_deref())?;
    println!("Found {} words to process", words.len());

    let words_to_process: Vec<String> = if let Some(l) = limit {
        words.into_iter().take(l).collect()
    } else {
        words
    };

    if words_to_process.is_empty() {
        println!("No words to process.");
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        process_words_concurrent(&api_key, &model, &words_to_process, &output_dir, concurrency)
            .await
    })?;

    println!("\n===============================================");
    println!("Done!");
    println!("===============================================");

    Ok(())
}

fn is_valid_json_file(path: &Path) -> bool {
    match fs::read_to_string(path) {
        Ok(content) => {
            if content.trim().is_empty() {
                return false;
            }
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    if let Some(obj) = value.as_object() {
                        !obj.is_empty()
                    } else {
                        false
                    }
                }
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

fn load_existing_valid_words(
    output_dir: &Path,
) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let mut existing = HashSet::new();

    if output_dir.exists() {
        for entry in fs::read_dir(output_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "json" {
                        if is_valid_json_file(&path) {
                            if let Some(stem) = path.file_stem() {
                                existing.insert(stem.to_string_lossy().to_string());
                            }
                        } else {
                            println!(
                                "  Invalid JSON file, will regenerate: {}",
                                path.display()
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(existing)
}

fn load_words(
    words_file: &Path,
    existing: &HashSet<String>,
    start_from: Option<&str>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let file = File::open(words_file)?;
    let reader = BufReader::new(file);

    let mut words = Vec::new();
    let mut started = start_from.is_none();

    for line in reader.lines() {
        let line = line?;
        let word = line.split('\t').next().unwrap_or(&line).trim().to_string();

        if word.is_empty() {
            continue;
        }

        if !is_valid_word(&word) {
            continue;
        }

        if !started {
            if let Some(start) = start_from {
                if word.eq_ignore_ascii_case(start) {
                    started = true;
                } else {
                    continue;
                }
            }
        }

        let word_lower = word.to_lowercase();
        if existing.contains(&word_lower) {
            continue;
        }

        words.push(word);
    }

    Ok(words)
}

fn is_valid_word(word: &str) -> bool {
    if word.len() < 2 || word.len() > 50 {
        return false;
    }

    if word.starts_with('.') || word.starts_with('#') || word.starts_with('/') {
        return false;
    }

    if word.contains("://") || word.contains("www.") {
        return false;
    }

    let has_letter = word.chars().any(|c| c.is_ascii_alphabetic());
    if !has_letter {
        return false;
    }

    let valid_chars = word
        .chars()
        .all(|c| c.is_ascii_alphabetic() || c == '-' || c == '\'' || c == ' ');
    if !valid_chars {
        return false;
    }

    true
}

async fn process_words_concurrent(
    api_key: &str,
    model: &str,
    words: &[String],
    output_dir: &Path,
    concurrency: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?,
    );

    let total = words.len();
    let processed = Arc::new(AtomicUsize::new(0));
    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let api_key = Arc::new(api_key.to_string());
    let model = Arc::new(model.to_string());
    let output_dir = Arc::new(output_dir.to_path_buf());

    println!("\nStarting concurrent processing with {} workers...\n", concurrency);

    stream::iter(words.iter().cloned())
        .map(|word| {
            let client = Arc::clone(&client);
            let api_key = Arc::clone(&api_key);
            let model = Arc::clone(&model);
            let output_dir = Arc::clone(&output_dir);
            let processed = Arc::clone(&processed);
            let success_count = Arc::clone(&success_count);
            let error_count = Arc::clone(&error_count);

            async move {
                let current = processed.fetch_add(1, Ordering::SeqCst) + 1;
                println!("[{}/{}] Processing: {}", current, total, word);

                match generate_word_data(&client, &api_key, &model, &word).await {
                    Ok(json_data) => {
                        let filename = format!("{}.json", word.to_lowercase());
                        let filepath = output_dir.join(&filename);

                        match File::create(&filepath) {
                            Ok(mut file) => {
                                if file.write_all(json_data.as_bytes()).is_ok() {
                                    success_count.fetch_add(1, Ordering::SeqCst);
                                    println!("  [OK] {}", word);
                                } else {
                                    error_count.fetch_add(1, Ordering::SeqCst);
                                    eprintln!("  [ERR] {} - Failed to write file", word);
                                }
                            }
                            Err(e) => {
                                error_count.fetch_add(1, Ordering::SeqCst);
                                eprintln!("  [ERR] {} - {}", word, e);
                            }
                        }
                    }
                    Err(e) => {
                        error_count.fetch_add(1, Ordering::SeqCst);
                        eprintln!("  [ERR] {} - {}", word, e);
                    }
                }
            }
        })
        .buffer_unordered(concurrency)
        .collect::<Vec<()>>()
        .await;

    let final_success = success_count.load(Ordering::SeqCst);
    let final_errors = error_count.load(Ordering::SeqCst);

    println!("\n-----------------------------------------------");
    println!("Processed: {} words", total);
    println!("Success: {}", final_success);
    println!("Errors: {}", final_errors);

    Ok(())
}

async fn generate_word_data(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    word: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let system_prompt = r#"You are a professional lexicographer with access to major English dictionaries including 牛津英语词典, 韦氏国际词典, 柯林斯 COBUILD 高阶英汉双解学习词典, 剑桥高级英语学习者词典, and 麦克米伦高阶英语词典.

Your task is to generate comprehensive dictionary entries in JSON format. For each word, provide:
1. Word metadata (type, frequency, difficulty, syllable count)
2. Pronunciations (UK and US IPA)
3. Definitions in both English and Chinese
4. Word forms (plural, past tense, etc.)
5. Example sentences from various sources
6. Etymology information
7. Categories and tags
8. Related words (synonyms, antonyms, broader terms)
9. Frequency data from major corpora

Always return valid JSON matching the exact structure provided in the example."#;

    let user_prompt = format!(
        r#"Generate a complete dictionary entry for the word "{word}" in the following JSON format:

{{
  "{word}": {{
    "word_type": "<part of speech: noun/verb/adjective/adverb/etc>",
    "language": "en",
    "frequency": <1-100 based on common usage>,
    "difficulty": <1-5, where 1 is easiest>,
    "syllable_count": <number of syllables>,
    "is_lemma": <true if this is the base form>,
    "word_count": <number of words, 1 for single words>,
    
    "pronunciations": [
      {{ "ipa": "<UK IPA>", "dialect": "UK" }},
      {{ "ipa": "<US IPA>", "dialect": "US" }}
    ],

    "definitions": [
      {{
        "language": "en",
        "part_of_speech": "<part of speech>",
        "definition": "<English definition>",
        "register": "<neutral/formal/informal/technical/literary>",
        "is_primary": true,
        "usage_notes": "<optional usage notes>"
      }},
      {{
        "language": "zh",
        "part_of_speech": "<part of speech>",
        "definition": "<Chinese definition>",
        "is_primary": true
      }}
    ],

    "forms": [
      {{ "form_type": "<plural/past/past_participle/present_participle/comparative/superlative>", "form": "<word form>", "is_irregular": <true/false> }}
    ],

    "sentences": [
      {{ "sentence": "<example sentence>", "source": "<source>", "difficulty": <1-5> }}
    ],

    "etymologies": [
      {{
        "language": "en",
        "origin_language": "<origin language>",
        "origin_word": "<original word>",
        "origin_meaning": "<original meaning>",
        "etymology": "<etymology description>",
        "first_attested_year": <year or null>
      }},
      {{
        "language": "zh",
        "origin_language": "<origin language>",
        "origin_word": "<original word>",
        "etymology": "<Chinese etymology description>"
      }}
    ],

    "categories": [
      {{ "name": "<category>", "confidence": <50-100> }}
    ],

    "relations": {{
      "synonyms": [
        {{ "word": "<synonym>", "strength": <50-100> }}
      ],
      "antonyms": [
        {{ "word": "<antonym>", "strength": <50-100> }}
      ],
      "related": [
        {{ "word": "<related word>", "strength": <50-100> }}
      ],
      "broader": [
        {{ "word": "<broader term>", "strength": <50-100> }}
      ]
    }},

    "frequencies": [
      {{ "corpus": "COCA", "corpus_type": "general", "band": "<top_1000/top_3000/top_5000/top_10000>", "rank": <rank>, "per_million": <frequency per million> }},
      {{ "corpus": "BNC", "corpus_type": "written", "band": "<band>", "rank": <rank>, "per_million": <frequency per million> }}
    ]
  }}
}}

Important:
- Provide accurate IPA pronunciations for both UK and US English
- Include at least 2-3 English definitions and their Chinese translations
- Add 3-5 example sentences with varying difficulty levels
- Include relevant word forms based on the part of speech
- Provide accurate etymology information
- List relevant synonyms, antonyms, and related words

CRITICAL - dictionaries field rules:
- The "dictionaries" array indicates which dictionaries/vocabulary lists CONTAIN this word
- ONLY include a dictionary entry if the word actually appears in that dictionary or vocabulary list
- Do NOT blindly include all 10 options - be selective based on the word's actual presence
- Available options with priority_order:
  1. 牛津英语词典 (most words appear here)
  2. 柯林斯 COBUILD 高阶英汉双解学习词典
  3. 韦氏国际词典
  4. 剑桥高级英语学习者词典
  5. 麦克米伦高阶英语词典
  6. 大学英语四级考试 (CET4 ~4500 common words)
  7. 大学英语六级考试 (CET6 ~6000 words, includes CET4)
  8. 英语专业四级考试 (TEM4 ~8000 words)
  9. 英语专业八级考试 (TEM8 ~13000 words)
  10. 考研英语 (NEEP ~5500 words)
- Example: "apple" (common) -> include all 10; "ubiquitous" (advanced) -> include dictionaries + CET6/TEM4/TEM8/NEEP but NOT CET4
- Only return valid JSON, no additional text or explanation"#,
        word = word
    );

    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: user_prompt,
            },
        ],
        temperature: 0.3,
        response_format: ResponseFormat {
            format_type: "json_object".to_string(),
        },
    };

    let mut last_error = None;

    for attempt in 0..DEFAULT_RETRY_COUNT {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(
                DEFAULT_RETRY_DELAY_MS * (attempt as u64 + 1),
            ))
            .await;
        }

        match client
            .post(BIGMODEL_API_URL)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<ChatResponse>().await {
                        Ok(chat_response) => {
                            if let Some(choice) = chat_response.choices.first() {
                                let content = &choice.message.content;

                                if serde_json::from_str::<serde_json::Value>(content).is_ok() {
                                    return Ok(content.clone());
                                } else {
                                    last_error =
                                        Some(format!("Invalid JSON response: {}", content));
                                }
                            } else {
                                last_error = Some("Empty response from API".to_string());
                            }
                        }
                        Err(e) => {
                            last_error = Some(format!("Failed to parse response: {}", e));
                        }
                    }
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    last_error = Some(format!("API error {}: {}", status, body));

                    if status.as_u16() == 429 {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            Err(e) => {
                last_error = Some(format!("Request failed: {}", e));
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| "Unknown error".to_string())
        .into())
}

fn print_usage() {
    println!("Dictionary Generator using BigModel AI");
    println!();
    println!("Usage:");
    println!("  cargo run --bin words [OPTIONS]");
    println!();
    println!("Options:");
    println!(
        "  --words-file <PATH>   Path to words file (default: {})",
        DEFAULT_WORDS_FILE
    );
    println!(
        "  --output-dir <PATH>   Output directory for JSON files (default: {})",
        DEFAULT_OUTPUT_DIR
    );
    println!(
        "  --concurrency <N>     Number of concurrent requests (default: {})",
        DEFAULT_CONCURRENCY
    );
    println!("  --start-from <WORD>   Start processing from this word");
    println!("  --limit <N>           Limit number of words to process");
    println!(
        "  --model <MODEL>       BigModel model name (default: {})",
        DEFAULT_MODEL
    );
    println!("  --help                Show this help message");
    println!();
    println!("Environment:");
    println!("  BIGMODEL_API_KEY      BigModel API key (required)");
    println!();
    println!("Examples:");
    println!("  cargo run --bin words --limit 10");
    println!("  cargo run --bin words --start-from apple --limit 100");
    println!("  cargo run --bin words --concurrency 5 --limit 50");
}
