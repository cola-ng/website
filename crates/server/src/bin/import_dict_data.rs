// Dictionary Data Import Tool
//
// This script imports dictionary data from various sources into the database.
// It parses multiple dictionary formats and populates the dict_ tables.
//
// Usage:
//   cargo run --bin import_dict_data [--dict-dir PATH] [--batch-size N]
//
// Dictionary Sources:
// - 英汉词根辞典(李平武+蒋真,7291条)汇总.txt - Tab-separated format
// - Youdict优词英语词根词源词典(18677条).txt - Space-separated format
// - Etym Word Origins Dictionary词源词典46104条.txt - Multi-line format
// - 英语词根词源记忆词典(31503条).txt - Etymology format
// - 简明英汉字典增强版(3407926条).txt - Large dictionary
// - Various other specialized dictionaries

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Utc;
use diesel::prelude::*;
use diesel::PgConnection;
use regex::Regex;use diesel::r2d2::Pool;

use colang::db::pool::DieselPool;
use colang::models::dict::{
    NewDictWord, NewDictWordDefinition, NewDictWordEtymology, NewDictWordExample,
};

// Default dictionary directory
const DEFAULT_DICT_DIR: &str = r"D:\Works\colang\dicts";

// Default batch size for inserts
const DEFAULT_BATCH_SIZE: usize = 100;

/// Dictionary entry parsed from source files
#[derive(Debug, Clone)]
struct DictEntry {
    word: String,
    word_lower: String,
    definition_en: Option<String>,
    definition_zh: Option<String>,
    etymology_en: Option<String>,
    etymology_zh: Option<String>,
    origin_language: Option<String>,
    origin_word: Option<String>,
    origin_meaning: Option<String>,
    example_en: Option<String>,
    example_zh: Option<String>,
    part_of_speech: Option<String>,
    source: String,
}

/// Dictionary file format
#[derive(Debug, Clone, Copy)]
enum DictFormat {
    TabSeparated,   // word\tetymology/definition
    SpaceSeparated, // word [spaces] word [spaces] definition
    MultiLine,      // word followed by multi-line etymology
    Simple,         // Simple word:definition format
}

/// Dictionary file metadata
#[derive(Debug, Clone)]
struct DictFile {
    path: PathBuf,
    format: DictFormat,
    name: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut dict_dir = PathBuf::from(DEFAULT_DICT_DIR);
    let mut batch_size = DEFAULT_BATCH_SIZE;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dict-dir" => {
                if i + 1 < args.len() {
                    dict_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --dict-dir requires a path argument");
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
    println!("Dictionary directory: {}", dict_dir.display());
    println!("Batch size: {}", batch_size);
    println!();

    // Initialize database connection
    println!("Initializing database connection...");
    let pool = DieselPool::new(
        &std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        &Default::default(),
        diesel::r2d2::Pool::builder(),
    )?;
    let mut conn = pool.get()?;

    // Discover dictionary files
    println!("\nScanning for dictionary files...");
    let dict_files = discover_dict_files(&dict_dir);
    println!("Found {} dictionary files:", dict_files.len());
    for file in &dict_files {
        println!("  - {} ({:?})", file.name, file.format);
    }

    // Import dictionaries
    let mut total_entries = 0;
    let mut total_words = 0;
    let start_time = Instant::now();

    for dict_file in &dict_files {
        println!("\n===============================================");
        println!("Processing: {}", dict_file.name);
        println!("===============================================");

        match import_dictionary(&mut conn, dict_file, batch_size) {
            Ok(stats) => {
                println!("Import completed:");
                println!("  - Entries processed: {}", stats.entries);
                println!("  - Words added: {}", stats.words_added);
                println!("  - Words skipped: {}", stats.words_skipped);
                total_entries += stats.entries;
                total_words += stats.words_added;
            }
            Err(e) => {
                eprintln!("Error importing {}: {}", dict_file.name, e);
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
    println!("Total time: {:.2}s", elapsed.as_secs_f64());
    println!();

    Ok(())
}

/// Import statistics
#[derive(Debug)]
struct ImportStats {
    entries: usize,
    words_added: usize,
    words_skipped: usize,
}

/// Discover dictionary files in the specified directory
fn discover_dict_files(dir: &Path) -> Vec<DictFile> {
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Determine format based on filename
                let format = if name.contains("英汉词根辞典") || name.contains("汇总") {
                    DictFormat::TabSeparated
                } else if name.contains("Youdict") || name.contains("优词") {
                    DictFormat::SpaceSeparated
                } else if name.contains("Etym") || name.contains("词源词典") {
                    DictFormat::MultiLine
                } else if name.contains("简明英汉字典") || name.contains("增强版") {
                    DictFormat::Simple
                } else {
                    // Try to detect format from content
                    detect_format_from_content(&path)
                };

                files.push(DictFile {
                    path,
                    format,
                    name,
                });
            }
        }
    }

    files
}

/// Detect dictionary format from file content
fn detect_format_from_content(path: &Path) -> DictFormat {
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        for line in reader.lines().take(20).flatten() {
            if line.contains('\t') {
                return DictFormat::TabSeparated;
            }
        }
    }
    DictFormat::Simple
}

