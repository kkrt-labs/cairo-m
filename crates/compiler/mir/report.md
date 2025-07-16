# Current state of MIR crate

##

### 1. Current Bugs in Implementation

I've identified a bug (marked as TODO) in the current implementation.

#### Bug 1: Double Allocation for Aggregate Literals

- **Symptom:** As noted in the `//TODO` in `mir_generation_tests.rs`, when a
  struct or tuple literal is assigned to a variable with `let`, the compiler
  allocates memory twice. First for the literal itself, and then again for the
  variable, storing the address of the first allocation into the second.

- **Evidence:** The snapshot for `aggregates_struct_literal.snap` clearly shows
  this:

  ```mir
  // In test() from struct_literal.cm
  // let p = Point { x: 10, y: 20 };

  // First allocation (for the struct literal expression)
  %0 = stackalloc 2
  ... // field initialization on %0

  // Second allocation (for the 'let p' statement)
  %3 = stackalloc 2
  store %3, %0 // Stores the address of the first allocation into the second
  ```

  This is inefficient and incorrect. The variable `p` _is_ the struct on the
  stack; it should not be a pointer to another stack allocation.

- **Root Cause:** The issue stems from a disconnect between `lower_expression`
  and `lower_statement`:

  1.  `lower_expression` for `Expression::StructLiteral` and `Expression::Tuple`
      correctly performs a `stackalloc` to create the aggregate and returns its
      address (`Value::Operand(struct_addr)`).
  2.  `lower_statement` for `Statement::Let` unconditionally performs another
      `stackalloc` for the variable itself and then stores the value from the
      RHS into it.

- **Solution: Make `let` Smarter** The `lower_statement` for `let` bindings
  needs to be modified to avoid redundant allocations. When the right-hand side
  is an aggregate literal that already produces a stack address, the `let`
  statement should simply reuse that address.

  Modify `ir_generation.rs` in `lower_statement` for `Statement::Let`:

  ```rust
  // In MirBuilder::lower_statement
  Statement::Let { name, value, .. } => {
      let rhs_value = self.lower_expression(value)?;

      // ... resolve definition, get type, etc. ...
      let def_id = DefinitionId::new(self.db, self.file, def_idx);
      let mir_def_id = self.convert_definition_id(def_id);

      // --- PROPOSED CHANGE START ---

      // Check if the RHS is already a stack-allocated aggregate.
      // If so, we can bind the variable name directly to its address.
      if let Expression::StructLiteral { .. } | Expression::Tuple(_) = value.value() {
          if let Value::Operand(addr) = rhs_value {
              // The RHS expression already allocated the object and returned its address.
              // We just need to map the variable `name` to this address.
              self.definition_to_value.insert(mir_def_id, addr);
          } else {
              // This case should ideally not happen if aggregates always return addresses.
              // Handle as an error or fall back to old behavior.
              return Err("Expected an address from aggregate literal".to_string());
          }
      } else {
          // Original behavior for all other expression types (literals, binary ops, etc.)
          let semantic_type = definition_semantic_type(self.db, def_id);
          let var_type = MirType::from_semantic_type(self.db, semantic_type);

          let var_addr = self
              .mir_function
              .new_typed_value_id(MirType::pointer(var_type.clone()));
          self.add_instruction(Instruction::stack_alloc(var_addr, var_type.size_units()));
          self.add_instruction(Instruction::store(Value::operand(var_addr), rhs_value));
          self.definition_to_value.insert(mir_def_id, var_addr);
      }
      // --- PROPOSED CHANGE END ---

      Ok(())
  }
  ```

### 2. Assessment of Current Implementation

Overall, this is a very strong and well-architected start.

**Strengths:**

- **Excellent Structure:** The code is well-organized into modules
  (`ir_generation`, `module`, `function`, `instruction`, etc.), which clearly
  separates concerns.
- **Solid MIR Design:** The MIR data structures are well-thought-out, taking
  clear inspiration from LLVM (e.g., `BasicBlock`, `Instruction` vs
  `Terminator`, `getelementptr`). This is a proven and effective design.
- **Tight Integration with Semantics:** The generator correctly uses the
  semantic layer (`SemanticDb`) to resolve definitions and, most importantly, to
  drive type-aware MIR generation. The `MirType::from_semantic_type` function is
  a crucial and well-implemented piece of this.
- **Robust Testing:** The test harness is fantastic. Using `insta` for snapshots
  and custom `//!ASSERT` comments is a powerful way to ensure correctness and
  prevent regressions. This is the single biggest strength of the current
  codebase.
- **Good Coverage of Core Features:** You have already implemented a significant
  set of fundamental language features:
  - Functions, parameters, and returns.
  - `let` bindings and variable reassignment.
  - Binary expressions and operator precedence.
  - Full `if/else` control flow, including handling of branches that return.
  - Aggregate (struct/tuple) creation and member access (both for reading and
    writing).

**Weaknesses/Current State:**

- **Bugs:** The bugs listed above are the most significant weaknesses.
- **Incomplete Features:** As expected at this stage, many language features are
  missing (loops, enums, `match`, etc.).
- **Placeholder Pointer/Array Model:** The tests correctly note that `felt*` is
  a placeholder. A more formal model for pointers and arrays/slices is needed.
- **Basic Optimization Passes:** The `passes` module exists, which is great, but
  `DeadCodeElimination` is very basic and `Validation` is minimal.

---

### 3. Step-by-Step Plan for Next Steps (No Loops)

Here is a logical, incremental plan to build on the current foundation, focusing
on expanding language features without touching loops.

#### **Phase 1: Stabilization and Refinement**

1.  **Fix Critical Bugs:** Implement the solutions for the **Double Allocation**
    detailed in section 1. Add the new test cases to lock in the fixes.
2.  **Refine Pointer & Array Types:**
    - Update `array_access.cm` to use the `MirType::Pointer` properly.
    - In the semantic model, introduce a real array type (e.g., `[felt; 3]`) or
      a slice type.
    - Update `MirType` and `ir_generation.rs` to handle these new types. For a
      fixed-size array, `stackalloc` size will be known. `getelementptr` will
      need to be used with a scaling factor for the element size if the index is
      not constant.

#### **Phase 2: Expanding Expression and Statement Capabilities**

3.  **Implement Unary Operators:**
    - Support operators like negation (`-`) and logical not (`!`).
    - This will likely require adding a `UnaryOp` variant to `InstructionKind`
      and handling it in `lower_expression`.

#### **Phase 3: Introducing Enums and Pattern Matching**

#### **Phase 4: Improving the Compiler Backend**

7.  **Enhance MIR Passes:**
    - **Constant Folding:** Implement a pass to evaluate constant expressions.
      For example, if an instruction is `add %1, %2` and the values for `%1` and
      `%2` are known constants, replace the instruction with an assignment of
      the result.
    - **Dead Instruction Elimination:** Implement a pass to remove instructions
      whose results are never used (and that have no side effects). This
      requires simple dataflow analysis to track value usage.

By following this plan, you will systematically build upon your strong
foundation, stabilize the existing code, and add powerful new language features
in a logical order.
