Here’s a full audit of the MIR crate you shared. I focused on: (A)
correctness/bugs, (B) optimization pipeline soundness, (C) duplication &
unnecessary abstractions, (D) API quality, and (E) making the crate
backend-pluggable with MIR-level opts applied cleanly.

I cite files and the closest function/construct so you can jump to it quickly.

---

# A) Bugs & correctness issues (❗fix first)

1. ## Pre-opt pass forgets to run `eliminate_dead_stores`

- **Where:** `passes/pre_opt.rs`
- **Symptoms:** You implemented `eliminate_dead_stores`, but `run()` never calls
  it. Dead stores won’t be removed.
- **Fix:** Call it in `run()` between `eliminate_dead_instructions` and
  `eliminate_dead_allocations` (or first).

  ```rust
  fn run(&mut self, function: &mut MirFunction) -> bool {
      let mut modified = false;
      modified |= self.eliminate_dead_instructions(function);
      modified |= self.eliminate_dead_stores(function);          // ← missing call
      modified |= self.eliminate_dead_allocations(function);
      ...
  }
  ```

2. ## SSA destruction can be semantically wrong (needs parallel copy semantics)

- **Where:** `passes/ssa_destruction.rs::eliminate_phi_nodes`
- **Problem:**
  - You insert `Instruction::assign(dest, value, ty)` on predecessors (or split
    edges) for each φ source. When there are **multiple φ nodes** in the same
    successor or **overlapping sources/dests**, classic SSA destruction requires
    **parallel copy** semantics to avoid clobbering.
  - Current code inserts linear assignments in arbitrary order → can produce
    wrong results if one φ’s `dest` is used as another φ’s `value` on the same
    edge (copy cycles).

- **Fix:** On each (pred→succ) edge, aggregate all φ moves into a **parallel
  copy** and then lower it to a sequence using a temporary as needed (standard
  algorithm):
  - Compute move graph per edge.
  - Break cycles with temps.
  - Emit the ordered sequence into the **edge block** (split if critical).

- **Also:** You reuse the φ `dest` as an LHS on multiple different predecessors,
  so each `dest` is assigned in multiple blocks. This is expected after SSA
  destruction, but your **Validation** pass later warns about multiple
  definitions. Consider downgrading or gating that check post-SSA-destruction.

3. ## SROA can’t trigger with current lowering: typed GEP vs untyped GEP mismatch

- **Where:**
  - SROA inspects `InstructionKind::GetElementPtrTyped` and
    `BuildStruct/BuildTuple` (see `passes/sroa.rs`).
  - Lowering **never emits** `get_element_ptr_typed`, `build_struct`, or
    `build_tuple` (see `lowering/expr.rs` et al.). It only emits **untyped**
    `get_element_ptr` + loads/stores.

- **Result:** SROA’s main analysis (`identify_alloca_candidates` &
  `identify_ssa_aggregates`) finds nothing (or flags aggregates as escaping due
  to untyped GEPs) → SROA effectively **no-ops** on real programs.
- **Remedy (pick one and apply consistently):**
  - **Option A (prefer):** Update lowering to emit **typed** GEPs (and
    optionally `BuildStruct/BuildTuple` for SSA aggregates). Add an
    `InstrBuilder::get_element_ptr_typed` and use it whenever you know layout
    (struct/tuple/tuple-index/field). Then keep SROA as is.
  - **Option B:** Teach SROA to **understand untyped GEP + DataLayout** paths
    (rebuild paths by chasing constant offsets), and keep lowering as
    “address-based”. Remove reliance on `*_typed` and `Build*` if you don’t use
    them.

4. ## Mem2Reg SSA pass is overly restrictive and has latent offset bugs

- **Where:** `passes/mem2reg_ssa.rs`
- **Issues:**
  - **Promotable types:** `DataLayout::is_promotable` == `size==1`, so **u32
    (size=2)** is never promoted. That’s fine as a first cut but surprising
    since you have u32 ops. Consider allowing scalars with size>1 if the backend
    treats them as single SSA registers.
  - **GEP offset handling:** The pass tracks constant integer offsets in
    `gep_values` but:
    - It **marks escaping** when offsets are non-literals (e.g., computed index)
      → prevents promotion even when safe.
    - When populating φ sources in `rename_block`, it **always reads from stack
      key `(alloc_id, 0)`**, ignoring nonzero offsets (see the successor
      handling that looks up `value_stacks.get(&(*alloc_id, 0))`). If you ever
      had offset-specific stacks, you’d feed the wrong value into φs. ➜ Today
      this _mostly_ doesn’t bite because only size-1 allocas are promotable and
      you rarely use offsets on a scalar, but it’s an invariant worth enforcing
      or generalizing.

