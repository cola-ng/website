//! Generate TTS audio for script turns
//!
//! Reads asset_script_turns table and generates audio for each turn's content_en
//! using Zhipu TTS API. Saves audio files and updates audio_path in database.

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use colang::db::pool::DieselPool;
use diesel::prelude::*;

const DEFAULT_OUTPUT_DIR: &str = r"D:\Works\cola-ng\space\script_turns";

#[derive(Debug, Queryable)]
struct ScriptTurn {
    id: i64,
    script_id: i64,
    turn_number: i32,
    content_en: String,
    audio_path: Option<String>,
}

#[derive(Debug, Default)]
struct Stats {
    generated: usize,
    skipped: usize,
    errors: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let mut output_dir = PathBuf::from(DEFAULT_OUTPUT_DIR);
    let mut dry_run = false;
    let mut force = false;
    let mut limit: Option<usize> = None;
    let mut script_id_filter: Option<i64> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--output" | "-o" => {
                if i + 1 < args.len() {
                    output_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --output requires a path argument");
                    std::process::exit(1);
                }
            }
            "--script" | "-s" => {
                if i + 1 < args.len() {
                    script_id_filter = Some(args[i + 1].parse().unwrap_or(0));
                    i += 2;
                } else {
                    eprintln!("Error: --script requires an ID argument");
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
            "--force" | "-f" => {
                force = true;
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
    println!("Script Turns TTS Generator");
    println!("===============================================");
    println!("Output directory: {}", output_dir.display());
    println!("Dry run: {}", dry_run);
    println!("Force regenerate: {}", force);
    if let Some(sid) = script_id_filter {
        println!("Script ID filter: {}", sid);
    }
    if let Some(l) = limit {
        println!("Limit: {} turns", l);
    }
    println!();

    // Check Zhipu API key
    let api_key = std::env::var("ZHIPU_API_KEY")
        .or_else(|_| std::env::var("ZHIPUAI_API_KEY"))
        .ok()
        .filter(|s| !s.is_empty());

    if api_key.is_none() {
        eprintln!("Error: ZHIPU_API_KEY environment variable not set");
        std::process::exit(1);
    }

    // Connect to database
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = DieselPool::new(
        &database_url,
        &Default::default(),
        diesel::r2d2::Pool::builder(),
    )?;
    let mut conn = pool.get()?;

    // Load script turns
    println!("Loading script turns from database...");
    let turns = load_script_turns(&mut conn, script_id_filter, force, limit)?;
    println!("Found {} turns to process", turns.len());

    if turns.is_empty() {
        println!("No turns to process.");
        return Ok(());
    }

    if dry_run {
        println!("\n--- DRY RUN: Preview ---");
        for (i, turn) in turns.iter().take(10).enumerate() {
            println!(
                "  {}. script_{}_turn_{}: \"{}...\"",
                i + 1,
                turn.script_id,
                turn.turn_number,
                &turn.content_en[..turn.content_en.len().min(50)]
            );
        }
        if turns.len() > 10 {
            println!("  ... and {} more", turns.len() - 10);
        }
        println!("\nDry run complete. Use without --dry-run to generate audio.");
        return Ok(());
    }

    // Create output directory
    fs::create_dir_all(&output_dir)?;

    // Initialize Zhipu client
    let zhipu = colang::services::zhipu::ZhipuClient::from_env()
        .expect("Failed to create Zhipu client");

    let start_time = Instant::now();
    let mut stats = Stats::default();
    let total = turns.len();

    // Process each turn
    for (idx, turn) in turns.iter().enumerate() {
        if (idx + 1) % 10 == 0 || idx + 1 == total {
            println!("Progress: {}/{} turns", idx + 1, total);
        }

        let mp3_filename = format!("script_{}_turn_{}.mp3", turn.script_id, turn.turn_number);
        let mp3_path = output_dir.join(&mp3_filename);
        let relative_path = format!("script_turns/{}", mp3_filename);

        // Temporary WAV file path
        let wav_filename = format!("script_{}_turn_{}.wav", turn.script_id, turn.turn_number);
        let wav_path = output_dir.join(&wav_filename);

        // Check if MP3 or WAV file already exists on disk (skip if exists and not force)
        if !force {
            if mp3_path.exists() {
                println!(
                    "  Skipping turn {} (script={}, turn={}): MP3 file already exists",
                    turn.id, turn.script_id, turn.turn_number
                );
                // Update database if audio_path is not set
                if turn.audio_path.is_none() {
                    if let Err(e) = update_audio_path(&mut conn, turn.id, &relative_path) {
                        eprintln!("  Error updating database for turn {}: {}", turn.id, e);
                    }
                }
                stats.skipped += 1;
                continue;
            }
            if wav_path.exists() {
                println!(
                    "  Skipping turn {} (script={}, turn={}): WAV file already exists",
                    turn.id, turn.script_id, turn.turn_number
                );
                stats.skipped += 1;
                continue;
            }
        }

        // Generate TTS
        use colang::services::ai_provider::TtsService;
        match zhipu.synthesize(&turn.content_en, None, None).await {
            Ok(response) => {
                // Save audio file as WAV first
                match File::create(&wav_path) {
                    Ok(mut file) => {
                        if let Err(e) = file.write_all(&response.audio_data) {
                            eprintln!("  Error writing WAV file for turn {}: {}", turn.id, e);
                            stats.errors += 1;
                            continue;
                        }
                    }
                    Err(e) => {
                        eprintln!("  Error creating WAV file for turn {}: {}", turn.id, e);
                        stats.errors += 1;
                        continue;
                    }
                }

                // Convert WAV to MP3 using ffmpeg
                let ffmpeg_result = Command::new("ffmpeg")
                    .args([
                        "-y",           // Overwrite output file
                        "-i",
                        wav_path.to_str().unwrap(),
                        "-codec:a",
                        "libmp3lame",
                        "-qscale:a",
                        "2",            // High quality VBR
                        mp3_path.to_str().unwrap(),
                    ])
                    .output();

                match ffmpeg_result {
                    Ok(output) => {
                        if !output.status.success() {
                            eprintln!(
                                "  Error converting to MP3 for turn {}: {}",
                                turn.id,
                                String::from_utf8_lossy(&output.stderr)
                            );
                            stats.errors += 1;
                            // Clean up WAV file
                            let _ = fs::remove_file(&wav_path);
                            continue;
                        }
                        // Remove temporary WAV file after successful conversion
                        let _ = fs::remove_file(&wav_path);
                    }
                    Err(e) => {
                        eprintln!(
                            "  Error running ffmpeg for turn {}: {} (is ffmpeg installed?)",
                            turn.id, e
                        );
                        stats.errors += 1;
                        // Clean up WAV file
                        let _ = fs::remove_file(&wav_path);
                        continue;
                    }
                }

                // Update database
                if let Err(e) = update_audio_path(&mut conn, turn.id, &relative_path) {
                    eprintln!("  Error updating database for turn {}: {}", turn.id, e);
                    stats.errors += 1;
                    continue;
                }

                stats.generated += 1;
            }
            Err(e) => {
                eprintln!(
                    "  Error generating TTS for turn {} (script={}, turn={}): {}",
                    turn.id, turn.script_id, turn.turn_number, e
                );
                stats.errors += 1;
            }
        }

        // Small delay to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    let elapsed = start_time.elapsed();

    println!("\n===============================================");
    println!("Generation Summary");
    println!("===============================================");
    println!("Audio files generated: {}", stats.generated);
    println!("Skipped (already have audio): {}", stats.skipped);
    println!("Errors: {}", stats.errors);
    println!("Total time: {:.2}s", elapsed.as_secs_f64());

    Ok(())
}

fn load_script_turns(
    conn: &mut PgConnection,
    script_id_filter: Option<i64>,
    _force: bool,
    limit: Option<usize>,
) -> Result<Vec<ScriptTurn>, Box<dyn std::error::Error>> {
    use colang::db::schema::asset_script_turns;

    let mut query = asset_script_turns::table
        .select((
            asset_script_turns::id,
            asset_script_turns::script_id,
            asset_script_turns::turn_number,
            asset_script_turns::content_en,
            asset_script_turns::audio_path,
        ))
        .order((
            asset_script_turns::script_id.asc(),
            asset_script_turns::turn_number.asc(),
        ))
        .into_boxed();

    // Filter by script_id if specified
    if let Some(sid) = script_id_filter {
        query = query.filter(asset_script_turns::script_id.eq(sid));
    }

    // Note: We load all turns and check file existence on disk instead of
    // filtering by audio_path, since the file might have been deleted.

    // Apply limit
    if let Some(l) = limit {
        query = query.limit(l as i64);
    }

    let turns = query.load::<ScriptTurn>(conn)?;
    Ok(turns)
}

fn update_audio_path(
    conn: &mut PgConnection,
    turn_id: i64,
    audio_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use colang::db::schema::asset_script_turns;

    diesel::update(asset_script_turns::table.find(turn_id))
        .set(asset_script_turns::audio_path.eq(audio_path))
        .execute(conn)?;

    Ok(())
}

fn print_usage() {
    println!("Script Turns TTS Generator");
    println!();
    println!("Generates TTS audio for script turns using Zhipu API.");
    println!();
    println!("Usage:");
    println!("  cargo run --bin scripts_audio [OPTIONS]");
    println!();
    println!("Options:");
    println!(
        "  -o, --output <PATH>    Output directory (default: {})",
        DEFAULT_OUTPUT_DIR
    );
    println!("  -s, --script <ID>      Only process turns for specific script ID");
    println!("  -l, --limit <N>        Limit number of turns to process");
    println!("  -f, --force            Regenerate audio even if file already exists");
    println!("  --dry-run              Preview without generating audio");
    println!("  -h, --help             Show this help message");
    println!();
    println!("Environment:");
    println!("  DATABASE_URL           PostgreSQL connection URL (required)");
    println!("  ZHIPU_API_KEY          Zhipu API key (required)");
    println!();
    println!("Dependencies:");
    println!("  ffmpeg                 Required for WAV to MP3 conversion");
    println!();
    println!("Examples:");
    println!("  cargo run --bin scripts_audio --dry-run");
    println!("  cargo run --bin scripts_audio --script 1");
    println!("  cargo run --bin scripts_audio --limit 50");
    println!("  cargo run --bin scripts_audio --force");
    println!();
    println!("Note: Audio is generated as WAV and converted to MP3 using ffmpeg.");
    println!("      Skips generation if MP3 or WAV file already exists (use --force to override).");
    println!("      Audio files are saved to: script_turns/script_{{id}}_turn_{{n}}.mp3");
}
