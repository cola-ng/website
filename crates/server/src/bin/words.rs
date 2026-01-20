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
const DEFAULT_RETRY_COUNT: usize = 3;
const DEFAULT_RETRY_DELAY_MS: u64 = 1000;
const DEFAULT_BATCH_SIZE: usize = 5;

/// 通用模型及其并发限制
/// 筛选适合文本生成的模型（排除 Vision/Voice/Search/Phone/AllTools 等特殊用途模型）
const GENERAL_MODELS: &[(&str, usize)] = &[
    ("GLM-4-Flash", 50),
    // ("GLM-4-FlashX-250414", 25),
    // ("GLM-4-Air", 25),
    // ("GLM-Zero-Preview", 2),
    // ("GLM-4-FlashX", 5),
    // ("GLM-Z1-FlashX", 5),
    // ("GLM-3-Turbo", 5),
    // ("GLM-Z1-Air", 5),
    // ("GLM-Z1-Flash", 5),
    // ("GLM-Z1-AirX", 5),
    // ("GLM-4-Air-250414", 5),
    // ("GLM-4", 5),
    // ("GLM-4-Plus", 5),
    // ("GLM-4-0520", 5),
    // ("GLM-4-Flash-250414", 5),
    // // ("GLM-4-32B-0414-128K", 5),
    // ("GLM-4.5", 3),
    // ("GLM-4-Long", 3),
    // ("GLM-4.5-Air", 2),
    // ("GLM-4.5-AirX", 2),
    // ("GLM-4-AirX", 2),
    // ("GLM-4-9B", 2),
    // ("GLM-4.7", 1),
    // ("GLM-4.7-FlashX", 2),
    // ("GLM-4.6", 1),
    // ("GLM-4.5-Flash", 1),
    // ("GLM-4.7-Flash", 1),
];

#[derive(Debug)]
struct ModelInfo {
    name: String,
    max_concurrency: usize,
    current_usage: AtomicUsize,
}

impl ModelInfo {
    fn new(name: &str, max_concurrency: usize) -> Self {
        Self {
            name: name.to_string(),
            max_concurrency,
            current_usage: AtomicUsize::new(0),
        }
    }

    /// 获取剩余并发量的百分比 (0.0 - 1.0)
    fn available_ratio(&self) -> f64 {
        let current = self.current_usage.load(Ordering::SeqCst);
        if current >= self.max_concurrency {
            0.0
        } else {
            (self.max_concurrency - current) as f64 / self.max_concurrency as f64
        }
    }

