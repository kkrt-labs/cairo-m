# 01 — Variables

Goals:
- Declare variables with `let` and initialize them.
- Use shadowing by re‑declaring with `let`.
- Mutate variables (no `mut` keyword in Cairo‑M).
- Use `const` with arrays (e.g., `[u32; N]`).

Tips:
- Variables must be initialized when declared.
- Shadowing: `let x = x + 1;` creates a new `x`.
- Prefer `u32` for integer math; `felt` is field arithmetic.

Open each `.cm` file, follow the TODOs, and save.

