//! Command-line intake for Sudoku Generator 2.
//!
//! `cargo run` generates a few unique puzzles into a local SQLite file.
//! Run the same seed again and intake skips puzzles it already stored under
//! that canonical form.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sudoku_generator::{generate_into, Bank, GenerateOptions, InsertResult};

#[derive(Parser)]
#[command(
    name = "sudoku-generator",
    version,
    about = "Generate uniquely solvable 9×9 Sudoku puzzles into a local SQLite bank"
)]
struct Cli {
    /// How many puzzles to generate this run.
    #[arg(short, long, default_value_t = 5)]
    count: u32,

    /// SQLite database path.
    #[arg(short = 'd', long, default_value = "puzzles.sqlite")]
    database: PathBuf,

    /// Deterministic seed. The same seed skips puzzles already stored.
    #[arg(short, long)]
    seed: Option<u64>,

    /// Stop digging once this many clues remain.
    #[arg(long, default_value_t = 36)]
    clues: u8,

    /// Keep removing clues until every remaining clue is required for uniqueness.
    #[arg(long)]
    minimal: bool,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    if cli.clues > 81 {
        return Err("clues must be from 0 to 81".into());
    }
    let seed = cli.seed.unwrap_or_else(random_seed);
    let options = GenerateOptions {
        target_clues: if cli.minimal { None } else { Some(cli.clues) },
    };

    println!("seed {seed}");
    println!("database {}", cli.database.display());
    match options.target_clues {
        Some(clues) => println!("dig until {clues} clues, or until no further clue can be removed"),
        None => println!("dig until the puzzle is minimal for this removal order"),
    }

    let mut bank = Bank::open(&cli.database)?;
    let mut rng = StdRng::seed_from_u64(seed);
    let report = generate_into(&mut bank, &mut rng, cli.count, &options)?;
    for (index, result) in report.results.iter().enumerate() {
        match result {
            InsertResult::Stored {
                puzzle_id,
                clue_count,
            } => {
                println!(
                    "  {:>3} stored   clues {clue_count:<2}  {}",
                    index + 1,
                    short_id(puzzle_id)
                );
            }
            InsertResult::Duplicate {
                puzzle_id,
                clue_count,
            } => {
                println!(
                    "  {:>3} skipped  clues {clue_count:<2}  {}  same canonical form",
                    index + 1,
                    short_id(puzzle_id)
                );
            }
        }
    }
    println!(
        "done: stored {}, skipped {}, rows {}",
        report.stored(),
        report.skipped(),
        bank.puzzle_count()?
    );
    Ok(())
}

fn short_id(puzzle_id: &str) -> &str {
    let end = puzzle_id.len().min(12);
    &puzzle_id[..end]
}

fn random_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x5eed)
}
