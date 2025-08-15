# Task: Fix Misleading Basic Block Naming API

## Priority

HIGH

## Status

✅ COMPLETED

## Why

The current `add_basic_block_with_name` function in `MirFunction` is misleading
and harmful to the debugging experience:

1. **API Contract Violation**: The function signature suggests it accepts and
   uses a name parameter, but the implementation completely ignores it (prefixed
   with underscore `_name`), misleading developers about the function's
   behavior.

2. **Lost Debug Information**: Names provided for basic blocks (like "then",
   "else", "loop_header", "loop_body") are valuable debugging information that
   gets discarded, making MIR debugging significantly harder.

3. **Inconsistent Developer Experience**: The `cfg_builder.rs` carefully
   constructs meaningful block names like "then", "else", "merge",
   "loop_header", etc., expecting them to be preserved, but they're silently
   ignored.

4. **Code Maintenance Issues**: The current approach creates confusion for
   anyone reading or maintaining the code, as the function name and signature
   don't match its behavior.

## What

The issue is in
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/function.rs` at lines
95-101:

```rust
/// Adds a new basic block with a name and returns its ID
pub fn add_basic_block_with_name(&mut self, _name: String) -> BasicBlockId {
    let block = BasicBlock::new();
    // Store the name as a comment or label if we want to preserve it for debugging
    // For now, we just create the block
    self.basic_blocks.push(block)
}
```

The `BasicBlock` struct in
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/basic_block.rs`
currently has no `name` field, which explains why the name parameter is ignored.

There are two possible solutions:

### Solution A: Add Name Field to BasicBlock (Preferred)

Add an optional `name` field to `BasicBlock` struct to preserve debug
information.

### Solution B: Remove Misleading API

Remove `add_basic_block_with_name` and update callers to use `add_basic_block()`
directly.

**Solution A is preferred** because it provides better debugging experience and
maintains the semantic intent of the CFG builder's naming scheme.

## How

### Implementation Steps for Solution A (Preferred):

1. **Modify BasicBlock Structure** in
   `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/basic_block.rs`:

   ```rust
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub struct BasicBlock {
       /// Optional name for debugging purposes
       pub name: Option<String>,

       /// The sequence of instructions in this block
       pub instructions: Vec<Instruction>,

       /// The terminator that ends this block
       pub terminator: Terminator,
   }
   ```

2. **Update BasicBlock Constructor**:

   ```rust
   pub const fn new() -> Self {
       Self {
           name: None,
           instructions: Vec::new(),
           terminator: Terminator::Unreachable,
       }
   }

   pub const fn with_name(name: String) -> Self {
       Self {
           name: Some(name),
           instructions: Vec::new(),
           terminator: Terminator::Unreachable,
       }
   }
   ```

3. **Fix add_basic_block_with_name** in
   `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/function.rs`:

   ```rust
   /// Adds a new basic block with a name and returns its ID
   pub fn add_basic_block_with_name(&mut self, name: String) -> BasicBlockId {
       let block = BasicBlock::with_name(name);
       self.basic_blocks.push(block)
   }
   ```

4. **Update PrettyPrint Implementation** in `BasicBlock`:

   ```rust
   impl PrettyPrint for BasicBlock {
       fn pretty_print(&self, indent: usize) -> String {
           let mut result = String::new();
           let base_indent = indent_str(indent);

           // Print block name if available
           if let Some(ref name) = self.name {
               result.push_str(&format!("{}; {}\n", base_indent, name));
           }

           // ... rest of implementation
       }
   }
   ```

5. **Update MirFunction pretty printing** to show block names:
   ```rust
   // In pretty_print method, around line 299-301
   for (block_id, block) in self.basic_blocks() {
       let block_display = if let Some(ref name) = block.name {
           format!("{block_id:?} ({name})")
       } else {
           format!("{block_id:?}")
       };
       result.push_str(&format!("{base_indent}  {block_display}:\n"));
       result.push_str(&block.pretty_print(indent + 2));
       result.push('\n');
   }
   ```

