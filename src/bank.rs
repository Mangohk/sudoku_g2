//! SQLite intake.
//!
//! A puzzle is stored only when its solution is a completed grid, the givens
//! agree with that solution, and DLX finds that solution alone. The primary
//! key is the SHA-256 of the min-lex automorphism key. A later image of the
//! same puzzle is a duplicate, not a new row.
//!
//! Phase 1 leaves `skills_used`, `solvability`, and `trace_catalog_rev` null.
//! The `steps` table is ready for facts, reasons, highlights, and candidate
//! snapshots. There is no difficulty, band, or score column.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

use crate::exact_cover::solve;
use crate::symmetry::canonical_key;
use crate::trace::Step;
use crate::{Error, Puzzle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertResult {
    Stored { puzzle_id: String, clue_count: u8 },
    Duplicate { puzzle_id: String, clue_count: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PuzzleRecord {
    pub puzzle_id: String,
    pub givens: String,
    pub solution: String,
    pub canonical_key: String,
    pub clue_count: u8,
    pub created_at: String,
    pub trace_catalog_rev: Option<String>,
    pub skills_used: Option<String>,
    pub solvability: Option<String>,
}

pub struct Bank {
    conn: Connection,
}

impl Bank {
    pub fn open(path: &Path) -> Result<Self, Error> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;",
        )?;
        let _: String = conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
        let bank = Bank { conn };
        bank.migrate()?;
        Ok(bank)
    }

    pub fn insert_puzzle(&mut self, puzzle: &Puzzle) -> Result<InsertResult, Error> {
        if !puzzle.solution.is_complete() {
            return Err(Error::InvalidPuzzle("solution is not a completed Sudoku"));
        }
        for index in 0..81 {
            let given = puzzle.givens.cells()[index];
            if given != 0 && given != puzzle.solution.cells()[index] {
                return Err(Error::InvalidPuzzle("givens disagree with the solution"));
            }
        }

        let key = canonical_key(&puzzle.givens);
        let puzzle_id = puzzle_id(&key);
        let clue_count = puzzle.givens.clue_count();
        if self.canonical_exists(&key)? {
            return Ok(InsertResult::Duplicate {
                puzzle_id,
                clue_count,
            });
        }

        let solved = solve(&puzzle.givens, 2);
        if solved.found != 1 {
            return Err(Error::NotUnique {
                found: solved.found,
            });
        }
        if solved.grids[0] != puzzle.solution {
            return Err(Error::SolutionMismatch);
        }

        let created_at = utc_now();
        self.conn.execute(
            "INSERT INTO puzzles (
                puzzle_id, givens, solution, canonical_key, clue_count, created_at,
                trace_catalog_rev, skills_used, solvability
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, NULL)",
            params![
                puzzle_id,
                puzzle.givens.to_line(),
                puzzle.solution.to_line(),
                key,
                i64::from(clue_count),
                created_at,
            ],
        )?;
        Ok(InsertResult::Stored {
            puzzle_id,
            clue_count,
        })
    }

    pub fn puzzle_count(&self) -> Result<i64, Error> {
        let count = self
            .conn
            .query_row("SELECT COUNT(*) FROM puzzles", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn get(&self, puzzle_id: &str) -> Result<Option<PuzzleRecord>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT puzzle_id, givens, solution, canonical_key, clue_count, created_at,
                    trace_catalog_rev, skills_used, solvability
             FROM puzzles WHERE puzzle_id = ?1",
        )?;
        let mut rows = stmt.query(params![puzzle_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(PuzzleRecord {
            puzzle_id: row.get(0)?,
            givens: row.get(1)?,
            solution: row.get(2)?,
            canonical_key: row.get(3)?,
            clue_count: row.get::<_, i64>(4)? as u8,
            created_at: row.get(5)?,
            trace_catalog_rev: row.get(6)?,
            skills_used: row.get(7)?,
            solvability: row.get(8)?,
        }))
    }

    /// Store one trace step. Phase 1 does not call this from the generator.
    ///
    /// A step with an empty observation, deduction, or action is rejected so a
    /// later solver cannot write a reason the UI would be unable to show.
    pub fn insert_step(&mut self, puzzle_id: &str, step: &Step) -> Result<(), Error> {
        validate_step(step)?;
        let exists: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM puzzles WHERE puzzle_id = ?1",
            params![puzzle_id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Err(Error::InvalidStep(format!("unknown puzzle {puzzle_id}")));
        }
        let facts = serde_json::to_string(&step.facts)?;
        let pattern = serde_json::to_string(&step.pattern)?;
        let placements = serde_json::to_string(&step.placements)?;
        let eliminations = serde_json::to_string(&step.eliminations)?;
        self.conn.execute(
            "INSERT INTO steps (
                puzzle_id, step_index, skill_id, template_id, facts_json, pattern_json,
                placements_json, eliminations_json, reason_short, reason_detailed,
                reason_observation, reason_deduction, reason_action, candidates_after
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                puzzle_id,
                i64::from(step.index),
                &step.skill_id,
                &step.template_id,
                facts,
                pattern,
                placements,
                eliminations,
                &step.reason.short,
                &step.reason.detailed,
                &step.reason.observation,
                &step.reason.deduction,
                &step.reason.action,
                &step.candidates_after,
            ],
        )?;
        Ok(())
    }

    pub fn steps(&self, puzzle_id: &str) -> Result<Vec<Step>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT step_index, skill_id, template_id, facts_json, pattern_json,
                    placements_json, eliminations_json, reason_short, reason_detailed,
                    reason_observation, reason_deduction, reason_action, candidates_after
             FROM steps WHERE puzzle_id = ?1 ORDER BY step_index",
        )?;
        let rows = stmt.query_map(params![puzzle_id], |row| {
            let facts: String = row.get(3)?;
            let pattern: String = row.get(4)?;
            let placements: String = row.get(5)?;
            let eliminations: String = row.get(6)?;
            let candidates: Vec<u8> = row.get(12)?;
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                facts,
                pattern,
                placements,
                eliminations,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                candidates,
            ))
        })?;
        let mut steps = Vec::new();
        for row in rows {
            let (
                index,
                skill_id,
                template_id,
                facts,
                pattern,
                placements,
                eliminations,
                short,
                detailed,
                observation,
                deduction,
                action,
                candidates_after,
            ) = row?;
            steps.push(Step {
                index: index as u32,
                skill_id,
                template_id,
                facts: serde_json::from_str(&facts)?,
                pattern: serde_json::from_str(&pattern)?,
                placements: serde_json::from_str(&placements)?,
                eliminations: serde_json::from_str(&eliminations)?,
                reason: crate::trace::Reason {
                    short,
                    detailed,
                    observation,
                    deduction,
                    action,
                },
                candidates_after,
            });
        }
        Ok(steps)
    }

    fn canonical_exists(&self, key: &str) -> Result<bool, Error> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM puzzles WHERE canonical_key = ?1",
            params![key],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    fn migrate(&self) -> Result<(), Error> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- One row per distinct puzzle shape. solvability is a later derived
            -- filter (game, appendix, engine_only), not a difficulty score.
            CREATE TABLE IF NOT EXISTS puzzles (
                puzzle_id TEXT PRIMARY KEY,
                givens TEXT NOT NULL CHECK (length(givens) = 81),
                solution TEXT NOT NULL CHECK (length(solution) = 81),
                canonical_key TEXT NOT NULL UNIQUE,
                clue_count INTEGER NOT NULL CHECK (clue_count BETWEEN 0 AND 81),
                created_at TEXT NOT NULL,
                trace_catalog_rev TEXT,
                skills_used TEXT,
                solvability TEXT CHECK (
                    solvability IS NULL
                    OR solvability IN ('game', 'appendix', 'engine_only')
                )
            );

            CREATE TABLE IF NOT EXISTS steps (
                puzzle_id TEXT NOT NULL REFERENCES puzzles(puzzle_id) ON DELETE CASCADE,
                step_index INTEGER NOT NULL CHECK (step_index >= 0),
                skill_id TEXT NOT NULL,
                template_id TEXT,
                facts_json TEXT NOT NULL,
                pattern_json TEXT NOT NULL,
                placements_json TEXT NOT NULL,
                eliminations_json TEXT NOT NULL,
                reason_short TEXT NOT NULL,
                reason_detailed TEXT NOT NULL,
                reason_observation TEXT NOT NULL,
                reason_deduction TEXT NOT NULL,
                reason_action TEXT NOT NULL,
                candidates_after BLOB NOT NULL CHECK (length(candidates_after) = 81),
                PRIMARY KEY (puzzle_id, step_index)
            );

            CREATE INDEX IF NOT EXISTS idx_puzzles_created_at ON puzzles(created_at);
            CREATE INDEX IF NOT EXISTS idx_puzzles_clue_count ON puzzles(clue_count);
            CREATE INDEX IF NOT EXISTS idx_puzzles_solvability ON puzzles(solvability);
            ",
        )?;
        self.conn.execute(
            "INSERT INTO schema_meta (key, value) VALUES ('version', '1')
             ON CONFLICT(key) DO NOTHING",
            [],
        )?;
        Ok(())
    }
}