/// Import a single dictionary file
fn import_dictionary(
    conn: &mut PgConnection,
    dict_file: &DictFile,
    batch_size: usize,
) -> Result<ImportStats, Box<dyn std::error::Error>> {
    use colang::db::schema::dict_words;

    let file = File::open(&dict_file.path)?;
    let reader = BufReader::new(file);

    let mut entries = Vec::new();
    let mut stats = ImportStats {
        entries: 0,
        words_added: 0,
        words_skipped: 0,
    };

    // Parse entries based on format
    match dict_file.format {
        DictFormat::TabSeparated => {
            entries = parse_tab_separated(reader, &dict_file.name)?;
        }
        DictFormat::SpaceSeparated => {
            entries = parse_space_separated(reader, &dict_file.name)?;
        }
        DictFormat::MultiLine => {
            entries = parse_multiline(reader, &dict_file.name)?;
        }
        DictFormat::Simple => {
            entries = parse_simple(reader, &dict_file.name)?;
        }
    }

    stats.entries = entries.len();

    // Process in batches
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
                    // Word exists, update etymology if missing
                    if let Some(etymology_zh) = &entry.etymology_zh {
                        use colang::db::schema::dict_word_etymology;

                        let has_etymology: Option<i64> = dict_word_etymology::table
                            .select(dict_word_etymology::id)
                            .filter(dict_word_etymology::word_id.eq(word_id))
                            .first(conn)
                            .optional()?;

                        if has_etymology.is_none() {
                            let new_etymology = NewDictWordEtymology {
                                word_id,
                                origin_language: entry.origin_language.clone(),
                                origin_word: entry.origin_word.clone(),
                                origin_meaning: entry.origin_meaning.clone(),
                                etymology_en: entry.etymology_en.clone(),
                                etymology_zh: Some(etymology_zh.clone()),
                                first_attested_year: None,
                                historical_forms: None,
                                cognate_words: None,
                            };

                            diesel::insert_into(dict_word_etymology::table)
                                .values(&new_etymology)
                                .execute(conn)?;
                        }
                    }
                    stats.words_skipped += 1;
                } else {
                    // Insert new word
                    let new_word = NewDictWord {
                        word: entry.word.clone(),
                        word_lower: entry.word_lower.clone(),
                        word_type: entry.part_of_speech.clone(),
                        frequency_score: None,
                        difficulty_level: None,
                        syllable_count: None,
                        is_lemma: Some(true),
                        lemma_id: None,
                        audio_url: None,
                        audio_path: None,
                        phonetic_transcription: None,
                        ipa_text: None,
                        word_count: Some(1),
                        is_active: Some(true),
                        created_by: None,
                        updated_by: None,
                    };

                    let word_id: i64 = diesel::insert_into(dict_words::table)
                        .values(&new_word)
                        .returning(dict_words::id)
                        .get_result(conn)?;

                    // Insert definition if available
                    if let Some(definition_en) = &entry.definition_en {
                        let new_definition = NewDictWordDefinition {
                            word_id,
                            definition_en: definition_en.clone(),
                            definition_zh: entry.definition_zh.clone(),
                            part_of_speech: entry.part_of_speech.clone(),
                            definition_order: Some(1),
                            register: None,
                            region: None,
                            context: None,
                            usage_notes: None,
                            is_primary: Some(true),
                        };

                        diesel::insert_into(colang::db::schema::dict_word_definitions::table)
                            .values(&new_definition)
                            .execute(conn)?;
                    }

                    // Insert etymology if available
                    if let Some(etymology_zh) = &entry.etymology_zh {
                        let new_etymology = NewDictWordEtymology {
                            word_id,
                            origin_language: entry.origin_language.clone(),
                            origin_word: entry.origin_word.clone(),
                            origin_meaning: entry.origin_meaning.clone(),
                            etymology_en: entry.etymology_en.clone(),
                            etymology_zh: Some(etymology_zh.clone()),
                            first_attested_year: None,
                            historical_forms: None,
                            cognate_words: None,
                        };

                        diesel::insert_into(colang::db::schema::dict_word_etymology::table)
                            .values(&new_etymology)
                            .execute(conn)?;
                    }

                    // Insert example if available
                    if let Some(example_en) = &entry.example_en {
                        let new_example = NewDictWordExample {
                            word_id,
                            definition_id: None,
                            sentence_en: example_en.clone(),
                            sentence_zh: entry.example_zh.clone(),
                            source: Some(entry.source.clone()),
                            author: None,
                            example_order: Some(1),
                            difficulty_level: None,
                            is_common: Some(true),
                        };

                        diesel::insert_into(colang::db::schema::dict_word_examples::table)
                            .values(&new_example)
                            .execute(conn)?;
                    }

                    stats.words_added += 1;
                }
            }
            Ok(())
        })?;
    }

    Ok(stats)
}

