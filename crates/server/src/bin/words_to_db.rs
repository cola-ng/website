use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Utc;
use horme::diesel_async::AsyncPgConnection;
use horme::prelude::*;
use serde::{Deserialize, Deserializer};
use std::result::Result;

const DEFAULT_WORDS_DIR: &str = "D:/Works/cola-ng/dict/final-en-dict";
const DEFAULT_PRONUNCIATIONS_DIR: &str = "D:/Works/cola-ng/space/pronunciations";
const DEFAULT_BATCH_SIZE: usize = 100;

#[derive(Debug, Deserialize)]
struct WordEntry {
    word_type: Option<String>,
    language: Option<String>,
    frequency: Option<i16>,
    difficulty: Option<i16>,
    syllable_count: Option<i16>,
    is_lemma: Option<bool>,
    word_count: Option<i32>,
    dictionaries: Option<Vec<DictionaryRef>>,
    pronunciations: Option<Vec<Pronunciation>>,
    definitions: Option<Vec<Definition>>,
    forms: Option<Vec<WordForm>>,
    sentences: Option<Vec<Sentence>>,
    #[serde(default, deserialize_with = "deserialize_one_or_many_etymologies")]
    etymologies: Option<Vec<Etymology>>,
    categories: Option<Vec<Category>>,
    relations: Option<Relations>,
    frequencies: Option<Vec<Frequency>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DictionaryRef {
    Id(i64),
    Name(String),
    Full {
        name: String,
        #[serde(default, deserialize_with = "deserialize_optional_i32_from_string")]
        priority_order: Option<i32>,
    },
}

/// Deserialize Option<i32> that may be a number, null, or empty string
fn deserialize_optional_i32_from_string<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt {
        Int(i32),
        String(String),
        Null,
    }

    match Option::<StringOrInt>::deserialize(deserializer)? {
        None | Some(StringOrInt::Null) => Ok(None),
        Some(StringOrInt::Int(i)) => Ok(Some(i)),
        Some(StringOrInt::String(s)) => {
            if s.is_empty() {
                Ok(None)
            } else {
                s.parse::<i32>().map(Some).map_err(D::Error::custom)
            }
        }
    }
}

impl DictionaryRef {
    fn name(&self) -> Option<&str> {
        match self {
            DictionaryRef::Id(_) => None,
            DictionaryRef::Name(name) => Some(name),
            DictionaryRef::Full { name, .. } => Some(name),
        }
    }

    fn id(&self) -> Option<i64> {
        match self {
            DictionaryRef::Id(id) => Some(*id),
            _ => None,
        }
    }

    fn priority_order(&self) -> Option<i32> {
        match self {
            DictionaryRef::Name(_) | DictionaryRef::Id(_) => None,
            DictionaryRef::Full { priority_order, .. } => *priority_order,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Pronunciation {
    ipa: String,
    dialect: Option<String>,
    audio_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Definition {
    language: String,
    part_of_speech: Option<String>,
    definition: String,
    register: Option<String>,
    is_primary: Option<bool>,
    usage_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WordForm {
    form_type: Option<String>,
    form: String,
    is_irregular: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct Sentence {
    sentence: String,
    source: Option<String>,
    difficulty: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct Etymology {
    language: String,
    origin_language: Option<String>,
    origin_word: Option<String>,
    origin_meaning: Option<String>,
    #[serde(default)]
    etymology: String,
    #[serde(default, deserialize_with = "deserialize_year_as_string")]
    first_attested_year: Option<String>,
}

fn deserialize_year_as_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::Number(n)) => Ok(Some(n.to_string())),
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        _ => Ok(None),
    }
}

fn deserialize_string_or_number_as_i32<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::Number(n)) => Ok(n.as_i64().map(|v| v as i32)),
        Some(serde_json::Value::String(s)) => Ok(s.parse::<i32>().ok()),
        _ => Ok(None),
    }
}

fn deserialize_string_or_number_as_f32<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::Number(n)) => Ok(n.as_f64().map(|v| v as f32)),
        Some(serde_json::Value::String(s)) => Ok(s.parse::<f32>().ok()),
        _ => Ok(None),
    }
}

fn deserialize_one_or_many_etymologies<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<Etymology>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::Array(arr)) => {
            let etymologies: Result<Vec<Etymology>, _> = arr
                .into_iter()
                .map(|v| serde_json::from_value(v).map_err(serde::de::Error::custom))
                .collect();
            Ok(Some(etymologies?))
        }
        Some(obj @ serde_json::Value::Object(_)) => {
            let etymology: Etymology =
                serde_json::from_value(obj).map_err(serde::de::Error::custom)?;
            Ok(Some(vec![etymology]))
        }
        _ => Ok(None),
    }
}

#[derive(Debug, Deserialize)]
struct Category {
    name: String,
    confidence: Option<i16>,
}

