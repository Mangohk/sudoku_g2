//! Sudoku automorphisms and the min-lex canonical key.
//!
//! The group used for dedup is the automorphism group of the empty board:
//! transpose, the 1,296 band-preserving row permutations, and the 1,296
//! stack-preserving column permutations. That group contains rotations,
//! reflections, band and stack swaps, and row or column swaps inside a band
//! or stack. Digit relabeling is applied after each geometric image.
//!
//! The canonical key is the lexicographically least 81-character string in
//! that orbit. Empty cells are `.` and sort before digits, which is ordinary
//! byte order. The generator must not store extra images under this group;
//! the transforms are here for dedup and for later gameplay.

use std::sync::OnceLock;

use crate::grid::Grid;
use crate::Error;

pub fn transpose(grid: &Grid) -> Grid {
    map_cells(grid, |row, col| (col, row))
}

/// Quarter turn clockwise.
pub fn rotate90(grid: &Grid) -> Grid {
    map_cells(grid, |row, col| (col, 8 - row))
}

pub fn rotate180(grid: &Grid) -> Grid {
    rotate90(&rotate90(grid))
}

pub fn rotate270(grid: &Grid) -> Grid {
    rotate90(&rotate180(grid))
}

/// Top-to-bottom reflection. Rows are reversed.
pub fn mirror_horizontal(grid: &Grid) -> Grid {
    map_cells(grid, |row, col| (8 - row, col))
}

/// Left-to-right reflection. Columns are reversed.
pub fn mirror_vertical(grid: &Grid) -> Grid {
    map_cells(grid, |row, col| (row, 8 - col))
}

pub fn swap_bands(grid: &Grid, a: usize, b: usize) -> Result<Grid, Error> {
    if a > 2 || b > 2 {
        return Err(Error::InvalidTransform("band index must be 0, 1, or 2"));
    }
    if a == b {
        return Ok(*grid);
    }
    let mut out = *grid;
    for local in 0..3 {
        let row_a = a * 3 + local;
        let row_b = b * 3 + local;
        for col in 0..9 {
            out.set(row_a, col, grid.cell(row_b, col));
            out.set(row_b, col, grid.cell(row_a, col));
        }
    }
    Ok(out)
}

pub fn swap_stacks(grid: &Grid, a: usize, b: usize) -> Result<Grid, Error> {
    if a > 2 || b > 2 {
        return Err(Error::InvalidTransform("stack index must be 0, 1, or 2"));
    }
    if a == b {
        return Ok(*grid);
    }
    let mut out = *grid;
    for local in 0..3 {
        let col_a = a * 3 + local;
        let col_b = b * 3 + local;
        for row in 0..9 {
            out.set(row, col_a, grid.cell(row, col_b));
            out.set(row, col_b, grid.cell(row, col_a));
        }
    }
    Ok(out)
}

pub fn swap_rows_in_band(grid: &Grid, row_a: usize, row_b: usize) -> Result<Grid, Error> {
    if row_a > 8 || row_b > 8 {
        return Err(Error::InvalidTransform("row index must be 0..=8"));
    }
    if row_a / 3 != row_b / 3 {
        return Err(Error::InvalidTransform(
            "rows must sit in the same band; a cross-band swap is not a Sudoku automorphism",
        ));
    }
    if row_a == row_b {
        return Ok(*grid);
    }
    let mut out = *grid;
    for col in 0..9 {
        out.set(row_a, col, grid.cell(row_b, col));
        out.set(row_b, col, grid.cell(row_a, col));
    }
    Ok(out)
}

pub fn swap_cols_in_stack(grid: &Grid, col_a: usize, col_b: usize) -> Result<Grid, Error> {
    if col_a > 8 || col_b > 8 {
        return Err(Error::InvalidTransform("column index must be 0..=8"));
    }
    if col_a / 3 != col_b / 3 {
        return Err(Error::InvalidTransform(
            "columns must sit in the same stack; a cross-stack swap is not a Sudoku automorphism",
        ));
    }
    if col_a == col_b {
        return Ok(*grid);
    }
    let mut out = *grid;
    for row in 0..9 {
        out.set(row, col_a, grid.cell(row, col_b));
        out.set(row, col_b, grid.cell(row, col_a));
    }
    Ok(out)
}

