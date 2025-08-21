# Fix SROA Materialization and Aggregate Reconstruction

**Priority**: MEDIUM  
**Component**: MIR SROA Pass  
**Impact**: Correctness, Performance

## Problem

The SROA (Scalar Replacement of Aggregates) pass has critical gaps in handling
aggregate reconstruction and materialization:

### Current Issues

1. **Missing materialization logic**: When SROA fails to scalarize completely,
   aggregates aren't properly reconstructed
2. **Partial scalarization handling**: No fallback when some fields can be
   scalarized but others cannot
3. **Complex aggregate patterns**: Nested structs and mixed aggregate types not
   handled
4. **Use-def chain corruption**: SROA can break SSA form when reconstruction
   fails

### Specific Problems in Current Code

**File**: `crates/compiler/mir/src/passes/sroa.rs`

```rust
// Current problematic pattern around line 180
if let Some(replacement) = self.try_scalarize_aggregate(aggregate_id, &aggregate_info) {
    // Scalarization successful, but what if it's partial?
    // No handling for mixed success/failure cases
} else {
    // Complete failure - aggregate left unchanged
    // But some fields might have been analyzed as scalarizable
}
```

### Impact

- **Silent correctness bugs**: Partially scalarized aggregates with broken
  references
- **Missed optimization opportunities**: Conservative fallback when partial
  scalarization would help
- **Performance regression**: Unnecessary aggregate operations when fields could
  be scalar

## Solution

### Enhanced SROA with Proper Materialization

**Update**: `crates/compiler/mir/src/passes/sroa.rs`