- **Fixes:**
  - Either forbid/promote only offset==0 allocas & assert that invariant, or
    carry the offset info into φ location/source selection (key by
    `(alloc_id, offset)` consistently everywhere).
  - Optionally lift the literal-offset constraint by basic expression
    equivalence (out of scope if you want to keep it simple).

5. ## `add_basic_block_with_name` drops the name (debuggability / misleading API)

- **Where:** `function.rs::add_basic_block_with_name`
- **Problem:** You accept a `name: String` in `CfgBuilder::new_block` →
  `MirFunction::add_basic_block_with_name(_name: String)` but **ignore it**;
  `BasicBlock` has no `name`. ➜ Misleading API and lost debug info.
- **Fix:** Either (a) add `pub name: Option<String>` to `BasicBlock` and use it
  in `PrettyPrint`, or (b) remove the name parameter entirely from the API.

6. ## Potential silent type mismatch during struct literal lowering

- **Where:** `lowering/expr.rs::lower_struct_literal`
- **Issue:** You fetch `field_type` twice. First: from the **expression’s**
  semantic type (correct), but then overwrite it with:

  ```rust
  let field_type = struct_type.field_type(field_name.value())
      .unwrap_or(&MirType::felt()) // ← silently falls back to felt(!)
      .clone();
  ```

  If the field name is wrong or mapping is out of sync, you silently fall back
  to `felt`, hiding a bug.

- **Fix:** Replace with a **hard error**:

  ```rust
  let field_type = struct_type.field_type(field_name.value())
      .ok_or_else(|| format!("Field '{}' not found in {:?}", field_name.value(), struct_type))?
      .clone();
  ```

7. ## `FuseCmpBranch` may invert branches subtly with “zero compare”

- **Where:** `passes.rs::FuseCmpBranch`
- **Note:** The rewrites like `(Eq 0, cond)` → `branch(cond == 0)` → flipping
  then/else are correct, but brittle if `0` is not the appropriate “false” for
  non-boolean types (e.g., future types). It’s fine today for felt/u32 but
  document this assumption in the pass (or gate on operand type==bool/u32/felt).

8. ## Tests create an unused extra block

- **Where:** `cfg.rs::tests::create_diamond_cfg`
- **Issue:** After `MirFunction::new` (which creates 1 block), you **push four
  more** (`for _ in 0..4`) and then use indices 0..3, leaving block 4 unused.
  Not a bug at runtime, but noisy.
- **Fix:** Push **three** new blocks (indices 1..3).

9. ## Validation pass warnings after SSA destruction

- **Where:** `passes.rs::Validation`
- **Issue:** After SSA destruction, multiple assignments to the same `ValueId`
  are expected. `validate_single_definition` always warns if a value is defined
  more than once. Your `PassManager` runs `Validation` **after**
  `SsaDestructionPass`.
- **Fix:** Either relax that rule “post-SSA” or move the check earlier
  (pre-SSA-destruction). You already return `false` (no pass modification), so
  this is “just” noisy, but confusing.

---

# B) Optimization pipeline & analysis soundness

- **Pass order** (`PassManager::standard_pipeline`):

  ```
  PreOptimization → SROA → Mem2RegSSA → SSADestruction → FuseCmpBranch → DCE → Validation
  ```

  This is broadly OK, **provided** SROA/Mem2Reg actually see the patterns they
  expect. Right now:
  - SROA finds almost nothing (see A-3).
  - Mem2Reg is conservative (see A-4).
  - SSA destruction removes φ’s but lacks parallel copies (A-2).

- **Recommendation:**
  - Make lowering and passes agree on IR shape (typed GEP + SSA aggregates
    **or** fully address-based).
  - Add an early, cheap **CFG cleanup** (split critical edges globally) before φ
    lowering if you keep the present SSA destruction model (helps insertion).
  - Consider **dominator tree caching** if you’ll run more dom-based passes
    later (right now only mem2reg uses it).

---

# C) Duplication (N_OCCURRENCES > 3 → extract helpers)