#[derive(Debug, Deserialize)]
struct Relations {
    synonyms: Option<Vec<RelatedWord>>,
    antonyms: Option<Vec<RelatedWord>>,
    related: Option<Vec<RelatedWord>>,
    broader: Option<Vec<RelatedWord>>,
}

#[derive(Debug, Deserialize)]
struct RelatedWord {
    word: Option<String>,
    strength: Option<i16>,
}

#[derive(Debug, Deserialize)]
struct Frequency {
    corpus: String,
    corpus_type: Option<String>,
    band: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_number_as_i32")]
    rank: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_string_or_number_as_f32")]
    per_million: Option<f32>,
}

#[derive(Debug)]
struct ImportStats {
    files_processed: usize,
    words_added: usize,
    words_skipped: usize,
    errors: usize,
}

#[derive(Debug)]
struct WordError {
    file: String,
    word: String,
    error: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let mut words_dir = PathBuf::from(DEFAULT_WORDS_DIR);
    let mut pronunciations_dir = PathBuf::from(DEFAULT_PRONUNCIATIONS_DIR);
    let mut batch_size = DEFAULT_BATCH_SIZE;
    let mut limit: Option<usize> = None;
    let mut skip_existing = true;

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
            "--pronunciations-dir" => {
                if i + 1 < args.len() {
                    pronunciations_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --pronunciations-dir requires a path argument");
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
            "--limit" => {
                if i + 1 < args.len() {
                    limit = Some(args[i + 1].parse().unwrap_or(100));
                    i += 2;
                } else {
                    eprintln!("Error: --limit requires a number argument");
                    std::process::exit(1);
                }
            }
            "--force" => {
                skip_existing = false;
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
    println!("Words JSON to Database Import Tool");
    println!("===============================================");
    println!("Words directory: {}", words_dir.display());
    println!("Pronunciations directory: {}", pronunciations_dir.display());
    println!("Batch size: {}", batch_size);
    println!("Skip existing: {}", skip_existing);
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

    println!("\nLoading dictionaries cache...");
    let mut dictionaries_cache = load_dictionaries_cache(&mut conn).await?;
    println!("Loaded {} dictionaries", dictionaries_cache.len());

    println!("\nLoading categories cache...");
    let mut categories_cache = load_categories_cache(&mut conn).await?;
    println!("Loaded {} categories", categories_cache.len());

    let (stats, word_errors) = import_words(
        &mut conn,
        &json_files,
        batch_size,
        skip_existing,
        &mut dictionaries_cache,
        &mut categories_cache,
        &pronunciations_dir,
    )
    .await?;

    let elapsed = start_time.elapsed();

    println!("\n===============================================");
    println!("Import Summary");
    println!("===============================================");
    println!("Files processed: {}", stats.files_processed);
    println!("Words added: {}", stats.words_added);
    println!("Words skipped: {}", stats.words_skipped);
    println!("Errors: {}", stats.errors);
    println!("Word parse errors: {}", word_errors.len());
    println!("Total time: {:.2}s", elapsed.as_secs_f64());

    // Generate error report if there are word errors
    if !word_errors.is_empty() {
        let report_path = words_dir.join("import_errors.txt");
        let mut report = String::new();
        report.push_str("# Word Import Error Report\n");
        report.push_str(&format!("# Generated at: {}\n", Utc::now()));
        report.push_str(&format!("# Total errors: {}\n\n", word_errors.len()));

        for err in &word_errors {
            report.push_str(&format!("File: {}\n", err.file));
            report.push_str(&format!("Word: {}\n", err.word));
            report.push_str(&format!("Error: {}\n", err.error));
            report.push_str("\n---\n\n");
        }

        fs::write(&report_path, &report)?;
        println!("\nError report written to: {}", report_path.display());
    }

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

async fn load_dictionaries_cache(
    conn: &mut AsyncPgConnection,
) -> Result<HashMap<String, i64>, Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::dict_dictionaries;

    let dictionaries: Vec<(i64, String)> = dict_dictionaries::table
        .select((dict_dictionaries::id, dict_dictionaries::name_zh))
        .load(conn)
        .await?;

    let mut cache = HashMap::new();
    for (id, name) in dictionaries {
        cache.insert(name, id);
    }

    Ok(cache)
}

async fn load_categories_cache(
    conn: &mut AsyncPgConnection,
) -> Result<HashMap<String, i64>, Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::{taxon_categories, taxon_domains};

    let domain_id: Option<i64> = taxon_domains::table
        .filter(taxon_domains::code.eq("dictionary"))
        .select(taxon_domains::id)
        .first::<i64>(conn)
        .await
        .optional()?;

    let mut cache = HashMap::new();

    if let Some(did) = domain_id {
        let categories: Vec<(i64, String)> = taxon_categories::table
            .select((taxon_categories::id, taxon_categories::name_en))
            .filter(taxon_categories::domain_id.eq(did))
            .load(conn)
            .await?;

        for (id, name) in categories {
            cache.insert(name, id);
        }
    }

    Ok(cache)
}

async fn import_words(
    conn: &mut AsyncPgConnection,
    json_files: &[PathBuf],
    _batch_size: usize,
    skip_existing: bool,
    dictionaries_cache: &mut HashMap<String, i64>,
    categories_cache: &mut HashMap<String, i64>,
    pronunciations_dir: &Path,
) -> Result<(ImportStats, Vec<WordError>), Box<dyn std::error::Error + Send + Sync>> {
    let mut stats = ImportStats {
        files_processed: 0,
        words_added: 0,
        words_skipped: 0,
        errors: 0,
    };
    let mut word_errors: Vec<WordError> = Vec::new();

    for json_file in json_files {
        stats.files_processed += 1;

        // Each file gets its own transaction to prevent cascading failures
        let result = horme::transaction(conn, async |conn| {
            process_json_file(
                conn,
                json_file,
                skip_existing,
                dictionaries_cache,
                categories_cache,
                pronunciations_dir,
                &mut word_errors,
            )
            .await
        })
        .await;

        match result {
            Ok((added, skipped)) => {
                stats.words_added += added;
                stats.words_skipped += skipped;
                if stats.words_added % 100 == 0 && stats.words_added > 0 {
                    println!("  Progress: {} words added", stats.words_added);
                }
            }
            Err(e) => {
                eprintln!("  Error processing {}: {}", json_file.display(), e);
                stats.errors += 1;
            }
        }
    }

    Ok((stats, word_errors))
}

async fn process_json_file(
    conn: &mut AsyncPgConnection,
    json_file: &Path,
    _skip_existing: bool,
    dictionaries_cache: &mut HashMap<String, i64>,
    categories_cache: &mut HashMap<String, i64>,
    pronunciations_dir: &Path,
    word_errors: &mut Vec<WordError>,
) -> Result<(usize, usize), Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::dict_words;

    let file = File::open(json_file)?;
    let reader = BufReader::new(file);

    // Parse as generic JSON Value first to handle individual entry errors
    let json_value: serde_json::Value = serde_json::from_reader(reader)?;

    let json_obj = json_value
        .as_object()
        .ok_or("JSON file must be an object")?;

    let mut words_added = 0;
    let mut words_skipped = 0;
    let file_name = json_file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| json_file.display().to_string());

