//! Phase 1 of Sudoku Generator 2.
//!
//! The generator fills a 9×9 grid, digs clues while DLX proves there is still
//! one solution, and stores the puzzle through SQLite intake. Intake keeps the
//! original grid and rejects any later puzzle with the same min-lex
//! automorphism key. Skill solving, solvability layers, and the local web UI
//! are later phases. This crate does not classify puzzles by skill, band, or score.

mod bank;
mod catalog;
mod exact_cover;
mod generate;
mod grid;
mod solver;
mod symmetry;
mod trace;

pub use bank::{Bank, InsertResult, PuzzleRecord};
pub use catalog::{Technique, TechniqueFamily, APPENDIX, GAME};
pub use exact_cover::{solve, SolveResult};
pub use generate::{generate_into, generate_one, BatchReport, GenerateOptions, Puzzle};
pub use grid::Grid;
pub use solver::SolveWithTrace;
pub use symmetry::{
    canonical_key, mirror_horizontal, mirror_vertical, relabel, rotate180, rotate270, rotate90,
    swap_bands, swap_cols_in_stack, swap_rows_in_band, swap_stacks, transpose,
};
pub use trace::{PatternCell, Placement, Reason, Step, Trace};

use std::fmt;

/// Failures from parsing, generation, intake, or the database.
#[derive(Debug)]
pub enum Error {
    InvalidGrid(String),
    InvalidPuzzle(&'static str),
    /// `found` is the number of solutions discovered before the search stopped.
    /// A uniqueness check stops at 2, so `found == 2` means two or more.
    NotUnique {
        found: u32,
    },
    SolutionMismatch,
    NoSolution,
    InvalidTransform(&'static str),
    InvalidStep(String),
    Io(std::io::Error),
    Database(rusqlite::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidGrid(msg) => write!(f, "invalid grid: {msg}"),
            Error::InvalidPuzzle(msg) => write!(f, "invalid puzzle: {msg}"),
            Error::NotUnique { found } => write!(
                f,
                "expected exactly one solution, found {found} before the search limit"
            ),
            Error::SolutionMismatch => {
                write!(f, "the supplied solution is not the unique completion")
            }
            Error::NoSolution => write!(f, "the exact-cover search found no solution"),
            Error::InvalidTransform(msg) => write!(f, "invalid transform: {msg}"),
            Error::InvalidStep(msg) => write!(f, "invalid solving step: {msg}"),
            Error::Io(err) => write!(f, "io error: {err}"),
            Error::Database(err) => write!(f, "database error: {err}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Error::Database(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::InvalidStep(value.to_string())
    }
}