1. **Semantic type → `MirType` lookups** Repeated many times:
   `expression_semantic_type(...)` then `MirType::from_semantic_type(...)`.
   - **Where:** `lowering/expr.rs` (`lower_unary_op`, `lower_binary_op`,
     `lower_function_call_expr`, `lower_member_access`, `lower_index_access`,
     `lower_tuple_literal`, `lower_tuple_index`, etc.)
   - **Action:** Use the already-present cache method
     `LoweringContext::get_expr_type(expr_id)` everywhere. Add small helpers:

     ```rust
     fn expr_ty(&self, span: SimpleSpan) -> Result<MirType, String>;
     fn def_ty(&self, def_id: DefinitionId) -> MirType;
     ```

2. **“Compute address then load/store” pattern** Sequences like:

   ```
   let dest = new_typed_value_id(*ptr(T));
   Instruction::get_element_ptr(dest, base, offset);
   let val = new_typed_value_id(T.clone());
   Instruction::load(val, T, Value::operand(dest));
   ```

   - **Where:** `lowering/expr.rs` across `lower_member_access`,
     `lower_index_access`, `lower_tuple_*`, and in `stmt.rs` for tuple
     destructuring.
   - **Action:** Extract small helpers in a `MemoryBuilder` mixin:
     - `addr_of_field(base_addr, &MirType, "field") -> ValueId /* ptr */`
     - `addr_of_tuple_index(base_addr, &MirType, usize) -> ValueId /* ptr */`
     - `load_field(base_addr, T, idx/name) -> Value`
     - `store_field(base_addr, T, idx/name, src_val)`

   - You already have variants in `InstrBuilder::load_field/store_field`, but
     they return raw `Instruction`s and are **not used** widely. Promote them,
     return `ValueId`, and **emit** directly.

3. **CFG “terminate if not already terminated”** Pattern:

   ```rust
   if !self.is_current_block_terminated() {
       self.terminate_with_jump(target);
   }
   ```

   - **Where:** `lowering/control_flow.rs::goto`, `branch`, `stmt.rs` in
     multiple places.
   - **Action:** Keep `goto`/`branch` as the one-stop helpers and always use
     them.

4. **Error messages for missing `ExpressionId` / scope resolution** Many
   occurrences of virtually the same
   `ok_or_else(|| format!("MIR: No ExpressionId..."))`.
   - **Action:** one helper:

     ```rust
     fn expr_id_for_span(&self, span) -> Result<ExpressionId, String>;
     ```

5. **Validation checks for pointer type on load/store** Repeated patterns in
   `passes.rs::Validation` for load/store pointer checks & warnings. Extract
   inner “check ptr operand type” helper and reuse.

---

# D) Unnecessary abstractions & API cleanup

- **Dead abstractions:**
  - `InstructionKind::{BuildStruct, BuildTuple, ExtractValue, InsertValue}`
    exist but lowering **never emits** them. Either:
    - adopt them (preferred if you want SSA aggregates & strong SROA), or
    - remove them (and simplify SROA accordingly).

  - `Instruction::get_element_ptr_typed` is unused. Same story as above.
  - `InstrBuilder::mov` returns `(Option<Instruction>, ValueId)` but **never**
    returns a created instruction for literal/operand; It’s not used elsewhere.
    Remove or rework it to do something meaningful.

- **Confusing duplicated maps for locals:**
  - `MirFunction.locals` vs `MirBuilder.state.definition_to_value`. Only the
    latter is used during lowering. After lowering, `MirFunction.locals` stays
    empty. Either:
    - Populate `MirFunction.locals` at the end of lowering (single source of
      truth), or
    - Remove `locals` from `MirFunction` to avoid confusion.

- **Naming inconsistencies (APIs):**
  - `InstrBuilder::binary_op_with_dest` vs `binary_op` vs `binary_op_auto` in
    `MirBuilder`. Pick one naming scheme. Suggest:
    - `emit_binary(op, lhs, rhs, result_ty) -> ValueId`
    - `emit_unary(op, src, result_ty) -> ValueId`
    - Remove “\_auto” suffixes; types are already explicit.

  - `CfgBuilder::new_block(name)` → `MirFunction::add_basic_block_with_name`
    drops the name (see bug). Either support names or rename to
    `add_basic_block()` everywhere.

- **`Instruction::pretty_print` special-cases felt to hide types** This is
  inconsistent (others print types). Choose one style. Consistency helps
  debugging.

