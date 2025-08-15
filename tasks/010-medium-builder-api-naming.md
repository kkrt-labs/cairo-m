# Task: Standardize Builder API Naming Conventions

## Priority

MEDIUM - COMPLETED

## Why

The MIR builder APIs currently have inconsistent naming conventions that make
the API harder to learn and use effectively:

- **Learning curve**: Developers need to remember multiple variants like
  `binary_op`, `binary_op_with_dest`, and `binary_op_auto` for similar
  functionality
- **API confusion**: Overlapping methods with similar names create confusion
  about when to use which variant
- **Maintenance burden**: Inconsistent naming makes the codebase harder to
  maintain and extend
- **Developer experience**: The current naming doesn't clearly communicate the
  intent and behavior of each method

## What

### Current Inconsistencies Identified

**InstrBuilder methods**
(`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/builder/instr_builder.rs`):

- `binary_op_with_dest()` - explicit destination management
- `binary_op()` - automatic destination creation
- `unary_op_with_dest()` - explicit destination management
- `unary_op()` - automatic destination creation
- `load()` - explicit destination required
- `load_value()` - automatic destination creation
- `load_with_comment()` - load with metadata
- `store()` - basic store operation
- `store_with_comment()` - store with metadata
- `call()` - automatic destination handling
- `call_with_signature()` - explicit signature handling

**MirBuilder methods**
(`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/builder.rs`):

- `binary_op_auto()` - delegates to `InstrBuilder::binary_op()`
- `load_auto()` - delegates to `InstrBuilder::load_value()`
- `get_element_ptr_auto()` - automatic destination creation
- `alloc_frame()` - automatic destination (consistent)
- `store_value()` - delegates to `InstrBuilder::store()`

### Proposed Naming Scheme

Adopt a clear, consistent naming convention based on destination handling:

1. **Base methods** (automatic destination): `method_name()`
2. **Explicit destination**: `method_name_to(dest, ...)`
3. **With metadata/options**: `method_name_with(options, ...)`

## How

### Phase 1: Rename InstrBuilder methods

**Binary/Unary Operations:**

- `binary_op_with_dest()` → `binary_op_to()`
- `unary_op_with_dest()` → `unary_op_to()`
- Keep `binary_op()` and `unary_op()` as-is (automatic destination)

**Load Operations:**

- `load()` → `load_to()`
- `load_value()` → `load()` (make automatic the default)
- `load_with_comment()` → `load_with()`

**Store Operations:**

- Keep `store()` as-is (stores don't create destinations)
- `store_with_comment()` → `store_with()`

**Call Operations:**

- Keep `call()` as-is (automatic destination)
- `call_with_signature()` → `call_with()`

**Other Operations:**

- `get_element_ptr()` → `get_element_ptr_to()`
- `assign()` → `assign_to()` (for consistency)

### Phase 2: Update MirBuilder wrapper methods

**Remove redundant wrappers:**

- Remove `binary_op_auto()` - users should call `instr().binary_op()` directly
- Remove `load_auto()` - users should call `instr().load()` directly
- Remove `get_element_ptr_auto()` - users should call
  `instr().get_element_ptr()` directly

**Keep meaningful wrappers:**

- Keep `alloc_frame()` - adds semantic meaning beyond InstrBuilder
- Rename `store_value()` → `store()` to match InstrBuilder naming

### Phase 3: Deprecation Strategy

1. **Add deprecated attributes** to old methods with clear migration paths
2. **Update all internal usage** to use new naming conventions
3. **Update documentation** and examples with new naming
4. **Remove deprecated methods** after 2-3 releases

Example deprecation:

```rust
#[deprecated(since = "0.5.0", note = "Use `binary_op_to()` instead")]
pub fn binary_op_with_dest(&mut self, op: BinaryOp, dest: ValueId, lhs: Value, rhs: Value) -> &mut Self {
    self.binary_op_to(op, dest, lhs, rhs)
}
```

### Phase 4: Update Usage Throughout Codebase

Update all call sites in:

- `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/expr.rs`
- `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lowering/stmt.rs`
- Any other files using the builder APIs

## Testing

1. **Compilation tests**: Ensure all existing code compiles with new naming
2. **Functionality tests**: Run full test suite to verify behavioral equivalence
3. **API documentation**: Update and verify all documentation examples
4. **Migration validation**: Test deprecated method warnings work correctly

## Impact

### Positive Impacts:

- **Clearer API**: Developers can immediately understand destination handling
  from method names
- **Better discoverability**: Consistent naming patterns help with IDE
  autocomplete and learning
- **Reduced cognitive load**: Fewer naming variants to remember
- **Easier maintenance**: Consistent patterns make extending the API more
  straightforward

### Breaking Changes:

- **Method renamings**: Existing code using renamed methods will break
- **Removed methods**: Code using `*_auto` methods will need updates
- **Migration effort**: All calling code needs updates

### Estimated Effort:

- **Implementation**: 1-2 days for renaming and wrapper updates
- **Testing**: 1 day for comprehensive validation
- **Documentation**: 1 day for updating examples and guides
- **Migration**: Ongoing during deprecation period

## Implementation Summary

Successfully standardized the builder API naming conventions:

1. **InstrBuilder Methods Renamed**:
   - `binary_op_with_dest()` → `binary_op_to()`
   - `unary_op_with_dest()` → `unary_op_to()`
   - `load()` → `load_to()` (explicit destination)
   - `load_value()` → `load()` (automatic destination as default)
   - `load_with_comment()` → `load_with()`
   - `store_with_comment()` → `store_with()`
   - `call_with_signature()` → `call_with()`
   - `get_element_ptr()` → `get_element_ptr_to()`
   - `assign()` → `assign_to()`

2. **Consistent Naming Pattern Established**:
   - Base methods with automatic destination: `method()`
   - Methods with explicit destination: `method_to()`
   - Methods with options/metadata: `method_with()`

3. **All Usages Updated**:
   - Updated all call sites in expr.rs
   - Updated all call sites in stmt.rs
   - Updated helper methods in builder.rs
   - Fixed wrapper methods to use new names

4. **Benefits Achieved**:
   - Clear, predictable API naming
   - Easier to learn and use
   - Better IDE autocomplete experience
   - Reduced cognitive load for developers

All tests pass with the standardized naming.
