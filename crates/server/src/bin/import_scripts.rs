//! Import stages and scripts from JSON to database
//!
//! Reads stages.json and scripts.json and imports data into:
//! - asset_stages
//! - asset_scripts
//! - asset_script_turns

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use horme::diesel_async::AsyncPgConnection;
use horme::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use std::result::Result;

const DEFAULT_STAGES_FILE: &str = r"D:\Works\cola-ng\space-raw\stages.json";
const DEFAULT_SCRIPTS_FILE: &str = r"D:\Works\cola-ng\space-raw\scripts.json";

// ============================================================================
// JSON structures
// ============================================================================

#[derive(Debug, Deserialize)]
struct StagesFile {
    stages: Vec<RawStage>,
}

#[derive(Debug, Deserialize)]
struct RawStage {
    code: String,
    name_en: String,
    name_zh: String,
    description_en: Option<String>,
    description_zh: Option<String>,
    icon_emoji: Option<String>,
    display_order: Option<i32>,
    difficulty: Option<i16>,
    is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ScriptsFile {
    scripts: Vec<RawScript>,
}

#[derive(Debug, Deserialize)]
struct RawScript {
    stage_code: String,
    code: String,
    title_en: String,
    title_zh: String,
    description_en: Option<String>,
    description_zh: Option<String>,
    total_turns: Option<i32>,
    estimated_duration_seconds: Option<i32>,
    difficulty: Option<i16>,
    turns: Vec<RawTurn>,
}

#[derive(Debug, Deserialize)]
struct RawTurn {
    turn_number: i32,
    speaker_role: String,
    speaker_name: Option<String>,
    content_en: String,
    content_zh: String,
    phonetic_transcription: Option<String>,
    asset_phrases: Option<Vec<String>>,
    notes: Option<String>,
}

// ============================================================================
// Import stats
// ============================================================================

#[derive(Debug, Default)]
struct ImportStats {
    stages_added: usize,
    stages_skipped: usize,
    scripts_added: usize,
    scripts_skipped: usize,
    turns_added: usize,
    errors: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let mut stages_file = PathBuf::from(DEFAULT_STAGES_FILE);
    let mut scripts_file = PathBuf::from(DEFAULT_SCRIPTS_FILE);
    let mut dry_run = false;
    let mut skip_stages = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--stages" => {
                if i + 1 < args.len() {
                    stages_file = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --stages requires a path argument");
                    std::process::exit(1);
                }
            }
            "--scripts" => {
                if i + 1 < args.len() {
                    scripts_file = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --scripts requires a path argument");
                    std::process::exit(1);
                }
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            "--skip-stages" => {
                skip_stages = true;
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
    println!("Scripts Import Tool");
    println!("===============================================");
    println!("Stages file: {}", stages_file.display());
    println!("Scripts file: {}", scripts_file.display());
    println!("Dry run: {}", dry_run);
    println!("Skip stages: {}", skip_stages);
    println!();

    // Load stages JSON
    println!("Loading stages JSON...");
    let stages_content = fs::read_to_string(&stages_file)?;
    let stages_data: StagesFile = serde_json::from_str(&stages_content)?;
    println!("Loaded {} stages", stages_data.stages.len());

    // Load scripts JSON
    println!("Loading scripts JSON...");
    let scripts_content = fs::read_to_string(&scripts_file)?;
    let scripts_data: ScriptsFile = serde_json::from_str(&scripts_content)?;
    let total_turns: usize = scripts_data.scripts.iter().map(|s| s.turns.len()).sum();
    println!(
        "Loaded {} scripts with {} total turns",
        scripts_data.scripts.len(),
        total_turns
    );

    if dry_run {
        println!("\n--- DRY RUN: Preview ---");
        println!("\nStages to import:");
        for (i, stage) in stages_data.stages.iter().take(10).enumerate() {
            println!(
                "  {}. {} - {} (difficulty: {:?})",
                i + 1,
                stage.code,
                stage.name_zh,
                stage.difficulty
            );
        }
        if stages_data.stages.len() > 10 {
            println!("  ... and {} more", stages_data.stages.len() - 10);
        }

        println!("\nScripts to import:");
        for (i, script) in scripts_data.scripts.iter().take(10).enumerate() {
            println!(
                "  {}. [{}] {} - {} ({} turns)",
                i + 1,
                script.stage_code,
                script.code,
                script.title_zh,
                script.turns.len()
            );
        }
        if scripts_data.scripts.len() > 10 {
            println!("  ... and {} more", scripts_data.scripts.len() - 10);
        }

        println!("\nDry run complete. Use without --dry-run to import.");
        return Ok(());
    }

    // Connect to database
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = AsyncPgConnection::establish(&database_url).await?;

    let start_time = Instant::now();
    let mut stats = ImportStats::default();

    // Import stages
    let stage_map = if skip_stages {
        println!("\nSkipping stages import, loading existing stages...");
        load_existing_stages(&mut conn).await?
    } else {
        println!("\nImporting stages...");
        import_stages(&mut conn, &stages_data.stages, &mut stats).await?
    };

    println!("Stage code -> ID mapping: {} entries", stage_map.len());

    // Import scripts and turns
    println!("\nImporting scripts and turns...");
    import_scripts(&mut conn, &scripts_data.scripts, &stage_map, &mut stats).await?;

    let elapsed = start_time.elapsed();

    println!("\n===============================================");
    println!("Import Summary");
    println!("===============================================");
    println!("Stages added: {}", stats.stages_added);
    println!("Stages skipped: {}", stats.stages_skipped);
    println!("Scripts added: {}", stats.scripts_added);
    println!("Scripts skipped: {}", stats.scripts_skipped);
    println!("Turns added: {}", stats.turns_added);
    println!("Errors: {}", stats.errors);
    println!("Total time: {:.2}s", elapsed.as_secs_f64());

    Ok(())
}

async fn load_existing_stages(
    conn: &mut AsyncPgConnection,
) -> Result<HashMap<String, i64>, Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::asset_stages;

    let stages: Vec<(i64, String)> = asset_stages::table
        .select((asset_stages::id, asset_stages::code))
        .load(conn)
        .await?;

    let map: HashMap<String, i64> = stages.into_iter().map(|(id, code)| (code, id)).collect();

    Ok(map)
}

async fn import_stages(
    conn: &mut AsyncPgConnection,
    stages: &[RawStage],
    stats: &mut ImportStats,
) -> Result<HashMap<String, i64>, Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::asset_stages;

