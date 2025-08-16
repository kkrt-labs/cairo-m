Of course. Here is a list of well-scoped issues based on your request to
refactor the MIR towards a value-based, aggregate-first design. These issues are
structured to follow an incremental migration path, minimizing disruption and
allowing for verification at each stage.

### Issue 1: Introduce First-Class Aggregate Instructions in MIR

- **What:** Add new instruction kinds to the MIR for creating and accessing
  aggregate types (tuples and structs) as first-class SSA values. This is the
  foundational step for moving away from a memory-centric model.

- **Why:** The current MIR represents aggregates using memory operations
  (`frame_alloc`, `store`, `load`, `get_element_ptr`), which is verbose and
  requires complex optimization passes like SROA and Mem2Reg to convert back
  into registers. Introducing direct value-based operations for aggregates will
  make the MIR simpler, more readable, and enable more straightforward
  optimizations.

- **How:**
  1.  Navigate to `crates/compiler/mir/src/instruction.rs`.
  2.  Modify the `InstructionKind` enum to include the following new variants:

      ```rust
      // In InstructionKind enum

      /// Build a tuple from a list of values: `dest = make_tuple(v0, v1, ...)`
      MakeTuple {
          dest: ValueId,
          elements: Vec<Value>,
      },

      /// Extract an element from a tuple value: `dest = extract_tuple_element(tuple_val, index)`
      ExtractTupleElement {
          dest: ValueId,
          tuple: Value,
          index: usize,
          element_ty: MirType,
      },

      /// Build a struct from a list of field values: `dest = make_struct { field1: v1, ... }`
      MakeStruct {
          dest: ValueId,
          fields: Vec<(String, Value)>,
          struct_ty: MirType,
      },

      /// Extract a field from a struct value: `dest = extract_struct_field(struct_val, "field_name")`
      ExtractStructField {
          dest: ValueId,
          struct_val: Value,
          field_name: String,
          field_ty: MirType,
      },
      ```

  3.  Implement the corresponding constructor functions (e.g.,
      `Instruction::make_tuple(...)`) and update helper methods like
      `destinations()`, `used_values()`, and the `PrettyPrint` implementation to
      support the new instructions.
  4.  This change is purely additive and should not break existing lowering or
      optimization passes until the new instructions are actually generated.

---

### Issue 2: Refactor Lowering to Use Value-Based Aggregate Instructions

- **What:** Update the MIR lowering logic to emit the new value-based aggregate
  instructions (`MakeTuple`, `ExtractTupleElement`, `MakeStruct`,
  `ExtractStructField`) instead of the old memory-based approach (`frame_alloc`,
  `store`, `gep`, `load`).

- **Why:** This change is the core of the refactoring. By generating a
  value-oriented MIR directly from the AST, we avoid creating unnecessary memory
  traffic and stack allocations, which simplifies the IR and reduces the need
  for heavy optimization passes later.

- **How:**
  1.  **Modify `crates/compiler/mir/src/lowering/expr.rs`:**
      - In `lower_tuple_literal`, replace the `frame_alloc` and sequence of
        `store`s with a single `Instruction::make_tuple`. The result will be a
        `Value::Operand(dest)` representing the new tuple value.
      - In `lower_struct_literal`, do the same, replacing memory operations with
        a single `Instruction::make_struct`.
      - In `lower_tuple_index`, replace `lower_lvalue_expression` +
        `load_tuple_element` (which does `gep` + `load`) with a single
        `Instruction::extract_tuple_element`.
      - In `lower_member_access`, replace `lower_lvalue_expression` +
        `load_field` with `Instruction::extract_struct_field`.
  2.  **Modify `crates/compiler/mir/src/lowering/stmt.rs`:**
      - In `lower_let_statement`, when the RHS is a tuple or struct literal, the
        `lower_expression` call will now return a direct SSA value. The
        `bind_variable` helper should be updated to simply map the variable's
        `MirDefinitionId` to this new `ValueId`. The special cases for tuple
        destructuring can be simplified.
      - In `lower_assignment_statement`, an assignment like `x = y` where `y` is
        an aggregate value now becomes a simple SSA renaming (handled by
        `bind_variable` or a direct `assign` instruction). Composite copies
        (`tuple_a = tuple_b`) no longer require an element-wise memory copy
        loop.
  3.  Initially, retain the old memory-based lowering path as a fallback for
      cases that might require an address (though for now, assume all aggregates
      are by-value).