```rust
#[derive(Debug, Clone)]
pub enum ScalarizationResult {
    /// All fields successfully scalarized
    Complete(HashMap<FieldPath, ValueId>),
    /// Some fields scalarized, others need materialization
    Partial {
        scalar_fields: HashMap<FieldPath, ValueId>,
        materialized_fields: HashMap<FieldPath, ValueId>,
        reconstructed_aggregate: ValueId,
    },
    /// Cannot scalarize - leave aggregate unchanged
    Failed,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum FieldPath {
    TupleField(usize),
    StructField(String),
    Nested(Box<FieldPath>, Box<FieldPath>),
}

impl SROA {
    /// Enhanced scalarization with materialization support
    fn try_scalarize_with_materialization(
        &mut self,
        aggregate_id: ValueId,
        aggregate_info: &AggregateInfo,
        function: &mut MirFunction,
    ) -> ScalarizationResult {
        let mut scalar_fields = HashMap::new();
        let mut failed_fields = HashMap::new();

        // Analyze each field independently
        for (field_path, field_info) in &aggregate_info.field_analysis {
            if self.can_scalarize_field(field_info) {
                if let Some(scalar_value) = self.scalarize_field(
                    aggregate_id,
                    field_path,
                    field_info,
                    function
                ) {
                    scalar_fields.insert(field_path.clone(), scalar_value);
                } else {
                    failed_fields.insert(field_path.clone(), field_info.clone());
                }
            } else {
                failed_fields.insert(field_path.clone(), field_info.clone());
            }
        }

        if failed_fields.is_empty() {
            // Complete scalarization
            ScalarizationResult::Complete(scalar_fields)
        } else if scalar_fields.is_empty() {
            // No scalarization possible
            ScalarizationResult::Failed
        } else {
            // Partial scalarization - need materialization
            let materialized_fields = self.materialize_failed_fields(
                &failed_fields,
                function
            );

            let reconstructed = self.reconstruct_partial_aggregate(
                &scalar_fields,
                &materialized_fields,
                &aggregate_info.aggregate_type,
                function
            );

            ScalarizationResult::Partial {
                scalar_fields,
                materialized_fields,
                reconstructed_aggregate: reconstructed,
            }
        }
    }

    /// Reconstruct aggregate from mix of scalar and materialized fields
    fn reconstruct_partial_aggregate(
        &mut self,
        scalar_fields: &HashMap<FieldPath, ValueId>,
        materialized_fields: &HashMap<FieldPath, ValueId>,
        aggregate_type: &MirType,
        function: &mut MirFunction,
    ) -> ValueId {
        match aggregate_type {
            MirType::Tuple(field_types) => {
                self.reconstruct_tuple(scalar_fields, materialized_fields, field_types, function)
            }
            MirType::Struct(struct_info) => {
                self.reconstruct_struct(scalar_fields, materialized_fields, struct_info, function)
            }
            _ => panic!("Invalid aggregate type for reconstruction"),
        }
    }

    /// Reconstruct tuple from mix of sources
    fn reconstruct_tuple(
        &mut self,
        scalar_fields: &HashMap<FieldPath, ValueId>,
        materialized_fields: &HashMap<FieldPath, ValueId>,
        field_types: &[MirType],
        function: &mut MirFunction,
    ) -> ValueId {
        let mut field_values = Vec::new();

        for (index, field_type) in field_types.iter().enumerate() {
            let field_path = FieldPath::TupleField(index);

            let field_value = if let Some(&scalar_value) = scalar_fields.get(&field_path) {
                scalar_value
            } else if let Some(&materialized_value) = materialized_fields.get(&field_path) {
                materialized_value
            } else {
                // Create undefined value as fallback
                self.create_undefined_value(field_type, function)
            };

            field_values.push(field_value);
        }

        // Create make_tuple instruction
        let result_value = function.new_typed_value_id(aggregate_type.clone());
        let make_tuple_instr = Instruction::make_tuple(result_value, field_values);

        // Insert at appropriate location
        self.insert_reconstruction_instruction(make_tuple_instr, function);

        result_value
    }

    /// Handle materialization of fields that couldn't be scalarized
    fn materialize_failed_fields(
        &mut self,
        failed_fields: &HashMap<FieldPath, FieldInfo>,
        function: &mut MirFunction,
    ) -> HashMap<FieldPath, ValueId> {
        let mut materialized = HashMap::new();

        for (field_path, field_info) in failed_fields {
            // Extract field from original aggregate using appropriate method
            let materialized_value = match field_path {
                FieldPath::TupleField(index) => {
                    self.extract_tuple_field(field_info.aggregate_id, *index, function)
                }
                FieldPath::StructField(field_name) => {
                    self.extract_struct_field(field_info.aggregate_id, field_name, function)
                }
                FieldPath::Nested(_, _) => {
                    self.extract_nested_field(field_info.aggregate_id, field_path, function)
                }
            };

            materialized.insert(field_path.clone(), materialized_value);
        }

        materialized
    }

    /// Create extract instruction for tuple field
    fn extract_tuple_field(
        &mut self,
        tuple_id: ValueId,
        field_index: usize,
        function: &mut MirFunction,
    ) -> ValueId {
        let field_type = self.get_tuple_field_type(tuple_id, field_index, function);
        let result_value = function.new_typed_value_id(field_type);

        let extract_instr = Instruction::extract_tuple_field(
            result_value,
            Value::operand(tuple_id),
            field_index,
        );

        self.insert_reconstruction_instruction(extract_instr, function);
        result_value
    }

    /// Apply scalarization result to function
    fn apply_scalarization_result(
        &mut self,
        aggregate_id: ValueId,
        result: ScalarizationResult,
        function: &mut MirFunction,
    ) {
        match result {
            ScalarizationResult::Complete(scalar_fields) => {
                // Replace all uses of aggregate with scalar field accesses
                self.replace_aggregate_uses_with_scalars(aggregate_id, &scalar_fields, function);
                // Remove original aggregate definition
                self.remove_aggregate_definition(aggregate_id, function);
            }

            ScalarizationResult::Partial {
                scalar_fields,
                reconstructed_aggregate,
                ..
            } => {
                // Replace aggregate uses that can use scalars
                self.replace_partial_aggregate_uses(
                    aggregate_id,
                    &scalar_fields,
                    reconstructed_aggregate,
                    function
                );
                // Update aggregate definition to point to reconstructed version
                self.redirect_aggregate_definition(aggregate_id, reconstructed_aggregate, function);
            }

            ScalarizationResult::Failed => {
                // No changes needed - leave aggregate as-is
            }
        }
    }
}
```

### Enhanced Use-Def Analysis for Partial Scalarization