    for (word, entry_value) in json_obj {
        // Try to deserialize this specific word entry
        let entry: WordEntry = match serde_json::from_value(entry_value.clone()) {
            Ok(e) => e,
            Err(e) => {
                word_errors.push(WordError {
                    file: file_name.clone(),
                    word: word.clone(),
                    error: e.to_string(),
                });
                continue;
            }
        };
        let word_lower = word.to_lowercase();

        // Step 1: Check if word already exists
        let existing_word_id: Option<i64> = dict_words::table
            .select(dict_words::id)
            .filter(dict_words::word_lower.eq(&word_lower))
            .first(conn)
            .await
            .optional()?;

        let (word_id, is_new_word) = match existing_word_id {
            Some(existing_id) => {
                // Word exists
                (existing_id, false)
            }
            None => {
                // Word doesn't exist, insert it
                let new_id = insert_word(conn, &word, &word_lower, &entry).await?;
                (new_id, true)
            }
        };

        // Step 2: Insert related data
        // - For new words: insert all related data
        // - For existing words: only insert missing related data (each function checks internally)
        let mut has_errors = false;

        if let Err(e) =
            insert_pronunciations(conn, word_id, &word_lower, &entry, pronunciations_dir).await
        {
            eprintln!(
                "  Warning: failed to insert pronunciations for '{}': {}",
                word, e
            );
            has_errors = true;
        }
        if let Err(e) = insert_definitions(conn, word_id, &entry).await {
            eprintln!(
                "  Warning: failed to insert definitions for '{}': {}",
                word, e
            );
            has_errors = true;
        }
        if let Err(e) = insert_forms(conn, word_id, &entry).await {
            eprintln!("  Warning: failed to insert forms for '{}': {}", word, e);
            has_errors = true;
        }
        if let Err(e) = insert_sentences(conn, word_id, &entry).await {
            eprintln!(
                "  Warning: failed to insert sentences for '{}': {}",
                word, e
            );
            has_errors = true;
        }
        if let Err(e) = insert_etymologies(conn, word_id, &entry).await {
            eprintln!(
                "  Warning: failed to insert etymologies for '{}': {}",
                word, e
            );
            has_errors = true;
        }
        if let Err(e) = insert_categories(conn, word_id, &entry, categories_cache).await {
            eprintln!(
                "  Warning: failed to insert categories for '{}': {}",
                word, e
            );
            has_errors = true;
        }
        if let Err(e) = insert_relations(conn, word_id, &entry).await {
            eprintln!(
                "  Warning: failed to insert relations for '{}': {}",
                word, e
            );
            has_errors = true;
        }
        if let Err(e) = insert_frequencies(conn, word_id, &entry).await {
            eprintln!(
                "  Warning: failed to insert frequencies for '{}': {}",
                word, e
            );
            has_errors = true;
        }
        if let Err(e) = insert_word_dictionaries(conn, word_id, &entry, dictionaries_cache).await {
            eprintln!(
                "  Warning: failed to insert dictionaries for '{}': {}",
                word, e
            );
            has_errors = true;
        }

        if has_errors {
            let status = if is_new_word { "inserted" } else { "updated" };
            eprintln!(
                "  Error: word '{}' was {} but some related data failed to insert",
                word, status
            );
        }

        if is_new_word {
            words_added += 1;
        } else {
            words_skipped += 1;
        }
    }