/// `digit_map[old]` is the new digit. Index 0 must stay 0. Indexes 1..=9 are a permutation.
pub fn relabel(grid: &Grid, digit_map: &[u8; 10]) -> Result<Grid, Error> {
    if digit_map[0] != 0 {
        return Err(Error::InvalidTransform(
            "digit map must keep empty cells empty",
        ));
    }
    let mut seen = [false; 10];
    for old in 1..=9 {
        let new_digit = digit_map[old] as usize;
        if !(1..=9).contains(&new_digit) || seen[new_digit] {
            return Err(Error::InvalidTransform(
                "digit map must be a permutation of 1..=9",
            ));
        }
        seen[new_digit] = true;
    }
    let mut cells = [0u8; 81];
    for (index, &digit) in grid.cells().iter().enumerate() {
        cells[index] = digit_map[digit as usize];
    }
    Ok(Grid::from_cells(cells))
}

/// Min-lex key of the given grid under the Sudoku automorphism group.
pub fn canonical_key(grid: &Grid) -> String {
    let perms = source_perms();
    let original = *grid.cells();
    let transposed = transposed_cells(&original);
    let mut best: Option<[u8; 81]> = None;
    for cells in [original, transposed] {
        let rows = rows_of(&cells);
        for row_src in perms.iter() {
            let mut arranged = [[0u8; 9]; 9];
            for row in 0..9 {
                arranged[row] = rows[row_src[row] as usize];
            }
            for col_src in perms.iter() {
                consider(&arranged, col_src, &mut best);
            }
        }
    }
    let bytes = best.expect("the identity transform yields a key");
    String::from_utf8(bytes.to_vec()).expect("canonical key is ASCII")
}

fn map_cells(grid: &Grid, dest: impl Fn(usize, usize) -> (usize, usize)) -> Grid {
    let mut out = Grid::empty();
    for row in 0..9 {
        for col in 0..9 {
            let (new_row, new_col) = dest(row, col);
            out.set(new_row, new_col, grid.cell(row, col));
        }
    }
    out
}

fn transposed_cells(cells: &[u8; 81]) -> [u8; 81] {
    let mut out = [0u8; 81];
    for row in 0..9 {
        for col in 0..9 {
            out[col * 9 + row] = cells[row * 9 + col];
        }
    }
    out
}

fn rows_of(cells: &[u8; 81]) -> [[u8; 9]; 9] {
    let mut rows = [[0u8; 9]; 9];
    for row in 0..9 {
        rows[row].copy_from_slice(&cells[row * 9..row * 9 + 9]);
    }
    rows
}

/// `src[new_index] = old_index` for band-preserving permutations. There are 1,296.
fn source_perms() -> &'static Vec<[u8; 9]> {
    static CACHE: OnceLock<Vec<[u8; 9]>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let within = [
            [0u8, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        let mut perms = Vec::with_capacity(1296);
        for bands in within {
            for w0 in within {
                for w1 in within {
                    for w2 in within {
                        let local = [w0, w1, w2];
                        let mut src = [0u8; 9];
                        for new_band in 0..3 {
                            let old_band = bands[new_band] as usize;
                            for new_local in 0..3 {
                                let old_local = local[old_band][new_local];
                                src[new_band * 3 + new_local] = (old_band * 3) as u8 + old_local;
                            }
                        }
                        perms.push(src);
                    }
                }
            }
        }
        perms
    })
}

fn consider(rows: &[[u8; 9]; 9], col_src: &[u8; 9], best: &mut Option<[u8; 81]>) {
    let mut map = [0u8; 10];
    let mut next = 1u8;
    let mut out = [0u8; 81];
    let mut better = false;
    for row in 0..9 {
        for col in 0..9 {
            let index = row * 9 + col;
            let symbol = label(&mut map, &mut next, rows[row][col_src[col] as usize]);
            out[index] = symbol;
            if !better {
                if let Some(champion) = best.as_ref() {
                    if symbol > champion[index] {
                        return;
                    }
                    if symbol < champion[index] {
                        better = true;
                    }
                }
            }
        }
    }
    *best = Some(out);
}