---

### Issue 3: Simplify the Optimization Pipeline

- **What:** Now that the MIR no longer relies on memory operations for simple
  aggregates, conditionally disable or entirely remove the `SroaPass` and
  `Mem2RegSsaPass`.

- **Why:** These passes exist solely to reverse the "all aggregates in memory"
  lowering strategy. With a value-based aggregate model, they are redundant for
  most functions and add significant complexity and compile-time overhead
  (dominance analysis, renaming walks, etc.). Removing them is the primary
  payoff of this refactoring.

- **How:**
  1.  **Modify `crates/compiler/mir/src/passes.rs` in
      `PassManager::standard_pipeline()`:**
      - Comment out or remove the lines that add `sroa::SroaPass` and
        `mem2reg_ssa::Mem2RegSsaPass`.
  2.  **Delete Unused Code (in a follow-up PR):**
      - Delete the files:
        - `crates/compiler/mir/src/passes/sroa.rs`
        - `crates/compiler/mir/src/passes/mem2reg_ssa.rs`
        - `crates/compiler/mir/src/analysis/dominance.rs`
      - Remove the corresponding module declarations in `passes/mod.rs` and
        `analysis/mod.rs`.
  3.  **Verify Correctness:** Run the compiler test suite. Simple functions that
      use structs and tuples should now compile correctly without ever invoking
      the SROA or Mem2Reg machinery.

---

### Issue 4: Implement a Constant Folding & Algebraic Simplification Pass

- **What:** Create a new, simple optimization pass that performs constant
  folding and basic algebraic simplifications on MIR instructions.

- **Why:** With a cleaner, value-based MIR, simple optimizations like constant
  folding become highly effective. This pass will handle many cases that were
  previously obscured by memory operations. For example,
  `extract_tuple_element(make_tuple(1, 2), 0)` can be folded directly to the
  constant `1`. This is a high-value, low-complexity optimization.

- **How:**
  1.  Create a new file `crates/compiler/mir/src/passes/const_fold.rs`.
  2.  Implement a `MirPass` that iterates through each instruction in a
      function.
  3.  Maintain a map of `ValueId` to `Option<Literal>` for constant values.
  4.  Implement folding logic:
      - For `BinaryOp`, if both `left` and `right` operands are known constants,
        compute the result at compile time and replace the instruction with an
        `Assign` of the constant result.
      - For `ExtractTupleElement`, if the `tuple` operand comes from a
        `MakeTuple` instruction, replace the instruction with an `Assign` of the
        corresponding element.
      - Do the same for `ExtractStructField` and `MakeStruct`.
  5.  Add this new pass to the `PassManager::standard_pipeline()` in
      `passes.rs`, placing it early in the pipeline (e.g., after
      `PreOptimizationPass`).

## 1) MIR: add first-class aggregate instructions

**What** Add new `InstructionKind`s and builders:

- `MakeTuple { dest, elems: Vec<Value> }`
- `ExtractTuple { dest, tuple: Value, index: usize }`
- `MakeStruct { dest, fields: Vec<(String, Value)> }`
- `ExtractField { dest, struct_: Value, field: String }`
- _(Optional)_
  `InsertField { dest, struct_: Value, field: String, value: Value }`

**Why** These are the primitives for aggregate-as-values. They let us eliminate
early `frame_alloc/GEP/load/store` noise and massively simplify lowering/opts.

**How**

- Edit: `mir/src/instruction.rs`
  - Add enum variants, `Instruction` ctors, `destinations()`, `used_values()`,
    `validate()`, `pretty_print()`.

- Edit: `mir/src/builder/instr_builder.rs` (not shown but referenced) to expose
  `make_tuple()`, `extract_tuple()`, `make_struct()`, `extract_field()`,
  `insert_field()`.
