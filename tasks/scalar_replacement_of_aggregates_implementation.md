# Scalar Replacement of Aggregates (SROA) with Phi Optimization

## Overview

This document describes a sophisticated Scalar Replacement of Aggregates (SROA)
implementation that handles complex control flow scenarios, particularly
focusing on optimal phi node handling. The key insight is that **we only need to
materialize aggregate fields that are actually used**, even when aggregates flow
through phi nodes from different control flow paths.

## Core Problem Statement

### The Challenge

```mir
block1: %t1 = make_tuple(a, b, c, d)  // 4-field tuple jump block3

block2: %t2 = make_tuple(x, y, z, w)  // 4-field tuple jump block3

block3:
  %phi = phi [%t1, block1], [%t2, block2] %result = extract_tuple(%phi, 0)  //
  Only field 0 is used! // Fields 1, 2, 3 are never accessed
```

**Naive approach**: Create full aggregate phi, then extract field 0 **Optimal
approach**: Only create phi for field 0, eliminate unused fields entirely

### The Opportunity

Even when aggregates flow through complex control flow with phi nodes, we can:

1. **Eliminate unused fields** completely (no phi creation for unused fields)
2. **Scalarize necessary fields** into individual scalar phis
3. **Delay materialization** until aggregates truly escape (function calls)
4. **Handle partial updates** efficiently through SSA

## Algorithm Strategy

### Phase 1: Aggregate Flow Analysis

Build complete def-use chains for all aggregate values through the control flow
graph.

### Phase 2: Demand Analysis

Determine which fields of each aggregate are actually needed by transitively
analyzing all uses.

### Phase 3: Phi Classification

Classify each phi node based on how its result is used:

- **Scalarizable**: Only used for field extractions
- **Materializable**: Escapes to function calls
- **Mixed**: Both field extractions and escaping uses

### Phase 4: Phi Scalarization

Replace aggregate phis with per-field scalar phis, creating only the fields that
are demanded.

### Phase 5: Materialization Insertion

Insert aggregate reconstruction only at true escape points.

### Phase 6: Cleanup

Remove unused aggregate construction and extraction instructions.

## Detailed Algorithm

### Data Structures

```rust
/// Tracks the virtual state of an aggregate value
#[derive(Debug, Clone)]
struct AggregateInfo {
    /// The aggregate ValueId this info is for
    aggregate_id: ValueId,
    /// Maps field index to the SSA value for that field
    fields: HashMap<usize, ValueId>,
    /// The type of the aggregate (tuple/struct)
    aggregate_type: MirType,
}

/// Tracks which fields are actually demanded for each aggregate
#[derive(Debug, Default)]
struct FieldDemand {
    /// Set of field indices that are extracted/used
    demanded_fields: HashSet<usize>,
    /// Whether the full aggregate escapes (needs materialization)
    escapes: bool,
}

/// Represents the scalarization of an aggregate phi
#[derive(Debug)]
struct PhiScalarization {
    /// Original aggregate phi
    original_phi: ValueId,
    /// Map from field index to the scalar phi for that field
    scalar_phis: HashMap<usize, ValueId>,
    /// Block where the phi resides
    block_id: BasicBlockId,
}

/// Points where aggregate materialization is required
#[derive(Debug)]
struct MaterializationPoint {
    aggregate: ValueId,
    location: (BasicBlockId, usize),
    reason: MaterializationReason,
    /// Which fields need to be materialized (may be subset)
    required_fields: HashSet<usize>,
}

#[derive(Debug)]
enum MaterializationReason {
    FunctionCall,
    Return,
    ArrayElement,
    GlobalStore,
}
```

### Algorithm Implementation

```rust
impl ScalarReplacementOfAggregates {
    pub fn run(&mut self, function: &mut MirFunction) -> bool {
        // Phase 1: Build aggregate flow analysis
        let flow_info = self.analyze_aggregate_flow(function);

        // Phase 2: Analyze field demand
        let demand_analysis = self.analyze_field_demand(function, &flow_info);

        // Phase 3: Classify and plan phi handling
        let phi_plan = self.plan_phi_scalarization(function, &flow_info, &demand_analysis);

        // Phase 4: Execute phi scalarization
        let scalar_mappings = self.scalarize_phis(function, &phi_plan);

        // Phase 5: Replace extractions with scalar uses
        self.replace_extractions(function, &flow_info, &scalar_mappings);

        // Phase 6: Insert materializations
        let materialization_points = self.find_materialization_points(function, &demand_analysis);
        self.insert_materializations(function, &materialization_points, &flow_info);

        // Phase 7: Cleanup unused instructions
        self.cleanup_unused_instructions(function);

        true
    }
}
```

