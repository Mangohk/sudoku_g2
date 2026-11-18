use std::path::Path;

use rand::rngs::StdRng;
use rand::SeedableRng;
use sudoku_generator::{
    generate_into, generate_one, solve, Bank, GenerateOptions, InsertResult, Puzzle,
};
use sudoku_generator::{mirror_vertical, relabel, rotate180, swap_bands};

fn temp_db() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "sudoku-generate-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn cleanup(path: &Path) {
    let text = path.to_string_lossy().to_string();
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{text}-wal"));
    let _ = std::fs::remove_file(format!("{text}-shm"));
}

#[test]
fn generate_and_store_skips_canonical_duplicates() {
    let path = temp_db();
    let options = GenerateOptions {
        target_clues: Some(48),
    };
    let mut bank = Bank::open(&path).unwrap();

    let mut rng = StdRng::seed_from_u64(11);
    let first = generate_one(&mut rng, &options).unwrap();
    let second = generate_one(&mut rng, &options).unwrap();
    assert_ne!(first.givens.to_line(), second.givens.to_line());

    let stored_first = bank.insert_puzzle(&first).unwrap();
    let stored_second = bank.insert_puzzle(&second).unwrap();
    assert!(matches!(stored_first, InsertResult::Stored { .. }));
    assert!(matches!(stored_second, InsertResult::Stored { .. }));
    assert_eq!(bank.puzzle_count().unwrap(), 2);

    for puzzle in [&first, &second] {
        assert!(puzzle.solution.is_complete());
        assert!(puzzle.givens.clue_count() <= 48);
        let solved = solve(&puzzle.givens, 2);
        assert_eq!(solved.found, 1);
        assert_eq!(solved.grids[0], puzzle.solution);
    }
    for stored in [&stored_first, &stored_second] {
        let InsertResult::Stored { puzzle_id, .. } = stored else {
            panic!("expected the generated puzzle to be stored");
        };
        let record = bank.get(puzzle_id).unwrap().unwrap();
        assert!(record.solvability.is_none());
        assert!(record.skills_used.is_none());
        assert_eq!(record.solution.len(), 81);
        assert!(!record.solution.contains('.'));
    }

    let map = {
        let mut map = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        map[3] = 7;
        map[7] = 3;
        map
    };
    let twisted = Puzzle {
        givens: relabel(
            &mirror_vertical(&rotate180(&swap_bands(&first.givens, 0, 1).unwrap())),
            &map,
        )
        .unwrap(),
        solution: relabel(
            &mirror_vertical(&rotate180(&swap_bands(&first.solution, 0, 1).unwrap())),
            &map,
        )
        .unwrap(),
    };
    assert_ne!(twisted.givens.to_line(), first.givens.to_line());
    assert!(twisted.solution.is_complete());
    match bank.insert_puzzle(&twisted).unwrap() {
        InsertResult::Duplicate { puzzle_id, .. } => {
            let InsertResult::Stored {
                puzzle_id: original,
                ..
            } = stored_first
            else {
                unreachable!();
            };
            assert_eq!(puzzle_id, original);
        }
        InsertResult::Stored { puzzle_id, .. } => {
            panic!("isomorphic puzzle was stored as {puzzle_id}")
        }
    }
    assert_eq!(bank.puzzle_count().unwrap(), 2);

    let mut replay = StdRng::seed_from_u64(11);
    let report = generate_into(&mut bank, &mut replay, 2, &options).unwrap();
    assert_eq!(report.stored(), 0);
    assert_eq!(report.skipped(), 2);
    assert_eq!(bank.puzzle_count().unwrap(), 2);

    drop(bank);
    cleanup(&path);
}
