# Cairo‑M Rustlings (cairomlings) — Product Requirements

## Overview

A standalone Rust CLI, inspired by Rustlings, that guides learners through small
Cairo‑M exercises. It watches files for changes, recompiles/retests the current
exercise automatically, and provides interactive feedback, hints, progress
tracking, and an exercise list. The tool reuses the existing Cairo‑M
compiler/runner infrastructure.

- Working title: `cairomlings` (binary/crate name)
- Runs outside the main Cairo‑M workspace (same repo different workspace crate).
  For local development, allow path dependencies on the existing Cairo‑M crates.
- Exercises are small `.cm` files organized by topic under an `exercises/`
  directory. A manifest (`info.toml`) defines ordering, metadata, and how to
  verify correctness.

## Goals

- Fast feedback learning loop for Cairo‑M.
- Watch mode that reruns the current exercise when its file changes.
- Deterministic, clear verification using the Cairo‑M compiler/runner.
- Interactive experience: hints, reset, list view, progress tracking.
- Minimal setup: `cargo install`, `cairomlings init`, `cairomlings`.

## Non‑Goals

- IDE plugin development (beyond basic terminal file links).
- Online services, telemetry, or leaderboards.
- A package manager for Cairo‑M; this is a tutorial runner.

## Audience & Use Cases

- New Cairo‑M learners: fix exercises and immediately see results.
- Educators/contributors: author and test exercise packs (future community
  flows).

## Background & References

- Rustlings provides a proven model: watch mode, state file, list UI, hints,
  reset.
- Cairo‑M has a test template showing how to compile and run a `.cm` program
  from Rust: `crates/cargo-cairo-m/templates/integration_test.rs:1`.
  - Compilation: `cairo_m_compiler::compile_cairo` with
    `CompilerOptions::default()`
  - Execution: `cairo_m_runner::run_cairo_program` with
    `RunnerOptions::default()`

## Key Features

- Watch mode: auto rerun on save; manual-run fallback (`--manual-run` + press
  `r`).
- Deterministic checks: each exercise specifies entrypoint and test cases.
- Interactive controls (single-key): next, run, hint, list, check-all, reset,
  quit.
- Exercise list UI: filter pending/done, search, set current, reset.
- Progress tracking via a dotfile in the exercises root.

## Exercise Content Model

- Directory layout:
  - `exercises/<topic>/<name>.cm` — the file learners edit.
  - `exercises/<topic>/README.md` — optional topic intro.
  - `solutions/<topic>/<name>.cm` — reference solution (shown after completion).
  - `info.toml` — manifest of exercises and metadata.
- Each exercise metadata includes:
  - `name` (unique, matches filename stem)
  - `dir` (optional topic subdirectory)
  - `test` (bool, default true)
  - `hint` (string)
  - `entrypoint` (Cairo‑M function name to call)
  - `cases` (list of deterministic input/expected pairs)

## Verification Strategy

- For each exercise:
  - Read `.cm` source from `exercises/.../<name>.cm`.
  - Compile with `cairo_m_compiler::compile_cairo`.
  - Run with `cairo_m_runner::run_cairo_program(entrypoint, args)` for each
    case.
  - Validate the first return value against `expected` (start with `felt` type
    support, extend later).
- Output:
  - Show compile/runner output; on success display “Exercise done ✓”.
  - On failure, keep exercise pending, display errors, allow showing `hint`.

## CLI & Commands

- Binary: `cairomlings`
- Default: watch mode for the current/next pending exercise.
- Subcommands:
  - `watch` — watch mode for the current/next pending exercise.
  - `run [<exercise>]` — run once and print results for current or named
    exercise.
  - `check-all` — verify all exercises; update done/pending; print first pending
    if any.
  - `reset <exercise>` — reset the file to its original version.
  - `hint [<exercise>]` — print hint for current or named exercise.
  - `list` — open interactive exercise list UI (also accessible from watch
    mode).
  - Global flag: `--manual-run` — disable file watching; press `r` to run.

## Watch Mode UX

- Watches `exercises/` recursively for `.cm` changes (using `notify`) with
  debounce.
- Key bindings:
  - `n`: next (if done)
  - `r`: run (only with `--manual-run`)
  - `h`: show hint
  - `l`: open list
  - `c`: check all
  - `x`: reset current (with confirmation)
  - `q`: quit
- Footer shows: progress bar, current exercise link, and key hints.

## Files & Layout

