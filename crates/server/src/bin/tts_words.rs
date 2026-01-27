//! Generate TTS audio files for word sentences using Doubao TTS API
//!
//! Reads word-s.json and generates MP3 audio files for each sentence.
//! Output files are saved to the word-stences directory.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream::{self, StreamExt};
use outfox_doubao::config::DoubaoConfig;
use outfox_doubao::spec::tts::CreateSpeechRequestArgs;
use outfox_doubao::Client as DoubaoClient;
use serde::{Deserialize, Serialize};

const DEFAULT_INPUT_FILE: &str = r"D:\Works\cola-ng\movies\word-s.json";
const DEFAULT_OUTPUT_DIR: &str = r"D:\Works\cola-ng\movies\word-stences";
const DEFAULT_CONCURRENCY: usize = 5;
const DEFAULT_RETRY_COUNT: usize = 3;
const DEFAULT_RETRY_DELAY_MS: u64 = 1000;

// TTS voice options
const DEFAULT_VOICE: &str = "zh_female_vv_uranus_bigtts";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WordEntry {
    word: String,
    sentence: String,
    chinese: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let mut input_file = PathBuf::from(DEFAULT_INPUT_FILE);
    let mut output_dir = PathBuf::from(DEFAULT_OUTPUT_DIR);
    let mut limit: Option<usize> = None;
    let mut concurrency = DEFAULT_CONCURRENCY;
    let mut voice = DEFAULT_VOICE.to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" | "-i" => {
                if i + 1 < args.len() {
                    input_file = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --input requires a path argument");
                    std::process::exit(1);
                }
            }
            "--output" | "-o" => {
                if i + 1 < args.len() {
                    output_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --output requires a path argument");
                    std::process::exit(1);
                }
            }
            "--limit" | "-l" => {
                if i + 1 < args.len() {
                    limit = Some(args[i + 1].parse().unwrap_or(100));
                    i += 2;
                } else {
                    eprintln!("Error: --limit requires a number argument");
                    std::process::exit(1);
                }
            }
            "--concurrency" | "-c" => {
                if i + 1 < args.len() {
                    concurrency = args[i + 1].parse().unwrap_or(DEFAULT_CONCURRENCY);
                    i += 2;
                } else {
                    eprintln!("Error: --concurrency requires a number argument");
                    std::process::exit(1);
                }
            }
            "--voice" | "-v" => {
                if i + 1 < args.len() {
                    voice = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --voice requires a voice name argument");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
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

    // Load Doubao credentials from environment
    let app_id = std::env::var("DOUBAO_APP_ID").expect("DOUBAO_APP_ID must be set");
    let access_token =
        std::env::var("DOUBAO_ACCESS_TOKEN").expect("DOUBAO_ACCESS_TOKEN must be set");
    let resource_id = std::env::var("DOUBAO_RESOURCE_ID").unwrap_or_else(|_| "seed-tts-2.0".to_string());

    println!("===============================================");
    println!("TTS Generator for Word Sentences");
    println!("Using Doubao TTS API");
    println!("===============================================");
    println!("Input file: {}", input_file.display());
    println!("Output directory: {}", output_dir.display());
    println!("Voice: {}", voice);
    println!("Concurrency: {}", concurrency);
    if let Some(l) = limit {
        println!("Limit: {} words", l);
    }
    println!();

    // Create output directory
    fs::create_dir_all(&output_dir)?;

    // Load existing files to skip
    let existing_files = load_existing_files(&output_dir)?;
    println!("Found {} existing audio files", existing_files.len());

    // Load word entries from JSON
    let content = fs::read_to_string(&input_file)?;
    let all_entries: Vec<WordEntry> = serde_json::from_str(&content)?;
    println!("Loaded {} word entries from JSON", all_entries.len());

    // Filter out already processed entries
    let entries_to_process: Vec<WordEntry> = all_entries
        .into_iter()
        .filter(|e| !existing_files.contains(&sanitize_filename(&e.word)))
        .collect();

    println!(
        "{} entries need processing (after filtering existing)",
        entries_to_process.len()
    );

    // Apply limit if specified
    let entries_to_process: Vec<WordEntry> = if let Some(l) = limit {
        entries_to_process.into_iter().take(l).collect()
    } else {
        entries_to_process
    };

    if entries_to_process.is_empty() {
        println!("No entries to process.");
        return Ok(());
    }

    println!("Will process {} entries\n", entries_to_process.len());

    // Create Doubao client
    let config = DoubaoConfig::new()
        .with_app_id(&app_id)
        .with_access_token(&access_token)
        .with_resource_id(&resource_id)
        .with_voice_type(&voice);

    let client = DoubaoClient::with_config(config);

    // Run async processing
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        process_entries_concurrent(&client, &entries_to_process, &output_dir, &voice, concurrency)
            .await
    })?;

    println!("\n===============================================");
    println!("Done!");
    println!("===============================================");

    Ok(())
}