### Alternative Solution B Implementation:

If Solution A is deemed too invasive:

1. **Remove add_basic_block_with_name** from `MirFunction`
2. **Update CfgBuilder** in
   `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/builder/cfg_builder.rs`
   line 53:
   ```rust
   pub fn new_block(&mut self, _name: Option<String>) -> BasicBlockId {
       // Ignore the name parameter since BasicBlock doesn't support names
       self.function.add_basic_block()
   }
   ```
3. **Update all callers** to use `add_basic_block()` directly

## Testing

1. **Unit Tests**: Verify that block names are properly stored and retrieved:

   ```rust
   #[test]
   fn test_basic_block_naming() {
       let mut function = MirFunction::new("test".to_string());
       let block_id = function.add_basic_block_with_name("test_block".to_string());
       let block = function.get_basic_block(block_id).unwrap();
       assert_eq!(block.name, Some("test_block".to_string()));
   }
   ```

2. **Integration Tests**: Verify that CFG builder names are preserved in MIR
   output:

   ```rust
   #[test]
   fn test_cfg_builder_preserves_names() {
       let mut function = MirFunction::new("test".to_string());
       let mut builder = CfgBuilder::new(&mut function, function.entry_block);
       let (then_block, else_block, merge_block) = builder.create_if_blocks();

       assert_eq!(function.get_basic_block(then_block).unwrap().name, Some("then".to_string()));
       assert_eq!(function.get_basic_block(else_block).unwrap().name, Some("else".to_string()));
       assert_eq!(function.get_basic_block(merge_block).unwrap().name, Some("merge".to_string()));
   }
   ```

3. **Pretty Print Tests**: Verify that block names appear in debug output.

4. **Regression Tests**: Ensure existing MIR compilation tests continue to pass.

## Impact

### Positive Improvements:

1. **Enhanced Debugging Experience**: MIR dumps will show meaningful block names
   like "then", "else", "loop_header" instead of just numeric IDs, making
   control flow analysis much easier.

2. **API Consistency**: The function name and behavior will finally match,
   eliminating confusion for developers working with the MIR API.

3. **Preserved Semantic Information**: The careful naming scheme in CFG builder
   will be preserved, maintaining the developer's intent about control flow
   structure.

4. **Better Development Workflow**: Debugging MIR optimizations and codegen will
   be significantly easier with named blocks.

### Minimal Risk:

1. **Backward Compatibility**: The change is additive - existing code using
   `add_basic_block()` continues to work unchanged.

2. **Memory Overhead**: Minimal - just an optional String per basic block, only
   allocated when names are provided.

3. **Performance Impact**: Negligible - no impact on compilation or runtime
   performance, only affects debug output generation.

The fix addresses a clear API design flaw that has been causing confusion and
hampering debugging capabilities, while providing substantial benefits for MIR
development and maintenance.

## Implementation Summary

### Changes Made

- Added optional `name` field to `BasicBlock` struct
- Added `with_name` constructor to create named basic blocks
- Updated `add_basic_block_with_name` to properly use the name parameter
- Modified pretty printing to display block names in both BasicBlock and
  MirFunction output
- Updated critical edge splitting to generate descriptive edge block names
- Fixed tests that create BasicBlock structs directly

### Testing Results

- ✅ All 54 MIR unit tests pass
- ✅ All integration tests pass
- ✅ Snapshot tests updated to reflect named blocks in output
- ✅ No regressions in functionality

### Impact

The implementation provides:

- **Better debugging**: Block names now appear in MIR output making control flow
  easier to understand
- **API consistency**: Function signature now matches its actual behavior
- **Preserved semantic intent**: CFG builder's careful naming scheme is now
  utilized
- **Enhanced developer experience**: Debugging MIR optimizations and codegen is
  significantly easier
