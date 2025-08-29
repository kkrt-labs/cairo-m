# Cairo-M Codegen Work Plan (Lifeline)

This document is a prioritized, detailed TODO plan to improve the Cairo-M codegen crate, based on the senior review. It is intended as a living checklist the team can execute and update as we land changes.

Status tags: P0 (critical correctness/architecture), P1 (semantics/tests), P2 (ergonomics/normalization), P3 (polish/hygiene).


## Goals

- Make emission intrinsically alias-safe; eliminate late fixup passes.
- Unify normalization and comparison semantics; cover boundary cases via tests.
- Stabilize frame size by pre-allocating scratch, and remove hidden frame growth.
- Replace panics with typed errors; assert contracts (e.g., addressing units) explicitly.
- Tighten instruction metadata for future static checks.


## Guiding Principles

- Single source of truth for aliasing rules in emission helpers.
- Precompute worst-case scratch once per function and reuse.
- Normalize shapes early (commutativity, immediates) to reduce emitter surface area.
- Property tests and boundary cases for all arithmetic/bitwise/comparison paths.
- No new global post-passes that rewrite instruction streams for correctness.

## Report

Below is a blunt, senior‑level review of your Cairo‑M codegen. I’ll keep it frank and practical. Overall, there’s a lot to like: the architecture is thoughtfully decomposed, tests exist and catch real logic, and the opcode mapping is clean. But there are several correctness and maintainability hazards—especially around the duplicate‑offset “fixup” pass, comparison normalization, and a few asymmetries and inconsistencies that will bite later.

---

## Executive summary

**What’s solid**

* **Clear layering**: `generator` orchestrates; `builder` is a façade with small focused submodules (`emit`, `felt`, `u32_ops`, `ctrlflow`, …); `layout` encapsulates frame mapping; `opcodes` is a single source of truth; `backend` validates preconditions. This is the right shape for a compiler backend.
* **Determinism & test hygiene**: plenty of unit tests close to logic; snapshot scaffolding exists; Salsa entrypoint is in place; label resolution to physical memory (QM31) is explicit.
* **Data‑driven choices**: normalization + opcode selection split is good; inlining M31 arithmetic (negate inverse) comments are helpful and reduce cognitive load.

**Where it’s risky**

* **Post‑hoc duplicate‑offset pass is fragile** and leaks complexity across the whole pipeline. It allocates late, does not “touch” frame usage consistently, may drift from assumptions used by prior optimizations (argument‑in‑place), and encodes inconsistent aliasing rules across felt/u32 shapes.
* **Comparison normalization & bias** are mostly correct but incomplete at the edges. Some boundary cases are handled; others aren’t verified by tests (and felt comparisons remain TODO).
* **Asymmetries**: immediate‑left vs immediate‑right for u32 bitwise ops isn’t normalized; felt fp‑imm vs u32 fp‑imm alias policy is inconsistent.
* **APIs panic in library code paths** (e.g., unresolved labels in `InstructionBuilder::build`), which makes integration brittle under partial failure.

**What to do now**

1. **Move alias/duplicate handling into the emission path** (or a dedicated pre‑emit pass with a clear pass manager), pre‑allocate scratch once, and make “safe emission” a local property.
2. **Unify aliasing rules** across felt and u32 (don’t special‑case in‑place fp‑imm), and add property tests that cover all conflict patterns.
3. **Finish and test comparison semantics** (felt `lt/le/gt/ge` or ban them intentionally at MIR), expand u32 normalization edge cases and test them with boundaries and wrap cases.
4. **Normalize immediates** (commute where legal) so bitwise and arithmetic are symmetric; remove unsupported left‑immediate corners.
5. **Replace panics with errors** in build & label resolve paths; assert early with diagnostics from the backend.

The rest of this memo details these points with concrete file‑level suggestions and test plans.

---

## High‑priority issues (correctness & architecture)

### 1) Duplicate‑offset “fixup” as a post‑pass is too brittle

**Files**: `builder.rs` (method `resolve_duplicate_offsets`), helpers in same file.

* You detect aliasing **after** building the instruction stream and then splice in copies with ad‑hoc temporaries (`felt_temp1/2`, `u32_temp1/2`). This introduces late `reserve_stack` calls that **move the top of frame** after earlier optimization decisions (e.g., argument‑in‑place and call `frame_off` computations). It works “by accident” because offsets are fp‑relative, but it couples phases and increases the risk of subtle off‑by‑frame bugs.

