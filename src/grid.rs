//! 9×9 Sudoku grid. Empty cells are `0`. Digits are `1` through `9`.

use crate::Error;

/// One Sudoku board, row-major.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Grid([u8; 81]);

impl Grid {
    pub fn empty() -> Self {
        Grid([0; 81])
    }

    pub fn from_cells(cells: [u8; 81]) -> Self {
        debug_assert!(cells.iter().all(|&digit| digit <= 9));
        Grid(cells)
    }

    /// Accepts 81 digits. `.` and `0` are empty. Whitespace is ignored.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut cells = [0u8; 81];
        let mut n = 0;
        for ch in text.chars() {
            if ch.is_whitespace() {
                continue;
            }
            if n >= 81 {
                return Err(Error::InvalidGrid(
                    "expected 81 cells after removing whitespace".into(),
                ));
            }
            cells[n] = match ch {
                '.' | '0' => 0,
                '1'..='9' => ch as u8 - b'0',
                _ => {
                    return Err(Error::InvalidGrid(format!(
                        "cell {n} is '{ch}', expected '.', '0', or '1'-'9'"
                    )))
                }
            };
            n += 1;
        }
        if n != 81 {
            return Err(Error::InvalidGrid(format!("expected 81 cells, found {n}")));
        }
        Ok(Grid(cells))
    }

    pub fn cells(&self) -> &[u8; 81] {
        &self.0
    }

    pub fn cell(&self, row: usize, col: usize) -> u8 {
        self.0[row * 9 + col]
    }

    pub fn set(&mut self, row: usize, col: usize, digit: u8) {
        self.set_index(row * 9 + col, digit);
    }

    pub fn set_index(&mut self, index: usize, digit: u8) {
        debug_assert!(index < 81 && digit <= 9);
        self.0[index] = digit;
    }

    pub fn clear_index(&mut self, index: usize) {
        self.set_index(index, 0);
    }

    pub fn clue_count(&self) -> u8 {
        self.0.iter().filter(|&&digit| digit != 0).count() as u8
    }

    /// 81-character line. Empty cells are `.`.
    pub fn to_line(&self) -> String {
        self.0
            .iter()
            .map(|&digit| {
                if digit == 0 {
                    '.'
                } else {
                    char::from(b'0' + digit)
                }
            })
            .collect()
    }

    /// True when filled cells never repeat inside a row, column, or box.
    pub fn is_consistent(&self) -> bool {
        let houses_ok = |digits: [u8; 9]| -> bool {
            let mut seen = [false; 10];
            for digit in digits {
                if digit == 0 {
                    continue;
                }
                if digit > 9 || seen[digit as usize] {
                    return false;
                }
                seen[digit as usize] = true;
            }
            true
        };

        for row in 0..9 {
            let mut digits = [0u8; 9];
            digits.copy_from_slice(&self.0[row * 9..row * 9 + 9]);
            if !houses_ok(digits) {
                return false;
            }
        }
        for col in 0..9 {
            let mut digits = [0u8; 9];
            for row in 0..9 {
                digits[row] = self.0[row * 9 + col];
            }
            if !houses_ok(digits) {
                return false;
            }
        }
        for box_index in 0..9 {
            let mut digits = [0u8; 9];
            let origin_row = (box_index / 3) * 3;
            let origin_col = (box_index % 3) * 3;
            let mut n = 0;
            for dr in 0..3 {
                for dc in 0..3 {
                    digits[n] = self.0[(origin_row + dr) * 9 + origin_col + dc];
                    n += 1;
                }
            }
            if !houses_ok(digits) {
                return false;
            }
        }
        true
    }

    /// True when every cell holds `1`–`9` and [`Self::is_consistent`] holds.
    pub fn is_complete(&self) -> bool {
        self.0.iter().all(|&digit| (1..=9).contains(&digit)) && self.is_consistent()
    }
}

impl std::fmt::Debug for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Grid({})", self.to_line())
    }
}

impl std::fmt::Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let line = self.to_line();
        for row in 0..9 {
            writeln!(f, "{}", &line[row * 9..row * 9 + 9])?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn sample_solution() -> Grid {
    let grid = Grid::parse(
        "\
123456789\
456789123\
789123456\
234567891\
567891234\
891234567\
345678912\
678912345\
912345678",
    )
    .expect("sample solution parses");
    assert!(
        grid.is_complete(),
        "sample solution is not a completed Sudoku"
    );
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ignores_whitespace_and_treats_dot_as_empty() {
        let grid = Grid::parse("1.3\n456789").unwrap_err();
        assert!(matches!(grid, Error::InvalidGrid(_)));

        let mut text = sample_solution().to_line();
        text.replace_range(0..1, ".");
        let parsed = Grid::parse(&format!("\n{text}\n")).unwrap();
        assert_eq!(parsed.cell(0, 0), 0);
        assert_eq!(parsed.clue_count(), 80);
        assert!(parsed.is_consistent());
        assert!(!parsed.is_complete());
    }

    #[test]
    fn duplicate_in_a_box_is_inconsistent() {
        let mut grid = sample_solution();
        grid.set(0, 0, grid.cell(1, 1));
        assert!(!grid.is_consistent());
    }
}