- In the initialized directory:
  - `exercises/` — editable exercise files.
  - `solutions/` — reference solutions (pre-created stubs filled separately).
  - `info.toml` — manifest (see schema below).
  - `.cairomlings-state.txt` — current exercise and done list.
  - `.gitignore`, `rust-analyzer.toml` — convenience files.

## Initialization Flow

- Unlike rustlings, will be ran directly from the cairo-m monorepo. As such,
  it's expected to run it with like `cargo run -p cairomlings -- <args>`.
- The `cairomlings/` directory contains with `exercises/`, `solutions/`,
  `info.toml`, `.gitignore`, and the source code of the crate.
  - Create starter exercises (6–10) covering: values, arithmetic, conditionals,
    loops, functions, as per info in @getting-started.md and @mdtests.

## Dependencies & Compatibility

- Rust crates: `notify`, `crossterm`, `serde`, `toml`, `anyhow`.
- Cairo‑M crates: `cairo_m_common`, `cairo_m_compiler`, `cairo_m_runner` (path
  deps during local dev; git/crates.io later).
- Platforms: Linux, macOS, Windows. Provide `--manual-run` fallback if watch
  fails.

## Performance & Reliability

- Target: change-to-feedback typically < 1s; compile+run per small exercise <
  500ms where feasible.
- Debounce FS events (~200ms) to avoid thrashing.
- Clear error messages and consistent terminal output.

## Security & Privacy

- No telemetry. Operates on local files only.
- `reset` operates on the current exercise path; confirmation prompt prevents
  accidental loss.

## Phased Delivery

- MVP (Phase 1):
  - Watch mode, manual-run fallback, state tracking, `run`, `check-all`, `hint`,
    `reset`, `list`.
  - `info.toml` with `entrypoint` and deterministic `cases`.
  - Starter exercises and hints.
  - The binary just lets us run the exercises and see the results.
- Phase 2:
  - Community exercise packs and discovery.
  - Property-based/dynamic tests with internal reference functions.
  - Richer TUI (search highlight, improved scrolling) and solution comparison
    tooling.

## Risks & Mitigations

- File watch reliability on some environments → `--manual-run` path and clear
  guidance.
- Externalization vs. local path deps → support both path and git deps; document
  setup.
- Compile/run performance for larger examples → keep exercises small; cache
  compiled output (future optimization).

## Success Metrics

- Time-to-first-exercise < 5 minutes.
- Edit-to-feedback latency consistently fast and reliable.
- Learners complete starter pack without external help; minimal friction in
  reports.

## Appendix A: Proposed `info.toml` Schema (v1)

```toml
# info.toml
format_version = 1
welcome_message = "Welcome to Cairo‑M Rustlings!"
final_message = "You finished the exercises. Great job!"

[[exercises]]
name = "fibonacci"
dir = "basics"
test = true
hint = "Implement iterative Fibonacci; pay attention to felt arithmetic."
entrypoint = "fibonacci"
# Deterministic checks; supports felt numbers initially
cases = [
  { args = [0], expected = 0 },
  { args = [1], expected = 1 },
  { args = [10], expected = 55 }
]

# Additional exercises follow…
```

Notes:

- `name` must match the file stem (`exercises/<dir>/<name>.cm`).
- `entrypoint` is the Cairo‑M function invoked.
- `cases` enumerate input arrays and expected first return value (felt).
  Extendable later.

## Appendix B: Example Exercise File

- Path: `exercises/basics/fibonacci.cm`
- Learner edits to pass the cases; hint references the required approach.

Example starter (iterative Fibonacci):

```cairo
// Iterative Fibonacci implementation
fn fibonacci(n: felt) -> felt {
    let current = 0;
    let next = 1;

    let counter = 0;
    while counter != n {
        let new_next = current + next;
        current = next;
        next = new_next;
        counter = counter + 1;
    }

    return current;
}
```

## Appendix C: Verification Integration (informative)

- Compile with `cairo_m_compiler::compile_cairo` and run with
  `cairo_m_runner::run_cairo_program` (see
  `crates/cargo-cairo-m/templates/integration_test.rs:1`).
- The runner extracts the return values, using the same decoding logic as the
  runner (see existing runner tests for mdtest diff) and compares to `expected`
  per case.

## Appendix D: Command Cheat Sheet

- `cairomlings` — start watch mode
- `cairomlings run [<exercise>]` — run a specific exercise
- `cairomlings check-all` — verify all exercises
- `cairomlings hint [<exercise>]` — show hint
- `cairomlings reset <exercise>` — reset file
- `cairomlings --manual-run` — disable watch; use `r` to run
