//! Constant folding pass for aggregate operations
//!
//! This pass performs constant folding and propagation specifically for
//! aggregate operations like MakeTuple, ExtractTupleElement, MakeStruct,
//! and ExtractStructField. It can fold patterns like:
//! - ExtractTuple(MakeTuple(a, b, c), 1) => b
//! - ExtractField(MakeStruct{x: a, y: b}, "x") => a

use rustc_hash::FxHashMap;

use crate::{BasicBlockId, Instruction, InstructionKind, MirFunction, Value, ValueId};

use super::MirPass;

/// Constant folding pass for aggregate operations
#[derive(Debug, Default)]
pub struct ConstFoldPass;

impl ConstFoldPass {
    /// Create a new constant folding pass
    pub const fn new() -> Self {
        Self
    }

    /// Process a basic block, folding instructions where possible
    pub fn fold_block(&self, function: &mut MirFunction, block_id: BasicBlockId) -> bool {
        let mut changed = false;
        let mut replacements: Vec<(usize, Instruction)> = Vec::new();
        let instructions_to_remove: Vec<usize> = Vec::new();

        // Build a map of value definitions for the current block
        let mut value_defs: FxHashMap<ValueId, &Instruction> = FxHashMap::default();

        // First pass: collect all value definitions in the block
        if let Some(block) = function.basic_blocks.get(block_id) {
            for instr in &block.instructions {
                if let Some(dest) = instr.destination() {
                    value_defs.insert(dest, instr);
                }
            }
        }

        // Second pass: try to fold instructions
        if let Some(block) = function.basic_blocks.get(block_id) {
            for (idx, instr) in block.instructions.iter().enumerate() {
                match &instr.kind {
                    // Fold ExtractTupleElement(MakeTuple(...), index)
                    InstructionKind::ExtractTupleElement {
                        dest,
                        tuple,
                        index,
                        element_ty,
                    } => {
                        if let Value::Operand(tuple_id) = tuple {
                            if let Some(tuple_def) = value_defs.get(tuple_id) {
                                if let InstructionKind::MakeTuple { elements, .. } = &tuple_def.kind
                                {
                                    if let Some(element) = elements.get(*index) {
                                        // Create an assign instruction to replace the extract
                                        let replacement = Instruction::assign(
                                            *dest,
                                            *element,
                                            element_ty.clone(),
                                        )
                                        .with_comment(format!(
                                            "Folded ExtractTupleElement({}, {})",
                                            tuple_id.index(),
                                            index
                                        ));
                                        replacements.push((idx, replacement));
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }

                    // Fold ExtractStructField(MakeStruct(...), field)
                    InstructionKind::ExtractStructField {
                        dest,
                        struct_val,
                        field_name,
                        field_ty,
                    } => {
                        if let Value::Operand(struct_id) = struct_val {
                            if let Some(struct_def) = value_defs.get(struct_id) {
                                if let InstructionKind::MakeStruct { fields, .. } = &struct_def.kind
                                {
                                    // Find the field in the struct
                                    if let Some((_, field_value)) =
                                        fields.iter().find(|(name, _)| name == field_name)
                                    {
                                        // Create an assign instruction to replace the extract
                                        let replacement = Instruction::assign(
                                            *dest,
                                            *field_value,
                                            field_ty.clone(),
                                        )
                                        .with_comment(format!(
                                            "Folded ExtractStructField({}, \"{}\")",
                                            struct_id.index(),
                                            field_name
                                        ));
                                        replacements.push((idx, replacement));
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }

                    // Fold InsertField(MakeStruct(...), field, value) into a new MakeStruct
                    InstructionKind::InsertField {
                        dest,
                        struct_val,
                        field_name,
                        new_value,
                        struct_ty,
                    } => {
                        if let Value::Operand(struct_id) = struct_val {
                            if let Some(struct_def) = value_defs.get(struct_id) {
                                if let InstructionKind::MakeStruct { fields, .. } = &struct_def.kind
                                {
                                    // Create a new field list with the updated value
                                    let mut new_fields = fields.clone();
                                    for (name, value) in &mut new_fields {
                                        if name == field_name {
                                            *value = *new_value;
                                            break;
                                        }
                                    }
                                    // Create a new MakeStruct instruction
                                    let replacement = Instruction::make_struct(
                                        *dest,
                                        new_fields,
                                        struct_ty.clone(),
                                    )
                                    .with_comment(format!(
                                        "Folded InsertField({}, \"{}\")",
                                        struct_id.index(),
                                        field_name
                                    ));
                                    replacements.push((idx, replacement));
                                    changed = true;
                                }
                            }
                        }
                    }

                    // Fold InsertTuple(MakeTuple(...), index, value) into a new MakeTuple
                    InstructionKind::InsertTuple {
                        dest,
                        tuple_val,
                        index,
                        new_value,
                        tuple_ty: _,
                    } => {
                        if let Value::Operand(tuple_id) = tuple_val {
                            if let Some(tuple_def) = value_defs.get(tuple_id) {
                                if let InstructionKind::MakeTuple { elements, .. } = &tuple_def.kind
                                {
                                    // Create a new element list with the updated value
                                    let mut new_elements = elements.clone();
                                    if *index < new_elements.len() {
                                        new_elements[*index] = *new_value;
                                        // Create a new MakeTuple instruction
                                        let replacement =
                                            Instruction::make_tuple(*dest, new_elements)
                                                .with_comment(format!(
                                                    "Folded InsertTuple({}, {})",
                                                    tuple_id.index(),
                                                    index
                                                ));
                                        replacements.push((idx, replacement));
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }
        }

        // Apply replacements
        if let Some(block) = function.basic_blocks.get_mut(block_id) {
            for (idx, replacement) in replacements.into_iter().rev() {
                block.instructions[idx] = replacement;
            }

            // Remove instructions marked for deletion (in reverse order to preserve indices)
            for idx in instructions_to_remove.into_iter().rev() {
                block.instructions.remove(idx);
            }
        }

        changed
    }

    /// Remove unused aggregate creation instructions
    pub fn eliminate_dead_aggregates(&self, function: &mut MirFunction) -> bool {
        let mut changed = false;
        let use_counts = function.get_value_use_counts();

        for block in function.basic_blocks.iter_mut() {
            let mut indices_to_remove = Vec::new();

            for (idx, instr) in block.instructions.iter().enumerate() {
                // Check if this is an aggregate creation with no uses
                let should_remove = match &instr.kind {
                    InstructionKind::MakeTuple { dest, .. }
                    | InstructionKind::MakeStruct { dest, .. } => {
                        use_counts.get(dest).copied().unwrap_or(0) == 0
                    }
                    _ => false,
                };

                if should_remove {
                    indices_to_remove.push(idx);
                }
            }

            // Remove dead instructions in reverse order
            for idx in indices_to_remove.into_iter().rev() {
                block.instructions.remove(idx);
                changed = true;
            }
        }

        changed
    }
}

impl MirPass for ConstFoldPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut changed = false;

        // Run constant folding on each block
        let block_ids: Vec<_> = function.basic_blocks.indices().collect();
        for block_id in block_ids {
            changed |= self.fold_block(function, block_id);
        }

        // Run dead aggregate elimination
        changed |= self.eliminate_dead_aggregates(function);

        changed
    }

    fn name(&self) -> &'static str {
        "ConstFold"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MirType, Terminator};

    #[test]
    fn test_tuple_extract_make_folding() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;

        // Create values
        let v1 = function.new_typed_value_id(MirType::felt());
        let v2 = function.new_typed_value_id(MirType::felt());
        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt()]);
        let tuple_dest = function.new_typed_value_id(tuple_type);
        let extract_dest = function.new_typed_value_id(MirType::felt());

        // Build instructions: make tuple then extract
        let block = function.get_basic_block_mut(entry).unwrap();
        block
            .instructions
            .push(Instruction::assign(v1, Value::integer(42), MirType::felt()));
        block
            .instructions
            .push(Instruction::assign(v2, Value::integer(24), MirType::felt()));
        block.instructions.push(Instruction::make_tuple(
            tuple_dest,
            vec![Value::operand(v1), Value::operand(v2)],
        ));
        block.instructions.push(Instruction::extract_tuple_element(
            extract_dest,
            Value::operand(tuple_dest),
            0,
            MirType::felt(),
        ));
        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(extract_dest)],
        });

        // Run only the folding part, not dead code elimination
        let pass = ConstFoldPass::new();
        let changed = pass.fold_block(&mut function, entry);
        assert!(changed);

        // Verify the extract was replaced with an assign
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.instructions.len(), 4); // All instructions still present
        let last_instr = &block.instructions[3];
        match &last_instr.kind {
            InstructionKind::Assign { source, dest, .. } => {
                assert_eq!(*dest, extract_dest);
                assert_eq!(*source, Value::operand(v1));
            }
            _ => panic!(
                "Expected Assign instruction after folding, got {:?}",
                last_instr.kind
            ),
        }

        // Now test that dead elimination would remove the unused tuple
        let eliminated = pass.eliminate_dead_aggregates(&mut function);
        assert!(eliminated);
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.instructions.len(), 3); // MakeTuple removed
    }

    #[test]
    fn test_struct_extract_make_folding() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;

        // Create values
        let x_val = function.new_typed_value_id(MirType::felt());
        let y_val = function.new_typed_value_id(MirType::felt());
        let struct_type = MirType::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), MirType::felt()),
                ("y".to_string(), MirType::felt()),
            ],
        };
        let struct_dest = function.new_typed_value_id(struct_type.clone());
        let extract_dest = function.new_typed_value_id(MirType::felt());

        // Build instructions: make struct then extract field
        let block = function.get_basic_block_mut(entry).unwrap();
        block.instructions.push(Instruction::assign(
            x_val,
            Value::integer(10),
            MirType::felt(),
        ));
        block.instructions.push(Instruction::assign(
            y_val,
            Value::integer(20),
            MirType::felt(),
        ));
        block.instructions.push(Instruction::make_struct(
            struct_dest,
            vec![
                ("x".to_string(), Value::operand(x_val)),
                ("y".to_string(), Value::operand(y_val)),
            ],
            struct_type,
        ));
        block.instructions.push(Instruction::extract_struct_field(
            extract_dest,
            Value::operand(struct_dest),
            "x".to_string(),
            MirType::felt(),
        ));

        // Run only the folding part
        let pass = ConstFoldPass::new();
        let changed = pass.fold_block(&mut function, entry);
        assert!(changed);

        // Verify the extract was replaced with an assign
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.instructions.len(), 4); // All instructions still present
        let last_instr = &block.instructions[3];
        match &last_instr.kind {
            InstructionKind::Assign { source, dest, .. } => {
                assert_eq!(*dest, extract_dest);
                assert_eq!(*source, Value::operand(x_val));
            }
            _ => panic!(
                "Expected Assign instruction after folding, got {:?}",
                last_instr.kind
            ),
        }
    }

    #[test]
    fn test_dead_aggregate_elimination() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;

        // Create an unused tuple
        let tuple_type = MirType::Tuple(vec![MirType::felt(), MirType::felt()]);
        let unused_tuple = function.new_typed_value_id(tuple_type);

        let block = function.get_basic_block_mut(entry).unwrap();
        block.instructions.push(Instruction::make_tuple(
            unused_tuple,
            vec![Value::integer(1), Value::integer(2)],
        ));
        block.set_terminator(Terminator::Return { values: vec![] });

        // Run the pass
        let mut pass = ConstFoldPass::new();
        let changed = pass.run(&mut function);
        assert!(changed);

        // Verify the unused tuple was removed
        let block = function.get_basic_block(entry).unwrap();
        assert!(block.instructions.is_empty());
    }

    #[test]
    fn test_insert_field_folding() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;

        // Create struct and new value
        let struct_type = MirType::Struct {
            name: "Data".to_string(),
            fields: vec![
                ("a".to_string(), MirType::felt()),
                ("b".to_string(), MirType::felt()),
            ],
        };
        let struct1 = function.new_typed_value_id(struct_type.clone());
        let struct2 = function.new_typed_value_id(struct_type.clone());
        let new_val = function.new_typed_value_id(MirType::felt());

        // Build: MakeStruct then InsertField
        let block = function.get_basic_block_mut(entry).unwrap();
        block.instructions.push(Instruction::make_struct(
            struct1,
            vec![
                ("a".to_string(), Value::integer(1)),
                ("b".to_string(), Value::integer(2)),
            ],
            struct_type.clone(),
        ));
        block.instructions.push(Instruction::assign(
            new_val,
            Value::integer(99),
            MirType::felt(),
        ));
        block.instructions.push(Instruction::insert_field(
            struct2,
            Value::operand(struct1),
            "a".to_string(),
            Value::operand(new_val),
            struct_type,
        ));

        // Run only the folding part
        let pass = ConstFoldPass::new();
        let changed = pass.fold_block(&mut function, entry);
        assert!(changed);

        // Verify InsertField was replaced with MakeStruct
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.instructions.len(), 3); // All instructions still present
        let last_instr = &block.instructions[2];
        match &last_instr.kind {
            InstructionKind::MakeStruct { fields, dest, .. } => {
                assert_eq!(*dest, struct2);
                let a_field = fields.iter().find(|(name, _)| name == "a");
                assert_eq!(a_field, Some(&("a".to_string(), Value::operand(new_val))));
                let b_field = fields.iter().find(|(name, _)| name == "b");
                assert_eq!(b_field, Some(&("b".to_string(), Value::integer(2))));
            }
            _ => panic!(
                "Expected MakeStruct after folding InsertField, got {:?}",
                last_instr.kind
            ),
        }
    }
}