* The pass creates instructions directly (`InstructionBuilder::new(...)`) and **never calls** `emit_touch`, so `max_written_offset` is not updated. You track “live” usage elsewhere (e.g., for the in‑place arg optimization). That’s an inconsistency and can lead to “I thought the top of frame was here” bugs.

* **Inconsistent rules**:

  * For **felt fp‑imm**, in‑place (src==dst) is illegal and expanded (good).
  * For **u32 fp‑imm**, a comment says *“in‑place operations … are fine!”* and you only copy on partial overlaps. This contradicts the stated prover limitation (“no two memory accesses to the same location in one instruction”)—a u32 in‑place op still reads and writes the same two words. If the prover truly forbids this, the current code will be incorrect.

**Fix (recommended pattern)**

* **Emit-time guard**: push alias handling down into the *emission helpers* (`felt_*`, `u32_op`, and/or `emit`) so each push of a multi‑mem op asserts non‑aliasing and expands locally if needed. That way:

  * Invariants hold per instruction; no second pass.
  * `emit_touch` is called consistently.
  * Temp allocation is centralized (see next point).
* **Scratch management**: compute worst‑case scratch once per function (e.g., 2 felts + 4 words for u32) and **reserve it in the prologue** (store the base offsets on the builder). Don’t grow the frame during fixups. This keeps `frame_off` consistent and makes reasoning simpler.

**Tests**

* Add parameterized tests that generate all pairwise overlaps for (src0, src1, dst) across:

  * felt fp‑fp, felt fp‑imm, u32 fp‑fp, u32 fp‑imm (arith, bitwise, compare).
  * Cases: all equal; src0==dst; src1==dst; src0==src1; partial overlaps at `+1` for u32.
  * Assert no single instruction reads/writes the same location; assert the resulting instruction sequence computes the same result (using a tiny interpreter or golden snapshots).

---

### 2) Comparison normalization: finish edges & prove it

**Files**: `builder/normalize.rs`, `builder/u32_ops.rs`, `builder/felt.rs`, `generator.rs`

* **u32**:

  * Your normalization tables are good (swap/complement/bias). You handle a few boundary constants (`> MAX` → false, `<= MAX` → true). But your tests don’t cover:

    * `c = 0` for `>=`/`>`; `c = 0` for `<`/`<=` (common fenceposts).
    * Wraparound of `biased_imm` for `c = 0xFFFF_FFFF` in all ops (not only `<=`).
    * Randomized operands vs. immediate (property tests).
  * **Immediate-left bitwise ops** (`imm & x`, `imm | x`, `imm ^ x`) are not supported. Either **normalize** commutative ops to immediate‑right (preferred) or implement both sides.

* **felt**:

  * `BinaryOp::Less/Greater/...` are `todo!` in `builder.rs`. Either:

    * **Ban them at MIR** (emit a diagnostic from the backend validation) **or**
    * **Lower them** through available primitives (`STORE_LOWER_THAN_FP_IMM` + algebra).
  * `felt_eq/neq` are done via `a-b` == `0`, which is fine. Consider constant‑folding `eq/neq` when both immediates are present (you already constant‑fold arith).

**Fix**

* Complete the normalization tables and make the emitters total (no fallthrough `Unsupported` for cases that can be normalized).
* Add **property tests** that compare your compiled result to a reference computation for:

  * Random `u32` pairs for all ops (arith, bitwise, cmp).
  * Random felt pairs for supported ops (eq/neq; later lt/le/gt/ge if implemented), using M31 modulo arithmetic.
* Extend boundary tests for all `c∈{0,1,2^16-1,2^16,2^31-2,2^32-1}` and random values.

---

### 3) Label resolution & addressing units: assert the contract

**Files**: `generator.rs` (`calculate_memory_layout`, `resolve_labels`), `ctrlflow.rs`

* You compute physical PCs in **QM31 words** and use **relative offsets** for `JNZ_FP_IMM` by subtracting physical addresses. That is correct ***if and only if*** the ISA’s `JNZ` takes a displacement in QM31 words. This assumption lives only in comments.
  **Action**: encode this contract and assert it:

  * Document it in `instruction.rs` near `JNZ_FP_IMM`.
  * Add unit tests where a tiny dummy program performs a `jnz` past a 3‑ or 5‑word instruction and verify the resolved encoding is correct (e.g., snapshot with `to_asm` plus a comment showing expected delta).
  * If the VM later changes addressing units, this *must* fail loudly.

