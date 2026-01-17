// Dictionary Data Import Tool
//
// This script imports dictionary data from JSON format into the database.
// It parses JSON files from endict1 and populates the dict_ tables.
//
// Usage:
//   cargo run --bin import_dict_data [--source-dir PATH] [--audio-dir PATH] [--batch-size N]
//
// Data Source Format:
// Each line in dict/*.json files contains a JSON object with:
//   - word: The word/phrase
//   - sw: Search word (lowercase without spaces)
//   - definition: Array of English definitions
//   - translation: Array of Chinese translations
//   - pos: Part of speech
//   - exchange: Array of word forms (e.g., "s:schools", "p:schooled")
//   - examples: Array of example sentences
//   - phonetic: Phonetic transcription (IPA)

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Utc;
use colang::db::pool::DieselPool;
use diesel::prelude::*;
use serde::Deserialize;

// Default directories
const DEFAULT_SOURCE_DIR: &str = r"D:\Works\colang\endict1\dict";
const DEFAULT_AUDIO_DIR: &str = r"D:\Works\colang\data\audios";

// Default batch size for inserts
const DEFAULT_BATCH_SIZE: usize = 100;

/// JSON entry structure from source files
#[derive(Debug, Deserialize, Clone)]
struct JsonDictEntry {
    word: String,
    #[serde(rename = "sw")]
    search_word: String,
    definition: Vec<String>,
    translation: Vec<String>,
    #[serde(rename = "pos")]
    part_of_speech: Vec<String>,
    exchange: Vec<String>,
    examples: Vec<String>,
    phonetic: String,
}

/// Word forms extracted from exchange field
#[derive(Debug, Clone)]
struct Form {
    form_type: String,
    form: String,
}

/// Dictionary entry ready for import
#[derive(Debug, Clone)]
struct ImportEntry {
    word: String,
    word_lower: String,
    part_of_speech: Option<String>,
    phonetic: String,
    definitions_en: Vec<String>,
    definitions_zh: Vec<String>,
    word_forms: Vec<Form>,
    examples: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut source_dir = PathBuf::from(DEFAULT_SOURCE_DIR);
    let mut audio_dir = PathBuf::from(DEFAULT_AUDIO_DIR);
    let mut batch_size = DEFAULT_BATCH_SIZE;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--source-dir" => {
                if i + 1 < args.len() {
                    source_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --source-dir requires a path argument");
                    std::process::exit(1);
                }
            }
            "--audio-dir" => {
                if i + 1 < args.len() {
                    audio_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --audio-dir requires a path argument");
                    std::process::exit(1);
                }
            }
            "--batch-size" => {
                if i + 1 < args.len() {
                    batch_size = args[i + 1].parse().unwrap_or(DEFAULT_BATCH_SIZE);
                    i += 2;
                } else {
                    eprintln!("Error: --batch-size requires a number argument");
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
    println!("Dictionary Data Import Tool");
    println!("===============================================");
    println!("Source directory: {}", source_dir.display());
    println!("Audio directory: {}", audio_dir.display());
    println!("Batch size: {}", batch_size);
    println!();

    // Initialize database connection
    println!("Initializing database connection...");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = DieselPool::new(
        &database_url,
        &Default::default(),
        diesel::r2d2::Pool::builder(),
    )?;
    let mut conn = pool.get()?;

    // Discover JSON files
    println!("\nScanning for JSON files...");
    let json_files = discover_json_files(&source_dir);
    println!("Found {} JSON files", json_files.len());

    // Build audio file cache
    println!("\nBuilding audio file cache...");
    let audio_cache = build_audio_cache(&audio_dir);
    println!(
        "Found {} UK audio files, {} US audio files",
        audio_cache.uk.len(),
        audio_cache.us.len()
    );

    // Process all JSON files
    let mut total_entries = 0;
    let mut total_words = 0;
    let mut total_skipped = 0;
    let start_time = Instant::now();

    for json_file in &json_files {
        println!(
            "\nProcessing: {}",
            json_file.file_name().unwrap_or_default().to_string_lossy()
        );

        match import_json_file(&mut conn, json_file, &audio_cache, batch_size) {
            Ok(stats) => {
                println!(
                    "  Entries: {}, Added: {}, Skipped: {}",
                    stats.entries, stats.added, stats.skipped
                );
                total_entries += stats.entries;
                total_words += stats.added;
                total_skipped += stats.skipped;
            }
            Err(e) => {
                eprintln!("  Error: {}", e);
            }
        }
    }

    // Print summary
    let elapsed = start_time.elapsed();
    println!("\n===============================================");
    println!("Import Summary");
    println!("===============================================");
    println!("Total entries processed: {}", total_entries);
    println!("Total words added: {}", total_words);
    println!("Total words skipped: {}", total_skipped);
    println!("Total time: {:.2}s", elapsed.as_secs_f64());
    println!();

    Ok(())
}

/// Audio file cache for both UK and US pronunciations
#[derive(Debug, Clone)]
struct AudioCache {
    uk: HashMap<String, PathBuf>,
    us: HashMap<String, PathBuf>,
}

/// Import statistics
#[derive(Debug)]
struct ImportStats {
    entries: usize,
    added: usize,
    skipped: usize,
}

/// Discover all JSON files in the source directory
fn discover_json_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
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

/// Build a cache of available audio files
fn build_audio_cache(audio_dir: &Path) -> AudioCache {
    let mut uk = HashMap::new();
    let mut us = HashMap::new();

    // Scan UK audio files
    let uk_dir = audio_dir.join("uk");
    if let Ok(entries) = fs::read_dir(&uk_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(stem) = path.file_stem() {
                    let word = stem.to_string_lossy().to_string();
                    uk.insert(word, path.clone());
                }
            }
        }
    }

    // Scan US audio files
    let us_dir = audio_dir.join("us");
    if let Ok(entries) = fs::read_dir(&us_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(stem) = path.file_stem() {
                    let word = stem.to_string_lossy().to_string();
                    us.insert(word, path.clone());
                }
            }
        }
    }

    AudioCache { uk, us }
}