/// Parse tab-separated dictionary format
fn parse_tab_separated(
    reader: BufReader<File>,
    source: &str,
) -> Result<Vec<DictEntry>, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();
    let mut skip_count = 0;

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        // Skip empty lines and headers
        if line.is_empty() || line.starts_with('#') || line.contains("词根辞典") {
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, '\t').collect();
        if parts.len() < 2 {
            skip_count += 1;
            continue;
        }

        let word = parts[0].trim();
        let etymology_info = parts[1].trim();

        if word.is_empty() || word.len() > 100 {
            skip_count += 1;
            continue;
        }

        // Extract definition from etymology info
        let (definition_zh, origin_language, origin_word, origin_meaning) =
            extract_etymology_info(etymology_info);

        entries.push(DictEntry {
            word: word.to_string(),
            word_lower: word.to_lowercase(),
            definition_en: None,
            definition_zh: extract_chinese_definition(etymology_info),
            etymology_en: None,
            etymology_zh: Some(etymology_info.to_string()),
            origin_language,
            origin_word,
            origin_meaning,
            example_en: None,
            example_zh: None,
            part_of_speech: extract_part_of_speech(etymology_info),
            source: source.to_string(),
        });
    }

    if skip_count > 0 {
        println!("  Skipped {} invalid lines", skip_count);
    }

    Ok(entries)
}

/// Parse space-separated dictionary format
fn parse_space_separated(
    reader: BufReader<File>,
    source: &str,
) -> Result<Vec<DictEntry>, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();
    let re = Regex::new(r"^(\S+)\s+(\S+)\s+(.+)$").unwrap();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        // Skip empty lines and headers
        if line.is_empty() || line.starts_with('#') || line.contains("词根词源") {
            continue;
        }

        if let Some(caps) = re.captures(line) {
            let word1 = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let word2 = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let definition = caps.get(3).map(|m| m.as_str()).unwrap_or("");

            // Use the first word if they match, otherwise skip
            if word1 != word2 && !word1.is_empty() && !word2.is_empty() {
                // Different words, might be a variant - use first one
            }

            let word = if word1.is_empty() { word2 } else { word1 };

            if word.is_empty() || word.len() > 100 {
                continue;
            }

            entries.push(DictEntry {
                word: word.to_string(),
                word_lower: word.to_lowercase(),
                definition_en: None,
                definition_zh: Some(definition.to_string()),
                etymology_en: None,
                etymology_zh: None,
                origin_language: None,
                origin_word: None,
                origin_meaning: None,
                example_en: extract_example_from_definition(definition),
                example_zh: None,
                part_of_speech: None,
                source: source.to_string(),
            });
        }
    }

    Ok(entries)
}

/// Parse multi-line dictionary format
fn parse_multiline(
    reader: BufReader<File>,
    source: &str,
) -> Result<Vec<DictEntry>, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();
    let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // Skip empty lines and headers
        if line.is_empty() || line.starts_with('#') || line.contains("词源") {
            i += 1;
            continue;
        }

        // Check if this line is a word entry (starts with a word followed by optional pronunciation)
        if line.len() < 100 && !line.contains(' ') {
            let word = line.to_string();
            let mut etymology_en = String::new();
            let mut etymology_zh = String::new();

            // Collect subsequent lines as etymology
            i += 1;
            while i < lines.len() {
                let next_line = lines[i].trim();
                if next_line.is_empty() || next_line.len() < 100 {
                    break;
                }
                etymology_en.push_str(next_line);
                etymology_en.push('\n');
                i += 1;
            }

            if !word.is_empty() {
                entries.push(DictEntry {
                    word: word.clone(),
                    word_lower: word.to_lowercase(),
                    definition_en: None,
                    definition_zh: None,
                    etymology_en: Some(etymology_en.trim().to_string()),
                    etymology_zh: None,
                    origin_language: Some("English".to_string()),
                    origin_word: None,
                    origin_meaning: None,
                    example_en: None,
                    example_zh: None,
                    part_of_speech: None,
                    source: source.to_string(),
                });
            }
        } else {
            i += 1;
        }
    }

    Ok(entries)
}

