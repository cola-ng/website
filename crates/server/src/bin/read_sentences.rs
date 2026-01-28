//! Import read sentences from JSON to database
//!
//! Reads read-sentences.json and imports sentences into existing asset_read_subjects.
//! Sentences are randomly distributed among existing subjects.

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use colang::db::pool::DieselPool;
use diesel::prelude::*;
use rand::prelude::IndexedRandom;
use serde::{Deserialize, Serialize};

const DEFAULT_INPUT_FILE: &str = r"D:\Works\cola-ng\space\read-sentences.json";
const DEFAULT_OUTPUT_FILE: &str = r"D:\Works\cola-ng\space\read-sentences-full.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawEntry {
    word: String,
    sentence: String,
    chinese: String,
}

#[derive(Debug, Clone)]
struct SentenceEntry {
    content_en: String,
    content_zh: String,
    difficulty: Option<i16>,
}

// Export format structures
#[derive(Debug, Serialize)]
struct ExportData {
    sentences: Vec<ExportSentence>,
}

#[derive(Debug, Serialize)]
struct ExportSentence {
    content_en: String,
    content_zh: String,
    difficulty: Option<i16>,
    word: String,
}

#[derive(Debug)]
struct ImportStats {
    sentences_added: usize,
    sentences_skipped: usize,
    errors: usize,
}

