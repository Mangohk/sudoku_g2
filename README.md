# Sudoku Generator 2

Desktop generator for uniquely solvable 9×9 Sudoku puzzles. This repository is
phase 1: a Rust engine, exact-cover uniqueness, clue digging, and a SQLite bank
that keeps one row per puzzle shape.

The generator does not classify puzzles by skill, difficulty, or score. It also
does not store rotations, reflections, band or stack swaps, or digit relabelings
as extra puzzles. Those transforms are used only to recognize the same puzzle.

The 32 game skills, the 15 appendix methods, and the local web UI are not in
this phase. The database already has a place for a full solving trace.

## Build and test

```bash
cargo test
cargo run -- --count 3 --seed 42
cargo run -- --count 3 --seed 42
```

The first run creates `puzzles.sqlite` and stores 3 puzzles. The second run
generates the same puzzles and skips them because their canonical forms are
already in the bank.

`cargo test` covers DLX uniqueness, automorphism dedup, and a small
generate-and-store run.

## Command line

```bash
cargo run -- --count 5 --seed 42 --database puzzles.sqlite --clues 36
cargo run -- --count 1 --seed 7 --minimal
```

| Flag | Meaning |
| --- | --- |
| `--count` | How many puzzles to attempt. Default 5. |
| `--database` / `-d` | SQLite path. Default `puzzles.sqlite`. |
| `--seed` | Deterministic seed. Omit it for a time-based seed. |
| `--clues` | Stop digging once this many clues remain. Default 36. |
| `--minimal` | Keep removing clues while the solution stays unique. Slower than `--clues`. |

A puzzle is stored only when the answer grid is complete and DLX finds that
answer and no other. `--clues` is a ceiling the digger tries to reach. It will
not remove a clue that would create a second solution, so a removal order can
finish a little above the target.

## What is stored

Each accepted row keeps the original givens, the unique solution, the clue
count, the time of intake, and the min-lex canonical key. `puzzle_id` is the
SHA-256 of that key. `skills_used`, `solvability`, and `trace_catalog_rev` are
present and left null until a solver writes them. There is no score column.

`steps` is ready for the later trace: skill id, template id, facts, pattern
roles, placements, eliminations, the short and detailed reason, observation,
deduction, action, and an 81-byte candidate snapshot. A step with an empty
reason part is rejected.

The canonical key is the lexicographically least 81-character image of the
givens. Empty cells are `.`. The search applies transpose, every
band-preserving row permutation, every stack-preserving column permutation,
and the digit relabeling that makes each image smallest. Rotations,
reflections, band and stack swaps, and in-band row or column swaps are inside
that group.

## Layout

```
src/grid.rs           9×9 grid
src/exact_cover.rs    dancing-links exact cover (uniqueness and completion)
src/generate.rs       randomized fill and clue digging
src/symmetry.rs       automorphism group and min-lex key
src/bank.rs           SQLite schema and intake
src/trace.rs          step, reason, highlight, and snapshot types
src/catalog.rs        32 game skill ids and 15 appendix ids
src/solver.rs         SolveWithTrace, the trait the skill solver will implement
src/main.rs           command-line generate-and-store
```

Phase 2 can implement `SolveWithTrace` against `catalog` and write `trace::Step`
values through `Bank::insert_step` without moving the grid, DLX, or intake
types. Uniqueness stays in `exact_cover`.