    let mut stage_map: HashMap<String, i64> = HashMap::new();

    for stage in stages {
        // Check if stage already exists
        let existing: Option<i64> = asset_stages::table
            .filter(asset_stages::code.eq(&stage.code))
            .select(asset_stages::id)
            .first(conn)
            .await
            .optional()?;

        if let Some(id) = existing {
            println!(
                "  Stage '{}' already exists (id={}), skipping",
                stage.code, id
            );
            stage_map.insert(stage.code.clone(), id);
            stats.stages_skipped += 1;
            continue;
        }

        // Insert new stage
        let result = diesel::insert_into(asset_stages::table)
            .values((
                asset_stages::code.eq(&stage.code),
                asset_stages::name_en.eq(&stage.name_en),
                asset_stages::name_zh.eq(&stage.name_zh),
                asset_stages::description_en.eq(&stage.description_en),
                asset_stages::description_zh.eq(&stage.description_zh),
                asset_stages::icon_emoji.eq(&stage.icon_emoji),
                asset_stages::display_order.eq(stage.display_order),
                asset_stages::difficulty.eq(stage.difficulty),
                asset_stages::is_active.eq(stage.is_active.unwrap_or(true)),
            ))
            .returning(asset_stages::id)
            .get_result::<i64>(conn)
            .await;

        match result {
            Ok(id) => {
                println!("  Added stage '{}' (id={})", stage.code, id);
                stage_map.insert(stage.code.clone(), id);
                stats.stages_added += 1;
            }
            Err(e) => {
                eprintln!("  Error inserting stage '{}': {}", stage.code, e);
                stats.errors += 1;
            }
        }
    }

    Ok(stage_map)
}

