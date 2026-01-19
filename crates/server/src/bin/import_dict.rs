use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;

use colang::db::pool::DieselPool;
use diesel::prelude::*;
use serde::Deserialize;

const DEFAULT_DICT_DIR: &str = r"D:\Works\colang\endict1\dict";
const DEFAULT_VOCAB_DIR: &str = r"D:\Works\colang\endict1\vocabulary";
const DEFAULT_AUDIO_DIR: &str = r"D:\Works\colang\data\pronunciations";
const DEFAULT_BATCH_SIZE: usize = 100;

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

#[derive(Debug, Clone)]
struct Form {
    form_type: String,
    form: String,
}

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

#[derive(Debug, Clone)]
struct AudioCache {
    uk: HashMap<String, String>,
    us: HashMap<String, String>,
}

#[derive(Debug)]
struct ImportStats {
    entries: usize,
    added: usize,
    skipped: usize,
}

#[derive(Debug)]
struct VocabImportStats {
    words: usize,
    associations: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let mut dict_dir = PathBuf::from(DEFAULT_DICT_DIR);
    let mut vocab_dir = PathBuf::from(DEFAULT_VOCAB_DIR);
    let mut audio_dir = PathBuf::from(DEFAULT_AUDIO_DIR);
    let mut batch_size = DEFAULT_BATCH_SIZE;
    let mut import_dict_data = true;
    let mut import_vocab = true;

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
            "--vocab-dir" => {
                if i + 1 < args.len() {
                    vocab_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --vocab-dir requires a path argument");
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
            "--dict-only" => {
                import_vocab = false;
                i += 1;
            }
            "--vocab-only" => {
                import_dict_data = false;
                i += 1;
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
    println!("Dictionary Import Tool");
    println!("===============================================");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = DieselPool::new(
        &database_url,
        &Default::default(),
        diesel::r2d2::Pool::builder(),
    )?;
    let mut conn = pool.get()?;

    let start_time = Instant::now();

    if import_dict_data {
        println!("\n--- Importing Dictionary Data ---");
        println!("Source directory: {}", dict_dir.display());
        println!("Audio directory: {}", audio_dir.display());
        println!("Batch size: {}", batch_size);

        let json_files = discover_json_files(&dict_dir);
        println!("Found {} JSON files", json_files.len());

        println!("\nBuilding audio file cache...");
        let audio_cache = build_audio_cache(&audio_dir);
        println!(
            "Found {} uk audio files, {} us audio files",
            audio_cache.uk.len(),
            audio_cache.us.len()
        );

        let mut total_entries = 0;
        let mut total_words = 0;
        let mut total_skipped = 0;

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

        println!("\n--- Dictionary Data Summary ---");
        println!("Total entries processed: {}", total_entries);
        println!("Total words added: {}", total_words);
        println!("Total words skipped: {}", total_skipped);
    }

    if import_vocab {
        println!("\n--- Importing Vocabulary Data ---");
        println!("Source directory: {}", vocab_dir.display());

        let vocabulary_files = discover_vocabulary_files(&vocab_dir);
        println!("Found {} vocabulary files", vocabulary_files.len());

        let mut total_words = 0;
        let mut total_associations = 0;

        for vocab_file in &vocabulary_files {
            println!("\nProcessing: {}", vocab_file.file_name().unwrap_or_default().to_string_lossy());

            match import_vocabulary_file(&mut conn, vocab_file) {
                Ok(stats) => {
                    println!("  Words found: {}, Associations added: {}", stats.words, stats.associations);
                    total_words += stats.words;
                    total_associations += stats.associations;
                }
                Err(e) => {
                    eprintln!("  Error: {}", e);
                }
            }
        }

        println!("\n--- Vocabulary Data Summary ---");
        println!("Total words found: {}", total_words);
        println!("Total associations added: {}", total_associations);
    }

    let elapsed = start_time.elapsed();
    println!("\n===============================================");
    println!("Overall Summary");
    println!("===============================================");
    println!("Total time: {:.2}s", elapsed.as_secs_f64());
    println!();

    Ok(())
}

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

fn discover_vocabulary_files(dir: &Path) -> Vec<PathBuf> {
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

fn build_audio_cache(audio_dir: &Path) -> AudioCache {
    let mut uk = HashMap::new();
    let mut us = HashMap::new();

    let uk_dir = audio_dir.join("uk");
    if let Ok(entries) = fs::read_dir(&uk_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(stem) = path.file_stem() {
                    let word = stem.to_string_lossy().to_string();
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                    let relative_path = format!("uk/{}", file_name);
                    uk.insert(word, relative_path);
                }
            }
        }
    }

    let us_dir = audio_dir.join("us");
    if let Ok(entries) = fs::read_dir(&us_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(stem) = path.file_stem() {
                    let word = stem.to_string_lossy().to_string();
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                    let relative_path = format!("us/{}", file_name);
                    us.insert(word, relative_path);
                }
            }
        }
    }