## Phase 2: Demand Analysis (Critical)

This is the most crucial phase - we need to determine which fields are actually
used.

```rust
fn analyze_field_demand(
    &self,
    function: &MirFunction,
    flow_info: &AggregateFlowInfo
) -> HashMap<ValueId, FieldDemand> {
    let mut demands = HashMap::new();
    let mut worklist = VecDeque::new();

    // Initialize with direct field extractions
    for (block_id, block) in function.basic_blocks() {
        for instr in &block.instructions {
            match &instr.kind {
                InstructionKind::ExtractTupleElement { tuple, index, dest, .. } => {
                    if let Value::Operand(tuple_id) = tuple {
                        demands.entry(*tuple_id).or_insert_with(FieldDemand::default)
                            .demanded_fields.insert(*index);
                        worklist.push_back(*tuple_id);
                    }
                }
                InstructionKind::ExtractField { object, field_index, dest, .. } => {
                    if let Value::Operand(obj_id) = object {
                        demands.entry(*obj_id).or_insert_with(FieldDemand::default)
                            .demanded_fields.insert(*field_index);
                        worklist.push_back(*obj_id);
                    }
                }
                // Function calls mark full aggregate as escaping
                InstructionKind::Call { args, .. } => {
                    for arg in args {
                        if let Value::Operand(arg_id) = arg {
                            if flow_info.is_aggregate(*arg_id) {
                                demands.entry(*arg_id).or_insert_with(FieldDemand::default)
                                    .escapes = true;
                                worklist.push_back(*arg_id);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Check terminator (return values)
        match &block.terminator {
            Terminator::Return { value: Some(Value::Operand(ret_id)) } => {
                if flow_info.is_aggregate(*ret_id) {
                    demands.entry(*ret_id).or_insert_with(FieldDemand::default)
                        .escapes = true;
                    worklist.push_back(*ret_id);
                }
            }
            _ => {}
        }
    }

    // Propagate demands backwards through def-use chains
    while let Some(agg_id) = worklist.pop_front() {
        let current_demand = demands.get(&agg_id).cloned()
            .unwrap_or_default();

        // Find the definition of this aggregate
        if let Some(def_instr) = flow_info.get_definition(agg_id) {
            match &def_instr.kind {
                // Phi nodes: propagate demand to all operands
                InstructionKind::Phi { sources, .. } => {
                    for (_, source_val) in sources {
                        if let Value::Operand(source_id) = source_val {
                            let source_demand = demands.entry(*source_id)
                                .or_insert_with(FieldDemand::default);

                            let mut changed = false;

                            // Propagate demanded fields
                            for &field in &current_demand.demanded_fields {
                                if source_demand.demanded_fields.insert(field) {
                                    changed = true;
                                }
                            }

                            // Propagate escape status
                            if current_demand.escapes && !source_demand.escapes {
                                source_demand.escapes = true;
                                changed = true;
                            }

                            if changed {
                                worklist.push_back(*source_id);
                            }
                        }
                    }
                }

                // Insert operations: specific field demand propagation
                InstructionKind::InsertTupleElement { tuple, index, .. } => {
                    if let Value::Operand(base_id) = tuple {
                        let base_demand = demands.entry(*base_id)
                            .or_insert_with(FieldDemand::default);

                        let mut changed = false;

                        // All fields except the inserted one come from base
                        for &field in &current_demand.demanded_fields {
                            if field != *index && base_demand.demanded_fields.insert(field) {
                                changed = true;
                            }
                        }

                        // Escape propagates fully
                        if current_demand.escapes && !base_demand.escapes {
                            base_demand.escapes = true;
                            changed = true;
                        }

                        if changed {
                            worklist.push_back(*base_id);
                        }
                    }
                }

                _ => {}
            }
        }
    }

    demands
}
```