async fn import_scripts(
    conn: &mut AsyncPgConnection,
    scripts: &[RawScript],
    stage_map: &HashMap<String, i64>,
    stats: &mut ImportStats,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use colang::db::schema::{asset_script_turns, asset_scripts};

    let total = scripts.len();

    for (idx, script) in scripts.iter().enumerate() {
        if (idx + 1) % 50 == 0 || idx + 1 == total {
            println!("Progress: {}/{} scripts", idx + 1, total);
        }

        // Find stage_id
        let stage_id = match stage_map.get(&script.stage_code) {
            Some(id) => *id,
            None => {
                eprintln!(
                    "  Error: Stage '{}' not found for script '{}'",
                    script.stage_code, script.code
                );
                stats.errors += 1;
                continue;
            }
        };

        // Check if script already exists
        let existing: Option<i64> = asset_scripts::table
            .filter(asset_scripts::code.eq(&script.code))
            .select(asset_scripts::id)
            .first(conn)
            .await
            .optional()?;

        if existing.is_some() {
            stats.scripts_skipped += 1;
            continue;
        }

        // Insert script
        let script_id = match diesel::insert_into(asset_scripts::table)
            .values((
                asset_scripts::stage_id.eq(stage_id),
                asset_scripts::code.eq(&script.code),
                asset_scripts::title_en.eq(&script.title_en),
                asset_scripts::title_zh.eq(&script.title_zh),
                asset_scripts::description_en.eq(&script.description_en),
                asset_scripts::description_zh.eq(&script.description_zh),
                asset_scripts::total_turns
                    .eq(script.total_turns.or(Some(script.turns.len() as i32))),
                asset_scripts::estimated_duration_seconds.eq(script.estimated_duration_seconds),
                asset_scripts::difficulty.eq(script.difficulty),
            ))
            .returning(asset_scripts::id)
            .get_result::<i64>(conn)
            .await
        {
            Ok(id) => {
                stats.scripts_added += 1;
                id
            }
            Err(e) => {
                eprintln!("  Error inserting script '{}': {}", script.code, e);
                stats.errors += 1;
                continue;
            }
        };

        // Insert turns
        for turn in &script.turns {
            let phrases_json: Option<Value> = turn
                .asset_phrases
                .as_ref()
                .map(|p| serde_json::to_value(p).unwrap_or(Value::Null));

            let result = diesel::insert_into(asset_script_turns::table)
                .values((
                    asset_script_turns::script_id.eq(script_id),
                    asset_script_turns::turn_number.eq(turn.turn_number),
                    asset_script_turns::speaker_role.eq(&turn.speaker_role),
                    asset_script_turns::speaker_name.eq(&turn.speaker_name),
                    asset_script_turns::content_en.eq(&turn.content_en),
                    asset_script_turns::content_zh.eq(&turn.content_zh),
                    asset_script_turns::phonetic_transcription.eq(&turn.phonetic_transcription),
                    asset_script_turns::asset_phrases.eq(phrases_json),
                    asset_script_turns::notes.eq(&turn.notes),
                ))
                .execute(conn)
                .await;

            match result {
                Ok(_) => {
                    stats.turns_added += 1;
                }
                Err(e) => {
                    eprintln!(
                        "  Error inserting turn {} for script '{}': {}",
                        turn.turn_number, script.code, e
                    );
                    stats.errors += 1;
                }
            }
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Scripts Import Tool");
    println!();
    println!("Imports stages and scripts from JSON files into the database.");
    println!();
    println!("Usage:");
    println!("  cargo run --bin import_scripts [OPTIONS]");
    println!();
    println!("Options:");
    println!(
        "  --stages <PATH>    Stages JSON file (default: {})",
        DEFAULT_STAGES_FILE
    );
    println!(
        "  --scripts <PATH>   Scripts JSON file (default: {})",
        DEFAULT_SCRIPTS_FILE
    );
    println!("  --skip-stages      Skip importing stages (use existing)");
    println!("  --dry-run          Preview without importing to database");
    println!("  -h, --help         Show this help message");
    println!();
    println!("Environment:");
    println!("  DATABASE_URL       PostgreSQL connection URL (required)");
    println!();
    println!("Examples:");
    println!("  cargo run --bin import_scripts --dry-run");
    println!("  cargo run --bin import_scripts");
    println!("  cargo run --bin import_scripts --skip-stages");
}
