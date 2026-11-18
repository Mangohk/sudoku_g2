//! Skill-solver extension point.
//!
//! Phase 1 does not apply human techniques. The next phase implements this
//! trait for the 32 game skills, then the appendix. Uniqueness checks and the
//! completed answer stay in [`crate::exact_cover`]; this module must not grow
//! a second copy of dancing links.

use crate::grid::Grid;
use crate::trace::Trace;

/// Produce a replayable trace for one puzzle.
///
/// The implementor walks [`crate::catalog::GAME`] and then
/// [`crate::catalog::APPENDIX`]. Each step carries facts, a filled reason,
/// highlight roles, and the candidate snapshot after the step. Phase 1 has no
/// implementor.
pub trait SolveWithTrace {
    fn solve_with_trace(&self, puzzle: &Grid) -> Trace;
}