    Ok((words_added, words_skipped))
}

async fn insert_word(
    conn: &mut AsyncPgConnection,
    word: &str,
    word_lower: &str,
    entry: &WordEntry,
) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::dict_words;

    let word_type = if word.contains(' ') {
        Some("phrase".to_string())
    } else {
        entry
            .word_type
            .clone()
            .and_then(|t| normalize_word_type(&t))
    };

    // Validate and clamp difficulty to 1-5 range
    let difficulty = entry.difficulty.map(|d| {
        if d < 1 || d > 9 {
            eprintln!(
                "  Warning: difficulty {} for word '{}' is out of range [1-9], clamping to {}",
                d,
                word,
                d.clamp(1, 9)
            );
            d.clamp(1, 9)
        } else {
            d
        }
    });

    let word_id: i64 = diesel::insert_into(dict_words::table)
        .values((
            dict_words::word.eq(word),
            dict_words::word_lower.eq(word_lower),
            dict_words::word_type.eq(word_type),
            dict_words::language.eq(entry.language.as_deref().unwrap_or("en")),
            dict_words::frequency.eq(entry.frequency),
            dict_words::difficulty.eq(difficulty),
            dict_words::syllable_count.eq(entry.syllable_count),
            dict_words::is_lemma.eq(entry.is_lemma),
            dict_words::word_count.eq(entry
                .word_count
                .or(Some(word.split_whitespace().count() as i32))),
            dict_words::is_active.eq(true),
        ))
        .returning(dict_words::id)
        .get_result(conn)
        .await?;

    Ok(word_id)
}

fn normalize_word_type(word_type: &str) -> Option<String> {
    let lower = word_type.to_lowercase();
    match lower.as_str() {
        "noun" | "n" | "n." => Some("noun".to_string()),
        "verb" | "v" | "v." => Some("verb".to_string()),
        "adjective" | "adj" | "adj." | "a" | "a." => Some("adjective".to_string()),
        "adverb" | "adv" | "adv." => Some("adverb".to_string()),
        "pronoun" | "pron" | "pron." => Some("pronoun".to_string()),
        "preposition" | "prep" | "prep." => Some("preposition".to_string()),
        "conjunction" | "conj" | "conj." => Some("conjunction".to_string()),
        "interjection" | "int" | "int." => Some("interjection".to_string()),
        "article" | "art" | "art." => Some("article".to_string()),
        "abbreviation" | "abbr" | "abbr." => Some("abbreviation".to_string()),
        "phrase" => Some("phrase".to_string()),
        "idiom" => Some("idiom".to_string()),
        _ => None,
    }
}

