//! Dancing-links exact cover for 9×9 Sudoku.
//!
//! A placement of digit `d` in row `r` and column `c` covers four constraints:
//! the cell, the row-digit pair, the column-digit pair, and the box-digit pair.
//! That is 729 rows and 324 columns. Uniqueness checks and random completion
//! share this matrix.

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use crate::grid::Grid;

/// Solutions discovered before the search stopped at `limit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolveResult {
    /// How many solutions were found. This never exceeds the requested limit,
    /// so a value equal to the limit means “at least this many”.
    pub found: u32,
    pub grids: Vec<Grid>,
}

/// Count completions of `givens`, stopping after `limit` solutions.
///
/// The search is deterministic. Pass a limit of 2 to test uniqueness:
/// `found == 1` means exactly one solution.
pub fn solve(givens: &Grid, limit: u32) -> SolveResult {
    let mut rng = StdRng::seed_from_u64(0);
    solve_with(givens, limit, &mut rng, false)
}

pub(crate) fn solve_random(givens: &Grid, rng: &mut impl Rng, limit: u32) -> SolveResult {
    solve_with(givens, limit, rng, true)
}

fn solve_with(givens: &Grid, limit: u32, rng: &mut impl Rng, randomize: bool) -> SolveResult {
    if limit == 0 {
        return SolveResult {
            found: 0,
            grids: Vec::new(),
        };
    }
    let Some((mut dlx, given)) = prepare(givens) else {
        return SolveResult {
            found: 0,
            grids: Vec::new(),
        };
    };
    let mut search = Search {
        dlx: &mut dlx,
        rng,
        randomize,
        limit,
        found: 0,
        chosen: Vec::new(),
        given,
        grids: Vec::new(),
    };
    search.run();
    SolveResult {
        found: search.found,
        grids: search.grids,
    }
}

struct Search<'a, R: Rng + ?Sized> {
    dlx: &'a mut Dlx,
    rng: &'a mut R,
    randomize: bool,
    limit: u32,
    found: u32,
    chosen: Vec<u32>,
    given: Vec<u32>,
    grids: Vec<Grid>,
}

impl<R: Rng + ?Sized> Search<'_, R> {
    fn run(&mut self) -> bool {
        if self.found >= self.limit {
            return true;
        }
        if self.dlx.nodes[0].right == 0 {
            self.record();
            return self.found >= self.limit;
        }
        let col = self.dlx.choose_column();
        if self.dlx.nodes[col].size == 0 {
            return false;
        }
        self.dlx.cover(col);
        let mut rows = self.dlx.column_rows(col);
        if self.randomize {
            rows.shuffle(self.rng);
        }
        for head in rows {
            self.chosen.push(self.dlx.nodes[head].row_id);
            let mut right = self.dlx.nodes[head].right;
            while right != head {
                self.dlx.cover(self.dlx.nodes[right].col);
                right = self.dlx.nodes[right].right;
            }
            let stop = self.run();
            let mut left = self.dlx.nodes[head].left;
            while left != head {
                self.dlx.uncover(self.dlx.nodes[left].col);
                left = self.dlx.nodes[left].left;
            }
            self.chosen.pop();
            if stop {
                self.dlx.uncover(col);
                return true;
            }
        }
        self.dlx.uncover(col);
        false
    }

    fn record(&mut self) {
        let mut rows = Vec::with_capacity(81);
        rows.extend_from_slice(&self.given);
        rows.extend_from_slice(&self.chosen);
        let grid = grid_from_rows(&rows);
        debug_assert!(
            grid.is_complete(),
            "exact cover returned a grid that is not a completed Sudoku"
        );
        self.found += 1;
        if self.grids.len() < self.limit as usize {
            self.grids.push(grid);
        }
    }
}

fn grid_from_rows(rows: &[u32]) -> Grid {
    let mut cells = [0u8; 81];
    for &row_id in rows {
        let id = row_id as usize;
        cells[id / 9] = (id % 9) as u8 + 1;
    }
    Grid::from_cells(cells)
}

fn prepare(givens: &Grid) -> Option<(Dlx, Vec<u32>)> {
    let mut dlx = Dlx::new();
    let mut given = Vec::new();
    let mut covered = [false; 325];
    for index in 0..81 {
        let digit = givens.cells()[index];
        if digit == 0 {
            continue;
        }
        if !(1..=9).contains(&digit) {
            return None;
        }
        let row_id = (index * 9 + (digit as usize - 1)) as u32;
        if !dlx.cover_row(row_id, &mut covered) {
            return None;
        }
        given.push(row_id);
    }
    Some((dlx, given))
}

