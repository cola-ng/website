use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;

use colang::db::pool::DieselPool;
use diesel::prelude::*;

const DEFAULT_SOURCE_DIR: &str = r"D:\Works\colang\endict1\vocabulary";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    
    let args: Vec<String> = std::env::args().collect();
    let mut source_dir = PathBuf::from(DEFAULT_SOURCE_DIR);
    
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
    println!("Dictionary Vocabulary Import Tool");
    println!("===============================================");
    println!("Source directory: {}", source_dir.display());
    println!();

    println!("Initializing database connection...");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = DieselPool::new(
        &database_url,
        &Default::default(),
        diesel::r2d2::Pool::builder(),
    )?;
    let mut conn = pool.get()?;

    let vocabulary_files = discover_vocabulary_files(&source_dir);
    println!("Found {} vocabulary files", vocabulary_files.len());

    let mut total_words = 0;
    let mut total_associations = 0;
    let start_time = Instant::now();

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

    let elapsed = start_time.elapsed();
    println!("\n===============================================");
    println!("Import Summary");
    println!("===============================================");
    println!("Total words found: {}", total_words);
    println!("Total associations added: {}", total_associations);
    println!("Total time: {:.2}s", elapsed.as_secs_f64());
    println!();

    Ok(())
}

#[derive(Debug)]
struct ImportStats {
    words: usize,
    associations: usize,
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

fn import_vocabulary_file(
    conn: &mut PgConnection,
    vocab_file: &Path,
) -> Result<ImportStats, Box<dyn std::error::Error>> {
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
            .filter(dict_dictionaries::name.eq(dictionary_name))
            .first(conn)
            .optional()?;

        let dictionary_id = match dictionary_id {
            Some(id) => id,
            None => {
                let id: i64 = diesel::insert_into(dict_dictionaries::table)
                    .values((
                        dict_dictionaries::name.eq(dictionary_name),
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

    Ok(ImportStats {
        words: words_count,
        associations: associations_count,
    })
}

fn print_usage() {
    println!("Dictionary Vocabulary Import Tool");
    println!();
    println!("Usage:");
    println!("  cargo run --bin import_dict_name [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --source-dir <PATH>  Source directory with vocabulary JSON files");
    println!("                      (default: {})", DEFAULT_SOURCE_DIR);
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
