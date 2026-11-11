//! Import read sentences data with TTS audio generation
//!
//! Reads read-sentences.json and:
//! 1. Generates WAV audio files for each sentence (if not exists)
//! 2. Inserts data into asset_read_subjects and asset_read_sentences tables

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use colang::db::schema::{asset_read_sentences, asset_read_subjects};
use horme::prelude::*;
use horme::diesel_async::AsyncPgConnection;
use futures_util::stream::{self, StreamExt};
use outfox_zhipu::Client as ZhipuClient;
use outfox_zhipu::spec::tts::CreateSpeechRequest;
use serde::{Deserialize, Serialize};
use std::result::Result;

const DEFAULT_INPUT_FILE: &str = r"D:\Works\cola-ng\space-raw\read-sentences.json";
const DEFAULT_OUTPUT_DIR: &str = r"D:\Works\cola-ng\space\read-sentences";
const DEFAULT_CONCURRENCY: usize = 5;
const DEFAULT_RETRY_COUNT: usize = 3;
const DEFAULT_RETRY_DELAY_MS: u64 = 1000;

// Default Zhipu voice
const DEFAULT_VOICE: &str = "tongtong";

// ============================================================================
// Data structures for JSON input
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReadSentencesData {
    subjects: Vec<SubjectData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubjectData {
    code: String,
    title_en: String,
    title_zh: String,
    description_en: Option<String>,
    description_zh: Option<String>,
    icon_emoji: Option<String>,
    difficulty: Option<i16>,
    subject_type: Option<String>,
    sentences: Vec<SentenceData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SentenceData {
    audio_name: String,
    content_en: String,
    content_zh: String,
    difficulty: Option<i16>,
    focus_sounds: Option<serde_json::Value>,
    common_mistakes: Option<serde_json::Value>,
}

// ============================================================================
// Database insertable structures
// ============================================================================

#[derive(Insertable)]
#[diesel(table_name = asset_read_subjects)]
struct NewReadSubject {
    code: String,
    title_en: String,
    title_zh: String,
    description_en: Option<String>,
    description_zh: Option<String>,
    icon_emoji: Option<String>,
    difficulty: Option<i16>,
    subject_type: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = asset_read_sentences)]
struct NewReadSentence {
    subject_id: i64,
    sentence_order: i32,
    content_en: String,
    content_zh: String,
    audio_path: Option<String>,
    difficulty: Option<i16>,
    focus_sounds: Option<serde_json::Value>,
    common_mistakes: Option<serde_json::Value>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let mut input_file = PathBuf::from(DEFAULT_INPUT_FILE);
    let mut output_dir = PathBuf::from(DEFAULT_OUTPUT_DIR);
    let mut concurrency = DEFAULT_CONCURRENCY;
    let mut voice = DEFAULT_VOICE.to_string();
    let mut skip_tts = false;
    let mut skip_db = false;

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
            "--skip-tts" => {
                skip_tts = true;
                i += 1;
            }
            "--skip-db" => {
                skip_db = true;
                i += 1;
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

    println!("===============================================");
    println!("Read Sentences Importer");
    println!("===============================================");
    println!("Input file: {}", input_file.display());
    println!("Output directory: {}", output_dir.display());
    println!("Voice: {}", voice);
    println!("Concurrency: {}", concurrency);
    println!("Skip TTS: {}", skip_tts);
    println!("Skip DB: {}", skip_db);
    println!();

    // Create output directory
    fs::create_dir_all(&output_dir)?;

    // Load existing audio files to skip
    let existing_files = load_existing_files(&output_dir)?;
    println!("Found {} existing audio files", existing_files.len());

    // Load data from JSON
    let content = fs::read_to_string(&input_file)?;
    let data: ReadSentencesData = serde_json::from_str(&content)?;
    println!("Loaded {} subjects from JSON", data.subjects.len());

    let total_sentences: usize = data.subjects.iter().map(|s| s.sentences.len()).sum();
    println!("Total sentences: {}", total_sentences);

    // Step 1: Generate TTS audio files
    if !skip_tts {
        println!("\n--- Phase 1: TTS Audio Generation ---\n");

        let api_key = std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY must be set");

        let client = ZhipuClient::with_config(
        outfox_zhipu::config::ZhipuConfig::new().with_api_key(api_key),
    );

        // Collect all sentences that need TTS
        let sentences_to_process: Vec<(String, String)> = data
            .subjects
            .iter()
            .flat_map(|subject| {
                subject.sentences.iter().filter_map(|sentence| {
                    if existing_files.contains(&sentence.audio_name) {
                        None
                    } else {
                        Some((sentence.audio_name.clone(), sentence.content_en.clone()))
                    }
                })
            })
            .collect();

        println!(
            "{} sentences need TTS generation (after filtering existing)",
            sentences_to_process.len()
        );

        if !sentences_to_process.is_empty() {
            // Run async processing
                process_tts_concurrent(
                    &client,
                    &sentences_to_process,
                    &output_dir,
                    &voice,
                    concurrency,
                )
                .await?;
        }
    }

    // Step 2: Insert into database
    if !skip_db {
        println!("\n--- Phase 2: Database Import ---\n");

        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let mut conn = AsyncPgConnection::establish(&database_url).await?;

        insert_to_database(&mut conn, &data, &output_dir).await?;
    }

    println!("\n===============================================");
    println!("Done!");
    println!("===============================================");

    Ok(())
}

fn load_existing_files(
    output_dir: &Path,
) -> Result<HashSet<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut existing = HashSet::new();

    if output_dir.exists() {
        for entry in fs::read_dir(output_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "wav" || ext == "mp3" {
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

async fn process_tts_concurrent(
    client: &ZhipuClient,
    sentences: &[(String, String)],
    output_dir: &Path,
    voice: &str,
    concurrency: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let total = sentences.len();
    let processed = Arc::new(AtomicUsize::new(0));
    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let client = Arc::new(client.clone());
    let output_dir = Arc::new(output_dir.to_path_buf());
    let voice = Arc::new(voice.to_string());

    println!(
        "Starting TTS generation with {} concurrency...\n",
        concurrency
    );

    stream::iter(sentences.iter().cloned())
        .map(|(audio_name, text)| {
            let client = Arc::clone(&client);
            let output_dir = Arc::clone(&output_dir);
            let voice = Arc::clone(&voice);
            let processed = Arc::clone(&processed);
            let success_count = Arc::clone(&success_count);
            let error_count = Arc::clone(&error_count);

            async move {
                let current = processed.fetch_add(1, Ordering::SeqCst) + 1;
                let filename = format!("{}.wav", audio_name);
                let filepath = output_dir.join(&filename);

                // Skip if file already exists and is non-empty
                if filepath.exists() {
                    if let Ok(metadata) = filepath.metadata() {
                        if metadata.len() > 0 {
                            success_count.fetch_add(1, Ordering::SeqCst);
                            println!(
                                "[{}/{}] {} ... SKIPPED (exists)",
                                current, total, audio_name
                            );
                            return;
                        }
                    }
                }

                print!("[{}/{}] {} ... ", current, total, audio_name);
                std::io::stdout().flush().ok();

                match generate_tts_with_retry(&client, &text, &voice).await {
                    Ok(audio_data) => match File::create(&filepath) {
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
                    },
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
    client: &ZhipuClient,
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

        let request = CreateSpeechRequest {
            input: text.to_string(),
            voice: serde_json::from_value(serde_json::json!(voice))?,
            ..Default::default()
        };

        match client.tts().create(request).await {
            Ok(response) => {
                return Ok(response.audio.to_vec());
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

async fn insert_to_database(
    conn: &mut AsyncPgConnection,
    data: &ReadSentencesData,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for subject_data in &data.subjects {
        println!("Processing subject: {} ...", subject_data.code);

        // Find all existing subjects that conflict with any unique field
        let conflicting_ids: Vec<i64> = asset_read_subjects::table
            .filter(
                asset_read_subjects::code
                    .eq(&subject_data.code)
                    .or(asset_read_subjects::title_en.eq(&subject_data.title_en))
                    .or(asset_read_subjects::title_zh.eq(&subject_data.title_zh)),
            )
            .select(asset_read_subjects::id)
            .load(conn)
            .await?;

        // Delete related sentences first (foreign key constraint)
        if !conflicting_ids.is_empty() {
            let deleted_sentences = diesel::delete(
                asset_read_sentences::table
                    .filter(asset_read_sentences::subject_id.eq_any(&conflicting_ids)),
            )
            .execute(conn)
            .await?;

            // Delete conflicting subjects
            let deleted_subjects = diesel::delete(
                asset_read_subjects::table.filter(asset_read_subjects::id.eq_any(&conflicting_ids)),
            )
            .execute(conn)
            .await?;

            println!(
                "  Deleted {} existing subject(s) and {} sentence(s)",
                deleted_subjects, deleted_sentences
            );
        }

        // Insert new subject
        let new_subject = NewReadSubject {
            code: subject_data.code.clone(),
            title_en: subject_data.title_en.clone(),
            title_zh: subject_data.title_zh.clone(),
            description_en: subject_data.description_en.clone(),
            description_zh: subject_data.description_zh.clone(),
            icon_emoji: subject_data.icon_emoji.clone(),
            difficulty: subject_data.difficulty,
            subject_type: subject_data.subject_type.clone(),
        };

        let subject_id: i64 = diesel::insert_into(asset_read_subjects::table)
            .values(&new_subject)
            .returning(asset_read_subjects::id)
            .get_result(conn)
            .await?;

        println!("  Created subject (id={})", subject_id);

        // Insert sentences
        let mut inserted_count = 0;
        for (order, sentence_data) in subject_data.sentences.iter().enumerate() {
            let audio_path = ["wav", "mp3"].into_iter().find_map(|extension| {
                let filename = format!("{}.{extension}", sentence_data.audio_name);
                output_dir.join(&filename).exists()
                    .then(|| format!("read-sentences/{filename}"))
            });

            let new_sentence = NewReadSentence {
                subject_id,
                sentence_order: (order + 1) as i32,
                content_en: sentence_data.content_en.clone(),
                content_zh: sentence_data.content_zh.clone(),
                audio_path,
                difficulty: sentence_data.difficulty,
                focus_sounds: sentence_data.focus_sounds.clone(),
                common_mistakes: sentence_data.common_mistakes.clone(),
            };

            diesel::insert_into(asset_read_sentences::table)
                .values(&new_sentence)
                .execute(conn)
                .await?;

            inserted_count += 1;
        }

        println!("  Inserted {} sentences", inserted_count);
    }

    Ok(())
}

fn print_usage() {
    println!("Read Sentences Importer");
    println!("Generates TTS audio and imports data to database");
    println!();
    println!("Usage:");
    println!("  cargo run --bin import_sentences [OPTIONS]");
    println!();
    println!("Options:");
    println!(
        "  -i, --input <PATH>       Input JSON file (default: {})",
        DEFAULT_INPUT_FILE
    );
    println!(
        "  -o, --output <PATH>      Output directory for WAV files (default: {})",
        DEFAULT_OUTPUT_DIR
    );
    println!(
        "  -c, --concurrency <N>    Number of concurrent TTS requests (default: {})",
        DEFAULT_CONCURRENCY
    );
    println!(
        "  -v, --voice <NAME>       TTS voice name (default: {})",
        DEFAULT_VOICE
    );
    println!("  --skip-tts               Skip TTS audio generation");
    println!("  --skip-db                Skip database import");
    println!("  -h, --help               Show this help message");
    println!();
    println!("Available English voices:");
    println!();
    println!("Available Chinese voices:");
    println!("  tongtong, chuichui, xiaochen, jam, kazi, douji, luodo");
    println!();
    println!("Environment Variables:");
    println!("  ZHIPU_API_KEY          Zhipu API key (required for TTS)");
    println!("  DATABASE_URL           PostgreSQL connection string (required for DB)");
    println!();
    println!("Examples:");
    println!("  cargo run --bin import_sentences");
    println!("  cargo run --bin import_sentences --skip-db");
    println!("  cargo run --bin import_sentences --skip-tts");
    println!("  cargo run --bin import_sentences -c 10");
}
