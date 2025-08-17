# Task 005: Remove Deprecated Memory-Based Aggregate APIs [HIGH PRIORITY]

## Priority: HIGH - Clean up technical debt

## Summary

Complete the migration from memory-based to value-based aggregate operations by
removing all usage of deprecated APIs and then deleting the deprecated methods
themselves.

## Current State

- ⚠️ Deprecated methods still actively used in 2 locations
- ⚠️ Deprecated methods still defined in builder
- ❌ Migration incomplete for return statements and patterns

## Deprecated APIs to Remove

### Methods in `crates/compiler/mir/src/lowering/builder.rs`

1. `load_field()` (lines ~516-540)
2. `store_field()` (lines ~564-590)
3. `load_tuple_element()` (lines ~603-630)
4. `store_tuple_element()` (lines ~650-675)

### Active Usage Locations

#### 1. `crates/compiler/mir/src/lowering/stmt.rs:197`

```rust
// Current (WRONG):
let element_value = self.load_tuple_element(tuple_addr, i, element_type);

// Should be:
let element_value = self.extract_tuple_element(tuple_value, i);
```

#### 2. `crates/compiler/mir/src/lowering/stmt.rs:690-694`

```rust
// Current (WRONG):
let element_value = self.load_tuple_element(tuple_addr, index, element_type);

// Should be:
let element_value = self.extract_tuple_element(tuple_value, index);
```

## Implementation Steps

### Phase 1: Fix Active Usage

#### Step 1: Fix Return Statement (stmt.rs:189-197)

```rust
// Replace memory-based tuple element loading
match &return_value {
    Value::Aggregate(AggregatePath::Tuple(tuple_value)) => {
        // Use value-based extraction
        for i in 0..tuple_size {
            let element = self.extract_tuple_element(*tuple_value, i);
            ret_vals.push(element);
        }
    }
    // ... other cases
}
```

#### Step 2: Fix Pattern Destructuring (stmt.rs:690-694)

```rust
// Replace memory-based loading in lower_pattern
PatternKind::Tuple(patterns) => {
    // Get tuple as value, not address
    let tuple_value = self.resolve_to_value(init_value)?;

    for (i, pattern) in patterns.iter().enumerate() {
        // Use value-based extraction
        let element = self.extract_tuple_element(tuple_value, i);
        self.lower_pattern(pattern, Value::operand(element))?;
    }
}
```

### Phase 2: Remove Deprecated Methods

#### Step 1: Delete from `lowering/builder.rs`

Remove these entire method implementations:

```rust
#[deprecated(note = "Use extract_struct_field for value-based access")]
pub fn load_field(...) { ... }

#[deprecated(note = "Use make_struct or insert_field")]
pub fn store_field(...) { ... }

#[deprecated(note = "Use extract_tuple_element for value-based access")]
pub fn load_tuple_element(...) { ... }

#[deprecated(note = "Use make_tuple or insert_tuple")]
pub fn store_tuple_element(...) { ... }
```

#### Step 2: Clean Builder Interface

In `crates/compiler/mir/src/builder/instr_builder.rs`:

- Remove `load_field()` (line 169)
- Remove `store_field()` (line 202)
- Ensure only value-based methods remain

### Phase 3: Update Tests

Update any tests that rely on deprecated methods:

```rust
// Before:
context.load_tuple_element(addr, 0, ty);

// After:
context.extract_tuple_element(value, 0);
```

## Testing Requirements

### Unit Tests

```rust
#[test]
fn test_return_uses_extract_not_load() {
    // Verify return statements use ExtractTupleElement
}

#[test]
fn test_pattern_uses_extract_not_load() {
    // Verify pattern matching uses ExtractTupleElement
}

#[test]
fn test_no_deprecated_api_usage() {
    // Compile-time test: deprecated methods don't exist
}
```

### Integration Tests

- Run full test suite after removal
- Verify no regressions in tuple/struct handling
- Check optimization passes still work

## Verification Checklist

- [ ] No usage of `load_tuple_element` in codebase
- [ ] No usage of `store_tuple_element` in codebase
- [ ] No usage of `load_field` in codebase
- [ ] No usage of `store_field` in codebase
- [ ] Deprecated methods deleted from builder.rs
- [ ] All tests updated and passing
- [ ] No compilation warnings about deprecated usage

## Migration Guide Updates

After completion, update `MIGRATION_GUIDE.md`:

```markdown
## Deprecated API Removal Complete

The following memory-based APIs have been removed:

- `load_field()` → Use `extract_struct_field()`
- `store_field()` → Use `insert_field()` or `make_struct()`
- `load_tuple_element()` → Use `extract_tuple_element()`
- `store_tuple_element()` → Use `insert_tuple()` or `make_tuple()`

All aggregate operations now use value-based semantics.
```

## Search Commands for Verification

```bash
# Find any remaining usage
rg "load_field|store_field|load_tuple_element|store_tuple_element" \
   --type rust \
   crates/compiler/mir/

# Check for deprecated attributes
rg "#\[deprecated" crates/compiler/mir/src/lowering/builder.rs
```

## Risk Assessment

- **Low Risk**: Changes are localized to 2 files
- **Testing**: Comprehensive test suite will catch issues
- **Fallback**: Git history preserves old implementation

## Success Criteria

1. Zero usage of deprecated APIs in codebase
2. Deprecated methods completely removed
3. All tests pass with value-based operations
4. No performance regression
5. Clean compilation with no deprecation warnings