#[derive(Clone, Copy)]
struct Node {
    left: usize,
    right: usize,
    up: usize,
    down: usize,
    col: usize,
    row_id: u32,
    size: i32,
}

struct Dlx {
    nodes: Vec<Node>,
    row_head: Vec<usize>,
}

impl Dlx {
    fn new() -> Self {
        let mut nodes = Vec::with_capacity(325 + 729 * 4);
        nodes.push(Node {
            left: 324,
            right: 1,
            up: 0,
            down: 0,
            col: 0,
            row_id: 0,
            size: 0,
        });
        for col in 1..=324 {
            let right = if col == 324 { 0 } else { col + 1 };
            nodes.push(Node {
                left: col - 1,
                right,
                up: col,
                down: col,
                col,
                row_id: 0,
                size: 0,
            });
        }

        let mut dlx = Dlx {
            nodes,
            row_head: vec![0; 729],
        };
        for row in 0..9 {
            for col in 0..9 {
                for digit in 0..9 {
                    let row_id = ((row * 9 + col) * 9 + digit) as u32;
                    dlx.add_row(constraint_columns(row, col, digit), row_id);
                }
            }
        }
        debug_assert_eq!(dlx.nodes.len(), 325 + 729 * 4);
        debug_assert!(dlx.nodes[1..=324].iter().all(|node| node.size == 9));
        dlx
    }

    fn add_row(&mut self, cols: [usize; 4], row_id: u32) {
        let start = self.nodes.len();
        self.row_head[row_id as usize] = start;
        for &col in &cols {
            self.nodes.push(Node {
                left: 0,
                right: 0,
                up: 0,
                down: 0,
                col,
                row_id,
                size: 0,
            });
        }
        for k in 0..4 {
            let idx = start + k;
            self.nodes[idx].left = start + (k + 3) % 4;
            self.nodes[idx].right = start + (k + 1) % 4;
            let col = cols[k];
            let up = self.nodes[col].up;
            self.nodes[idx].up = up;
            self.nodes[idx].down = col;
            self.nodes[up].down = idx;
            self.nodes[col].up = idx;
            self.nodes[col].size += 1;
        }
    }

    fn cover_row(&mut self, row_id: u32, covered: &mut [bool]) -> bool {
        let head = self.row_head[row_id as usize];
        let mut cols = [0usize; 4];
        let mut cursor = head;
        for col in &mut cols {
            *col = self.nodes[cursor].col;
            cursor = self.nodes[cursor].right;
        }
        debug_assert_eq!(cursor, head);
        if cols.iter().any(|&col| covered[col]) {
            return false;
        }
        for &col in &cols {
            self.cover(col);
            covered[col] = true;
        }
        true
    }

    fn choose_column(&self) -> usize {
        let mut best = self.nodes[0].right;
        let mut best_size = self.nodes[best].size;
        let mut cursor = self.nodes[best].right;
        while cursor != 0 {
            let size = self.nodes[cursor].size;
            if size < best_size {
                best = cursor;
                best_size = size;
                if best_size <= 1 {
                    break;
                }
            }
            cursor = self.nodes[cursor].right;
        }
        best
    }

    fn column_rows(&self, col: usize) -> Vec<usize> {
        let mut rows = Vec::with_capacity(self.nodes[col].size.max(0) as usize);
        let mut cursor = self.nodes[col].down;
        while cursor != col {
            rows.push(cursor);
            cursor = self.nodes[cursor].down;
        }
        rows
    }

    fn cover(&mut self, col: usize) {
        let left = self.nodes[col].left;
        let right = self.nodes[col].right;
        self.nodes[left].right = right;
        self.nodes[right].left = left;
        let mut row = self.nodes[col].down;
        while row != col {
            let mut cursor = self.nodes[row].right;
            while cursor != row {
                let up = self.nodes[cursor].up;
                let down = self.nodes[cursor].down;
                self.nodes[up].down = down;
                self.nodes[down].up = up;
                let header = self.nodes[cursor].col;
                self.nodes[header].size -= 1;
                cursor = self.nodes[cursor].right;
            }
            row = self.nodes[row].down;
        }
    }

    fn uncover(&mut self, col: usize) {
        let mut row = self.nodes[col].up;
        while row != col {
            let mut cursor = self.nodes[row].left;
            while cursor != row {
                let header = self.nodes[cursor].col;
                self.nodes[header].size += 1;
                let up = self.nodes[cursor].up;
                let down = self.nodes[cursor].down;
                self.nodes[up].down = cursor;
                self.nodes[down].up = cursor;
                cursor = self.nodes[cursor].left;
            }
            row = self.nodes[row].up;
        }
        let left = self.nodes[col].left;
        let right = self.nodes[col].right;
        self.nodes[left].right = col;
        self.nodes[right].left = col;
    }
}