---

### 4) Panics in library paths

**Files**: `lib.rs` (`InstructionBuilder::build`), `generator.rs::resolve_labels`

* `InstructionBuilder::build` **panics** on unresolved labels; `resolve_labels` errors on unexpected label operands in opcodes. Panics are hostile in library code; prefer error returns and bubble them up via `CodegenError`.
  **Action**: return `Result<Instruction>` from `build`, make `compile()` validate and convert, and keep `Program` construction fallible. Tests can still `unwrap()`.

---

### 5) Inconsistent metadata & types in `instruction.rs`

**Files**: `common/src/instruction.rs`

* `Ret` (`RET`) is defined with `fields: []` but `mem_access: 2` and `operands: [Felt, Felt]`. That’s semantically… odd. If `mem_access` and `operand_types` are *documentary* (not enforced), the risk is low, but it’s misleading.
  **Action**: tighten the meaning:

  * Either remove `operand_types` for ops with no fields, or add explicit pseudo‑fields documenting implied reads (e.g., `reads_fp_minus2`, `reads_fp_minus1`) to keep the table truthful.
  * If `operand_types()` is intended for static checks later, this inconsistency will become landmines.

---

## Medium‑priority improvements (maintainability)

### 6) Centralize “safe store/copy” paths

**Files**: `builder/store.rs` (not shown), `builder.rs`

* You use `STORE_ADD_FP_IMM 0` for copies in multiple places. Centralize into a **single “copy felt”** and **“copy u32”** helper that *internally* handles aliasing and comments consistently. It reduces duplication and makes auditing the “copy is safe & canonical” path straightforward.

### 7) Pre‑allocate scratch once per function

**Files**: `generator.rs` / `builder.rs`

* See high‑priority #1: compute max temporary need (`felt_scratch:2`, `u32_scratch:4`) and reserve it in the builder at function start. Store those offsets and reuse. This stabilizes frame size and simplifies reasoning.

### 8) Normalize immediates up front

**Files**: `builder/u32_ops.rs`, `builder/felt.rs`

* For all commutative ops (felt `Add/Mul`, u32 `Add/Mul/And/Or/Xor`), **rewrite (imm, x)** → **(x, imm)** in a tiny canonicalization helper before selection. This removes left‑immediate special casing.

### 9) Complete the MIR‑side guardrail

**Files**: `backend.rs::validate_for_casm`, MIR passes

* If felt comparisons other than `Eq/Neq` are unsupported (currently they are), **reject them** at validation with a crisp diagnostic. Don’t leave TODOs in codegen paths. When you implement felt `<`/`<=`/`>`/`>=`, drop the guard.

---

## Low‑priority polish

* **`fmt_m31_imm` comments** are great. Consider emitting the “ (= -X mod M31)” only for `Sub/Div` cases to keep snapshots concise.
* **`InstructionBuilder::to_asm`** prints numeric opcodes; optionally produce symbolic mnemonics in debug output to make snapshots more legible (keep numbers for stability).
* **Docs**: Add a one‑pager on the calling convention (your `layout.rs` doc is good; link it from `README.md`).

---

## Test coverage report & gaps

**You have:**

* Unit tests for u32 normalization (some), felt arithmetic transforms, ctrlflow macros, calls/returns placement, duplicate handling for a few shapes, and integration tests over multi‑file crates and mdtests.

**You’re missing (add these):**