fn label(map: &mut [u8; 10], next: &mut u8, digit: u8) -> u8 {
    if digit == 0 {
        return b'.';
    }
    if map[digit as usize] == 0 {
        map[digit as usize] = *next;
        *next += 1;
    }
    b'0' + map[digit as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::sample_solution;

    fn swap_one_and_two() -> [u8; 10] {
        let mut map = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        map[1] = 2;
        map[2] = 1;
        map
    }

    #[test]
    fn band_preserving_perms_are_the_full_set() {
        let perms = source_perms();
        assert_eq!(perms.len(), 1296);
        let unique: std::collections::HashSet<[u8; 9]> = perms.iter().copied().collect();
        assert_eq!(unique.len(), 1296);
        assert!(perms.contains(&[0, 1, 2, 3, 4, 5, 6, 7, 8]));
        for perm in perms {
            let mut seen = [false; 9];
            for &old in perm {
                assert!(old < 9 && !seen[old as usize]);
                seen[old as usize] = true;
            }
            for band in 0..3 {
                let source_band = perm[band * 3] / 3;
                assert_eq!(perm[band * 3 + 1] / 3, source_band);
                assert_eq!(perm[band * 3 + 2] / 3, source_band);
            }
        }
    }

    #[test]
    fn greedy_relabel_assigns_digits_in_first_seen_order() {
        let mut cells = [0u8; 81];
        cells[0] = 8;
        cells[1] = 8;
        cells[2] = 3;
        let rows = rows_of(&cells);
        let ident = [0u8, 1, 2, 3, 4, 5, 6, 7, 8];
        let mut best = None;
        consider(&rows, &ident, &mut best);
        let key = best.unwrap();
        assert_eq!(&key[..3], b"112");
    }

    #[test]
    fn rotations_are_consistent_and_preserve_a_completed_grid() {
        let grid = sample_solution();
        assert_eq!(rotate90(&rotate90(&rotate90(&rotate90(&grid)))), grid);
        assert_eq!(rotate180(&grid), rotate90(&rotate90(&grid)));
        assert_eq!(transpose(&transpose(&grid)), grid);
        for image in [
            rotate90(&grid),
            rotate180(&grid),
            rotate270(&grid),
            transpose(&grid),
            mirror_horizontal(&grid),
            mirror_vertical(&grid),
            swap_bands(&grid, 0, 2).unwrap(),
            swap_stacks(&grid, 0, 1).unwrap(),
            swap_rows_in_band(&grid, 3, 5).unwrap(),
            swap_cols_in_stack(&grid, 6, 8).unwrap(),
            relabel(&grid, &swap_one_and_two()).unwrap(),
        ] {
            assert!(image.is_complete(), "{image}");
        }
    }

    #[test]
    fn cross_band_row_swap_is_rejected() {
        let grid = sample_solution();
        assert!(matches!(
            swap_rows_in_band(&grid, 0, 3),
            Err(Error::InvalidTransform(_))
        ));
        assert!(matches!(
            swap_cols_in_stack(&grid, 0, 3),
            Err(Error::InvalidTransform(_))
        ));
    }

    #[test]
    fn automorphisms_share_one_canonical_key() {
        let grid = sample_solution();
        let key = canonical_key(&grid);
        let map = swap_one_and_two();
        let images = [
            transpose(&grid),
            rotate90(&grid),
            rotate180(&grid),
            rotate270(&grid),
            mirror_horizontal(&grid),
            mirror_vertical(&grid),
            swap_bands(&grid, 0, 1).unwrap(),
            swap_stacks(&grid, 1, 2).unwrap(),
            swap_rows_in_band(&grid, 0, 2).unwrap(),
            swap_cols_in_stack(&grid, 3, 4).unwrap(),
            relabel(&grid, &map).unwrap(),
            relabel(&rotate180(&swap_bands(&grid, 0, 2).unwrap()), &map).unwrap(),
        ];
        for image in &images {
            assert_ne!(image.to_line(), grid.to_line());
            assert_eq!(canonical_key(image), key);
        }
        let parsed = Grid::parse(&key).unwrap();
        assert_eq!(canonical_key(&parsed), key);
    }

    #[test]
    fn one_clue_anywhere_is_the_same_puzzle() {
        let mut corner = Grid::empty();
        corner.set(0, 0, 1);
        let mut other = Grid::empty();
        other.set(8, 7, 6);
        let key = canonical_key(&corner);
        assert_eq!(key, canonical_key(&other));
        assert_eq!(key, format!("{}1", ".".repeat(80)));
    }

    #[test]
    fn same_digits_and_different_digits_are_not_isomorphic() {
        let mut same = Grid::empty();
        same.set(0, 0, 4);
        same.set(0, 1, 4);
        let mut different = Grid::empty();
        different.set(0, 0, 4);
        different.set(0, 1, 5);
        assert_ne!(canonical_key(&same), canonical_key(&different));
    }

    #[test]
    fn holes_in_one_box_are_not_holes_in_two_boxes() {
        let solved = sample_solution();
        let mut same_box = solved;
        same_box.clear_index(0);
        same_box.clear_index(1);
        let mut two_boxes = solved;
        two_boxes.clear_index(0);
        two_boxes.clear_index(4);
        assert_ne!(canonical_key(&same_box), canonical_key(&two_boxes));
        assert_eq!(
            canonical_key(&same_box),
            canonical_key(&transpose(&same_box))
        );
    }
}