## Phase 4: Phi Scalarization (Complex)

This phase handles the decomposition of aggregate phis into scalar phis.

```rust
fn scalarize_phis(
    &mut self,
    function: &mut MirFunction,
    phi_plan: &PhiScalarizationPlan
) -> HashMap<ValueId, HashMap<usize, ValueId>> {
    let mut scalar_mappings = HashMap::new();

    for scalarization in &phi_plan.scalarizations {
        let mut field_phis = HashMap::new();

        // Get the original phi instruction
        let original_phi = self.get_phi_instruction(function, scalarization.original_phi)
            .expect("Original phi must exist");

        // For each demanded field, create a scalar phi
        for &field_index in &scalarization.demanded_fields {
            let scalar_phi_id = function.new_typed_value_id(
                self.get_field_type(&original_phi.dest_type, field_index)
            );

            // Build operands for the scalar phi
            let mut scalar_operands = Vec::new();
            for (pred_block, operand_val) in &original_phi.sources {
                let scalar_operand = match operand_val {
                    Value::Operand(operand_id) => {
                        // Get the scalar value for this field from the operand aggregate
                        self.get_scalar_field(*operand_id, field_index, function)
                    }
                    Value::Literal(_) => {
                        // Extract field from literal aggregate
                        self.extract_field_from_literal(operand_val, field_index)
                    }
                };
                scalar_operands.push((*pred_block, scalar_operand));
            }

            // Create the scalar phi instruction
            let scalar_phi_instr = Instruction::phi(
                scalar_phi_id,
                scalar_operands,
                self.get_field_type(&original_phi.dest_type, field_index)
            );

            // Insert the scalar phi in the same block as the original
            let block = function.get_basic_block_mut(scalarization.block_id).unwrap();

            // Find insertion point (before first non-phi instruction)
            let insert_pos = block.instructions.iter()
                .position(|instr| !matches!(instr.kind, InstructionKind::Phi { .. }))
                .unwrap_or(block.instructions.len());

            block.instructions.insert(insert_pos, scalar_phi_instr);

            field_phis.insert(field_index, scalar_phi_id);
        }

        scalar_mappings.insert(scalarization.original_phi, field_phis);
    }

    scalar_mappings
}

fn get_scalar_field(
    &self,
    aggregate_id: ValueId,
    field_index: usize,
    function: &MirFunction
) -> Value {
    // Check if this aggregate is already scalarized
    if let Some(scalar_fields) = self.virtual_aggregates.get(&aggregate_id) {
        if let Some(&scalar_id) = scalar_fields.get(&field_index) {
            return Value::operand(scalar_id);
        }
    }

    // Check if this aggregate is defined by make_tuple/make_struct
    if let Some(def_instr) = self.find_definition(function, aggregate_id) {
        match &def_instr.kind {
            InstructionKind::MakeTuple { elements, .. } => {
                if field_index < elements.len() {
                    return elements[field_index].clone();
                }
            }
            InstructionKind::MakeStruct { fields, .. } => {
                if let Some(field_val) = fields.get(&field_index) {
                    return field_val.clone();
                }
            }
            _ => {}
        }
    }

    // Fallback: create an extract instruction (will be cleaned up later)
    let extract_id = function.new_typed_value_id(
        self.get_field_type(&self.get_aggregate_type(aggregate_id), field_index)
    );

    // This is a placeholder - in practice we'd insert the extract instruction
    Value::operand(extract_id)
}
```

## Complex Test Cases

### Test 1: Single Field Usage Through Phi