- Edit: `mir/src/passes/validation.rs` to accept & lightly sanity-check new ops.
- DoD: A tiny unit test that builds a function with each new op and
  pretty-prints it.

---

## 2) Lowering: tuple/struct literals produce SSA, not stack

**What** Change lowering so `Expression::Tuple` and `Expression::StructLiteral`
emit `MakeTuple/MakeStruct` and return an SSA value (not an address).

**Why** This removes the current tuple/struct allocation + stores in
`lower_tuple_literal()` and `lower_struct_literal()`.

**How**

- Edit: `lowering/expr.rs`
  - Replace `frame_alloc` + per-element `store_*` with a single `Make*` to a
    fresh typed value id.
  - Return `Value::operand(dest)` directly.

- Edit: `lowering/stmt.rs::lower_let_statement`
  - When pattern is `Identifier` and RHS is tuple/struct, bind the SSA value
    (not address).

- DoD: MIR dump for a simple `let p = Point { x, y };` shows one `MakeStruct`
  and **no** `framealloc/store`.

---

## 3) Lowering: field/tuple access use extract ops

**What** Replace `load_field/load_tuple_element` + `get_element_ptr` paths with
`ExtractField/ExtractTuple`.

**Why** Eliminates GEP & memory round-trip for by-value aggregates.

**How**

- Edit: `lowering/expr.rs`
  - `lower_member_access` → `ExtractField` on the **SSA struct value**.
  - `lower_tuple_index` → `ExtractTuple(tuple, i)`.

- Keep the existing memory path only if the base is genuinely an address (e.g.,
  result of `AddressOf`, array element address, or ABI reasons).
- DoD: MIR for `p.x + t.1` contains two `Extract*` and no loads.

---

## 4) Lowering: function call results and tuple contexts

**What** When a call returns multiple values and the _expression context_
requires a **single tuple value**, synthesize it with `MakeTuple`. For direct
indexing `f(...).k`, just pick result `k`.

**Why** Your calls already support multi-result; this bridges expression
semantics without spilling to memory.

**How**

- Edit: `lowering/expr.rs::lower_function_call_expr`
  - If the expression type is tuple and the call returns multiple values, emit
    `MakeTuple(vec![vals...])` into a fresh `%t`, and return `%t`.
  - Keep the existing fast path for `TupleIndex` on calls (already present).

- DoD: `let a = f(); let b = (f())` and `let c = (f()).1` lower to `MakeTuple`
  once and `ExtractTuple` once.

---

## 5) Lowering: returns from tuple/struct expressions without memory

**What** Allow `return (a, b)` or `return t` (where `t` is a tuple SSA) to lower
into multiple return values by extracting elements as needed—no tuple
allocation.

**Why** Your function ABI already models returns as `Vec<Value>`.

**How**

- Edit: `lowering/stmt.rs::lower_return_statement`
  - If returning a tuple SSA, emit `ExtractTuple` per element (in order) to feed
    `Terminator::Return { values }`.
  - If the return expression is a struct (and your language allows returning a
    struct by value), pass the struct SSA through; keep late lowering
    responsibility to backend if needed.

- DoD: Returning tuples produces no `framealloc` in MIR.

---

## 6) Keep mutable variables correct: add “Variable-SSA” (phi) pass

**What** Introduce a new pass that inserts `Phi` nodes for **variables**
(definitions) when they are rebound across control flow, i.e., turn “variables
as names” into SSA without the memory crutch.

**Why** Today, correctness after merges relies on stack slots. Once we bind
aggregates as SSA, we need phis for reassignments (e.g., `if` merges).

**How**

- New file: `mir/src/passes/var_ssa.rs`
  - Reuse your existing dominance infra (`analysis/dominance.rs`).
  - Model “promoted entities” as `MirDefinitionId` (not allocas).
  - Two phases akin to `mem2reg_ssa`:
    1. **Phi placement**: for each variable with multiple assign blocks, place
       phis at dominance frontiers (block-local, not per-field).
    2. **Rename**: DFS over dom tree, maintain a stack `var -> current Value`,
       rewrite uses to current, push on assignment, pop on exit.

