use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;

use colang::db::pool::DieselPool;
use diesel::prelude::*;
use serde::Deserialize;

const DEFAULT_WORDS_DIR: &str = "../words";
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
    Full { name: String, priority_order: Option<i32> },
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
    rank: Option<i32>,
    per_million: Option<f32>,
}

#[derive(Debug)]
struct ImportStats {
    files_processed: usize,
    words_added: usize,
    words_skipped: usize,
    errors: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let mut words_dir = PathBuf::from(DEFAULT_WORDS_DIR);
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
    println!("Batch size: {}", batch_size);
    println!("Skip existing: {}", skip_existing);
    if let Some(l) = limit {
        println!("Limit: {} files", l);
    }

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = DieselPool::new(
        &database_url,
        &Default::default(),
        diesel::r2d2::Pool::builder(),
    )?;
    let mut conn = pool.get()?;

    let start_time = Instant::now();

    let json_files = discover_json_files(&words_dir);
    let json_files: Vec<PathBuf> = if let Some(l) = limit {
        json_files.into_iter().take(l).collect()
    } else {
        json_files
    };
    println!("Found {} JSON files to process", json_files.len());

    println!("\nLoading dictionaries cache...");
    let mut dictionaries_cache = load_dictionaries_cache(&mut conn)?;
    println!("Loaded {} dictionaries", dictionaries_cache.len());

    println!("\nLoading categories cache...");
    let mut categories_cache = load_categories_cache(&mut conn)?;
    println!("Loaded {} categories", categories_cache.len());

    let stats = import_words(
        &mut conn,
        &json_files,
        batch_size,
        skip_existing,
        &mut dictionaries_cache,
        &mut categories_cache,
    )?;

    let elapsed = start_time.elapsed();