/// Import a single JSON file
fn import_json_file(
    conn: &mut PgConnection,
    json_file: &Path,
    audio_cache: &AudioCache,
    batch_size: usize,
) -> Result<ImportStats, Box<dyn std::error::Error>> {
    use colang::db::schema::*;

    let file = File::open(json_file)?;
    let reader = BufReader::new(file);

    let mut entries = Vec::new();
    let mut line_number = 0;

    // Parse JSON lines
    for line in reader.lines() {
        line_number += 1;
        let line = line?;

        // Skip empty lines
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse JSON
        match serde_json::from_str::<JsonDictEntry>(line) {
            Ok(json_entry) => {
                if let Some(import_entry) = process_json_entry(json_entry) {
                    entries.push(import_entry);
                }
            }
            Err(e) => {
                eprintln!("  Warning: Failed to parse line {}: {}", line_number, e);
            }
        }
    }

    let total_entries = entries.len();

    // Process in batches
    let mut added = 0;
    let mut skipped = 0;

    for chunk in entries.chunks(batch_size) {
        conn.transaction::<_, Box<dyn std::error::Error>, _>(|conn| {
            for entry in chunk {
                // Check if word already exists
                let existing: Option<i64> = dict_words::table
                    .select(dict_words::id)
                    .filter(dict_words::word_lower.eq(&entry.word_lower))
                    .first(conn)
                    .optional()?;

                if let Some(word_id) = existing {
                    // Word exists, skip
                    skipped += 1;
                } else {
                    // Insert new word
                    let word_id = insert_word(conn, &entry)?;

                    // Insert definitions
                    insert_definitions(conn, word_id, &entry)?;

                    // Insert word forms
                    insert_word_forms(conn, word_id, &entry.word_forms)?;

                    // Insert pronunciations
                    insert_pronunciations(conn, word_id, &entry, audio_cache)?;

                    added += 1;
                }
            }
            Ok(())
        })?;
    }

    Ok(ImportStats {
        entries: total_entries,
        added,
        skipped,
    })
}