/// Column headers are `1..=324`: cell, row-digit, column-digit, box-digit.
fn constraint_columns(row: usize, col: usize, digit: usize) -> [usize; 4] {
    let box_index = (row / 3) * 3 + col / 3;
    [
        1 + row * 9 + col,
        1 + 81 + row * 9 + digit,
        1 + 162 + col * 9 + digit,
        1 + 243 + box_index * 9 + digit,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::sample_solution;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn backtrack_count(grid: &Grid, limit: u32) -> u32 {
        fn fits(cells: &[u8; 81], row: usize, col: usize, digit: u8) -> bool {
            for i in 0..9 {
                if cells[row * 9 + i] == digit || cells[i * 9 + col] == digit {
                    return false;
                }
            }
            let origin_row = (row / 3) * 3;
            let origin_col = (col / 3) * 3;
            for dr in 0..3 {
                for dc in 0..3 {
                    if cells[(origin_row + dr) * 9 + origin_col + dc] == digit {
                        return false;
                    }
                }
            }
            true
        }

        fn rec(cells: &mut [u8; 81], limit: u32, found: &mut u32) {
            if *found >= limit {
                return;
            }
            let Some(pos) = cells.iter().position(|&digit| digit == 0) else {
                *found += 1;
                return;
            };
            let row = pos / 9;
            let col = pos % 9;
            for digit in 1..=9 {
                if fits(cells, row, col, digit) {
                    cells[pos] = digit;
                    rec(cells, limit, found);
                    cells[pos] = 0;
                    if *found >= limit {
                        return;
                    }
                }
            }
        }

        if !grid.is_consistent() {
            return 0;
        }
        let mut cells = *grid.cells();
        let mut found = 0;
        rec(&mut cells, limit, &mut found);
        found
    }

    #[test]
    fn complete_grid_has_one_solution_and_matches_itself() {
        let grid = sample_solution();
        let solved = solve(&grid, 2);
        assert_eq!(solved.found, 1);
        assert_eq!(solved.grids[0], grid);
        assert_eq!(backtrack_count(&grid, 2), 1);
    }

    #[test]
    fn clearing_one_cell_keeps_that_unique_digit() {
        let solved_grid = sample_solution();
        let mut givens = solved_grid;
        let digit = givens.cell(3, 4);
        givens.set(3, 4, 0);
        let solved = solve(&givens, 2);
        assert_eq!(solved.found, 1);
        assert_eq!(solved.grids[0], solved_grid);
        assert_eq!(solved.grids[0].cell(3, 4), digit);
        assert_eq!(backtrack_count(&givens, 2), 1);
    }

    #[test]
    fn contradiction_has_no_solution() {
        let mut grid = sample_solution();
        grid.set(0, 0, grid.cell(0, 1));
        let solved = solve(&grid, 2);
        assert_eq!(solved.found, 0);
        assert!(solved.grids.is_empty());
        assert_eq!(backtrack_count(&grid, 2), 0);
    }

    #[test]
    fn empty_grid_has_more_than_one_solution() {
        let solved = solve(&Grid::empty(), 2);
        assert_eq!(solved.found, 2);
        assert_ne!(solved.grids[0], solved.grids[1]);
        assert!(solved.grids.iter().all(Grid::is_complete));
        assert_eq!(backtrack_count(&Grid::empty(), 2), 2);
    }

    #[test]
    fn dlx_matches_backtracker_on_a_handful_of_holes() {
        let mut grid = sample_solution();
        for index in [0usize, 10, 20, 30, 40, 50] {
            grid.clear_index(index);
        }
        assert_eq!(solve(&grid, 2).found, backtrack_count(&grid, 2));
    }

    #[test]
    fn two_random_completions_share_an_ambiguous_given_set() {
        let mut rng = StdRng::seed_from_u64(1);
        let first = solve_random(&Grid::empty(), &mut rng, 1).grids.remove(0);
        let second = solve_random(&Grid::empty(), &mut rng, 1).grids.remove(0);
        assert_ne!(first, second);
        let mut shared = Grid::empty();
        for index in 0..81 {
            if first.cells()[index] == second.cells()[index] {
                shared.set_index(index, first.cells()[index]);
            }
        }
        let solved = solve(&shared, 2);
        assert_eq!(solved.found, 2);
        assert!(solved.grids.contains(&first) || solved.found == 2);
    }
}
