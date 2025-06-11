# Cairo-M Semantic Analysis: Next Steps

## Introduction

The `cairo-m-compiler-semantic` crate has a robust, well-tested foundational
architecture built on Salsa for incremental computation. The core data
structures for the type system (`TypeId`, `StructTypeId`, etc.), symbol tracking
(`SemanticIndex`), and validation (`Validator` trait) are complete and align
perfectly with the initial design plan. The existing `ScopeValidator` is fully
functional and provides excellent test coverage for scope-related diagnostics.

However, the primary goal of the semantic analysis phase—**type resolution and
inference**—is currently blocked by a few critical implementation gaps. The
immediate priority is to unblock this core functionality. Once type resolution
is operational, we can proceed to implement the suite of type-dependent
validators, which currently exist only as placeholders.

This document outlines a series of atomic issues, ordered by priority, to guide
the completion of the semantic analysis implementation. The plan is divided into
three main phases:

1.  **Phase 1: Critical Fixes & Blockers:** Resolve the issues preventing type
    inference from working.
2.  **Phase 2: Core Feature Completion:** Complete the type system and activate
    all planned type-dependent validators.
3.  **Phase 3: Refactoring & Advanced Features:** Enhance the robustness of the
    system and lay the groundwork for future language features.

---

## Phase 1: Critical Fixes & Blockers

_This phase focuses on resolving the two main issues that are preventing the
type system from functioning as intended._

### 1. `[FIX]` Link Variable Definitions to their Initializer Expressions

- **Priority:** `Highest`
- **Context:** As identified in `assessment.md`, the `LetDefRef`, `LocalDefRef`,
  and `ConstDefRef` structs do not currently store a reference to their
  initializer expressions. Their `value_expr_id` field is always `None`. This
  makes it impossible for `definition_semantic_type` to infer the type of a
  variable from its assigned value (e.g., `let x = 42;`). This is the single
  biggest blocker to completing the type system.
- **Implementation Plan:**

  1.  **File:** `crates/compiler/semantic/src/semantic_index.rs`

      - **Location:** `SemanticIndexBuilder::visit_statement`
      - **Change:** In the `match` arms for `Statement::Let`,
        `Statement::Local`, and `Statement::Const`, you are already calling
        `self.visit_expression(value)`. This call returns an `ExpressionId`. You
        must capture this ID.
      - **Logic:**
        - For `Statement::Let`:
          ```rust
          // ...
          let value_expr_id = self.visit_expression(value);
          let def_kind = DefinitionKind::Let(LetDefRef::from_let_statement(
              name.value(),
              statement_type.clone(),
              Some(value_expr_id), // Pass the new ID
          ));
          // ...
          ```
        - Apply the same logic for `Statement::Local` and `Statement::Const`.

  2.  **File:** `crates/compiler/semantic/src/definition.rs`
      - **Location:** `LetDefRef::from_let_statement`,
        `LocalDefRef::from_local_statement`
      - **Change:** Update the function signatures to accept the
        `Option<ExpressionId>`.
      - **Logic:**
        - For `LetDefRef`:
          ```rust
          // Change signature
          pub fn from_let_statement(name: &str, explicit_type_ast: Option<TypeExpr>, value_expr_id: Option<ExpressionId>) -> Self {
              Self {
                  name: name.to_string(),
                  value_expr_id, // Assign the passed-in ID
                  explicit_type_ast,
              }
          }
          ```
        - Update `LocalDefRef` similarly. The `ConstDefRef::from_ast` will need
          to be updated in the builder to pass the ID.

### 2. `[REFACTOR]` Replace Span-Based AST Lookup with Direct AST Node Access

- **Priority:** `Highest`
- **Context:** The `expression_semantic_type` query currently uses a highly
  inefficient and fragile helper function, `find_expression_in_module`, to
  re-discover an AST node from its span. This is a performance bottleneck and
  can lead to incorrect lookups. We need to store the necessary AST information
  directly.