/// Process a JSON entry into an import entry
fn process_json_entry(json_entry: JsonDictEntry) -> Option<ImportEntry> {
    let word = json_entry.word.trim();
    let word_lower = json_entry.search_word.trim();

    // Skip invalid words
    if word.is_empty() || word.len() > 200 {
        return None;
    }

    // Extract part of speech (use first one if multiple)
    let part_of_speech = json_entry
        .part_of_speech
        .first()
        .map(|p| normalize_part_of_speech(p))
        .flatten();

    // Parse word forms from exchange field
    let word_forms = parse_exchange_forms(&json_entry.exchange);

    Some(ImportEntry {
        word: word.to_string(),
        word_lower: word_lower.to_string(),
        part_of_speech,
        phonetic: json_entry.phonetic,
        definitions_en: json_entry.definition,
        definitions_zh: json_entry.translation,
        word_forms,
        examples: json_entry.examples,
    })
}

/// Normalize part of speech to database values
fn normalize_part_of_speech(pos: &str) -> Option<String> {
    let pos_lower = pos.to_lowercase();
    match pos_lower.as_str() {
        "n." | "n" | "noun" => Some("noun".to_string()),
        "v." | "v" | "verb" => Some("verb".to_string()),
        "adj." | "adj" | "a." | "a" | "adjective" => Some("adjective".to_string()),
        "adv." | "adv" | "adverb" => Some("adverb".to_string()),
        "pron." | "pron" | "pronoun" => Some("pronoun".to_string()),
        "prep." | "prep" | "preposition" => Some("preposition".to_string()),
        "conj." | "conj" | "conjunction" => Some("conjunction".to_string()),
        "int." | "interjection" => Some("interjection".to_string()),
        "art." | "article" => Some("article".to_string()),
        "abbr." | "abbreviation" => Some("abbreviation".to_string()),
        "phrase" => Some("phrase".to_string()),
        "idiom" => Some("idiom".to_string()),
        _ => None,
    }
}

/// Parse exchange field to extract word forms
/// Exchange format: ["s:schools", "p:schooled", "d:schooled", "3:schools"]
/// Prefixes: 0=third person, 1=past, 2=past participle, 3=plural, 4=present participle,
///           5=comparative, 6=superlative, s=plural, p=past participle, i=present participle,
/// d=past
fn parse_exchange_forms(exchange: &[String]) -> Vec<Form> {
    let mut forms = Vec::new();

    for item in exchange {
        if let Some(colon_pos) = item.find(':') {
            let prefix = &item[..colon_pos];
            let form = &item[colon_pos + 1..];

            if form.is_empty() {
                continue;
            }

            let form_type = match prefix {
                "0" => "present",                  // Third person singular
                "1" | "d" => "past",               // Past tense
                "2" | "p" => "past_participle",    // Past participle
                "3" | "s" => "plural",             // Plural
                "4" | "i" => "present_participle", // Present participle
                "5" => "comparative",              // Comparative
                "6" => "superlative",              // Superlative
                _ => continue,                     // Skip unknown prefixes
            };

            forms.push(Form {
                form_type: form_type.to_string(),
                form: form.to_string(),
            });
        }
    }

    forms
}

/// Insert a word into the database
fn insert_word(
    conn: &mut PgConnection,
    entry: &ImportEntry,
) -> Result<i64, Box<dyn std::error::Error>> {
    use colang::db::schema::dict_words;

    // Determine word type based on whether it contains spaces
    let word_type = if entry.word.contains(' ') {
        Some("phrase".to_string())
    } else {
        entry.part_of_speech.clone()
    };

    let word_id: i64 = diesel::insert_into(dict_words::table)
        .values((
            dict_words::word.eq(&entry.word),
            dict_words::word_lower.eq(&entry.word_lower),
            dict_words::word_type.eq(word_type),
            dict_words::language.eq("en"),
            dict_words::is_lemma.eq(true),
            dict_words::word_count.eq(1),
            dict_words::is_active.eq(true),
        ))
        .returning(dict_words::id)
        .get_result(conn)?;

    Ok(word_id)
}