async fn insert_pronunciations(
    conn: &mut AsyncPgConnection,
    word_id: i64,
    word_lower: &str,
    entry: &WordEntry,
    pronunciations_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::dict_pronunciations;

    // Delete existing pronunciations for this word (we'll re-insert fresh data)
    diesel::delete(dict_pronunciations::table.filter(dict_pronunciations::word_id.eq(word_id)))
        .execute(conn)
        .await?;

    // Check for pronunciation MP3 files
    let mp3_filename = format!("{}.mp3", word_lower);
    let uk_path = pronunciations_dir.join("uk").join(&mp3_filename);
    let us_path = pronunciations_dir.join("us").join(&mp3_filename);
    let has_uk_audio = uk_path.exists();
    let has_us_audio = us_path.exists();

    // Insert pronunciations from JSON if available
    if let Some(pronunciations) = &entry.pronunciations {
        let mut is_first = true;
        for pron in pronunciations {
            // Determine dialect and audio_path
            let dialect = pron.dialect.clone();
            let audio_path = match dialect.as_deref() {
                Some("uk") | Some("UK") | Some("british") | Some("British") if has_uk_audio => {
                    Some(format!("uk/{}", mp3_filename))
                }
                Some("us") | Some("US") | Some("american") | Some("American") if has_us_audio => {
                    Some(format!("us/{}", mp3_filename))
                }
                _ => {
                    // Default: try UK first, then US
                    if has_uk_audio {
                        Some(format!("uk/{}", mp3_filename))
                    } else if has_us_audio {
                        Some(format!("us/{}", mp3_filename))
                    } else {
                        None
                    }
                }
            };

            diesel::insert_into(dict_pronunciations::table)
                .values((
                    dict_pronunciations::word_id.eq(word_id),
                    dict_pronunciations::ipa.eq(&pron.ipa),
                    dict_pronunciations::dialect.eq(&pron.dialect),
                    dict_pronunciations::audio_url.eq(&pron.audio_url),
                    dict_pronunciations::audio_path.eq(&audio_path),
                    dict_pronunciations::is_primary.eq(is_first),
                ))
                .execute(conn)
                .await?;
            is_first = false;
        }
    }

    // If no pronunciations in JSON but we have audio files, create entries for them
    if entry.pronunciations.is_none()
        || entry
            .pronunciations
            .as_ref()
            .map(|p| p.is_empty())
            .unwrap_or(true)
    {
        if has_uk_audio {
            diesel::insert_into(dict_pronunciations::table)
                .values((
                    dict_pronunciations::word_id.eq(word_id),
                    dict_pronunciations::ipa.eq(""),
                    dict_pronunciations::dialect.eq(Some("uk")),
                    dict_pronunciations::audio_path.eq(Some(format!("uk/{}", mp3_filename))),
                    dict_pronunciations::is_primary.eq(true),
                ))
                .execute(conn)
                .await?;
        }
        if has_us_audio {
            diesel::insert_into(dict_pronunciations::table)
                .values((
                    dict_pronunciations::word_id.eq(word_id),
                    dict_pronunciations::ipa.eq(""),
                    dict_pronunciations::dialect.eq(Some("us")),
                    dict_pronunciations::audio_path.eq(Some(format!("us/{}", mp3_filename))),
                    dict_pronunciations::is_primary.eq(!has_uk_audio),
                ))
                .execute(conn)
                .await?;
        }
    }

    Ok(())
}

async fn insert_definitions(
    conn: &mut AsyncPgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::dict_definitions;

    if let Some(definitions) = &entry.definitions {
        // Delete existing definitions for this word
        diesel::delete(dict_definitions::table.filter(dict_definitions::word_id.eq(word_id)))
            .execute(conn)
            .await?;

        let mut order = 1;
        for def in definitions {
            let part_of_speech = def
                .part_of_speech
                .as_ref()
                .and_then(|p| normalize_part_of_speech(p));

            let register = def.register.as_ref().and_then(|r| normalize_register(r));

            diesel::insert_into(dict_definitions::table)
                .values((
                    dict_definitions::word_id.eq(word_id),
                    dict_definitions::language.eq(&def.language),
                    dict_definitions::definition.eq(&def.definition),
                    dict_definitions::part_of_speech.eq(part_of_speech),
                    dict_definitions::definition_order.eq(order),
                    dict_definitions::register.eq(register),
                    dict_definitions::usage_notes.eq(&def.usage_notes),
                    dict_definitions::is_primary.eq(def.is_primary.unwrap_or(order == 1)),
                ))
                .execute(conn).await?;
            order += 1;
        }
    }

    Ok(())
}

fn normalize_part_of_speech(pos: &str) -> Option<String> {
    let pos_lower = pos.to_lowercase();
    match pos_lower.as_str() {
        "n." | "n" | "noun" | "名词" => Some("noun".to_string()),
        "v." | "v" | "verb" | "动词" => Some("verb".to_string()),
        "adj." | "adj" | "a." | "a" | "adjective" | "形容词" => Some("adjective".to_string()),
        "adv." | "adv" | "adverb" | "副词" => Some("adverb".to_string()),
        "pron." | "pron" | "pronoun" | "代词" => Some("pronoun".to_string()),
        "prep." | "prep" | "preposition" | "介词" => Some("preposition".to_string()),
        "conj." | "conj" | "conjunction" | "连词" => Some("conjunction".to_string()),
        "int." | "interjection" | "感叹词" => Some("interjection".to_string()),
        "art." | "article" | "冠词" => Some("article".to_string()),
        "abbr." | "abbreviation" | "缩写" => Some("abbreviation".to_string()),
        "phrase" | "短语" => Some("phrase".to_string()),
        "idiom" | "习语" => Some("idiom".to_string()),
        _ => None,
    }
}

fn normalize_register(register: &str) -> Option<String> {
    let lower = register.to_lowercase();
    match lower.as_str() {
        "formal" | "正式" => Some("formal".to_string()),
        "informal" | "非正式" => Some("informal".to_string()),
        "slang" | "俚语" => Some("slang".to_string()),
        "archaic" | "historical" | "古语" | "古旧" | "历史" => Some("archaic".to_string()),
        "literary" | "文学" | "书面" => Some("literary".to_string()),
        "technical" | "tech" | "专业" | "术语" => Some("technical".to_string()),
        "colloquial" | "口语" => Some("colloquial".to_string()),
        "neutral" | "中性" | "标准" | "standard" => Some("neutral".to_string()),
        _ => None,
    }
}