- **`Validation` severity & pass order:** Consider splitting into “compiler
  debug validation” (verbose logs; dev only) vs “hard errors”. As is, it prints
  warnings based on env var and returns `false`, which is easy to miss.

---

# E) Dominance/frontier & CFG details

- **Dominance:** `analysis/dominance.rs` looks standard Lengauer-Tarjan-style
  (iterative simplified). A couple of notes:
  - `compute_reverse_postorder()` walks from **entry only**; unreachable blocks
    are excluded. That’s fine, but document that `DominatorTree` excludes
    unreachable.
  - In `compute_dominance_frontiers`, when `block_id` has no `idom`
    (`block_idom == None`), the `while Some(&runner) != block_idom` loop falls
    into the `else` branch and breaks. Today `idom` is missing only for
    `entry`—which never has `preds.len() >= 2`—so it’s OK. Add a comment or an
    `if let Some(block_idom)` to make the logic clearer.

- **CFG utils:** `split_critical_edge` mutates the predecessor terminator (good)
  and returns the new block. Consider a convenience that splits **all** critical
  edges _and_ returns an edge map (you already have `split_all_critical_edges`)
  and **reuse it** from SSA destruction to avoid recomputing splits per φ (minor
  perf/clarity win).

---

# F) Making backends pluggable & ensuring MIR-level opts apply

**Goal:** Any backend (e.g., Cairo bytecode, LLVM, custom VM) should be able to
consume MIR after a stable optimization pipeline.

1. **Define a backend trait**:

   ```rust
   pub trait MirBackend {
       type Output;
       type Error;
       fn codegen_module(&mut self, module: &MirModule) -> Result<Self::Output, Self::Error>;
   }
   ```

   - Provide a simple adapter that runs a `PassManager` before invoking
     `codegen_module`.
   - Allow the backend to supply or override a pass pipeline:

     ```rust
     pub struct Pipeline {
         passes: Vec<Box<dyn MirPass>>,
     }
     impl Pipeline {
         pub fn default() -> Self { PassManager::standard_pipeline().into() }
         pub fn with_pass<P: MirPass + 'static>(mut self, p: P) -> Self { ... }
     }
     ```

2. **Stabilize the MIR shape consumed by backends** Choose **one** of these and
   stick with it:
   - **Address-based MIR** (loads/stores & GEPs): keep SROA modest (or drop),
     rely on mem2reg for scalars, and avoid `Build*/Extract/Insert`.
   - **SSA aggregates** (build/extract/insert + typed GEP): upgrade lowering
     accordingly and let SROA shine. Backends will thank you for consistency.

3. **PassManager configurability** Allow backends to:
   - turn off `Validation` (or switch to warn only),
   - choose a lighter pipeline for debug builds,
   - inject backend-specific cleanup passes if needed.

---

# G) Smaller quality-of-life fixes

- **`passes/pre_opt.rs`**: the three elimination functions all compute
  `use_counts` separately. Compute once in `run()` and pass by reference to
  reduce work & duplication.
- **`layout/DataLayout`**: Document that sizes are in “word slots” (felt=1,
  u32=2). Consider a `WordSize` constant to make it explicit.
- **`Terminator::pretty_print`** prints function ids via `{:?}`; prefer showing
  names (lookup in module when pretty printing the whole module).
- **`Instruction::Cast`** has no explicit target type in the instruction. Ensure
  the `dest`’s type is always set on creation. If not, add it to the instruction
  to make the IR self-describing.
- **Logging**: `eprintln!` sprinkled throughout; prefer a feature-gated logger
  (or `log` crate facade) so downstreams can control verbosity.

---

# H) Actionable checklist (prioritized)

**Critical**

- [ ] Call `eliminate_dead_stores()` in `PreOptimizationPass::run()`
      (`passes/pre_opt.rs`).
- [ ] Implement **parallel copy lowering** in `ssa_destruction.rs` to handle φ
      cycles and overlapping sources safely.
- [ ] Decide IR shape: **typed GEP + SSA aggregates** **or** **address-based
      only**; make **lowering** and **SROA** match. Today they don’t.
- [ ] Fix silent fallback to `felt` in `lower_struct_literal` (hard error
      instead).

**High**

- [ ] Either store and print BasicBlock names or remove redundant “with_name”
      API (`function.rs`, `builder/cfg_builder.rs`).