/// Parse simple dictionary format
fn parse_simple(
    reader: BufReader<File>,
    source: &str,
) -> Result<Vec<DictEntry>, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        // Skip empty lines and headers
        if line.is_empty() || line.starts_with('#') || line.starts_with('\t') {
            continue;
        }

        // Try to parse: word pronunciation definition
        if let Some(pos) = line.find(' ') {
            let word = &line[..pos];
            let rest = &line[pos..].trim();

            if !word.is_empty() && word.len() < 100 {
                // Extract pronunciation and definition
                let (pronunciation, definition) = if let Some(end_pos) = rest.find(']') {
                    let pron = &rest[..=end_pos];
                    let def = &rest[end_pos + 1..].trim();
                    (Some(pron.to_string()), Some(def.to_string()))
                } else {
                    (None, Some(rest.to_string()))
                };

                entries.push(DictEntry {
                    word: word.to_string(),
                    word_lower: word.to_lowercase(),
                    definition_en: None,
                    definition_zh: definition,
                    etymology_en: None,
                    etymology_zh: None,
                    origin_language: None,
                    origin_word: None,
                    origin_meaning: None,
                    example_en: None,
                    example_zh: None,
                    part_of_speech: None,
                    source: source.to_string(),
                });
            }
        }
    }

    Ok(entries)
}

/// Extract Chinese definition from etymology info
fn extract_chinese_definition(info: &str) -> Option<String> {
    // Look for Chinese characters in the definition
    let re = Regex::new(r"[\u4e00-\u9fff]+.*").unwrap();
    re.find(info).map(|m| m.as_str().to_string())
}

/// Extract etymology information
fn extract_etymology_info(info: &str) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    // Extract origin language (e.g., "L." for Latin, "Gk." for Greek)
    let origin_language = if info.contains("(L ") || info.contains("L.") {
        Some("Latin".to_string())
    } else if info.contains("(Gk ") || info.contains("Gk.") {
        Some("Greek".to_string())
    } else if info.contains("(OE ") || info.contains("OE") {
        Some("Old English".to_string())
    } else if info.contains("(F ") || info.contains("F.") {
        Some("French".to_string())
    } else {
        None
    };

    // Try to extract origin word and meaning
    // Format: [词根]: bas, bass (L bassus)=low 低的
    let origin_info = if let Some(start) = info.find("【词根】:") {
        let rest = &info[start + 5..];
        if let Some(end) = rest.find('=') {
            Some(rest[..end].trim().to_string())
        } else {
            None
        }
    } else {
        None
    };

    (None, origin_language, origin_info, None)
}

/// Extract part of speech from etymology info
fn extract_part_of_speech(info: &str) -> Option<String> {
    if info.contains(" v.") || info.contains(" v ") {
        Some("verb".to_string())
    } else if info.contains(" n.") || info.contains(" n ") {
        Some("noun".to_string())
    } else if info.contains(" a.") || info.contains(" adj ") {
        Some("adjective".to_string())
    } else if info.contains(" adv.") || info.contains(" adv ") {
        Some("adverb".to_string())
    } else {
        None
    }
}

/// Extract example sentence from definition
fn extract_example_from_definition(definition: &str) -> Option<String> {
    // Look for example sentences in the definition
    // Format: ...／Example sentence. 翻译
    if let Some(pos) = definition.find("／") {
        let example_part = &definition[pos + 3..];
        if let Some(end) = example_part.find('。') {
            let char_count = example_part[..end].chars().count() + 1;
            return Some(example_part.chars().take(char_count).collect::<String>().trim().to_string());
        }
    }
    None
}

/// Print usage information
fn print_usage() {
    println!("Dictionary Data Import Tool");
    println!();
    println!("Usage:");
    println!("  cargo run --bin import_dict_data [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --dict-dir <PATH>   Dictionary directory (default: {})", DEFAULT_DICT_DIR);
    println!("  --batch-size <N>    Batch size for inserts (default: {})", DEFAULT_BATCH_SIZE);
    println!("  --help              Show this help message");
    println!();
    println!("Environment:");
    println!("  DATABASE_URL        PostgreSQL connection URL");
}