async fn insert_forms(
    conn: &mut AsyncPgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::dict_forms;

    if let Some(forms) = &entry.forms {
        // Delete existing forms for this word
        diesel::delete(dict_forms::table.filter(dict_forms::word_id.eq(word_id)))
            .execute(conn)
            .await?;

        for form in forms {
            let form_type = form.form_type.as_ref().and_then(|t| normalize_form_type(t));

            diesel::insert_into(dict_forms::table)
                .values((
                    dict_forms::word_id.eq(word_id),
                    dict_forms::form_type.eq(form_type),
                    dict_forms::form.eq(&form.form),
                    dict_forms::is_irregular.eq(form.is_irregular),
                ))
                .execute(conn)
                .await?;
        }
    }

    Ok(())
}

fn normalize_form_type(form_type: &str) -> Option<String> {
    let lower = form_type.to_lowercase();
    match lower.as_str() {
        "plural" | "pl" => Some("plural".to_string()),
        "singular" | "sg" => Some("singular".to_string()),
        "past" | "past_tense" => Some("past".to_string()),
        "present" | "present_tense" => Some("present".to_string()),
        "future" | "future_tense" => Some("future".to_string()),
        "present_participle" | "ing" => Some("present_participle".to_string()),
        "past_participle" | "pp" => Some("past_participle".to_string()),
        "comparative" | "comp" => Some("comparative".to_string()),
        "superlative" | "sup" => Some("superlative".to_string()),
        "adverbial" => Some("adverbial".to_string()),
        "nominalization" => Some("nominalization".to_string()),
        _ => Some("other".to_string()),
    }
}

async fn insert_sentences(
    conn: &mut AsyncPgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::*;

    if let Some(sentences) = &entry.sentences {
        // Delete existing word-sentence links for this word
        diesel::delete(dict_word_sentences::table.filter(dict_word_sentences::word_id.eq(word_id)))
            .execute(conn)
            .await?;

        let mut order = 1;
        for sent in sentences {
            let sentence_id: i64 = diesel::insert_into(dict_sentences::table)
                .values((
                    dict_sentences::language.eq("en"),
                    dict_sentences::sentence.eq(&sent.sentence),
                    dict_sentences::source.eq(&sent.source),
                    dict_sentences::difficulty.eq(sent.difficulty),
                    dict_sentences::is_common.eq(false),
                ))
                .returning(dict_sentences::id)
                .get_result(conn)
                .await?;

            diesel::insert_into(dict_word_sentences::table)
                .values((
                    dict_word_sentences::word_id.eq(word_id),
                    dict_word_sentences::sentence_id.eq(sentence_id),
                    dict_word_sentences::priority_order.eq(order),
                ))
                .on_conflict_do_nothing()
                .execute(conn).await?;

            order += 1;
        }
    }

    Ok(())
}

async fn insert_etymologies(
    conn: &mut AsyncPgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::*;

    if let Some(etymologies) = &entry.etymologies {
        // Delete existing word-etymology links for this word
        diesel::delete(
            dict_word_etymologies::table.filter(dict_word_etymologies::word_id.eq(word_id)),
        )
        .execute(conn)
        .await?;

        let mut order = 1;
        for etym in etymologies {
            // Use origin_word as fallback if etymology is empty
            let etymology_text = if etym.etymology.is_empty() {
                etym.origin_word.clone().unwrap_or_default()
            } else {
                etym.etymology.clone()
            };

            let etymology_id: i64 = diesel::insert_into(dict_etymologies::table)
                .values((
                    dict_etymologies::origin_language.eq(&etym.origin_language),
                    dict_etymologies::origin_word.eq(&etym.origin_word),
                    dict_etymologies::origin_meaning.eq(&etym.origin_meaning),
                    dict_etymologies::language.eq(&etym.language),
                    dict_etymologies::etymology.eq(&etymology_text),
                    dict_etymologies::first_attested_year.eq(etym.first_attested_year.as_ref()),
                ))
                .returning(dict_etymologies::id)
                .get_result(conn)
                .await?;

            diesel::insert_into(dict_word_etymologies::table)
                .values((
                    dict_word_etymologies::word_id.eq(word_id),
                    dict_word_etymologies::etymology_id.eq(etymology_id),
                    dict_word_etymologies::priority_order.eq(order),
                ))
                .on_conflict_do_nothing()
                .execute(conn).await?;

            order += 1;
        }
    }

    Ok(())
}