#[derive(Debug, Clone)]
struct ExistingSubject {
    id: i64,
    code: String,
    max_order: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let mut input_file = PathBuf::from(DEFAULT_INPUT_FILE);
    let mut output_file = PathBuf::from(DEFAULT_OUTPUT_FILE);
    let mut limit: Option<usize> = None;
    let mut dry_run = false;
    let mut export_json = false;

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
                    output_file = PathBuf::from(&args[i + 1]);
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
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            "--export" => {
                export_json = true;
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
    println!("Read Sentences Import Tool");
    println!("===============================================");
    println!("Input file: {}", input_file.display());
    if export_json {
        println!("Mode: Export to JSON");
        println!("Output file: {}", output_file.display());
    } else {
        println!("Mode: Import to existing subjects (random distribution)");
        println!("Dry run: {}", dry_run);
    }
    if let Some(l) = limit {
        println!("Limit: {} sentences", l);
    }
    println!();

    // Load and parse JSON
    println!("Loading JSON file...");
    let content = fs::read_to_string(&input_file)?;
    let raw_entries: Vec<RawEntry> = serde_json::from_str(&content)?;
    println!("Loaded {} raw entries", raw_entries.len());

    // Convert to sentences
    let sentences: Vec<(String, SentenceEntry)> = raw_entries
        .into_iter()
        .map(|e| {
            (
                e.word.clone(),
                SentenceEntry {
                    content_en: e.sentence,
                    content_zh: e.chinese,
                    difficulty: Some(3),
                },
            )
        })
        .collect();

    // Apply limit
    let sentences: Vec<(String, SentenceEntry)> = if let Some(l) = limit {
        sentences.into_iter().take(l).collect()
    } else {
        sentences
    };

    println!("Will process {} sentences", sentences.len());

    // Export mode: write JSON format
    if export_json {
        return export_to_json(&sentences, &output_file);
    }

    // Connect to database
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = DieselPool::new(
        &database_url,
        &Default::default(),
        diesel::r2d2::Pool::builder(),
    )?;
    let mut conn = pool.get()?;

    // Load existing subjects
    println!("\nLoading existing subjects from database...");
    let subjects = load_existing_subjects(&mut conn)?;
    if subjects.is_empty() {
        eprintln!("Error: No existing subjects found in database. Please create subjects first.");
        std::process::exit(1);
    }
    println!("Found {} existing subjects", subjects.len());

    if dry_run {
        println!("\n--- DRY RUN: Preview ---");
        println!("Would distribute {} sentences among {} subjects", sentences.len(), subjects.len());
        println!("\nExisting subjects:");
        for (i, s) in subjects.iter().take(10).enumerate() {
            println!("  {}. {} (current max order: {})", i + 1, s.code, s.max_order);
        }
        if subjects.len() > 10 {
            println!("  ... and {} more", subjects.len() - 10);
        }
        println!("\nSample sentences to import:");
        for (i, (word, sent)) in sentences.iter().take(5).enumerate() {
            println!("  {}. [{}] {}", i + 1, word, sent.content_en);
        }
        if sentences.len() > 5 {
            println!("  ... and {} more", sentences.len() - 5);
        }
        println!("\nDry run complete. Use without --dry-run to import.");
        return Ok(());
    }

    let start_time = Instant::now();

    // Import sentences randomly into existing subjects
    let stats = import_sentences(&mut conn, &sentences, &subjects)?;

    let elapsed = start_time.elapsed();

    println!("\n===============================================");
    println!("Import Summary");
    println!("===============================================");
    println!("Sentences added: {}", stats.sentences_added);
    println!("Sentences skipped: {}", stats.sentences_skipped);
    println!("Errors: {}", stats.errors);
    println!("Total time: {:.2}s", elapsed.as_secs_f64());

    Ok(())
}

fn load_existing_subjects(
    conn: &mut PgConnection,
) -> Result<Vec<ExistingSubject>, Box<dyn std::error::Error>> {
    use colang::db::schema::{asset_read_sentences, asset_read_subjects};

    // Get all subjects
    let subjects: Vec<(i64, String)> = asset_read_subjects::table
        .select((asset_read_subjects::id, asset_read_subjects::code))
        .load(conn)?;

    // Get max sentence_order for each subject
    let mut result = Vec::new();
    for (id, code) in subjects {
        let max_order: Option<i32> = asset_read_sentences::table
            .filter(asset_read_sentences::subject_id.eq(id))
            .select(diesel::dsl::max(asset_read_sentences::sentence_order))
            .first(conn)?;

        result.push(ExistingSubject {
            id,
            code,
            max_order: max_order.unwrap_or(0),
        });
    }

    Ok(result)
}

fn import_sentences(
    conn: &mut PgConnection,
    sentences: &[(String, SentenceEntry)],
    subjects: &[ExistingSubject],
) -> Result<ImportStats, Box<dyn std::error::Error>> {
    use colang::db::schema::asset_read_sentences;

    let mut stats = ImportStats {
        sentences_added: 0,
        sentences_skipped: 0,
        errors: 0,
    };

    let mut rng = rand::rng();

    // Track current max order for each subject
    let mut subject_orders: std::collections::HashMap<i64, i32> = subjects
        .iter()
        .map(|s| (s.id, s.max_order))
        .collect();

    let total = sentences.len();

    for (idx, (_word, sentence)) in sentences.iter().enumerate() {
        if (idx + 1) % 500 == 0 || idx + 1 == total {
            println!("Progress: {}/{} sentences", idx + 1, total);
        }

        // Pick a random subject
        let subject = match subjects.choose(&mut rng) {
            Some(s) => s,
            None => {
                stats.errors += 1;
                continue;
            }
        };

        // Get next order for this subject
        let next_order = subject_orders.get(&subject.id).unwrap_or(&0) + 1;

        let result = diesel::insert_into(asset_read_sentences::table)
            .values((
                asset_read_sentences::subject_id.eq(subject.id),
                asset_read_sentences::sentence_order.eq(next_order),
                asset_read_sentences::content_en.eq(&sentence.content_en),
                asset_read_sentences::content_zh.eq(&sentence.content_zh),
                asset_read_sentences::difficulty.eq(sentence.difficulty),
            ))
            .execute(conn);

        match result {
            Ok(_) => {
                stats.sentences_added += 1;
                subject_orders.insert(subject.id, next_order);
            }
            Err(e) => {
                eprintln!("Error inserting sentence: {}", e);
                stats.errors += 1;
            }
        }
    }

    Ok(stats)
}

fn export_to_json(
    sentences: &[(String, SentenceEntry)],
    output_file: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let export_sentences: Vec<ExportSentence> = sentences
        .iter()
        .map(|(word, sent)| ExportSentence {
            content_en: sent.content_en.clone(),
            content_zh: sent.content_zh.clone(),
            difficulty: sent.difficulty,
            word: word.clone(),
        })
        .collect();

    let export_data = ExportData {
        sentences: export_sentences,
    };

    let json = serde_json::to_string_pretty(&export_data)?;

    let mut file = File::create(output_file)?;
    file.write_all(json.as_bytes())?;

    println!("Exported {} sentences to {}", sentences.len(), output_file.display());

    Ok(())
}

fn print_usage() {
    println!("Read Sentences Import Tool");
    println!();
    println!("Imports sentences into EXISTING subjects (randomly distributed).");
    println!("Subjects must already exist in the database.");
    println!();
    println!("Usage:");
    println!("  cargo run --bin read_sentences [OPTIONS]");
    println!();
    println!("Options:");
    println!(
        "  -i, --input <PATH>   Input JSON file (default: {})",
        DEFAULT_INPUT_FILE
    );
    println!(
        "  -o, --output <PATH>  Output JSON file for --export (default: {})",
        DEFAULT_OUTPUT_FILE
    );
    println!("  -l, --limit <N>      Limit number of sentences to process");
    println!("  --export             Export to JSON format instead of importing");
    println!("  --dry-run            Preview without importing to database");
    println!("  -h, --help           Show this help message");
    println!();
    println!("Environment:");
    println!("  DATABASE_URL         PostgreSQL connection URL (required for import)");
    println!();
    println!("Examples:");
    println!("  cargo run --bin read_sentences --dry-run         # Preview import");
    println!("  cargo run --bin read_sentences --limit 100       # Import first 100 sentences");
    println!("  cargo run --bin read_sentences --export          # Export to JSON format");
}