- Wire into pipeline before validation.
- DoD:
  - A test where `let x = 0; if c { x = 1 } else { x = 2 }; return x` yields one
    `Phi` in merge, no memory ops.
  - Works with structs:
    `x = MakeStruct(...); x = InsertField(x,"f",v); return x`.

---

## 7) Lowering: assignment becomes SSA rebinding (no stores) for non-address LHS

**What** For `lhs = rhs` where `lhs` is an identifier bound to a **value** (not
an address), just create a new SSA version and register it to the variable; for
`lhs.field = v`, use `InsertField`.

**Why** This is the key step that keeps us off the stack while preserving
mutation semantics via value replacement.

**How**

- Edit: `lowering/stmt.rs::lower_assignment_statement`
  - Case 1: `Identifier` LHS mapped to value → bind to new `rhs` value; mark
    block as definition site for Var-SSA pass.
  - Case 2: `MemberAccess` LHS (`x.f`) → compute `x' = InsertField(x,"f",v)` and
    **rebind** `x` to `x'`.
  - Case 3: Arrays or explicit address-taking keep using memory for now (see
    Issue #11).

- DoD: Assignments in straight-line code don’t introduce any memory ops; across
  branches, Var-SSA inserts the needed `Phi`.

---

## 8) PassManager: make optimization pipeline aggregate-aware

**What** Adjust the standard pipeline:

- Always run `PreOptimizationPass`.
- Run `VarSsa` (new) before `Validation`.
- Run `Mem2RegSsaPass` **only** if the function still contains
  `FrameAlloc/Load/Store`.
- Keep `SsaDestruction` if your backend expects no `Phi` (or move that to
  backend stage).
- Keep `FuseCmpBranch`, `DeadCodeElimination`, `Validation::new_post_ssa()`.

**Why** We reduce work on functions that never touch memory and preserve the old
path only where needed.

**How**

- Edit: `passes.rs::PassManager::standard_pipeline()`.
- Add helper `fn function_uses_memory(f: &MirFunction) -> bool`.
- DoD: Functions created by the new lowering (no memory) skip SROA/mem2reg
  completely.

---

## 9) Pre-opt: constant/copy folding for aggregate ops

**What** Add simple, local rewrites:

- `ExtractTuple(MakeTuple(vs), i) → vs[i]`
- `ExtractField(MakeStruct{… f: v …}, "f") → v`
- `InsertField(MakeStruct{… f: old …}, "f", v) → MakeStruct{… f: v …}` (local)
- Remove dead `Make*` whose result unused.

**Why** Catches 80% of wins without a general CSE.

**How**

- Edit: `passes/pre_opt.rs`
  - New pass methods scanning `instructions` with small pattern matches.

- DoD: Unit tests showing these simplifications fire and reduce instruction
  counts.

---

## 10) Validation: extend checks for aggregate ops

**What** Add light checks:

- `ExtractTuple` index < arity when statically known.
- `ExtractField` field name exists when type is known.
- `InsertField` field exists.

**Why** Keeps MIR well-formed early; matches existing validation style (warn if
types are unknown).

**How**

- Edit: `passes/validation.rs`
  - Use `function.get_value_type_or_unknown(dest/src)` where applicable; when
    unable to prove, skip strict check.

- DoD: Bad field/index emits validation messages under `RUST_LOG` like existing
  code.

---

## 11) Arrays & genuine addresses: keep the memory path (for now)

**What** Scope arrays and explicit `AddressOf` to the existing memory-based
lowering.

**Why** Lets us land aggregates without boiling the ocean.

**How**

- No changes except: guard the new extract/insert code so it only applies to
  struct/tuple values; when you detect array lvalues/loads, keep current
  `get_element_ptr + load/store`.
- DoD: Existing array tests still pass unchanged.

---

## 12) Builder API cleanup: retire field/tuple load/store helpers

**What** Deprecate (or no-op) helpers that enforce memory:

- `load_field`, `store_field`, `load_tuple_element`, `store_tuple_element`.

**Why** They encourage the old style. We want all new code to go through
`Extract*`/`InsertField`.

**How**

- Edit: `lowering/builder.rs`
  - Mark as `#[deprecated]` in comments; route internal use sites to the new
    ops.

- DoD: No remaining calls from lowering to the old helpers for structs/tuples.

---

## 13) Pretty-print polish for new ops

**What** Readable printing, e.g.:

- `%t = maketuple %0, %1`
- `%x = makestruct { x: %0, y: %1 }`
- `%v = extracttuple %t, 1`
- `%v = extractfield %p, "x"`
- `%s1 = insertfield %s0, "x", %nx`

**Why** Debuggability.

**How**

- Edit: `instruction.rs::PrettyPrint for Instruction`
- DoD: Snapshot tests (assert string) for a few snippets.

---

## 14) Backend guard: late aggregate lowering (feature-flagged)

**What** Add an optional late pass that rewrites aggregates to memory/registers
if your backend can’t directly consume `Make*/Extract*` (keep off by default if
backend is fine).

**Why** Isolates target-specific ABI pressure to one place.

**How**

- New pass: `passes/lower_aggregates.rs` (simple: spill tuple/struct values to a
  temp stack slot where required, or at call/return boundaries depending on
  ABI).
- Toggle via `PipelineConfig.backend_config.options["lower_aggregates"]=true`.
- DoD: With the flag on, MIR before the backend contains only familiar memory
  ops; with it off, aggregates remain as values.

---

## 15) Pipeline config: toggle new MIR on/off

**What** Introduce `PipelineConfig.backend_config.options["agg_mir"]="on|off"`
and a crate-level env flag `CAIROM_AGG_MIR=1` to enable the new lowering.
Default **on**.

**Why** Safe rollout, A/B during migration.

**How**

- Edit: `pipeline.rs` to plumb the flag to lowering (via a global or context
  option).
- Edit: `lowering/function.rs::generate_mir` to pass the option down to
  `MirBuilder`.
- DoD: CI can run both modes on a few golden files.

---

## 16) Update tests & add new ones

**What**

- Unit tests for new instructions, pre-opt rewrites, Var-SSA.
- Integration tests for common patterns:
  - Tuple/struct literals, indexing, member access.
  - Reassignment + merge (`if`, `while`).
  - Call returning multiple values into a tuple SSA.

**Why** Locks behavior and performance regressions.

**How**

- Add test files in existing `#[cfg(test)]` modules near each pass and in
  `lowering`.
- DoD: All tests pass in both “agg_mir on/off” where relevant.

---

## 17) Remove SROA/mem2reg special-casing once unused

**What** Once functions are aggregate-first and Var-SSA is in, delete:

- `passes/sroa.rs`
- `passes/mem2reg_ssa.rs` …and the heavy dominance frontier usage **unless**
  still needed for Var-SSA.

**Why** Codebase simplification—one of your stated goals.

**How**

- After a period behind the flag, remove modules and references; keep
  `analysis/dominance.rs` for Var-SSA.
- DoD: Build succeeds; pipeline still optimizes memory users
  (arrays/address-taking) via simpler passes or the late aggregate lowering.

---

## 18) Docs: mini MIR RFC in the repo

**What** Write `docs/mir_aggregate_first.md` with the design, ops, examples, and
migration notes.

**Why** Keeps contributors aligned and prevents regressions to memory-first
style.

**How**

- Include before/after MIR snippets from unit tests.
- DoD: Linked from README or dev guide; reviewed by 1–2 teammates.

---

### Notes on scope & compatibility

- This plan deliberately **does not** change arrays yet. That isolates the
  riskiest memory semantics.
- `Phi` nodes already exist and `SsaDestruction` is in place; Var-SSA will reuse
  your dom/frontier logic almost 1:1.
- Your `FuseCmpBranch` pass remains valuable and is unaffected.
- `Validation` currently prints warnings/errors gated by `RUST_LOG`; mirror that
  style for new checks.

If you want, I can turn one or two of these into concrete PR checklists (file
diffs & signatures) so a teammate can start coding right away.