async fn insert_categories(
    conn: &mut AsyncPgConnection,
    word_id: i64,
    entry: &WordEntry,
    categories_cache: &mut HashMap<String, i64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::{dict_word_categories, taxon_categories, taxon_domains};

    if let Some(categories) = &entry.categories {
        // Delete existing word-category links for this word
        diesel::delete(
            dict_word_categories::table.filter(dict_word_categories::word_id.eq(word_id)),
        )
        .execute(conn)
        .await?;

        for cat in categories {
            let category_id = if let Some(&id) = categories_cache.get(&cat.name) {
                id
            } else {
                // Get the dictionary domain
                let domain_id: i64 = taxon_domains::table
                    .filter(taxon_domains::code.eq("dictionary"))
                    .select(taxon_domains::id)
                    .first::<i64>(conn)
                    .await?;

                let id: i64 = diesel::insert_into(taxon_categories::table)
                    .values((
                        taxon_categories::code.eq(cat.name.to_lowercase().replace(' ', "_")),
                        taxon_categories::name_en.eq(&cat.name),
                        taxon_categories::name_zh.eq(&cat.name),
                        taxon_categories::domain_id.eq(domain_id),
                    ))
                    .returning(taxon_categories::id)
                    .get_result(conn)
                    .await?;
                categories_cache.insert(cat.name.clone(), id);
                id
            };

            diesel::insert_into(dict_word_categories::table)
                .values((
                    dict_word_categories::word_id.eq(word_id),
                    dict_word_categories::category_id.eq(category_id),
                    dict_word_categories::confidence.eq(cat.confidence.unwrap_or(50)),
                ))
                .on_conflict_do_nothing()
                .execute(conn)
                .await?;
        }
    }

    Ok(())
}

async fn insert_relations(
    conn: &mut AsyncPgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::dict_relations;

    if let Some(relations) = &entry.relations {
        // Delete existing relations for this word
        diesel::delete(dict_relations::table.filter(dict_relations::word_id.eq(word_id)))
            .execute(conn)
            .await?;

        insert_relation_type(conn, word_id, "synonym", &relations.synonyms).await?;
        insert_relation_type(conn, word_id, "antonym", &relations.antonyms).await?;
        insert_relation_type(conn, word_id, "related", &relations.related).await?;
        insert_relation_type(conn, word_id, "broader", &relations.broader).await?;
    }

    Ok(())
}

async fn insert_relation_type(
    conn: &mut AsyncPgConnection,
    word_id: i64,
    relation_type: &str,
    related_words: &Option<Vec<RelatedWord>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::{dict_relations, dict_words};

    if let Some(words) = related_words {
        for related in words {
            // Skip entries without a word field
            let word = match &related.word {
                Some(w) => w,
                None => continue,
            };

            let related_word_lower = word.to_lowercase();

            let related_word_id: Option<i64> = dict_words::table
                .select(dict_words::id)
                .filter(dict_words::word_lower.eq(&related_word_lower))
                .first(conn)
                .await
                .optional()?;

            let related_id = if let Some(id) = related_word_id {
                id
            } else {
                // Create related word if not exists
                diesel::insert_into(dict_words::table)
                    .values((
                        dict_words::word.eq(word),
                        dict_words::word_lower.eq(&related_word_lower),
                        dict_words::language.eq("en"),
                        dict_words::is_active.eq(true),
                        dict_words::word_count.eq(word.split_whitespace().count() as i32),
                    ))
                    .returning(dict_words::id)
                    .get_result(conn)
                    .await?
            };

            diesel::insert_into(dict_relations::table)
                .values((
                    dict_relations::word_id.eq(word_id),
                    dict_relations::relation_type.eq(relation_type),
                    dict_relations::related_word_id.eq(related_id),
                    dict_relations::relation_strength.eq(related.strength),
                ))
                .on_conflict_do_nothing()
                .execute(conn)
                .await?;
        }
    }

    Ok(())
}

async fn insert_frequencies(
    conn: &mut AsyncPgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::dict_frequencies;

    if let Some(frequencies) = &entry.frequencies {
        // Delete existing frequencies for this word
        diesel::delete(dict_frequencies::table.filter(dict_frequencies::word_id.eq(word_id)))
            .execute(conn)
            .await?;

        for freq in frequencies {
            let corpus_type = freq
                .corpus_type
                .as_ref()
                .and_then(|t| normalize_corpus_type(t));
            let band = freq.band.as_ref().and_then(|b| normalize_band(b));

            diesel::insert_into(dict_frequencies::table)
                .values((
                    dict_frequencies::word_id.eq(word_id),
                    dict_frequencies::corpus_name.eq(&freq.corpus),
                    dict_frequencies::corpus_type.eq(corpus_type),
                    dict_frequencies::band.eq(band),
                    dict_frequencies::rank.eq(freq.rank),
                    dict_frequencies::per_million.eq(freq.per_million),
                ))
                .on_conflict_do_nothing()
                .execute(conn)
                .await?;
        }
    }

    Ok(())
}

fn normalize_corpus_type(corpus_type: &str) -> Option<String> {
    let lower = corpus_type.to_lowercase();
    match lower.as_str() {
        "spoken" => Some("spoken".to_string()),
        "written" => Some("written".to_string()),
        "academic" => Some("academic".to_string()),
        "news" => Some("news".to_string()),
        "fiction" => Some("fiction".to_string()),
        "internet" => Some("internet".to_string()),
        "general" => Some("general".to_string()),
        _ => Some("general".to_string()),
    }
}