- **Implementation Plan:**

  1.  **File:** `crates/compiler/semantic/src/semantic_index.rs`

      - **Location:** `struct ExpressionInfo`
      - **Change:** Modify the struct to hold the `Expression` enum directly,
        rather than just its span. This makes each `ExpressionInfo`
        self-contained.

        ```rust
        // FROM:
        pub struct ExpressionInfo {
            pub file: File,
            pub ast_node_text_range: SimpleSpan<usize>,
            pub scope_id: FileScopeId,
        }

        // TO:
        use cairo_m_compiler_parser::parser::Expression;
        pub struct ExpressionInfo {
            pub file: File,
            pub ast_node: Expression, // Store the AST node itself
            pub ast_span: SimpleSpan<usize>, // Keep the span for diagnostics
            pub scope_id: FileScopeId,
        }
        ```

      - **Location:** `SemanticIndexBuilder::visit_expression`
      - **Change:** When creating the `ExpressionInfo`, clone the `Expression`
        value from the AST.
        ```rust
        // ...
        let expr_info = ExpressionInfo {
            file: self._file,
            ast_node: expr.value().clone(), // Clone the expression
            ast_span: expr.span(),
            scope_id: self.current_scope(),
        };
        self.index.add_expression(expr_info); // The new ExpressionId is returned and tracked by the builder.
        // ...
        ```

  2.  **File:** `crates/compiler/semantic/src/type_resolution.rs`

      - **Location:** `expression_semantic_type`
      - **Change:** Remove the entire dependency on `find_expression_in_module`.
        Access the `ast_node` directly from the `ExpressionInfo`.

        ```rust
        // ...
        let Some(expr_info) = semantic_index.expression(expression_id) else {
            return TypeId::new(db, TypeData::Error);
        };

        // REMOVE the call to parse_program and find_expression_in_module

        // USE the stored AST node directly
        match &expr_info.ast_node {
            Expression::Literal(_) => TypeId::new(db, TypeData::Felt),
            Expression::Identifier(name) => {
                // ...
            }
            // ...
        }
        ```

      - **Location:** `find_expression_in_module` and its helpers.
      - **Change:** Delete these functions entirely once they are no longer
        used.

## Phase 2: Core Feature Completion

_With the blockers resolved, this phase focuses on completing the type system
and activating the full suite of planned validators._

### 3. `[FEATURE]` Implement Full Type Inference for All Expression Types

- **Priority:** `High`
- **Context:** `expression_semantic_type` currently handles only a few
  expression types, falling back to `TypeData::Unknown` for most. We need to
  complete this implementation for a functional type system.
- **Implementation Plan:**
  1.  **File:** `crates/compiler/semantic/src/type_resolution.rs`
      - **Location:** `expression_semantic_type` `match` statement.
      - **Change:** Add logic for all remaining `Expression` variants.
        - `Expression::BinaryOp`: Get types of `left` and `right`. If both are
          `felt`, return `felt`. Otherwise, return `Error`. This can be expanded
          later for operator overloading.
        - `Expression::FunctionCall`:
          1. Get the `ExpressionId` for the `callee`.
          2. Infer the `callee`'s type using a recursive call to
             `expression_semantic_type`.
          3. If the type is `TypeData::Function(signature_id)`, return
             `signature_id.return_type(db)`.
          4. Otherwise, return `TypeData::Error`.
        - `Expression::StructLiteral { name, .. }`:
          1. Resolve the `name` to a definition.
          2. Call `definition_semantic_type` on that definition's ID.
          3. Ensure the result is a `TypeData::Struct` and return it.
        - `Expression::IndexAccess { array, .. }`:
          1. Infer the type of the `array` expression.
          2. If it's a `TypeData::Pointer(inner_type)`, the result is
             `inner_type`.
          3. (Future) If it's an array/slice type, return the element type.
          4. Otherwise, return `TypeData::Error`.
        - `Expression::Tuple`: Infer the type of each element and return a
          `TypeData::Tuple` containing those types.

### 4. `[FEATURE]` Implement `StructFieldValidator`

- **Priority:** `Medium`
- **Context:** This validator ensures that member access (e.g., `point.x`) is
  valid. It's a critical type-dependent check.
- **Implementation Plan:**
  1.  **File:**
      `crates/compiler/semantic/src/validation/struct_field_validator.rs`
      - **Location:** `StructFieldValidator::validate`
      - **Logic:**
        1. Iterate over all expressions in the `SemanticIndex`
           (`index.all_expressions()`).
        2. Find all `MemberAccess { object, field }` expressions.
        3. For each, get the `ExpressionId` for the `object` using
           `index.expression_id_by_span(object.span())`.
        4. Call `expression_semantic_type(db, file, object_expr_id)` to get the
           object's type.
        5. `match` on the object's `TypeData`:
           - If `TypeData::Struct(struct_id)`, use
             `struct_id.has_field(db, field.value())`. If `false`, create a
             `Diagnostic::InvalidFieldAccess`.
           - If any other concrete type (e.g., `Felt`), create a diagnostic, as
             primitives have no fields.
           - Ignore `Unknown` and `Error` types to prevent cascading
             diagnostics.
  2.  **File:** `crates/compiler/semantic/src/validation/validator.rs`
      - **Location:** `create_default_registry`
      - **Change:** Add the new validator to the registry.
        ```rust
        pub fn create_default_registry() -> ValidatorRegistry {
            ValidatorRegistry::new()
                .add_validator(crate::validation::scope_check::ScopeValidator)
                .add_validator(crate::validation::struct_field_validator::StructFieldValidator) // Add this
        }
        ```
  3.  **File:** `crates/compiler/semantic/src/validation/mod.rs`
      - **Change:** Uncomment the `pub use` for `StructFieldValidator`.

### 5. `[FEATURE]` Implement `FunctionCallValidator`

- **Priority:** `Medium`
- **Context:** This validator checks function call arity (argument count) and
  type compatibility.
