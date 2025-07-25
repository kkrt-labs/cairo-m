### 📝 Issue — Refactor early passes to emit **only “must‑have” diagnostics**

_(Stop duplicating rules across `SemanticIndexBuilder` and validators)_

---

## 1 ️⃣ What — Goal of this task

Refactor the **collection phase** (`SemanticIndexBuilder` and helpers) so that
it:

- **Keeps building the `SemanticIndex` its ONLY responsibility.** It must abort
  **only** when the index would become invalid (e.g. duplicate names in a single
  scope, or an AST node it literally cannot model).
- **Off‑loads every other diagnostic** (unused / undeclared / type‑mismatch /
  control‑flow / literal range …) to existing validators in `validation/*`.
- **Uses one shared, thread‑safe diagnostic sink** so that _all_ passes push
  into a single list—no more `Vec<Diagnostic>` living in each pass.

When done, **no rule will be duplicated**: a warning/error can come from exactly
one place in the pipeline.

---

## 2 ️⃣ Why — Benefits & problems solved

- **Deterministic, cache‑friendly builder** Small surface ⇒ fewer invalidations
  in incremental compiles & IDE.
- **Zero duplicated logic** Today the same rule lives in both builder and
  validator (e.g. duplicate‑field check). Keeping one source avoids divergence
  and double reporting.
- **Cleaner layering** Later phases can trust guaranteed invariants; earlier
  passes never need knowledge of typing or usage semantics.
- **Better UX** IDE can trigger lightweight validators continuously while
  editing without re‑running heavy analysis.

---

## 3 ️⃣ How — Step‑by‑step plan

| Step | Action                                                                                                                                                                              | Touches code                                  |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| 1    | **Introduce `DiagnosticSink` trait**<br>`rust<br>pub trait DiagnosticSink: Send + Sync { fn push(&self, d: Diagnostic); }<br>pub struct VecSink(Mutex<Vec<Diagnostic>>);<br>`       | - inside cairo-m-diagnostics crate            |
| 2    | Replace all `&mut DiagnosticCollection` in **builder & validators** with `&dyn DiagnosticSink`. Remove the `RefCell<DiagnosticCollection>` field from `SemanticIndexBuilder`.       | `semantic_index.rs`, each validator           |
| 3    | **Keep only “must‑have” checks in the builder**:<br>  • duplicate definition in same scope<br>  • illegal forward‑ref that would break the index<br>Delete every other `add_error`. | `SemanticIndexBuilder::with_semantic_checker` |
|      | Move the removed checks into the appropriate validator module (most already exist; relocate helpers if needed).                                                                     | `validation/*`                                |
| 4    | **Deduplicate helper logic**<br>Extract functions such as `check_duplicate_fields` / `check_duplicate_param_names` into plain fns in `validation/shared.rs`. Both builder _and_     | new `validation/shared.rs`                    |
|      | relevant validators call the same function, so the rule’s code lives once.                                                                                                          |                                               |
| 5    | **Single collection of diagnostics**<br>In `ValidatorRegistry::validate_all` create one `VecSink`, hand a `&sink` to builder **and** every validator, then sort & dedup at the end. | `validator.rs`                                |
| 6    | **Docs & Changelog**<br>Document the Phase‑N invariant rule in `docs/architecture.md`.                                                                                              | docs                                          |

### Scope boundaries

- **No new abstractions** beyond `DiagnosticSink`; everything else stays as flat
  functions/modules.
- Type system, control‑flow algorithms, etc. stay untouched—only _where_
  diagnostics are emitted changes.
- All existing rules done in AST traversal are properly moved to the
  `validation` modules in relevant places.

### Acceptance criteria

- Running `cargo test` passes with identical (or fewer) diagnostics; no
  duplicates.
- `SemanticIndexBuilder` compiles without `semantic_syntax_checker`.
- CI shows ≥ 10 % speed‑up on `cargo check --workspace` incremental rebuild
  (builder is lighter).
- Documentation updated.

---

Happy hacking — you’ll end up with a leaner, faster pipeline and crystal‑clear
ownership of every diagnostic.
