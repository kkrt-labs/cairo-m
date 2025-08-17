# Task 008: Clean Up TODO Comments and Minor Issues [LOW PRIORITY]

## Priority: LOW - Code cleanup and maintainability

## Summary

Address various TODO comments and minor issues throughout the MIR crate to
improve code quality and maintainability.

## TODO Items to Address

### 1. Array Index Support

**Location:** `crates/compiler/mir/src/instruction.rs:20`

```rust
// TODO: Add array index support when arrays are implemented
```

**Action:**

- Wait for full array implementation design
- Add index bounds checking in validation pass
- Consider adding `ArrayIndex` instruction variant

### 2. Type System Integration for Cast

**Location:** `crates/compiler/mir/src/instruction.rs:308`

```rust
Cast {
    value: ValueId,
    // target_type: TypeId, // TODO: Add when type system is integrated
},
```

**Action:**

- Add `target_type` field to Cast instruction
- Update validation to check cast validity
- Update pretty-printing to show target type

### 3. Statement Lowering Optimization

**Location:** `crates/compiler/mir/src/lowering/stmt.rs:82`

```rust
// TODO: eventually, this will need to be optimized in a better way.
```

**Context:** Related to return value handling **Action:**

- Profile to identify actual bottlenecks
- Consider caching or memoization if needed
- Document what specific optimization is needed

### 4. Mem2Reg Enhancement

**Location:** `crates/compiler/mir/src/passes/mem2reg_ssa.rs:143`

```rust
// TODO: Implement SROA or per-slot phi insertion for full support
```

**Action:**

- After aggregate-first transition, evaluate if still needed
- If needed for arrays, implement per-slot tracking
- Otherwise, remove TODO and document decision

### 5. Pre-Optimization Alias Analysis

**Location:** `crates/compiler/mir/src/passes/pre_opt.rs:217`

```rust
// TODO: Enhance with alias analysis to handle GEP-derived pointers more aggressively
```

**Action:** See Task 007 for full implementation plan

## Additional Cleanup Tasks

### Remove Commented Code

Search for and remove:

- Old implementations left as comments
- Debugging code that's commented out
- Alternative approaches that weren't chosen

### Standardize Error Messages

Ensure all error messages:

- Include helpful context
- Suggest fixes where possible
- Use consistent formatting

### Update Internal Documentation

- Add module-level documentation where missing
- Update outdated comments
- Add examples for complex functions

## Implementation Approach

### Phase 1: Document TODOs

For each TODO:

1. Assess if still relevant
2. Document why it exists
3. Create issue if it's substantial work
4. Remove if obsolete

### Phase 2: Code Cleanup

```bash
# Find all TODOs
rg "TODO|FIXME|XXX|HACK" crates/compiler/mir/ --type rust

# Find commented code blocks
rg "^\\s*//.*{" crates/compiler/mir/ --type rust

# Find long comment blocks that might be old code
rg "^\\s*//" crates/compiler/mir/ --type rust | awk 'length > 100'
```

### Phase 3: Documentation Pass

For each module:

```rust
//! Module-level documentation
//!
//! ## Purpose
//! Describe what this module does
//!
//! ## Design Decisions
//! Key architectural choices
//!
//! ## Examples
//! Usage examples if applicable
```

## Testing Requirements

### Validation Tests

```rust
#[test]
fn test_all_todos_documented() {
    // Scan for TODOs and ensure they have issue numbers
}
```

### Documentation Tests

````rust
/// Example usage:
/// ```
/// let pass = PreOptPass::new();
/// pass.run_on_function(&mut func);
/// ```
````

## Success Criteria

1. All TODOs either:
   - Resolved and removed
   - Documented with issue number
   - Marked as "won't fix" with explanation

2. No commented-out code blocks

3. All public APIs have documentation

4. Error messages are helpful and consistent

5. Module-level documentation exists for all modules

## Priority Order

1. **High Value, Low Effort**
   - Remove obsolete TODOs
   - Delete commented code
   - Fix typos

2. **Medium Value, Medium Effort**
   - Add missing documentation
   - Standardize error messages
   - Update outdated comments

3. **Low Value or High Effort**
   - Create issues for tracking
   - Defer to future work

## Tracking

Create GitHub issues for any TODOs that represent substantial work:

```markdown
Title: [MIR] Implement array index support Labels: enhancement, mir Body:

- Found in: instruction.rs:20
- Blocked by: Array design finalization
- Impact: Enables array indexing in MIR
```