    /// 尝试获取一个并发槽位，如果成功返回 true
    fn try_acquire(&self) -> bool {
        loop {
            let current = self.current_usage.load(Ordering::SeqCst);
            if current >= self.max_concurrency {
                return false;
            }
            if self
                .current_usage
                .compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// 释放一个并发槽位
    fn release(&self) {
        self.current_usage.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Debug)]
struct ModelPool {
    models: Vec<Arc<ModelInfo>>,
}

impl ModelPool {
    fn new() -> Self {
        let models = GENERAL_MODELS
            .iter()
            .map(|(name, concurrency)| Arc::new(ModelInfo::new(name, *concurrency)))
            .collect();
        Self { models }
    }

    /// 获取总并发能力
    fn total_concurrency(&self) -> usize {
        self.models.iter().map(|m| m.max_concurrency).sum()
    }

    /// 选择最空闲的模型（剩余量百分比最高）并获取槽位
    /// 如果所有模型都满了，会等待直到有可用槽位
    async fn acquire(&self) -> Arc<ModelInfo> {
        loop {
            // 按剩余量百分比排序，选择最空闲的模型
            let mut candidates: Vec<_> = self
                .models
                .iter()
                .map(|m| (m.clone(), m.available_ratio()))
                .filter(|(_, ratio)| *ratio > 0.0)
                .collect();

            // 按剩余量百分比降序排序
            candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // 尝试获取最空闲模型的槽位
            for (model, _) in candidates {
                if model.try_acquire() {
                    return model;
                }
            }

            // 所有模型都满了，等待一小段时间后重试
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// 打印模型池状态
    fn print_status(&self) {
        println!("\nModel Pool Status:");
        println!("{:-<60}", "");
        println!("{:<30} {:>10} {:>10} {:>8}", "Model", "Max", "Current", "Avail%");
        println!("{:-<60}", "");
        for model in &self.models {
            let current = model.current_usage.load(Ordering::SeqCst);
            let ratio = model.available_ratio() * 100.0;
            println!(
                "{:<30} {:>10} {:>10} {:>7.1}%",
                model.name, model.max_concurrency, current, ratio
            );
        }
        println!("{:-<60}", "");
        println!("Total concurrency capacity: {}", self.total_concurrency());
    }
}

/// RAII guard for model slot
struct ModelGuard {
    model: Arc<ModelInfo>,
}

impl ModelGuard {
    fn new(model: Arc<ModelInfo>) -> Self {
        Self { model }
    }

    fn name(&self) -> &str {
        &self.model.name
    }
}

impl Drop for ModelGuard {
    fn drop(&mut self) {
        self.model.release();
    }
}

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
    let mut start_from: Option<String> = None;
    let mut limit: Option<usize> = None;

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

    let model_pool = Arc::new(ModelPool::new());

    println!("===============================================");
    println!("Dictionary Generator using BigModel AI");
    println!("(Multi-Model Load Balancing Mode)");
    println!("===============================================");
    println!("Words file: {}", words_file.display());
    println!("Output directory: {}", output_dir.display());
    if let Some(ref start) = start_from {
        println!("Starting from: {}", start);
    }
    if let Some(l) = limit {
        println!("Limit: {} words", l);
    }

    model_pool.print_status();

    fs::create_dir_all(&output_dir)?;

    let existing_words = load_existing_valid_words(&output_dir)?;
    println!("\nFound {} existing valid word files", existing_words.len());

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
        process_words_concurrent(&api_key, &model_pool, &words_to_process, &output_dir).await
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
    model_pool: &Arc<ModelPool>,
    words: &[String],
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(600)) // 增加超时时间以适应批量请求
            .build()?,
    );

    // 将单词分成批次
    let batches: Vec<Vec<String>> = words
        .chunks(DEFAULT_BATCH_SIZE)
        .map(|chunk| chunk.to_vec())
        .collect();

    let total_batches = batches.len();
    let total_words = words.len();
    let processed_batches = Arc::new(AtomicUsize::new(0));
    let success_count = Arc::new(AtomicUsize::new(0));
    let skip_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let api_key = Arc::new(api_key.to_string());
    let output_dir = Arc::new(output_dir.to_path_buf());

    let concurrency = model_pool.total_concurrency();
    println!(
        "\nStarting batch processing with {} total concurrency across {} models...",
        concurrency,
        model_pool.models.len()
    );
    println!(
        "Total: {} words in {} batches (batch size: {})\n",
        total_words, total_batches, DEFAULT_BATCH_SIZE
    );

    stream::iter(batches.into_iter())
        .map(|batch| {
            let client = Arc::clone(&client);
            let api_key = Arc::clone(&api_key);
            let model_pool = Arc::clone(model_pool);
            let output_dir = Arc::clone(&output_dir);
            let processed_batches = Arc::clone(&processed_batches);
            let success_count = Arc::clone(&success_count);
            let skip_count = Arc::clone(&skip_count);
            let error_count = Arc::clone(&error_count);

            async move {
                // 获取最空闲的模型
                let model = model_pool.acquire().await;
                let guard = ModelGuard::new(model);

                let current_batch = processed_batches.fetch_add(1, Ordering::SeqCst) + 1;
                let batch_words: Vec<&str> = batch.iter().map(|s| s.as_str()).collect();
                println!(
                    "[Batch {}/{}] Processing {} words: {} (using {})",
                    current_batch,
                    total_batches,
                    batch.len(),
                    batch_words.join(", "),
                    guard.name()
                );

                match generate_batch_word_data(&client, &api_key, guard.name(), &batch).await {
                    Ok(word_map) => {
                        // 将每个单词的数据写入单独的 JSON 文件
                        for word in &batch {
                            let word_lower = word.to_lowercase();
                            if let Some(word_data) = word_map.get(&word_lower).or_else(|| word_map.get(word)) {
                                let filename = format!("{}.json", word_lower);
                                let filepath = output_dir.join(&filename);

                                // 创建单个单词的 JSON 对象
                                let mut single_word_obj = serde_json::Map::new();
                                single_word_obj.insert(word_lower.clone(), word_data.clone());
                                let json_content = serde_json::to_string_pretty(&single_word_obj)
                                    .unwrap_or_else(|_| "{}".to_string());

                                match File::create(&filepath) {
                                    Ok(mut file) => {
                                        if file.write_all(json_content.as_bytes()).is_ok() {
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
                            } else {
                                // 单词不存在于 API 响应中，跳过
                                skip_count.fetch_add(1, Ordering::SeqCst);
                                println!("  [SKIP] {}", word);
                            }
                        }
                    }
                    Err(e) => {
                        // 批次处理失败，所有单词都标记为错误
                        let batch_size = batch.len();
                        error_count.fetch_add(batch_size, Ordering::SeqCst);
                        eprintln!(
                            "  [ERR] Batch failed ({} words) using {} - {}",
                            batch_size,
                            guard.name(),
                            e
                        );
                    }
                }
                // guard 在这里自动释放，归还模型槽位
            }
        })
        .buffer_unordered(concurrency)
        .collect::<Vec<()>>()
        .await;

    let final_success = success_count.load(Ordering::SeqCst);
    let final_skip = skip_count.load(Ordering::SeqCst);
    let final_errors = error_count.load(Ordering::SeqCst);

    println!("\n-----------------------------------------------");
    println!("Processed: {} words in {} batches", total_words, total_batches);
    println!("Success: {}", final_success);
    println!("Skipped: {}", final_skip);
    println!("Errors: {}", final_errors);

    Ok(())
}

async fn generate_batch_word_data(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    words: &[String],
) -> Result<serde_json::Map<String, serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
    let system_prompt = r#"You are a professional lexicographer with access to major English dictionaries including 牛津英语词典, 韦氏国际词典, 柯林斯 COBUILD 高阶英汉双解学习词典, 剑桥高级英语学习者词典, and 麦克米伦高阶英语词典.

Your task is to generate comprehensive dictionary entries in JSON format. For EACH word provided, you must include:
1. Word metadata (type, frequency, difficulty, syllable count)
2. Pronunciations (UK and US IPA)
3. Definitions in both English and Chinese
4. Word forms (plural, past tense, etc.)
5. Example sentences from various sources
6. Etymology information
7. Categories and tags
8. Related words (synonyms, antonyms, broader terms)
9. Frequency data from major corpora

IMPORTANT: You MUST return entries for ALL words provided. Each word should be a top-level key in the returned JSON object.
Always return valid JSON matching the exact structure provided in the example."#;

    let words_list = words.join("\", \"");

    let user_prompt = format!(
        r#"Generate complete dictionary entries for ALL of the following {count} words: ["{words_list}"]

Return a SINGLE JSON object where each word is a top-level key. Format:

{{
  "word1": {{ ... full entry ... }},
  "word2": {{ ... full entry ... }},
  ...
}}

For EACH word entry, include this structure:
{{
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

CRITICAL REQUIREMENTS:
- You MUST return entries for ALL {count} words: ["{words_list}"]
- Each word should be a lowercase top-level key in the JSON
- Provide accurate IPA pronunciations for both UK and US English
- Include at least 2-3 English definitions and their Chinese translations
- Add 3-5 example sentences with varying difficulty levels
- Only return valid JSON, no additional text or explanation"#,
        count = words.len(),
        words_list = words_list
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

                                match serde_json::from_str::<serde_json::Value>(content) {
                                    Ok(value) => {
                                        if let Some(obj) = value.as_object() {
                                            return Ok(obj.clone());
                                        } else {
                                            last_error = Some("Response is not a JSON object".to_string());
                                        }
                                    }
                                    Err(e) => {
                                        last_error = Some(format!("Invalid JSON response: {}", e));
                                    }
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
    println!("(Multi-Model Load Balancing Mode with Batch Processing)");
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
    println!("  --start-from <WORD>   Start processing from this word");
    println!("  --limit <N>           Limit number of words to process");
    println!("  --help                Show this help message");
    println!();
    println!("Batch Processing:");
    println!(
        "  Words are processed in batches of {} words per API request.",
        DEFAULT_BATCH_SIZE
    );
    println!("  Each word's result is saved to a separate JSON file.");
    println!();
    println!("Models:");
    println!("  This tool automatically uses all available general models with load balancing.");
    println!("  Models are selected based on their available concurrency ratio.");
    println!();
    println!("  Available models and their concurrency limits:");
    for (name, limit) in GENERAL_MODELS {
        println!("    {:<30} {:>5}", name, limit);
    }
    println!();
    println!(
        "  Total concurrency capacity: {}",
        GENERAL_MODELS.iter().map(|(_, c)| c).sum::<usize>()
    );
    println!();
    println!("Environment:");
    println!("  BIGMODEL_API_KEY      BigModel API key (required)");
    println!();
    println!("Examples:");
    println!("  cargo run --bin words --limit 10");
    println!("  cargo run --bin words --start-from apple --limit 100");
}