- **Implementation Plan:**
  1.  **File:**
      `crates/compiler/semantic/src/validation/function_call_validator.rs`
      - **Location:** `FunctionCallValidator::validate`
      - **Logic:**
        1. Find all `FunctionCall { callee, args }` expressions.
        2. Infer the type of the `callee`. If it's not
           `TypeData::Function(signature_id)`, emit a diagnostic.
        3. Get the function's parameters via `signature_id.params(db)`.
        4. Compare `args.len()` with `params.len()`. If they don't match, emit
           an arity mismatch diagnostic.
        5. If they match, iterate through `args` and `params` together. For each
           pair:
           - Infer the type of the argument expression.
           - Use `are_types_compatible` to check if the argument type matches
             the parameter type.
           - If not compatible, emit a type mismatch diagnostic.
  2.  **File:** `crates/compiler/semantic/src/validation/validator.rs`
      - **Change:** Add `FunctionCallValidator` to the default registry.
  3.  **File:** `crates/compiler/semantic/src/validation/mod.rs`
      - **Change:** Uncomment the `pub use`.

### 6. `[FEATURE]` Implement `StructLiteralValidator` and `IndexingValidator`

- **Priority:** `Medium`
- **Context:** These validators are the remaining core type-dependent checks.
  They can be implemented in parallel or sequentially after the previous ones.
- **Implementation Plan:**
  - Follow the same pattern as the `StructFieldValidator` and
    `FunctionCallValidator`:
  - **`StructLiteralValidator`**:
    - Find `StructLiteral` expressions.
    - Resolve the struct's definition.
    - Check for missing fields, extra fields, and mismatched field types.
  - **`IndexingValidator`**:
    - Find `IndexAccess` expressions.
    - Validate that the base is of an indexable type (e.g., `Pointer`) and the
      index is `felt`.
  - Add both to the default validator registry and uncomment their exports.

## Phase 3: Refactoring & Advanced Features

_This phase focuses on improving the quality of the existing implementation and
starting on features for future language versions._

### 7. `[FEATURE]` Add Salsa Cycle Detection for Type Inference

- **Priority:** `Medium`
- **Context:** A direct recursive type definition (e.g.,
  `struct Node { next: Node }`) will cause the compiler to stack overflow.
  Salsa's cycle detection mechanism must be used to handle this gracefully.
- **Implementation Plan:**

  1.  **File:** `crates/compiler/semantic/src/type_resolution.rs`

      - **Location:** `definition_semantic_type` function.
      - **Change:** Add a `cycle` recovery function to the `#[salsa::tracked]`
        macro.

        ```rust
        // FROM
        #[salsa::tracked]

        // TO
        #[salsa::tracked(cycle = on_cycle_in_type_resolution)]
        ```

      - **Change:** Implement the recovery function. It should log the cycle and
        return an `Error` type to prevent cascading failures.
        ```rust
        fn on_cycle_in_type_resolution<'db>(
            _db: &'db dyn SemanticDb,
            _cycle: &salsa::Cycle,
            _def_id: DefinitionId<'db>,
        ) -> TypeId<'db> {
            // In the future, we can add a diagnostic here.
            // For now, just break the cycle.
            TypeId::new(_db, TypeData::Error)
        }
        ```
      - Apply similar logic to other potentially recursive queries like
        `expression_semantic_type`.

### 8. `[REFACTOR]` Use `IndexMap` for Struct and Function Fields

- **Priority:** `Low`
- **Context:** The `types.rs` file notes that `Vec<(String, TypeId)>` is used
  for fields and parameters instead of a map for Salsa compatibility reasons. We
  should investigate and switch to `indexmap::IndexMap` for O(1) lookups while
  preserving declaration order.
- **Implementation Plan:**
  1.  **File:** `crates/compiler/semantic/src/types.rs`
      - **Location:** `StructTypeId` and `FunctionSignatureId`.
      - **Change:** Replace
        `#[return_ref] pub fields: Vec<(String, TypeId<'db>)>` with
        `pub fields: indexmap::IndexMap<String, TypeId<'db>>`.
      - Update the helper methods (`field_type`, `has_field`, etc.) to use
        `IndexMap`'s API.
  2.  **File:** `crates/compiler/semantic/src/type_resolution.rs`
      - **Location:** `struct_semantic_data` and `function_semantic_signature`.
      - **Change:** Update the construction of the fields/params to build an
        `IndexMap`.

### 9. `[FEATURE]` Implement Control Flow Analysis for Unreachable Code

- **Priority:** `Low`
- **Context:** The compiler does not currently detect unreachable code (e.g.,
  code after a `return` statement). This is a valuable diagnostic for
  developers.
- **Implementation Plan:**
  1.  Create a new `ControlFlowValidator` in the `validation` module.
  2.  In its `validate` method, traverse the statement blocks of each function.
  3.  Maintain a boolean flag, `is_terminated`, for each block.
  4.  When a `Statement::Return` is encountered, set `is_terminated` to `true`.
  5.  If any statement is visited while `is_terminated` is `true`, emit an
      `UnreachableCode` diagnostic for that statement's span.
  6.  Add the new validator to the default registry.