1. **Alias/overlap matrix** (high priority): parameterized tests over all overlap patterns for felt/u32 fp‑fp and fp‑imm variants (see high‑priority #1).
2. **u32 comparison property tests**: randomized pairs against a pure‑Rust reference (wrapping arithmetic and standard comparisons). Include boundary immediates `{0,1,2^16-1,2^16,2^31-2,2^32-1}`.
3. **Felt constant‑folding**: tests for eq/neq/arithmetic with both operands immediate (already for arith; add eq/neq).
4. **Label resolution unit test**: ensure `JNZ` relative deltas are in QM31 units across mixed‑size instruction sequences.
5. **Cast u32→felt**: edge cases:

   * `(hi, lo) = (32767, 65534)` → ok
   * `(32767, 65535)` → reject
   * `(32768, 0)` → reject
   * Random pairs in range → ok
     Verify both the acceptance (assert doesn’t emit failing assert) and the emitted arithmetic.
6. **Immediate‑left normalization**: make sure `(imm op x)` compiles for all commutative ops (u32 and felt).

**How to make these tests cheap to write**

* Build **table‑driven tests**: arrays of `(op, left_kind, right_kind, shape)` with expected opcode(s) count and comments containing sentinel substrings. A single helper can allocate dummy values at controlled offsets to exercise aliasing.
* Add a **tiny interpreter** for a subset of ops (especially u32 arithmetic/comparison/bitwise) that runs on a flat fp‑relative memory; verify sequences produced by the builder compute the same result.

---

## Optimization pass strategy (what to change)

Right now, you have a handful of “micro‑passes” implied inside builder methods plus a *post* pass (`resolve_duplicate_offsets`). That spreads concerns and complicates reasoning. Adopt a minimal pass manager:

1. **MIR → MIR normalization** (pure): canonicalize commutative ops, push immediates to the right, lower forbidden MIR shapes to supported ones (or error).
2. **MIR → CASM** emission (single pass): builder methods ensure **per‑instruction alias safety**, using scratch reserved upfront; opcode selection and comment generation are data‑driven (you already do this).
3. **Label resolution** (single pass), then **encode** (no panics).

No global “rewrite instruction list” to fix aliasing afterwards. Eliminate the duplicate‑offset pass entirely by making emission safe.

---

## Smaller nits & dead‑code cleanup

* **`instruction.rs`**: If `operand_types()` isn’t used anywhere, either remove it or start using it for static checks (e.g., to validate store/copy shapes or to auto‑derive scratch needs).
* **Comment drift**: Some comments say “op” in generated comments (`handle_fp_fp_duplicates`). Replace with the actual operator or drop the placeholder to avoid confusion in snapshots.
* **Consistent comments**: You sometimes use `// assert ...`, sometimes `// [fp + X] = ...`. Keeping a uniform comment grammar pays dividends for snapshot diffs.

---

## Actionable checklist (prioritized)


## Phase 0 — Decisions & Baseline

- [ ] P0 Decide felt comparisons policy: implement `<, <=, >, >=` or reject at MIR.
  - If reject: add backend validation with clear diagnostics and tests.
  - If implement: choose lowering strategy and add tests (see Phase 2.2).
> DECISION: Out of Scope for this PR
- [ ] P0 Confirm prover rule on u32 in-place fp-imm ops (read==write same two words).
  - Default to “treat as conflict” unless formally documented as allowed.


## Phase 1 — Correctness & Architecture (P0)

1.1 Move alias handling into emission and delete post-pass
- [x] Refactor `crates/compiler/codegen/src/builder.rs` to remove `resolve_duplicate_offsets` and related helpers.
- [x] Add per-instruction alias guards inside emitters:
  - Felt emitters: `felt_add`, `felt_sub`, `felt_mul`, `felt_eq/neq`, etc.
  - U32 emitters: arithmetic, bitwise, compare in `builder/u32_ops.rs`.
  - Central `emit` path should never push a multi-mem op with aliasing.
- [x] On conflict, expand locally using canonical copy helpers (see 6.1).
- [x] Ensure every synthesized store/copy path calls `emit_touch` so `max_written_offset` is consistent.
- [x] Delete any remaining code paths depending on the post-hoc pass.
- Acceptance:
- [x] No code path constructs multi-mem instructions with aliased FP offsets.
- [x] All tests pass without a duplicate-offset fixup pass.
> DECISION: Out of Scope for this PR

1.2 Pre-allocate scratch once per function
- [ ] Compute worst-case scratch (minimum: 2 felt + 4 words for u32).
- [ ] Reserve scratch in function prologue; store base offsets on the builder (e.g., `felt_scratch0/1`, `u32_scratch_base`).
- [ ] Replace all ad-hoc `reserve_stack` growth during emission with these fixed scratch slots.
- [ ] Document invariant: frame size is fixed after prologue reservation.
- Acceptance:
  - [ ] `frame_off` computations are stable; no growth after prologue.
  - [ ] Builders only use pre-reserved scratch slots.
> DECISION: Out of Scope for this PR

1.3 Unify alias policy across felt and u32
- [ ] Treat all in-place and partial overlaps as conflicts consistently for felt and u32.
- [ ] Remove u32 special-casing that allowed in-place fp-imm unless prover guarantees otherwise.
- [ ] Encode the alias rules in one place and unit-test the matrix (see Tests 1.1).
- Acceptance:
  - [ ] Alias policy is identical across shapes; tests enforce it.


## Phase 2 — Semantics & Normalization (P1)

2.1 Finish u32 comparison normalization
- [ ] Audit normalization tables in `builder/u32_ops.rs` for `lt/le/gt/ge/eq/neq` with immediates and registers.
- [ ] Complete boundary handling for `c ∈ {0, 1, 2^16-1, 2^16, 2^31-2, 2^32-1}` across all ops.
- [ ] Verify wraparound of `biased_imm` at `c = 0xFFFF_FFFF` in every applicable path.
- [ ] Ensure emitters are total; avoid `Unsupported` where normalization is possible.
- [ ] Add property tests vs. a pure-Rust reference (see Tests 2.1) - can use `proptest` ?.
- Acceptance:
  - [ ] All u32 comparisons compile for reg/reg and reg/imm.
  - [ ] Property tests cover randomized cases and boundaries.

2.2 Felt comparisons (policy from Phase 0)
- Option A — Reject at MIR:
  - [ ] In `backend.rs::validate_for_casm`, reject `Less/Greater/Le/Ge` on felt with diagnostic.
  - [ ] Tests to assert rejection with helpful messages.
- Acceptance:
  - [ ] No `todo!` remains in felt comparison code paths.


## Phase 3 — Normalization & API Ergonomics (P2)

3.1 Normalize immediates up front
- [x] Add canonicalization to rewrite commutative ops `(imm, x)` → `(x, imm)` for felt `Add/Mul` and u32 `Add/Mul/And/Or/Xor`.
- [x] Route immediate-left bitwise ops through immediate-right code path.
- [x] Ensure builders never need special immediate-left handling.
- Acceptance:
- [x] `(imm op x)` works for all commutative ops via normalization.

3.2 Replace panics with errors
- [ ] Change `InstructionBuilder::build` to return `Result<Instruction, CodegenError>`.
- [ ] Bubble errors through `generator.rs::compile` and related entrypoints.
- [ ] Convert `resolve_labels` panics into typed errors (e.g., `UnresolvedLabel`).
- [ ] Update tests to expect errors or `unwrap()` in happy paths.
- Acceptance:
  - [ ] No panics in library code paths for expected error cases.
> DECISION: Out of Scope for this PR - panics accepted.


## Phase 4 — Metadata, Contracts, and Polish (P3)

4.1 Instruction metadata consistency
- [ ] Review `crates/common/src/instruction.rs` for `RET` and similar ops.
- [ ] Either remove misleading `operand_types` for ops with no explicit fields, or add pseudo-fields that document implied reads.
- [ ] Add comments clarifying the meaning of `mem_access` and `operand_types`.
- Acceptance:
  - [ ] Metadata table truthfully reflects behavior; ready for static checks.
> DECISION: Out of Scope for this PR.

4.2 Centralize copy helpers
- [ ] Review copy helpers in a single module (e.g., `builder/store.rs`)
- [ ] Internally handle aliasing, use canonical op (`STORE_ADD_FP_IMM 0`), and standardize comments.
- [ ] Replace ad-hoc copies across the codebase.
- Acceptance:
  - [ ] One canonical implementation; callers use only these helpers.

4.3 Label resolution contract and tests
- [ ] Document addressing units near `JNZ_FP_IMM` in `instruction.rs`.
- [ ] Add unit tests that place `jnz` across mixed-width instructions and assert correct displacement in QM31 words.
- Acceptance:
  - [ ] Tests fail loudly if addressing units change.

4.4 Debug output and docs
- [ ] Consider symbolic mnemonics in `InstructionBuilder::to_asm` for debug readability (keep numeric for stability in encodings).
- [ ] Add a short “calling convention and frame layout” doc; link from `README.md`.
- [ ] Tidy snapshot comments for consistency (drop placeholder “op”, unify grammar).
> DECISION: Out of Scope for this PR.


## Tests (Additions and Coverage)

1. Alias/overlap matrix (High priority)
- [ ] Parameterized tests to generate all overlaps for `(src0, src1, dst)`:
  - Shapes: felt fp-fp, felt fp-imm, u32 fp-fp, u32 fp-imm (arith, bitwise, compare).
  - Cases: all equal; `src0==dst`; `src1==dst`; `src0==src1`; partial `+1` overlaps for u32.
- [ ] Assert: no single instruction reads/writes the same location; sequences compute correct result.
- [ ] Optional: tiny interpreter over FP-relative memory for u32 subset to verify results.

2. u32 comparison property tests
- [ ] Randomized pairs against a pure-Rust reference (wrapping arithmetic + standard comparisons).
- [ ] Include boundaries `{0,1,2^16-1,2^16,2^31-2,2^32-1}`.

3. Felt constant-folding
- [ ] Tests for `eq/neq` and arithmetic when both operands are immediates.

4. Label resolution unit test
- [ ] Ensure `JNZ` deltas are in QM31 units across mixed-size instruction sequences.

5. Cast u32→felt edge cases
- [ ] Verify acceptance/rejection for:
  - `(hi, lo) = (32767, 65534)` → ok
  - `(32767, 65535)` → reject
  - `(32768, 0)` → reject
  - Random valid pairs → ok
- [ ] Assert both builder checks (non-failing assert) and emitted arithmetic.

6. Immediate-left normalization
- [x] Ensure `(imm op x)` compiles for all commutative ops via normalization.

Testing scaffolding
- [ ] Table-driven tests for shape/overlap with expected opcode counts and sentinel comments.
- [ ] Reuse helper to allocate dummy FP offsets to exercise aliasing precisely.
- [ ] Optional: a tiny interpreter for u32 arith/bitwise/compare to validate sequences.


## Pass Manager Strategy

- [ ] Adopt a minimal three-stage flow:
  1) MIR→MIR normalization (pure canonicalization; immediates to the right; ban/transform unsupported MIR).
  2) MIR→CASM emission (single pass; per-instruction alias safety using reserved scratch).
  3) Label resolution + encoding (typed errors; no panics).