```rust
#[test]
fn test_single_field_phi_optimization() {
    let mut function = MirFunction::new("test_phi_single_field".to_string());
    let entry = function.add_basic_block();
    let block1 = function.add_basic_block();
    let block2 = function.add_basic_block();
    let merge = function.add_basic_block();
    function.entry_block = entry;

    // Entry: branch to block1 or block2
    let cond = function.new_typed_value_id(MirType::felt());
    let entry_block = function.get_basic_block_mut(entry).unwrap();
    entry_block.set_terminator(Terminator::If {
        condition: Value::operand(cond),
        then_target: block1,
        else_target: block2,
    });

    // Block1: create tuple (a, b, c)
    let a = function.new_typed_value_id(MirType::felt());
    let b = function.new_typed_value_id(MirType::felt());
    let c = function.new_typed_value_id(MirType::felt());
    let t1 = function.new_typed_value_id(MirType::tuple(vec![MirType::felt(); 3]));

    let block1_ref = function.get_basic_block_mut(block1).unwrap();
    block1_ref.push_instruction(Instruction::make_tuple(
        t1, vec![Value::operand(a), Value::operand(b), Value::operand(c)]
    ));
    block1_ref.set_terminator(Terminator::jump(merge));

    // Block2: create tuple (x, y, z)
    let x = function.new_typed_value_id(MirType::felt());
    let y = function.new_typed_value_id(MirType::felt());
    let z = function.new_typed_value_id(MirType::felt());
    let t2 = function.new_typed_value_id(MirType::tuple(vec![MirType::felt(); 3]));

    let block2_ref = function.get_basic_block_mut(block2).unwrap();
    block2_ref.push_instruction(Instruction::make_tuple(
        t2, vec![Value::operand(x), Value::operand(y), Value::operand(z)]
    ));
    block2_ref.set_terminator(Terminator::jump(merge));

    // Merge: phi on tuples, extract only field 0
    let phi_tuple = function.new_typed_value_id(MirType::tuple(vec![MirType::felt(); 3]));
    let result = function.new_typed_value_id(MirType::felt());

    let merge_block = function.get_basic_block_mut(merge).unwrap();
    merge_block.push_instruction(Instruction::phi(
        phi_tuple,
        vec![(block1, Value::operand(t1)), (block2, Value::operand(t2))],
        MirType::tuple(vec![MirType::felt(); 3])
    ));
    merge_block.push_instruction(Instruction::extract_tuple_element(
        result, Value::operand(phi_tuple), 0, MirType::felt()
    ));
    merge_block.set_terminator(Terminator::return_value(Value::operand(result)));

    let mut pass = ScalarReplacementOfAggregates::new();
    let modified = pass.run(&mut function);

    assert!(modified);

    // Verify optimization:
    // 1. Original tuple constructions should be removed
    // 2. Aggregate phi should be removed
    // 3. Only scalar phi for field 0 should remain
    // 4. Extract instruction should be removed

    let merge_block = function.get_basic_block(merge).unwrap();

    // Should have only one phi (for field 0)
    let phi_count = merge_block.instructions.iter()
        .filter(|instr| matches!(instr.kind, InstructionKind::Phi { .. }))
        .count();
    assert_eq!(phi_count, 1);

    // The phi should be for scalar values (a vs x)
    let phi_instr = merge_block.instructions.iter()
        .find(|instr| matches!(instr.kind, InstructionKind::Phi { .. }))
        .unwrap();

    if let InstructionKind::Phi { sources, .. } = &phi_instr.kind {
        // Sources should be the scalar values a and x, not tuples
        assert_eq!(sources.len(), 2);
        // Additional verification that these are the correct scalar sources
    }

    // Should have no extract instructions
    let extract_count = merge_block.instructions.iter()
        .filter(|instr| matches!(instr.kind, InstructionKind::ExtractTupleElement { .. }))
        .count();
    assert_eq!(extract_count, 0);
}
```

### Test 2: Mixed Usage (Some Fields + Escaping)