fn load_existing_files(output_dir: &Path) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let mut existing = HashSet::new();

    if output_dir.exists() {
        for entry in fs::read_dir(output_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "mp3" {
                        // Check if file is non-empty
                        if let Ok(metadata) = path.metadata() {
                            if metadata.len() > 0 {
                                if let Some(stem) = path.file_stem() {
                                    existing.insert(stem.to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(existing)
}

fn sanitize_filename(word: &str) -> String {
    // Replace characters that are invalid in filenames
    word.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

async fn process_entries_concurrent(
    client: &DoubaoClient,
    entries: &[WordEntry],
    output_dir: &Path,
    voice: &str,
    concurrency: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let total = entries.len();
    let processed = Arc::new(AtomicUsize::new(0));
    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let client = Arc::new(client.clone());
    let output_dir = Arc::new(output_dir.to_path_buf());
    let voice = Arc::new(voice.to_string());

    println!("Starting TTS generation with {} concurrency...\n", concurrency);

    stream::iter(entries.iter().cloned())
        .map(|entry| {
            let client = Arc::clone(&client);
            let output_dir = Arc::clone(&output_dir);
            let voice = Arc::clone(&voice);
            let processed = Arc::clone(&processed);
            let success_count = Arc::clone(&success_count);
            let error_count = Arc::clone(&error_count);

            async move {
                let current = processed.fetch_add(1, Ordering::SeqCst) + 1;
                let filename = format!("{}.mp3", sanitize_filename(&entry.word));
                let filepath = output_dir.join(&filename);

                // Skip if file already exists and is non-empty
                if filepath.exists() {
                    if let Ok(metadata) = filepath.metadata() {
                        if metadata.len() > 0 {
                            success_count.fetch_add(1, Ordering::SeqCst);
                            println!("[{}/{}] {} ... SKIPPED (exists)", current, total, entry.word);
                            return;
                        }
                    }
                }

                print!("[{}/{}] {} ... ", current, total, entry.word);
                std::io::stdout().flush().ok();

                match generate_tts_with_retry(&client, &entry.sentence, &voice).await {
                    Ok(audio_data) => {
                        match File::create(&filepath) {
                            Ok(mut file) => {
                                if file.write_all(&audio_data).is_ok() {
                                    success_count.fetch_add(1, Ordering::SeqCst);
                                    println!("OK ({} bytes)", audio_data.len());
                                } else {
                                    error_count.fetch_add(1, Ordering::SeqCst);
                                    println!("FAILED (write error)");
                                }
                            }
                            Err(e) => {
                                error_count.fetch_add(1, Ordering::SeqCst);
                                println!("FAILED (create file: {})", e);
                            }
                        }
                    }
                    Err(e) => {
                        error_count.fetch_add(1, Ordering::SeqCst);
                        println!("FAILED ({})", e);
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
    println!("Total processed: {}", total);
    println!("Success: {}", final_success);
    println!("Errors: {}", final_errors);

    Ok(())
}

async fn generate_tts_with_retry(
    client: &DoubaoClient,
    text: &str,
    voice: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut last_error = None;

    for attempt in 0..DEFAULT_RETRY_COUNT {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(
                DEFAULT_RETRY_DELAY_MS * (attempt as u64 + 1),
            ))
            .await;
        }

        let request = match CreateSpeechRequestArgs::default()
            .text(text)
            .speaker(voice)
            .speech_rate(0i32) // Normal speed
            .sample_rate(48000u32)
            .build()
        {
            Ok(req) => req,
            Err(e) => {
                last_error = Some(format!("Failed to build request: {}", e));
                continue;
            }
        };

        match client.tts().speech_http_v3().create(request).await {
            Ok(response) => {
                return Ok(response.bytes.to_vec());
            }
            Err(e) => {
                last_error = Some(format!("TTS API error: {}", e));
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| "Unknown error".to_string())
        .into())
}

fn print_usage() {
    println!("TTS Generator for Word Sentences");
    println!("Using Doubao TTS API");
    println!();
    println!("Usage:");
    println!("  cargo run --bin tts_words [OPTIONS]");
    println!();
    println!("Options:");
    println!(
        "  -i, --input <PATH>       Input JSON file (default: {})",
        DEFAULT_INPUT_FILE
    );
    println!(
        "  -o, --output <PATH>      Output directory for MP3 files (default: {})",
        DEFAULT_OUTPUT_DIR
    );
    println!("  -l, --limit <N>          Limit number of entries to process");
    println!(
        "  -c, --concurrency <N>    Number of concurrent requests (default: {})",
        DEFAULT_CONCURRENCY
    );
    println!(
        "  -v, --voice <NAME>       TTS voice name (default: {})",
        DEFAULT_VOICE
    );
    println!("  -h, --help               Show this help message");
    println!();
    println!("Available voices:");
    println!("  zh_female_cancan_mars_bigtts");
    println!("  zh_female_shuangkuaisisi_moon_bigtts");
    println!("  zh_male_aojiaobazong_moon_bigtts");
    println!("  zh_female_tianmeixiaoyuan_moon_bigtts");
    println!("  zh_male_wennuanahu_moon_bigtts");
    println!("  zh_female_vv_uranus_bigtts");
    println!("  en_male_adam_moon_bigtts (default for English)");
    println!();
    println!("Environment Variables:");
    println!("  DOUBAO_APP_ID          Doubao App ID (required)");
    println!("  DOUBAO_ACCESS_TOKEN    Doubao Access Token (required)");
    println!("  DOUBAO_RESOURCE_ID     TTS Resource ID (default: seed-tts-2.0)");
    println!();
    println!("Examples:");
    println!("  cargo run --bin tts_words --limit 10");
    println!("  cargo run --bin tts_words --voice zh_female_cancan_mars_bigtts");
    println!("  cargo run --bin tts_words -c 10 -l 100");
}
