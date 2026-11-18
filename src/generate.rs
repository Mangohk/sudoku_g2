//! Randomized completion followed by clue digging.
//!
//! Digging removes a cell only while DLX still reports exactly one solution.
//! It does not aim at a skill, a band, or a score, and it does not emit
//! rotated or relabeled copies. Those transforms are intake's job to reject.

use rand::seq::SliceRandom;
use rand::Rng;

use crate::bank::{Bank, InsertResult};
use crate::exact_cover::{solve, solve_random};
use crate::grid::Grid;
use crate::Error;

/// A stored candidate: the clue grid and the unique completed answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    pub givens: Grid,
    pub solution: Grid,
}

#[derive(Debug, Clone, Copy)]
pub struct GenerateOptions {
    /// Dig until at most this many clues remain.
    ///
    /// `None` keeps removing while the puzzle stays unique, which produces a
    /// minimal puzzle for that removal order. The digger never goes below the
    /// target. A removal order can get stuck above it; a few reshuffles are tried.
    pub target_clues: Option<u8>,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            target_clues: Some(36),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchReport {
    pub results: Vec<InsertResult>,
}

impl BatchReport {
    pub fn stored(&self) -> usize {
        self.results
            .iter()
            .filter(|result| matches!(result, InsertResult::Stored { .. }))
            .count()
    }

    pub fn skipped(&self) -> usize {
        self.results
            .iter()
            .filter(|result| matches!(result, InsertResult::Duplicate { .. }))
            .count()
    }
}

/// Generate `count` puzzles and offer each one to intake.
///
/// A puzzle whose canonical form is already stored is counted as skipped.
/// The same seed therefore stores on the first run and skips on the next.
pub fn generate_into(
    bank: &mut Bank,
    rng: &mut impl Rng,
    count: u32,
    options: &GenerateOptions,
) -> Result<BatchReport, Error> {
    let mut results = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let puzzle = generate_one(rng, options)?;
        results.push(bank.insert_puzzle(&puzzle)?);
    }
    Ok(BatchReport { results })
}

pub fn generate_one(rng: &mut impl Rng, options: &GenerateOptions) -> Result<Puzzle, Error> {
    let solution = fill_grid(rng)?;
    let givens = dig(&solution, rng, options.target_clues);
    let solved = solve(&givens, 2);
    if solved.found != 1 {
        return Err(Error::NotUnique {
            found: solved.found,
        });
    }
    if solved.grids[0] != solution {
        return Err(Error::SolutionMismatch);
    }
    Ok(Puzzle { givens, solution })
}

pub(crate) fn fill_grid(rng: &mut impl Rng) -> Result<Grid, Error> {
    solve_random(&Grid::empty(), rng, 1)
        .grids
        .into_iter()
        .next()
        .ok_or(Error::NoSolution)
}

fn dig(solution: &Grid, rng: &mut impl Rng, target: Option<u8>) -> Grid {
    let Some(target) = target else {
        return dig_once(solution, rng, 0);
    };
    let mut best = dig_once(solution, rng, target);
    for _ in 0..3 {
        if best.clue_count() <= target {
            break;
        }
        let next = dig_once(solution, rng, target);
        if next.clue_count() < best.clue_count() {
            best = next;
        }
    }
    best
}

fn dig_once(solution: &Grid, rng: &mut impl Rng, target: u8) -> Grid {
    let mut puzzle = *solution;
    let mut order: Vec<usize> = (0..81).collect();
    order.shuffle(rng);
    for cell in order {
        if puzzle.clue_count() <= target {
            break;
        }
        let saved = puzzle.cells()[cell];
        if saved == 0 {
            continue;
        }
        puzzle.clear_index(cell);
        if solve(&puzzle, 2).found != 1 {
            puzzle.set_index(cell, saved);
        }
    }
    puzzle
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn generated_puzzle_is_unique_and_reaches_the_clue_target() {
        let mut rng = StdRng::seed_from_u64(11);
        let puzzle = generate_one(
            &mut rng,
            &GenerateOptions {
                target_clues: Some(50),
            },
        )
        .unwrap();
        assert!(puzzle.solution.is_complete());
        assert!(puzzle.givens.clue_count() <= 50);
        assert!(puzzle.givens.clue_count() < 81);
        for index in 0..81 {
            let given = puzzle.givens.cells()[index];
            if given != 0 {
                assert_eq!(given, puzzle.solution.cells()[index]);
            }
        }
        let solved = solve(&puzzle.givens, 2);
        assert_eq!(solved.found, 1);
        assert_eq!(solved.grids[0], puzzle.solution);

        let mut again = StdRng::seed_from_u64(11);
        let repeat = generate_one(
            &mut again,
            &GenerateOptions {
                target_clues: Some(50),
            },
        )
        .unwrap();
        assert_eq!(repeat, puzzle);
    }

    #[test]
    fn minimal_puzzle_loses_uniqueness_if_any_clue_is_removed() {
        let mut rng = StdRng::seed_from_u64(3);
        let puzzle = generate_one(&mut rng, &GenerateOptions { target_clues: None }).unwrap();
        assert!(puzzle.givens.clue_count() >= 17);
        for index in 0..81 {
            if puzzle.givens.cells()[index] == 0 {
                continue;
            }
            let mut probe = puzzle.givens;
            probe.clear_index(index);
            assert_ne!(
                solve(&probe, 2).found,
                1,
                "clue {index} was not required for uniqueness"
            );
        }
    }
}