```rust
#[test]
fn test_mixed_usage_phi_optimization() {
    // Setup similar to above but add a function call that uses the full tuple

    // Merge block:
    let phi_tuple = function.new_typed_value_id(MirType::tuple(vec![MirType::felt(); 3]));
    let field0 = function.new_typed_value_id(MirType::felt());
    let field1 = function.new_typed_value_id(MirType::felt());

    let merge_block = function.get_basic_block_mut(merge).unwrap();
    merge_block.push_instruction(Instruction::phi(
        phi_tuple,
        vec![(block1, Value::operand(t1)), (block2, Value::operand(t2))],
        MirType::tuple(vec![MirType::felt(); 3])
    ));

    // Extract some fields
    merge_block.push_instruction(Instruction::extract_tuple_element(
        field0, Value::operand(phi_tuple), 0, MirType::felt()
    ));
    merge_block.push_instruction(Instruction::extract_tuple_element(
        field1, Value::operand(phi_tuple), 1, MirType::felt()
    ));

    // But also pass full tuple to function (escaping use)
    merge_block.push_instruction(Instruction::call(
        None, "external_function".to_string(), vec![Value::operand(phi_tuple)]
    ));

    let result = function.new_typed_value_id(MirType::felt());
    merge_block.push_instruction(Instruction::binary_op(
        BinaryOp::Add, result, Value::operand(field0), Value::operand(field1)
    ));
    merge_block.set_terminator(Terminator::return_value(Value::operand(result)));

    let mut pass = ScalarReplacementOfAggregates::new();
    let modified = pass.run(&mut function);

    assert!(modified);

    // Verify optimization:
    // 1. Should have scalar phis for fields 0 and 1
    // 2. Should materialize full tuple just before function call
    // 3. Extract instructions should be eliminated
    // 4. Field 2 should not have a phi (unused)

    let merge_block = function.get_basic_block(merge).unwrap();

    // Should have two scalar phis (fields 0 and 1)
    let phi_count = merge_block.instructions.iter()
        .filter(|instr| matches!(instr.kind, InstructionKind::Phi { .. }))
        .count();
    assert_eq!(phi_count, 2);

    // Should have one materialization (make_tuple) before the call
    let make_tuple_count = merge_block.instructions.iter()
        .filter(|instr| matches!(instr.kind, InstructionKind::MakeTuple { .. }))
        .count();
    assert_eq!(make_tuple_count, 1);

    // Should have no extract instructions
    let extract_count = merge_block.instructions.iter()
        .filter(|instr| matches!(instr.kind, InstructionKind::ExtractTupleElement { .. }))
        .count();
    assert_eq!(extract_count, 0);
}
```

### Test 3: Transitive Phi Dependencies

```rust
#[test]
fn test_transitive_phi_dependencies() {
    // Create a case where:
    // phi1 = phi [tuple1, tuple2]
    // phi2 = phi [phi1, tuple3]
    // extract field 0 from phi2

    // This should create only scalar phis for field 0 throughout the chain

    // Implementation details...

    // After optimization:
    // - All aggregate phis should be eliminated
    // - Only scalar phis for field 0 should remain
    // - Should form a proper scalar phi chain
}
```

### Test 4: Partial Updates Through Phi

```rust
#[test]
fn test_partial_updates_through_phi() {
    // Create:
    // phi_tuple = phi [tuple1, tuple2]
    // updated_tuple = insert_tuple(phi_tuple, 1, new_value)
    // result = extract_tuple(updated_tuple, 0)  // field 0 unchanged
    // result2 = extract_tuple(updated_tuple, 1) // field 1 is new_value

    // Should optimize to:
    // phi_0 = phi [tuple1.field0, tuple2.field0]
    // result = phi_0
    // result2 = new_value

    // No tuple construction or extraction should remain
}
```

## Performance Considerations

### Complexity Analysis

- **Demand Analysis**: O(V + E) where V = values, E = uses
- **Phi Scalarization**: O(P × F) where P = phi nodes, F = max fields
- **Overall**: O(V + E + P × F) which is linear in practice

### Memory Usage

- Virtual aggregate tracking: O(A × F) where A = aggregates
- Demand analysis: O(A × F)
- Reasonable for typical programs

### Optimization Impact

- **Eliminates**: Unnecessary aggregate construction/destruction
- **Reduces**: Phi node complexity (scalar phis vs aggregate phis)
- **Enables**: Better downstream optimizations (copy propagation, etc.)

## Integration with Pipeline

**Recommended Position**: After basic optimizations, before final lowering

```rust
.add_pass(ArithmeticSimplify::new())
.add_pass(ConstantFolding::new())
.add_pass(CopyPropagation::new())
.add_pass(LocalCSE::new())
.add_pass(SimplifyBranches::new())
.add_pass(ScalarReplacementOfAggregates::new())  // HERE
.add_pass(DeadCodeElimination::new())
```

This position allows SROA to benefit from earlier simplifications while enabling
final cleanup passes to remove newly dead code.