fn normalize_band(band: &str) -> Option<String> {
    let lower = band.to_lowercase().replace('-', "_");
    match lower.as_str() {
        "top_1000" | "top1000" => Some("top_1000".to_string()),
        "top_2000" | "top2000" => Some("top_2000".to_string()),
        "top_3000" | "top3000" => Some("top_3000".to_string()),
        "top_5000" | "top5000" => Some("top_5000".to_string()),
        "top_10000" | "top10000" => Some("top_10000".to_string()),
        "beyond_10000" | "beyond10000" => Some("beyond_10000".to_string()),
        _ => None,
    }
}

async fn insert_word_dictionaries(
    conn: &mut AsyncPgConnection,
    word_id: i64,
    entry: &WordEntry,
    dictionaries_cache: &mut HashMap<String, i64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::{dict_dictionaries, dict_word_dictionaries};

    if let Some(dictionaries) = &entry.dictionaries {
        // Delete existing word-dictionary links for this word
        diesel::delete(
            dict_word_dictionaries::table.filter(dict_word_dictionaries::word_id.eq(word_id)),
        )
        .execute(conn)
        .await?;

        for dict_ref in dictionaries {
            // Handle dictionary ID directly
            let dictionary_id = if let Some(id) = dict_ref.id() {
                id
            } else if let Some(dict_name) = dict_ref.name() {
                if let Some(&id) = dictionaries_cache.get(dict_name) {
                    id
                } else {
                    // First try to find existing dictionary by name_zh, name_en, or short_en
                    let existing_id: Option<i64> = dict_dictionaries::table
                        .select(dict_dictionaries::id)
                        .filter(
                            dict_dictionaries::name_zh
                                .eq(dict_name)
                                .or(dict_dictionaries::name_en.eq(dict_name))
                                .or(dict_dictionaries::short_en.eq(dict_name)),
                        )
                        .first(conn)
                        .await
                        .optional()?;

                    let id = if let Some(id) = existing_id {
                        id
                    } else {
                        // Create dictionary if not exists, skip on conflict
                        let insert_result = diesel::insert_into(dict_dictionaries::table)
                            .values((
                                dict_dictionaries::name_en.eq(dict_name),
                                dict_dictionaries::name_zh.eq(dict_name),
                                dict_dictionaries::short_en.eq(dict_name),
                                dict_dictionaries::short_zh.eq(dict_name),
                                dict_dictionaries::is_active.eq(true),
                            ))
                            .on_conflict_do_nothing()
                            .returning(dict_dictionaries::id)
                            .get_result::<i64>(conn)
                            .await
                            .optional()?;

                        match insert_result {
                            Some(id) => id,
                            None => {
                                // Conflict occurred, fetch existing dictionary
                                dict_dictionaries::table
                                    .select(dict_dictionaries::id)
                                    .filter(
                                        dict_dictionaries::name_zh
                                            .eq(dict_name)
                                            .or(dict_dictionaries::name_en.eq(dict_name))
                                            .or(dict_dictionaries::short_en.eq(dict_name)),
                                    )
                                    .first(conn)
                                    .await?
                            }
                        }
                    };
                    dictionaries_cache.insert(dict_name.to_string(), id);
                    id
                }
            } else {
                continue; // Skip invalid dictionary reference
            };

            diesel::insert_into(dict_word_dictionaries::table)
                .values((
                    dict_word_dictionaries::word_id.eq(word_id),
                    dict_word_dictionaries::dictionary_id.eq(dictionary_id),
                    dict_word_dictionaries::priority_order.eq(dict_ref.priority_order()),
                ))
                .on_conflict_do_nothing()
                .execute(conn)
                .await?;
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Words JSON to Database Import Tool");
    println!();
    println!("Usage:");
    println!("  cargo run --bin words_to_db [OPTIONS]");
    println!();
    println!("Options:");
    println!(
        "  --words-dir <PATH>           Source directory with word JSON files (default: {})",
        DEFAULT_WORDS_DIR
    );
    println!(
        "  --pronunciations-dir <PATH>  Directory with pronunciation MP3 files (default: {})",
        DEFAULT_PRONUNCIATIONS_DIR
    );
    println!(
        "  --batch-size <N>             Batch size for transactions (default: {})",
        DEFAULT_BATCH_SIZE
    );
    println!("  --limit <N>                  Limit number of files to process");
    println!("  --force                      Force re-import existing words (skip check)");
    println!("  --help                       Show this help message");
    println!();
    println!("Environment:");
    println!("  DATABASE_URL                 PostgreSQL connection URL");
    println!();
    println!("Examples:");
    println!("  cargo run --bin words_to_db");
    println!("  cargo run --bin words_to_db --limit 100");
    println!(
        "  cargo run --bin words_to_db --words-dir D:/Works/cola-ng/dict/final-en-dict --batch-size 50"
    );
    println!("  cargo run --bin words_to_db --force");
}