/// Insert definitions for a word
fn insert_definitions(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &ImportEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::dict_definitions;

    let mut definition_order = 1;

    // Insert English definitions
    for definition in &entry.definitions_en {
        if !definition.is_empty() {
            diesel::insert_into(dict_definitions::table)
                .values((
                    dict_definitions::word_id.eq(word_id),
                    dict_definitions::language.eq("en"),
                    dict_definitions::definition.eq(definition),
                    dict_definitions::part_of_speech.eq(entry.part_of_speech.clone()),
                    dict_definitions::definition_order.eq(definition_order),
                    dict_definitions::is_primary.eq(definition_order == 1),
                ))
                .execute(conn)?;
            definition_order += 1;
        }
    }

    // Insert Chinese translations
    for translation in &entry.definitions_zh {
        if !translation.is_empty() {
            diesel::insert_into(dict_definitions::table)
                .values((
                    dict_definitions::word_id.eq(word_id),
                    dict_definitions::language.eq("zh"),
                    dict_definitions::definition.eq(translation),
                    dict_definitions::part_of_speech.eq(entry.part_of_speech.clone()),
                    dict_definitions::definition_order.eq(definition_order),
                    dict_definitions::is_primary.eq(false),
                ))
                .execute(conn)?;
            definition_order += 1;
        }
    }

    Ok(())
}

/// Insert word forms
fn insert_word_forms(
    conn: &mut PgConnection,
    word_id: i64,
    forms: &[Form],
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::dict_forms;

    for word_form in forms {
        diesel::insert_into(dict_forms::table)
            .values((
                dict_forms::word_id.eq(word_id),
                dict_forms::form_type.eq(&word_form.form_type),
                dict_forms::form.eq(&word_form.form),
                dict_forms::is_irregular.eq(false),
            ))
            .execute(conn)?;
    }

    Ok(())
}

/// Insert pronunciations for a word
fn insert_pronunciations(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &ImportEntry,
    audio_cache: &AudioCache,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::dict_pronunciations;

    let mut has_primary = false;

    // Check for UK pronunciation
    if let Some(uk_audio_path) = audio_cache.uk.get(&entry.word_lower) {
        let ipa = if !entry.phonetic.is_empty() {
            entry.phonetic.clone()
        } else {
            format!("[UK: {}]", entry.word)
        };

        diesel::insert_into(dict_pronunciations::table)
            .values((
                dict_pronunciations::word_id.eq(word_id),
                dict_pronunciations::ipa.eq(ipa),
                dict_pronunciations::audio_path.eq(uk_audio_path.to_str()),
                dict_pronunciations::dialect.eq("UK"),
                dict_pronunciations::is_primary.eq(!has_primary),
            ))
            .execute(conn)?;

        has_primary = true;
    }

    // Check for US pronunciation
    if let Some(us_audio_path) = audio_cache.us.get(&entry.word_lower) {
        let ipa = if !entry.phonetic.is_empty() {
            entry.phonetic.clone()
        } else {
            format!("[US: {}]", entry.word)
        };

        diesel::insert_into(dict_pronunciations::table)
            .values((
                dict_pronunciations::word_id.eq(word_id),
                dict_pronunciations::ipa.eq(ipa),
                dict_pronunciations::audio_path.eq(us_audio_path.to_str()),
                dict_pronunciations::dialect.eq("US"),
                dict_pronunciations::is_primary.eq(!has_primary),
            ))
            .execute(conn)?;

        has_primary = true;
    }

    // If no audio file but has phonetic, still add a pronunciation entry
    if !has_primary && !entry.phonetic.is_empty() {
        diesel::insert_into(dict_pronunciations::table)
            .values((
                dict_pronunciations::word_id.eq(word_id),
                dict_pronunciations::ipa.eq(&entry.phonetic),
                dict_pronunciations::dialect.eq("other" as &str),
                dict_pronunciations::is_primary.eq(true),
            ))
            .execute(conn)?;
    }

    Ok(())
}

/// Print usage information
fn print_usage() {
    println!("Dictionary Data Import Tool");
    println!();
    println!("Usage:");
    println!("  cargo run --bin import_dict_data [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --source-dir <PATH>  Source directory with JSON files");
    println!("                      (default: {})", DEFAULT_SOURCE_DIR);
    println!("  --audio-dir <PATH>   Audio directory with uk/us subdirs");
    println!("                      (default: {})", DEFAULT_AUDIO_DIR);
    println!(
        "  --batch-size <N>     Batch size for inserts (default: {})",
        DEFAULT_BATCH_SIZE
    );
    println!("  --help               Show this help message");
    println!();
    println!("Environment:");
    println!("  DATABASE_URL         PostgreSQL connection URL");
}