- [ ] Clean up `Validation` post-SSA warnings about multiple definitions
      (context-sensitive rule).
- [ ] Unify semantic type lookups via `LoweringContext::get_expr_type` (reduce
      duplication & bugs).

**Medium**

- [ ] Extract helpers for common “addr/load/store” patterns (see C-2).
- [ ] Normalize builder API naming (`binary_op_auto` → `emit_binary`, etc.).
- [ ] Remove or repurpose unused IR ops (`BuildStruct/Tuple`, `Extract/Insert`,
      `*_typed`) unless you adopt them fully.

**Low**

- [ ] Fix `cfg.rs` test extra block.
- [ ] Make pretty-printing consistent across types (don’t special-case felt
      unless desired).
- [ ] Consider allowing mem2reg to promote u32 if your backend treats it as one
      register, or document why not.

---

# I) Quick patches (diff-style snippets)

1. **Pre-opt run()**

```rust
// passes/pre_opt.rs
fn run(&mut self, function: &mut MirFunction) -> bool {
    let mut modified = false;

    modified |= self.eliminate_dead_instructions(function);
    modified |= self.eliminate_dead_stores(function);       // ← add
    modified |= self.eliminate_dead_allocations(function);

    if !self.optimizations_applied.is_empty() {
        eprintln!("Pre-optimizations applied: {:?}", self.optimizations_applied);
    }
    modified
}
```

2. **SSA destruction – parallel copies (sketch)**

```rust
// passes/ssa_destruction.rs (inside eliminate_phi_nodes loop)
let mut copies_per_edge: FxHashMap<(BasicBlockId, BasicBlockId), Vec<(ValueId, Value, MirType)>> = FxHashMap::default();
// collect all (dest, value, ty) for each (pred, succ) first

// after collection:
for ((pred, succ), copies) in copies_per_edge {
    let insert_block = if is_critical_edge(function, pred, succ) {
        *edge_splits.entry((pred, succ)).or_insert_with(|| split_critical_edge(function, pred, succ))
    } else { pred };

    let seq = lower_parallel_copies_to_sequential(&copies, &mut function.basic_blocks[insert_block]);
    function.basic_blocks[insert_block].instructions.extend(seq);
}
```

Implement `lower_parallel_copies_to_sequential` to break cycles with a temp.

3. **Lowering struct literal type safety**

```rust
// lowering/expr.rs::lower_struct_literal
let field_type = struct_type.field_type(field_name.value()).ok_or_else(|| {
    format!("Field '{}' not found on struct type {:?}", field_name.value(), struct_type)
})?.clone();
```

4. **Expose typed GEP in builder (if you choose typed path)**

```rust
// builder/instr_builder.rs
pub fn get_element_ptr_typed(
    &mut self,
    dest: ValueId,
    base: Value,
    path: FieldPath,
    base_type: MirType,
) -> &mut Self {
    let instr = Instruction::get_element_ptr_typed(dest, base, path, base_type);
    self.add_instruction(instr);
    self
}
```

…and then use it from `lower_member_access`, `lower_tuple_index`, etc.

---

# J) “Backend-pluggable” glue

Add a thin façade so any backend can run with the standard pipeline or a custom
one:

```rust
pub struct MirCompiler<B: MirBackend> {
    backend: B,
    pipeline: PassManager,
}

impl<B: MirBackend> MirCompiler<B> {
    pub fn new(backend: B) -> Self {
        Self { backend, pipeline: PassManager::standard_pipeline() }
    }
    pub fn with_pipeline(mut self, pm: PassManager) -> Self { self.pipeline = pm; self }
    pub fn compile(mut self, mut module: MirModule) -> Result<B::Output, B::Error> {
        for (_, func) in module.functions.iter_enumerated_mut() {
            self.pipeline.run(func);
        }
        self.backend.codegen_module(&module)
    }
}
```

This guarantees MIR-level optimizations are applied consistently before codegen,
and lets us swap in backends easily.

---

## Bottom line

- Fix the **missing pre-opt call**, **SSA destruction correctness**, and **typed
  vs untyped SROA mismatch** first.
- Decide on the MIR shape and make lowering + passes agree.
- Clean up duplicated patterns with small helpers, and simplify the
  builder/validation APIs.

With these addressed, the crate will be in a stable, mergeable state and ready
for pluggable backends.