- [ ] Eliminate global “duplicate-offset” rewrite passes.


## File-Level TODOs (Pointers)

- `crates/compiler/codegen/src/builder.rs`
  - [ ] Remove `resolve_duplicate_offsets`; relocate logic into emitters; ensure `emit_touch` is called.
  - [ ] Address `binary_op` felt comparisons (per policy).
- `crates/compiler/codegen/src/builder/u32_ops.rs`
  - [ ] Normalize immediate-left for commutative ops; complete comparison tables; avoid `Unsupported` when normalizable.
- `crates/compiler/codegen/src/builder/felt.rs`
  - [ ] Implement or reject felt `<, <=, >, >=`; constant-fold `eq/neq` for imm+imm.
- `crates/compiler/codegen/src/generator.rs`
  - [ ] Pre-reserve scratch in prologue; propagate error types; `resolve_labels` returns `Result`.
- `crates/common/src/instruction.rs`
  - [ ] Document addressing units for `JNZ_*`; fix `RET` metadata consistency.
- Tests (various `crates/**/tests/**`)
  - [ ] Add alias matrix, property tests, boundary tests, and label resolution tests.


## Acceptance Checklist (Summary)

- [ ] No duplicate-offset post-pass; emission is alias-safe by construction (P0).
- [ ] Scratch pre-reserved; frame size stable; no ad-hoc growth (P0).
- [ ] Unified alias policy across felt/u32 with tests (P0).
- [ ] Complete u32 comparison normalization with property tests (P1).
- [ ] Felt comparisons decided and implemented or rejected with diagnostics (P1).
- [x] Immediate-left normalization for commutative ops (P2).
- [ ] No panics in library code paths; typed errors instead (P2).
- [ ] Instruction metadata consistent; label addressing contract documented and tested (P3).
- [ ] Centralized copy helpers and consistent debug/snapshot comments (P3).


## Risks & Mitigations

- Refactor churn around emitters may destabilize tests → Land in small PRs, keep behavior snapshots, and gate with alias matrix tests.
- Prover semantics ambiguity for u32 in-place → Default to conservative policy; revisit only with formal docs and new tests.
- Property tests flakiness → Seeded RNG and deterministic shrinking; cap iteration counts in CI.


## Suggested Milestones

- M1 (Week 1): Phase 1.1 + 1.2 complete; tests green without post-pass.
- M2 (Week 2): Phase 1.3 + Phase 2.1 normalization + property tests.
- M3 (Week 3): Phase 2.2 policy finalized; implementation or validation; immediate-left normalization.
- M4 (Week 4): Error pluming, metadata cleanup, labels contract tests, docs polish.


## Definition of Done

- All P0/P1 items checked; baseline tests augmented (alias matrix, property tests, boundaries) are green.
- No unresolved `todo!` in codegen; panics removed from library flows.
- Emission path self-contained, alias-safe, and documented; frame size fixed after prologue.
- Clear documentation and tests enforce addressing units and instruction metadata invariants.