fn validate_step(step: &Step) -> Result<(), Error> {
    if step.skill_id.trim().is_empty() {
        return Err(Error::InvalidStep("skill_id is empty".into()));
    }
    if step.candidates_after.len() != 81 {
        return Err(Error::InvalidStep(
            "candidates_after must be 81 bytes".into(),
        ));
    }
    for (name, text) in [
        ("short", step.reason.short.as_str()),
        ("detailed", step.reason.detailed.as_str()),
        ("observation", step.reason.observation.as_str()),
        ("deduction", step.reason.deduction.as_str()),
        ("action", step.reason.action.as_str()),
    ] {
        if text.trim().is_empty() {
            return Err(Error::InvalidStep(format!("reason.{name} is empty")));
        }
    }
    Ok(())
}

fn puzzle_id(canonical: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(canonical.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push(HEX[(byte >> 4) as usize] as char);
        hex.push(HEX[(byte & 0x0f) as usize] as char);
    }
    hex
}

fn utc_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format_unix_utc(secs)
}

fn format_unix_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let tod = secs % 86_400;
    let hour = tod / 3_600;
    let minute = (tod % 3_600) / 60;
    let second = tod % 60;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days`, days relative to 1970-01-01.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::fill_grid;
    use crate::grid::{sample_solution, Grid};
    use crate::symmetry::{relabel, transpose};
    use crate::trace::{PatternCell, Placement, Reason};
    use crate::Puzzle;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn temp_db(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "sudoku-bank-{name}-{}-{}.sqlite",
            std::process::id(),
            name.len()
        ))
    }

    fn cleanup(path: &Path) {
        let text = path.to_string_lossy().to_string();
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{text}-wal"));
        let _ = std::fs::remove_file(format!("{text}-shm"));
    }

    fn complete_puzzle() -> Puzzle {
        let solution = sample_solution();
        Puzzle {
            givens: solution,
            solution,
        }
    }

    #[test]
    fn epoch_formats_as_utc() {
        assert_eq!(format_unix_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_unix_utc(86_400), "1970-01-02T00:00:00Z");
    }

    #[test]
    fn stores_without_skill_or_score_and_skips_an_isomorphic_copy() {
        let path = temp_db("dedup");
        let mut bank = Bank::open(&path).unwrap();
        let puzzle = complete_puzzle();
        let stored = bank.insert_puzzle(&puzzle).unwrap();
        let InsertResult::Stored {
            puzzle_id,
            clue_count,
        } = stored
        else {
            panic!("expected a new row");
        };
        assert_eq!(clue_count, 81);

        let record = bank.get(&puzzle_id).unwrap().unwrap();
        assert!(record.solvability.is_none());
        assert!(record.skills_used.is_none());
        assert!(record.trace_catalog_rev.is_none());
        assert_eq!(record.givens, puzzle.givens.to_line());
        assert_eq!(record.solution, puzzle.solution.to_line());
        assert_eq!(record.clue_count, 81);

        let mut map = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        map[1] = 2;
        map[2] = 1;
        let twisted_grid = relabel(&transpose(&puzzle.solution), &map).unwrap();
        assert_ne!(twisted_grid.to_line(), puzzle.solution.to_line());
        let twisted = Puzzle {
            givens: twisted_grid,
            solution: twisted_grid,
        };
        let duplicate = bank.insert_puzzle(&twisted).unwrap();
        assert_eq!(
            duplicate,
            InsertResult::Duplicate {
                puzzle_id: puzzle_id.clone(),
                clue_count: 81,
            }
        );
        assert_eq!(bank.puzzle_count().unwrap(), 1);

        let names = column_names(&bank, "puzzles");
        assert_eq!(
            names,
            vec![
                "puzzle_id",
                "givens",
                "solution",
                "canonical_key",
                "clue_count",
                "created_at",
                "trace_catalog_rev",
                "skills_used",
                "solvability",
            ]
        );
        for banned in ["score", "difficulty", "hardest", "band", "technique"] {
            assert!(!names.iter().any(|name| name == banned));
        }
        let bad = bank.conn.execute(
            "UPDATE puzzles SET solvability = 'hard' WHERE puzzle_id = ?1",
            params![puzzle_id],
        );
        assert!(bad.is_err());

        drop(bank);
        cleanup(&path);
    }

    #[test]
    fn rejects_a_puzzle_that_is_not_unique() {
        let path = temp_db("ambiguous");
        let mut bank = Bank::open(&path).unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        let first = fill_grid(&mut rng).unwrap();
        let second = fill_grid(&mut rng).unwrap();
        assert_ne!(first, second);
        let mut givens = Grid::empty();
        for index in 0..81 {
            if first.cells()[index] == second.cells()[index] {
                givens.set_index(index, first.cells()[index]);
            }
        }
        let err = bank
            .insert_puzzle(&Puzzle {
                givens,
                solution: first,
            })
            .unwrap_err();
        assert!(matches!(err, Error::NotUnique { found: 2 }));
        assert_eq!(bank.puzzle_count().unwrap(), 0);
        drop(bank);
        cleanup(&path);
    }

    #[test]
    fn rejects_a_solution_that_disagrees_with_the_givens() {
        let path = temp_db("mismatch");
        let mut bank = Bank::open(&path).unwrap();
        let solution = sample_solution();
        let mut givens = solution;
        let digit = givens.cell(0, 0);
        givens.set(0, 0, if digit == 1 { 2 } else { 1 });
        let err = bank
            .insert_puzzle(&Puzzle { givens, solution })
            .unwrap_err();
        assert!(matches!(err, Error::InvalidPuzzle(_)));
        drop(bank);
        cleanup(&path);
    }

    #[test]
    fn trace_step_roundtrip_requires_a_complete_reason() {
        let path = temp_db("trace");
        let mut bank = Bank::open(&path).unwrap();
        let InsertResult::Stored { puzzle_id, .. } =
            bank.insert_puzzle(&complete_puzzle()).unwrap()
        else {
            panic!("expected insert");
        };

        let mut candidates = vec![0u8; 81];
        candidates[0] = 1 << 4;
        let step = Step {
            index: 0,
            skill_id: "naked_single".into(),
            template_id: "naked_single.standard".into(),
            facts: serde_json::json!({"cell": "R1C1", "digit": 5}),
            pattern: vec![PatternCell {
                r: 0,
                c: 0,
                role: "placement".into(),
                notes: vec![5],
            }],
            placements: vec![Placement { r: 0, c: 0, d: 5 }],
            eliminations: vec![],
            reason: Reason {
                short: "R1C1 can only be 5.".into(),
                detailed: "Every other digit is already used in the row, column, or box.".into(),
                observation: "R1C1 has only 5 left in its notes.".into(),
                deduction: "A cell with one candidate must be that digit.".into(),
                action: "Place 5 in R1C1.".into(),
            },
            candidates_after: candidates,
        };
        bank.insert_step(&puzzle_id, &step).unwrap();
        assert_eq!(bank.steps(&puzzle_id).unwrap(), vec![step.clone()]);

        let mut empty_reason = step.clone();
        empty_reason.index = 1;
        empty_reason.reason.observation.clear();
        assert!(matches!(
            bank.insert_step(&puzzle_id, &empty_reason),
            Err(Error::InvalidStep(_))
        ));
        assert_eq!(bank.steps(&puzzle_id).unwrap().len(), 1);

        let step_names = column_names(&bank, "steps");
        assert!(step_names.contains(&"facts_json".to_string()));
        assert!(step_names.contains(&"reason_observation".to_string()));
        assert!(step_names.contains(&"candidates_after".to_string()));
        assert!(step_names.contains(&"pattern_json".to_string()));

        drop(bank);
        cleanup(&path);
    }

    fn column_names(bank: &Bank, table: &str) -> Vec<String> {
        let sql = format!("PRAGMA table_info({table})");
        let mut stmt = bank.conn.prepare(&sql).unwrap();
        stmt.query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }
}