    println!("\n===============================================");
    println!("Import Summary");
    println!("===============================================");
    println!("Files processed: {}", stats.files_processed);
    println!("Words added: {}", stats.words_added);
    println!("Words skipped: {}", stats.words_skipped);
    println!("Errors: {}", stats.errors);
    println!("Total time: {:.2}s", elapsed.as_secs_f64());

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

fn load_dictionaries_cache(
    conn: &mut PgConnection,
) -> Result<HashMap<String, i64>, Box<dyn std::error::Error>> {
    use colang::db::schema::dict_dictionaries;

    let dictionaries: Vec<(i64, String)> = dict_dictionaries::table
        .select((dict_dictionaries::id, dict_dictionaries::name_zh))
        .load(conn)?;

    let mut cache = HashMap::new();
    for (id, name) in dictionaries {
        cache.insert(name, id);
    }

    Ok(cache)
}

fn load_categories_cache(
    conn: &mut PgConnection,
) -> Result<HashMap<String, i64>, Box<dyn std::error::Error>> {
    use colang::db::schema::{taxon_categories, taxon_domains};

    let domain_id: Option<i64> = taxon_domains::table
        .filter(taxon_domains::code.eq("dictionary"))
        .select(taxon_domains::id)
        .first::<i64>(conn)
        .optional()?;

    let mut cache = HashMap::new();

    if let Some(did) = domain_id {
        let categories: Vec<(i64, String)> = taxon_categories::table
            .select((taxon_categories::id, taxon_categories::name_en))
            .filter(taxon_categories::domain_id.eq(did))
            .load(conn)?;

        for (id, name) in categories {
            cache.insert(name, id);
        }
    }

    Ok(cache)
}

fn import_words(
    conn: &mut PgConnection,
    json_files: &[PathBuf],
    batch_size: usize,
    skip_existing: bool,
    dictionaries_cache: &mut HashMap<String, i64>,
    categories_cache: &mut HashMap<String, i64>,
) -> Result<ImportStats, Box<dyn std::error::Error>> {
    let mut stats = ImportStats {
        files_processed: 0,
        words_added: 0,
        words_skipped: 0,
        errors: 0,
    };

    for json_file in json_files {
        stats.files_processed += 1;

        // Each file gets its own transaction to prevent cascading failures
        let result = conn.transaction::<_, Box<dyn std::error::Error>, _>(|conn| {
            process_json_file(
                conn,
                json_file,
                skip_existing,
                dictionaries_cache,
                categories_cache,
            )
        });

        match result {
            Ok(added) => {
                if added {
                    stats.words_added += 1;
                    if stats.words_added % 100 == 0 {
                        println!("  Progress: {} words added", stats.words_added);
                    }
                } else {
                    stats.words_skipped += 1;
                }
            }
            Err(e) => {
                eprintln!("  Error processing {}: {}", json_file.display(), e);
                stats.errors += 1;
            }
        }
    }

    Ok(stats)
}

fn process_json_file(
    conn: &mut PgConnection,
    json_file: &Path,
    skip_existing: bool,
    dictionaries_cache: &mut HashMap<String, i64>,
    categories_cache: &mut HashMap<String, i64>,
) -> Result<bool, Box<dyn std::error::Error>> {
    use colang::db::schema::dict_words;

    let file = File::open(json_file)?;
    let reader = BufReader::new(file);
    let json_data: HashMap<String, WordEntry> = serde_json::from_reader(reader)?;

    for (word, entry) in json_data {
        let word_lower = word.to_lowercase();

        if skip_existing {
            let existing: Option<i64> = dict_words::table
                .select(dict_words::id)
                .filter(dict_words::word_lower.eq(&word_lower))
                .first(conn)
                .optional()?;

            if existing.is_some() {
                return Ok(false);
            }
        }

        let word_id = insert_word(conn, &word, &word_lower, &entry)?;
        insert_pronunciations(conn, word_id, &entry)?;
        insert_definitions(conn, word_id, &entry)?;
        insert_forms(conn, word_id, &entry)?;
        insert_sentences(conn, word_id, &entry)?;
        insert_etymologies(conn, word_id, &entry)?;
        insert_categories(conn, word_id, &entry, categories_cache)?;
        insert_relations(conn, word_id, &entry)?;
        insert_frequencies(conn, word_id, &entry)?;
        insert_word_dictionaries(conn, word_id, &entry, dictionaries_cache)?;
    }

    Ok(true)
}

fn insert_word(
    conn: &mut PgConnection,
    word: &str,
    word_lower: &str,
    entry: &WordEntry,
) -> Result<i64, Box<dyn std::error::Error>> {
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
        if d < 1 || d > 5 {
            eprintln!(
                "  Warning: difficulty {} for word '{}' is out of range [1-5], clamping to {}",
                d,
                word,
                d.clamp(1, 5)
            );
            d.clamp(1, 5)
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
        .get_result(conn)?;

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

fn insert_pronunciations(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::dict_pronunciations;

    if let Some(pronunciations) = &entry.pronunciations {
        let mut is_first = true;
        for pron in pronunciations {
            diesel::insert_into(dict_pronunciations::table)
                .values((
                    dict_pronunciations::word_id.eq(word_id),
                    dict_pronunciations::ipa.eq(&pron.ipa),
                    dict_pronunciations::dialect.eq(&pron.dialect),
                    dict_pronunciations::audio_url.eq(&pron.audio_url),
                    dict_pronunciations::is_primary.eq(is_first),
                ))
                .execute(conn)?;
            is_first = false;
        }
    }

    Ok(())
}

fn insert_definitions(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::dict_definitions;

    if let Some(definitions) = &entry.definitions {
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
                .execute(conn)?;
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

fn insert_forms(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::dict_forms;

    if let Some(forms) = &entry.forms {
        for form in forms {
            let form_type = form.form_type.as_ref().and_then(|t| normalize_form_type(t));

            diesel::insert_into(dict_forms::table)
                .values((
                    dict_forms::word_id.eq(word_id),
                    dict_forms::form_type.eq(form_type),
                    dict_forms::form.eq(&form.form),
                    dict_forms::is_irregular.eq(form.is_irregular),
                ))
                .execute(conn)?;
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

fn insert_sentences(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::*;

    if let Some(sentences) = &entry.sentences {
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
                .get_result(conn)?;

            diesel::insert_into(dict_word_sentences::table)
                .values((
                    dict_word_sentences::word_id.eq(word_id),
                    dict_word_sentences::sentence_id.eq(sentence_id),
                    dict_word_sentences::priority_order.eq(order),
                ))
                .execute(conn)?;

            order += 1;
        }
    }

    Ok(())
}

fn insert_etymologies(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::*;

    if let Some(etymologies) = &entry.etymologies {
        let mut order = 1;
        for etym in etymologies {
            let etymology_id: i64 = diesel::insert_into(dict_etymologies::table)
                .values((
                    dict_etymologies::origin_language.eq(&etym.origin_language),
                    dict_etymologies::origin_word.eq(&etym.origin_word),
                    dict_etymologies::origin_meaning.eq(&etym.origin_meaning),
                    dict_etymologies::language.eq(&etym.language),
                    dict_etymologies::etymology.eq(&etym.etymology),
                    dict_etymologies::first_attested_year.eq(etym.first_attested_year.as_ref()),
                ))
                .returning(dict_etymologies::id)
                .get_result(conn)?;

            diesel::insert_into(dict_word_etymologies::table)
                .values((
                    dict_word_etymologies::word_id.eq(word_id),
                    dict_word_etymologies::etymology_id.eq(etymology_id),
                    dict_word_etymologies::priority_order.eq(order),
                ))
                .execute(conn)?;

            order += 1;
        }
    }

    Ok(())
}

fn insert_categories(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &WordEntry,
    categories_cache: &mut HashMap<String, i64>,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::{dict_word_categories, taxon_categories, taxon_domains};

    if let Some(categories) = &entry.categories {
        for cat in categories {
            let category_id = if let Some(&id) = categories_cache.get(&cat.name) {
                id
            } else {
                // Get the dictionary domain
                let domain_id: i64 = taxon_domains::table
                    .filter(taxon_domains::code.eq("dictionary"))
                    .select(taxon_domains::id)
                    .first::<i64>(conn)?;

                let id: i64 = diesel::insert_into(taxon_categories::table)
                    .values((
                        taxon_categories::code.eq(cat.name.to_lowercase().replace(' ', "_")),
                        taxon_categories::name_en.eq(&cat.name),
                        taxon_categories::name_zh.eq(&cat.name),
                        taxon_categories::domain_id.eq(domain_id),
                    ))
                    .returning(taxon_categories::id)
                    .get_result(conn)?;
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
                .execute(conn)?;
        }
    }

    Ok(())
}

fn insert_relations(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(relations) = &entry.relations {
        insert_relation_type(conn, word_id, "synonym", &relations.synonyms)?;
        insert_relation_type(conn, word_id, "antonym", &relations.antonyms)?;
        insert_relation_type(conn, word_id, "related", &relations.related)?;
        insert_relation_type(conn, word_id, "broader", &relations.broader)?;
    }

    Ok(())
}

fn insert_relation_type(
    conn: &mut PgConnection,
    word_id: i64,
    relation_type: &str,
    related_words: &Option<Vec<RelatedWord>>,
) -> Result<(), Box<dyn std::error::Error>> {
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
                    .get_result(conn)?
            };

            diesel::insert_into(dict_relations::table)
                .values((
                    dict_relations::word_id.eq(word_id),
                    dict_relations::relation_type.eq(relation_type),
                    dict_relations::related_word_id.eq(related_id),
                    dict_relations::relation_strength.eq(related.strength),
                ))
                .on_conflict_do_nothing()
                .execute(conn)?;
        }
    }

    Ok(())
}

fn insert_frequencies(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &WordEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::dict_frequencies;

    if let Some(frequencies) = &entry.frequencies {
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
                .execute(conn)?;
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

fn insert_word_dictionaries(
    conn: &mut PgConnection,
    word_id: i64,
    entry: &WordEntry,
    dictionaries_cache: &mut HashMap<String, i64>,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::{dict_dictionaries, dict_word_dictionaries};

    if let Some(dictionaries) = &entry.dictionaries {
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
                                    .first(conn)?
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
                .execute(conn)?;
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
        "  --words-dir <PATH>   Source directory with word JSON files (default: {})",
        DEFAULT_WORDS_DIR
    );
    println!(
        "  --batch-size <N>     Batch size for transactions (default: {})",
        DEFAULT_BATCH_SIZE
    );
    println!("  --limit <N>          Limit number of files to process");
    println!("  --force              Force re-import existing words (skip check)");
    println!("  --help               Show this help message");
    println!();
    println!("Environment:");
    println!("  DATABASE_URL         PostgreSQL connection URL");
    println!();
    println!("Examples:");
    println!("  cargo run --bin words_to_db");
    println!("  cargo run --bin words_to_db --limit 100");
    println!("  cargo run --bin words_to_db --words-dir ../../../words --batch-size 50");
    println!("  cargo run --bin words_to_db --force");
}
