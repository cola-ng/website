use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;

use horme::diesel_async::AsyncPgConnection;
use horme::prelude::*;
use std::result::Result;

const DEFAULT_WORDS_DIR: &str = "D:/Works/cola-ng/dict/final-en-dict";

#[derive(Debug)]
struct UpdateStats {
    files_processed: usize,
    words_updated: usize,
    words_not_found: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let mut words_dir = PathBuf::from(DEFAULT_WORDS_DIR);
    let mut limit: Option<usize> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--words-dir" => {
                if i + 1 < args.len() {
                    words_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --words-dir requires a path argument");
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

    println!("===============================================");
    println!("Update Words Memory Tool");
    println!("===============================================");
    println!("Words directory: {}", words_dir.display());
    if let Some(l) = limit {
        println!("Limit: {} files", l);
    }

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = AsyncPgConnection::establish(&database_url).await?;

    let start_time = Instant::now();

    let json_files = discover_json_files(&words_dir);
    let json_files: Vec<PathBuf> = if let Some(l) = limit {
        json_files.into_iter().take(l).collect()
    } else {
        json_files
    };
    println!("Found {} JSON files to process", json_files.len());

    let stats = update_memory_from_json(&mut conn, &json_files).await?;

    let elapsed = start_time.elapsed();

    println!("\n===============================================");
    println!("Update Summary");
    println!("===============================================");
    println!("Files processed: {}", stats.files_processed);
    println!("Words updated: {}", stats.words_updated);
    println!("Words not found: {}", stats.words_not_found);
    println!("Total time: {:.2}s", elapsed.as_secs_f64());

    Ok(())
}

fn discover_json_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "json" {
                        files.push(path);
                    }
                }
            }
        }
    }

    files.sort();
    files
}

async fn update_memory_from_json(
    conn: &mut AsyncPgConnection,
    json_files: &[PathBuf],
) -> Result<UpdateStats, Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::dict_words;

    let mut stats = UpdateStats {
        files_processed: 0,
        words_updated: 0,
        words_not_found: 0,
    };

    for json_file in json_files {
        stats.files_processed += 1;

        let file = match File::open(json_file) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("  Error opening {}: {}", json_file.display(), e);
                continue;
            }
        };
        let reader = BufReader::new(file);

        let json_value: serde_json::Value = match serde_json::from_reader(reader) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("  Error parsing {}: {}", json_file.display(), e);
                continue;
            }
        };

        let json_obj = match json_value.as_object() {
            Some(obj) => obj,
            None => {
                eprintln!("  Error: {} is not a JSON object", json_file.display());
                continue;
            }
        };

        for (word, entry_value) in json_obj {
            // Check if memory field exists and is not empty
            let memory = match entry_value.get("memory") {
                Some(serde_json::Value::String(s)) if !s.is_empty() => s.clone(),
                _ => continue, // Skip if no memory or empty
            };

            let word_lower = word.to_lowercase();

            // Update word in database
            let updated =
                diesel::update(dict_words::table.filter(dict_words::word_lower.eq(&word_lower)))
                    .set(dict_words::memory.eq(Some(&memory)))
                    .execute(conn)
                    .await?;

            if updated > 0 {
                stats.words_updated += 1;
                if stats.words_updated % 100 == 0 {
                    println!("  Progress: {} words updated", stats.words_updated);
                }
            } else {
                stats.words_not_found += 1;
                eprintln!("  Warning: word '{}' not found in database", word);
            }
        }
    }

    Ok(stats)
}

fn print_usage() {
    println!("Update Words Memory Tool");
    println!();
    println!("Updates the memory field of words in the database from JSON files.");
    println!();
    println!("Usage:");
    println!("  cargo run --bin update_words [OPTIONS]");
    println!();
    println!("Options:");
    println!(
        "  --words-dir <PATH>  Source directory with word JSON files (default: {})",
        DEFAULT_WORDS_DIR
    );
    println!("  --limit <N>         Limit number of files to process");
    println!("  --help              Show this help message");
    println!();
    println!("Environment:");
    println!("  DATABASE_URL        PostgreSQL connection URL");
    println!();
    println!("Examples:");
    println!("  cargo run --bin update_words");
    println!("  cargo run --bin update_words --limit 10");
    println!("  cargo run --bin update_words --words-dir /path/to/dict");
}