```rust
impl SROA {
    /// Analyze which uses can benefit from scalar fields vs need full aggregate
    fn analyze_use_compatibility(
        &self,
        aggregate_id: ValueId,
        function: &MirFunction,
    ) -> UseCompatibilityInfo {
        let mut field_uses = HashMap::new();
        let mut aggregate_uses = Vec::new();

        for use_site in self.find_all_uses(aggregate_id, function) {
            match self.classify_use(&use_site, function) {
                UseType::FieldAccess(field_path) => {
                    field_uses.entry(field_path).or_insert_with(Vec::new).push(use_site);
                }
                UseType::AggregateOperation => {
                    aggregate_uses.push(use_site);
                }
            }
        }

        UseCompatibilityInfo {
            field_uses,
            aggregate_uses,
            needs_reconstruction: !aggregate_uses.is_empty(),
        }
    }
}

#[derive(Debug)]
struct UseCompatibilityInfo {
    field_uses: HashMap<FieldPath, Vec<UseSite>>,
    aggregate_uses: Vec<UseSite>,
    needs_reconstruction: bool,
}

#[derive(Debug)]
enum UseType {
    FieldAccess(FieldPath),
    AggregateOperation,
}
```

### Validation and Testing

```rust
impl SROA {
    /// Validate SROA results maintain SSA form and type safety
    fn validate_scalarization_result(
        &self,
        result: &ScalarizationResult,
        function: &MirFunction,
    ) -> Result<(), String> {
        match result {
            ScalarizationResult::Complete(scalar_fields) => {
                // Verify all scalar fields have valid types and definitions
                for (field_path, &value_id) in scalar_fields {
                    if !function.is_value_defined(value_id) {
                        return Err(format!("Scalar field {:?} references undefined value", field_path));
                    }
                }
            }

            ScalarizationResult::Partial { reconstructed_aggregate, .. } => {
                // Verify reconstructed aggregate is properly defined
                if !function.is_value_defined(*reconstructed_aggregate) {
                    return Err("Reconstructed aggregate not properly defined".to_string());
                }
            }

            ScalarizationResult::Failed => {
                // No validation needed
            }
        }

        Ok(())
    }
}
```

## Files to Modify

- **Update**: `crates/compiler/mir/src/passes/sroa.rs` - Core SROA logic
- **Update**: `crates/compiler/mir/src/passes/sroa_tests.rs` - Enhanced test
  coverage
- **New**: `crates/compiler/mir/tests/sroa_materialization_tests.rs` -
  Materialization-specific tests

## Implementation Plan

### Phase 1: Materialization Infrastructure

1. Add ScalarizationResult enum and supporting types
2. Implement aggregate reconstruction methods
3. Add field extraction utilities

### Phase 2: Enhanced Analysis

1. Implement use compatibility analysis
2. Add partial scalarization decision logic
3. Enhance field access tracking

### Phase 3: Application Logic

1. Update main SROA loop to use new infrastructure
2. Add proper SSA maintenance
3. Implement validation checks

### Phase 4: Testing and Validation

1. Add comprehensive test cases for partial scalarization
2. Test nested aggregate scenarios
3. Validate SSA form preservation

## Test Strategy

```rust
#[test]
fn test_partial_scalarization() {
    // Create function with tuple where only some fields can be scalarized
    let mut function = create_test_function_partial_tuple();

    let mut sroa = SROA::new();
    let modified = sroa.run(&mut function);

    assert!(modified);
    // Verify partial scalarization occurred
    // Verify reconstructed aggregate is used where needed
}

#[test]
fn test_materialization_preserves_ssa() {
    let mut function = create_complex_aggregate_function();

    let mut sroa = SROA::new();
    sroa.run(&mut function);

    assert!(function.validate().is_ok());
    assert!(function.is_in_ssa_form());
}

#[test]
fn test_nested_aggregate_materialization() {
    // Test struct containing tuple, where struct can be scalarized but tuple cannot
    let mut function = create_nested_aggregate_function();

    let mut sroa = SROA::new();
    let modified = sroa.run(&mut function);

    assert!(modified);
    // Verify correct handling of nested aggregates
}
```

## Benefits

1. **Improved optimization coverage**: Handle cases where partial scalarization
   is beneficial
2. **Correctness**: Proper SSA form maintenance during complex transformations
3. **Performance**: Better optimization of mixed aggregate usage patterns
4. **Robustness**: Graceful fallback when full scalarization isn't possible

## Dependencies

- Should implement after core SROA pass is stable
- May interact with constant folding improvements

## Acceptance Criteria

- [ ] Partial scalarization with proper materialization
- [ ] Aggregate reconstruction maintains SSA form
- [ ] Nested aggregate support
- [ ] Comprehensive validation of transformations
- [ ] Use-def analysis for optimal scalarization decisions
- [ ] All existing SROA tests pass
- [ ] New tests cover materialization scenarios
- [ ] Performance neutral or improved on existing benchmarks