    AudioCache { uk, us }
}

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

    for line in reader.lines() {
        line_number += 1;
        let line = line?;

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

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
    let mut added = 0;
    let mut skipped = 0;

    for chunk in entries.chunks(batch_size) {
        conn.transaction::<_, Box<dyn std::error::Error>, _>(|conn| {
            for entry in chunk {
                let existing: Option<i64> = dict_words::table
                    .select(dict_words::id)
                    .filter(dict_words::word_lower.eq(&entry.word_lower))
                    .first(conn)
                    .optional()?;

                if existing.is_some() {
                    skipped += 1;
                } else {
                    let word_id = insert_word(conn, entry)?;
                    insert_definitions(conn, word_id, entry)?;
                    insert_word_forms(conn, word_id, &entry.word_forms)?;
                    insert_pronunciations(conn, word_id, entry, audio_cache)?;
                    insert_sentences(conn, word_id, &entry.examples)?;
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

fn process_json_entry(json_entry: JsonDictEntry) -> Option<ImportEntry> {
    let word = json_entry.word.trim();
    let word_lower = json_entry.search_word.trim();

    if word.is_empty() || word.len() > 200 {
        return None;
    }

    let part_of_speech = json_entry
        .part_of_speech
        .first()
        .and_then(|p| normalize_part_of_speech(p));

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
                "0" => "present",
                "1" | "d" => "past",
                "2" | "p" => "past_participle",
                "3" | "s" => "plural",
                "4" | "i" => "present_participle",
                "5" => "comparative",
                "6" => "superlative",
                _ => continue,
            };

            forms.push(Form {
                form_type: form_type.to_string(),
                form: form.to_string(),
            });
        }
    }

    forms
}

fn insert_word(
    conn: &mut PgConnection,
    entry: &ImportEntry,
) -> Result<i64, Box<dyn std::error::Error>> {
    use colang::db::schema::dict_words;

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

fn insert_definitions(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &ImportEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::dict_definitions;

    let mut definition_order = 1;

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

fn insert_pronunciations(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &ImportEntry,
    audio_cache: &AudioCache,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::dict_pronunciations;

    let mut has_primary = false;

    if let Some(uk_audio_path) = audio_cache.uk.get(&entry.word_lower) {
        let ipa = if !entry.phonetic.is_empty() {
            entry.phonetic.clone()
        } else {
            format!("[uk: {}]", entry.word)
        };

        diesel::insert_into(dict_pronunciations::table)
            .values((
                dict_pronunciations::word_id.eq(word_id),
                dict_pronunciations::ipa.eq(ipa),
                dict_pronunciations::audio_path.eq(uk_audio_path),
                dict_pronunciations::dialect.eq("uk"),
                dict_pronunciations::is_primary.eq(!has_primary),
            ))
            .execute(conn)?;

        has_primary = true;
    }

    if let Some(us_audio_path) = audio_cache.us.get(&entry.word_lower) {
        let ipa = if !entry.phonetic.is_empty() {
            entry.phonetic.clone()
        } else {
            format!("[us: {}]", entry.word)
        };

        diesel::insert_into(dict_pronunciations::table)
            .values((
                dict_pronunciations::word_id.eq(word_id),
                dict_pronunciations::ipa.eq(ipa),
                dict_pronunciations::audio_path.eq(us_audio_path),
                dict_pronunciations::dialect.eq("us"),
                dict_pronunciations::is_primary.eq(!has_primary),
            ))
            .execute(conn)?;

        has_primary = true;
    }

    if !has_primary && !entry.phonetic.is_empty() {
        diesel::insert_into(dict_pronunciations::table)
            .values((
                dict_pronunciations::word_id.eq(word_id),
                dict_pronunciations::ipa.eq(&entry.phonetic),
                dict_pronunciations::is_primary.eq(true),
            ))
            .execute(conn)?;
    }

    Ok(())
}

fn insert_sentences(
    conn: &mut PgConnection,
    word_id: i64,
    examples: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::*;

    for example in examples {
        if !example.is_empty() {
            let sentence_id: i64 = diesel::insert_into(dict_sentences::table)
                .values((
                    dict_sentences::language.eq("en"),
                    dict_sentences::sentence.eq(example),
                    dict_sentences::is_common.eq(false),
                ))
                .returning(dict_sentences::id)
                .get_result(conn)?;

            diesel::insert_into(dict_word_sentences::table)
                .values((
                    dict_word_sentences::word_id.eq(word_id),
                    dict_word_sentences::sentence_id.eq(sentence_id),
                ))
                .execute(conn)?;
        }
    }

    Ok(())
}

fn import_vocabulary_file(
    conn: &mut PgConnection,
    vocab_file: &Path,
) -> Result<VocabImportStats, Box<dyn std::error::Error>> {
    use colang::db::schema::*;

    let file_name = vocab_file.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let dictionary_name = match file_name.as_str() {
        "cet4" => "CET4",
        "cet6" => "CET6",
        "chuzhong" => "初中",
        "gaozhong" => "高中",
        "kaoyan" => "考研",
        "xiaoxue" => "小学",
        _ => &file_name,
    };

    let file = File::open(vocab_file)?;
    let reader = BufReader::new(file);

    let words: Vec<String> = serde_json::from_reader(reader)?;
    let mut words_count = 0;
    let mut associations_count = 0;

    conn.transaction::<_, Box<dyn std::error::Error>, _>(|conn| {
        let dictionary_id: Option<i64> = dict_dictionaries::table
            .select(dict_dictionaries::id)
            .filter(dict_dictionaries::name_en.eq(dictionary_name))
            .first(conn)
            .optional()?;

        let dictionary_id = match dictionary_id {
            Some(id) => id,
            None => {
                let id: i64 = diesel::insert_into(dict_dictionaries::table)
                    .values((
                        dict_dictionaries::name_en.eq(dictionary_name),
                        dict_dictionaries::name_zh.eq(dictionary_name),
                        dict_dictionaries::short_en.eq(dictionary_name),
                        dict_dictionaries::short_zh.eq(dictionary_name),
                        dict_dictionaries::is_active.eq(true),
                        dict_dictionaries::is_official.eq(true),
                        dict_dictionaries::priority_order.eq(1),
                    ))
                    .returning(dict_dictionaries::id)
                    .get_result(conn)?;
                id
            }
        };

        for word in &words {
            let word_lower = word.to_lowercase();
            let word_id: Option<i64> = dict_words::table
                .select(dict_words::id)
                .filter(dict_words::word_lower.eq(&word_lower))
                .first(conn)
                .optional()?;

            if let Some(word_id) = word_id {
                let existing: Option<i64> = dict_word_dictionaries::table
                    .select(dict_word_dictionaries::id)
                    .filter(dict_word_dictionaries::word_id.eq(word_id))
                    .filter(dict_word_dictionaries::dictionary_id.eq(dictionary_id))
                    .first(conn)
                    .optional()?;

                if existing.is_none() {
                    diesel::insert_into(dict_word_dictionaries::table)
                        .values((
                            dict_word_dictionaries::word_id.eq(word_id),
                            dict_word_dictionaries::dictionary_id.eq(dictionary_id),
                        ))
                        .execute(conn)?;
                    associations_count += 1;
                }
            }
            words_count += 1;
        }
        Ok(())
    })?;

    Ok(VocabImportStats {
        words: words_count,
        associations: associations_count,
    })
}

fn print_usage() {
    println!("Dictionary Import Tool");
    println!();
    println!("Usage:");
    println!("  cargo run --bin import_dict [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --dict-dir <PATH>    Source directory with dictionary JSON files");
    println!("                       (default: {})", DEFAULT_DICT_DIR);
    println!("  --vocab-dir <PATH>   Source directory with vocabulary JSON files");
    println!("                       (default: {})", DEFAULT_VOCAB_DIR);
    println!("  --audio-dir <PATH>   Audio directory with uk/us subdirs");
    println!("                       (default: {})", DEFAULT_AUDIO_DIR);
    println!("  --batch-size <N>     Batch size for inserts (default: {})", DEFAULT_BATCH_SIZE);
    println!("  --dict-only          Only import dictionary data, skip vocabulary");
    println!("  --vocab-only         Only import vocabulary data, skip dictionary");
    println!("  --help               Show this help message");
    println!();
    println!("Environment:");
    println!("  DATABASE_URL         PostgreSQL connection URL");
    println!();
    println!("Dictionary name mapping:");
    println!("  cet4      -> CET4");
    println!("  cet6      -> CET6");
    println!("  chuzhong  -> 初中");
    println!("  gaozhong  -> 高中");
    println!("  kaoyan    -> 考研");
    println!("  xiaoxue   -> 小学");
}
