//! Late-stage aggregate lowering pass for backend compatibility
//!
//! This pass converts value-based aggregate instructions back to memory operations
//! for backends that cannot handle first-class aggregates.

use std::collections::HashMap;

use crate::instruction::{Instruction, InstructionKind};
use crate::layout::DataLayout;
use crate::mir_types::MirType;
use crate::passes::MirPass;
use crate::value::Value;
use crate::{MirFunction, ValueId};

/// Pass that converts aggregate instructions to memory operations
/// for backends that cannot handle first-class aggregates
#[derive(Debug, Default)]
pub struct LowerAggregatesPass {
    /// Map from original aggregate values to their memory locations
    aggregate_allocas: HashMap<ValueId, ValueId>,
    /// Counter for generating unique names
    next_temp_id: u32,
}

impl LowerAggregatesPass {
    /// Create a new aggregate lowering pass
    pub fn new() -> Self {
        Self {
            aggregate_allocas: HashMap::new(),
            next_temp_id: 0,
        }
    }

    /// Extract ValueId from a Value if it's an operand
    const fn extract_value_id(value: &Value) -> Option<ValueId> {
        match value {
            Value::Operand(id) => Some(*id),
            _ => None,
        }
    }

    /// Create framealloc and stores for a tuple
    fn create_tuple_alloca_and_stores(
        &mut self,
        dest: ValueId,
        elements: &[Value],
        function: &mut MirFunction,
    ) -> Vec<Instruction> {
        let mut instructions = Vec::new();

        // Get the actual tuple type from the destination
        let tuple_type = function.get_value_type(dest).cloned().unwrap_or_else(|| {
            // Fallback: infer from elements
            let element_types: Vec<MirType> = elements
                .iter()
                .map(|elem| match elem {
                    Value::Operand(id) => function
                        .get_value_type(*id)
                        .cloned()
                        .unwrap_or(MirType::Unknown),
                    Value::Literal(lit) => match lit {
                        crate::Literal::Integer(_) => MirType::felt(),
                        crate::Literal::Boolean(_) => MirType::bool(),
                        crate::Literal::Unit => MirType::Unit,
                    },
                    Value::Error => MirType::Unknown,
                })
                .collect();
            MirType::Tuple(element_types)
        });

        let element_types = match &tuple_type {
            MirType::Tuple(types) => types.clone(),
            _ => vec![MirType::Unknown; elements.len()],
        };

        // Allocate memory for the tuple using proper ValueId
        let alloca_id = function.new_typed_value_id(MirType::pointer(tuple_type.clone()));
        instructions.push(
            Instruction::frame_alloc(alloca_id, tuple_type.clone())
                .with_comment(format!("Lowered tuple alloca for %{}", dest.index())),
        );

        // Store this mapping for later extract operations
        self.aggregate_allocas.insert(dest, alloca_id);

        // Store each element using proper offsets
        let layout = DataLayout::new();
        for (i, (elem, elem_type)) in elements.iter().zip(element_types.iter()).enumerate() {
            let offset = layout.tuple_offset(&tuple_type, i).unwrap_or(i) as i32;
            let elem_ptr = function.new_typed_value_id(MirType::pointer(elem_type.clone()));
            instructions.push(
                Instruction::get_element_ptr(
                    elem_ptr,
                    Value::operand(alloca_id),
                    Value::integer(offset),
                )
                .with_comment(format!(
                    "Get address (offset {}) of tuple element {}",
                    offset, i
                )),
            );
            instructions.push(
                Instruction::store(Value::operand(elem_ptr), *elem, elem_type.clone())
                    .with_comment(format!("Store tuple element {}", i)),
            );
        }

        // The original dest now becomes an alias for the alloca
        // We might need to add a load or assignment depending on usage
        instructions.push(
            Instruction::assign(
                dest,
                Value::operand(alloca_id),
                MirType::pointer(tuple_type),
            )
            .with_comment("Alias tuple value to alloca".to_string()),
        );

        instructions
    }

    /// Create framealloc and stores for a struct
    fn create_struct_alloca_and_stores(
        &mut self,
        dest: ValueId,
        fields: &[(String, Value)],
        struct_ty: &MirType,
        function: &mut MirFunction,
    ) -> Vec<Instruction> {
        let mut instructions = Vec::new();

        // Allocate memory for the struct using proper ValueId
        let alloca_id = function.new_typed_value_id(MirType::pointer(struct_ty.clone()));
        instructions.push(
            Instruction::frame_alloc(alloca_id, struct_ty.clone())
                .with_comment(format!("Lowered struct alloca for %{}", dest.index())),
        );

        // Store this mapping for later extract operations
        self.aggregate_allocas.insert(dest, alloca_id);

        // Get field offsets using DataLayout
        let layout = DataLayout::new();

        // Store each field
        for (field_name, field_value) in fields {
            // Get field offset
            if let Some(offset) = layout.field_offset(struct_ty, field_name) {
                // Get field type
                let field_type = if let MirType::Struct {
                    fields: field_defs, ..
                } = struct_ty
                {
                    field_defs
                        .iter()
                        .find(|(name, _)| name == field_name)
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or(MirType::Unknown)
                } else {
                    MirType::Unknown
                };

                let field_ptr = function.new_typed_value_id(MirType::pointer(field_type.clone()));
                instructions.push(
                    Instruction::get_element_ptr(
                        field_ptr,
                        Value::operand(alloca_id),
                        Value::integer(offset as i32),
                    )
                    .with_comment(format!("Get address of field '{}'", field_name)),
                );
                instructions.push(
                    Instruction::store(Value::operand(field_ptr), *field_value, field_type)
                        .with_comment(format!("Store field '{}'", field_name)),
                );
            }
        }

        // The original dest becomes an alias for the alloca
        instructions.push(
            Instruction::assign(
                dest,
                Value::operand(alloca_id),
                MirType::pointer(struct_ty.clone()),
            )
            .with_comment("Alias struct value to alloca".to_string()),
        );

        instructions
    }

    /// Create GEP and load for tuple element access
    fn create_tuple_gep_and_load(
        &mut self,
        dest: ValueId,
        alloca_value: ValueId,
        index: usize,
        element_ty: &MirType,
        function: &mut MirFunction,
    ) -> Vec<Instruction> {
        let mut instructions = Vec::new();

        // Get the tuple type to calculate proper offset
        let tuple_type =
            if let Some(MirType::Pointer(inner)) = function.get_value_type(alloca_value) {
                inner.as_ref().clone()
            } else {
                MirType::Unknown
            };

        let layout = DataLayout::new();
        let offset = layout.tuple_offset(&tuple_type, index).unwrap_or(index) as i32;

        let elem_ptr = function.new_typed_value_id(MirType::pointer(element_ty.clone()));
        instructions.push(
            Instruction::get_element_ptr(
                elem_ptr,
                Value::operand(alloca_value),
                Value::integer(offset),
            )
            .with_comment(format!(
                "Get address (offset {}) of tuple element {} (lowered)",
                offset, index
            )),
        );

        instructions.push(
            Instruction::load(dest, element_ty.clone(), Value::operand(elem_ptr))
                .with_comment(format!("Load tuple element {} (lowered)", index)),
        );

        instructions
    }

    /// Create GEP and load for struct field access
    fn create_struct_gep_and_load(
        &mut self,
        dest: ValueId,
        alloca_value: ValueId,
        field_name: &str,
        field_ty: &MirType,
        function: &mut MirFunction,
    ) -> Vec<Instruction> {
        let mut instructions = Vec::new();

        // Get the struct type from the alloca pointer type
        let struct_type =
            if let Some(MirType::Pointer(inner)) = function.get_value_type(alloca_value) {
                inner.as_ref().clone()
            } else {
                // Fallback - create an unknown struct type
                MirType::Unknown
            };

        // Get field offset using DataLayout
        let layout = DataLayout::new();
        let offset = layout.field_offset(&struct_type, field_name).unwrap_or(0);

        let field_ptr = function.new_typed_value_id(MirType::pointer(field_ty.clone()));
        instructions.push(
            Instruction::get_element_ptr(
                field_ptr,
                Value::operand(alloca_value),
                Value::integer(offset as i32),
            )
            .with_comment(format!("Get address of field '{}' (lowered)", field_name)),
        );

        instructions.push(
            Instruction::load(dest, field_ty.clone(), Value::operand(field_ptr))
                .with_comment(format!("Load field '{}' (lowered)", field_name)),
        );

        instructions
    }

    /// Process a single instruction, potentially lowering it
    fn lower_instruction(
        &mut self,
        instruction: &Instruction,
        function: &mut MirFunction,
    ) -> Vec<Instruction> {
        match &instruction.kind {
            // Lower aggregate creation
            InstructionKind::MakeTuple { dest, elements } => {
                self.create_tuple_alloca_and_stores(*dest, elements, function)
            }
            InstructionKind::MakeStruct {
                dest,
                fields,
                struct_ty,
            } => self.create_struct_alloca_and_stores(*dest, fields, struct_ty, function),

            // Lower aggregate access
            InstructionKind::ExtractTupleElement {
                dest,
                tuple,
                index,
                element_ty,
            } => {
                if let Some(value_id) = Self::extract_value_id(tuple) {
                    if let Some(&alloca_id) = self.aggregate_allocas.get(&value_id) {
                        return self.create_tuple_gep_and_load(
                            *dest, alloca_id, *index, element_ty, function,
                        );
                    }
                }
                // If we can't lower it, keep it as-is
                vec![instruction.clone()]
            }
            InstructionKind::ExtractStructField {
                dest,
                struct_val,
                field_name,
                field_ty,
            } => {
                if let Some(value_id) = Self::extract_value_id(struct_val) {
                    if let Some(&alloca_id) = self.aggregate_allocas.get(&value_id) {
                        return self.create_struct_gep_and_load(
                            *dest, alloca_id, field_name, field_ty, function,
                        );
                    }
                }
                // If we can't lower it, keep it as-is
                vec![instruction.clone()]
            }

            // InsertField and InsertTuple need special handling
            InstructionKind::InsertField {
                dest,
                struct_val,
                field_name,
                new_value,
                struct_ty,
            } => {
                // For InsertField, we need to copy the struct and update the field
                // This becomes: alloca new struct, copy old struct, update field
                let mut instructions = Vec::new();

                // Allocate new struct
                let new_alloca = function.new_typed_value_id(MirType::pointer(struct_ty.clone()));
                instructions.push(
                    Instruction::frame_alloc(new_alloca, struct_ty.clone())
                        .with_comment("Alloca for updated struct".to_string()),
                );

                // Copy old struct contents if we have an existing alloca
                if let Some(value_id) = Self::extract_value_id(struct_val) {
                    if let Some(&old_alloca) = self.aggregate_allocas.get(&value_id) {
                        // Copy each field from old to new
                        if let MirType::Struct { fields, .. } = struct_ty {
                            let layout = DataLayout::new();
                            for (fname, ftype) in fields {
                                if fname == field_name {
                                    continue; // Skip the field we're updating
                                }
                                let offset =
                                    layout.field_offset(struct_ty, fname).unwrap_or(0) as i32;

                                // Load from old
                                let old_field_ptr =
                                    function.new_typed_value_id(MirType::pointer(ftype.clone()));
                                instructions.push(
                                    Instruction::get_element_ptr(
                                        old_field_ptr,
                                        Value::operand(old_alloca),
                                        Value::integer(offset),
                                    )
                                    .with_comment(format!("Get old field '{}'", fname)),
                                );
                                let field_value = function.new_typed_value_id(ftype.clone());
                                instructions.push(
                                    Instruction::load(
                                        field_value,
                                        ftype.clone(),
                                        Value::operand(old_field_ptr),
                                    )
                                    .with_comment(format!("Load old field '{}'", fname)),
                                );

                                // Store to new
                                let new_field_ptr =
                                    function.new_typed_value_id(MirType::pointer(ftype.clone()));
                                instructions.push(
                                    Instruction::get_element_ptr(
                                        new_field_ptr,
                                        Value::operand(new_alloca),
                                        Value::integer(offset),
                                    )
                                    .with_comment(format!("Get new field '{}' location", fname)),
                                );
                                instructions.push(
                                    Instruction::store(
                                        Value::operand(new_field_ptr),
                                        Value::operand(field_value),
                                        ftype.clone(),
                                    )
                                    .with_comment(format!("Copy field '{}'", fname)),
                                );
                            }
                        }
                    }
                }

                // Now store the updated field value
                let layout = DataLayout::new();
                if let Some(offset) = layout.field_offset(struct_ty, field_name) {
                    let field_type = if let MirType::Struct { fields, .. } = struct_ty {
                        fields
                            .iter()
                            .find(|(name, _)| name == field_name)
                            .map(|(_, ty)| ty.clone())
                            .unwrap_or(MirType::Unknown)
                    } else {
                        MirType::Unknown
                    };

                    let field_ptr =
                        function.new_typed_value_id(MirType::pointer(field_type.clone()));
                    instructions.push(
                        Instruction::get_element_ptr(
                            field_ptr,
                            Value::operand(new_alloca),
                            Value::integer(offset as i32),
                        )
                        .with_comment(format!("Get address of field '{}' for update", field_name)),
                    );
                    instructions.push(
                        Instruction::store(Value::operand(field_ptr), *new_value, field_type)
                            .with_comment(format!("Update field '{}'", field_name)),
                    );
                }

                self.aggregate_allocas.insert(*dest, new_alloca);
                instructions.push(
                    Instruction::assign(
                        *dest,
                        Value::operand(new_alloca),
                        MirType::pointer(struct_ty.clone()),
                    )
                    .with_comment("Alias updated struct to alloca".to_string()),
                );

                instructions
            }

            InstructionKind::InsertTuple {
                dest,
                tuple_val,
                index,
                new_value,
                tuple_ty,
            } => {
                // Similar to InsertField, but for tuples
                let mut instructions = Vec::new();

                // Allocate new tuple
                let new_alloca = function.new_typed_value_id(MirType::pointer(tuple_ty.clone()));
                instructions.push(
                    Instruction::frame_alloc(new_alloca, tuple_ty.clone())
                        .with_comment("Alloca for updated tuple".to_string()),
                );

                // Copy old tuple contents if we have an existing alloca
                if let Some(value_id) = Self::extract_value_id(tuple_val) {
                    if let Some(&old_alloca) = self.aggregate_allocas.get(&value_id) {
                        // Copy each element from old to new
                        if let MirType::Tuple(types) = tuple_ty {
                            let layout = DataLayout::new();
                            for (i, elem_type) in types.iter().enumerate() {
                                if i == *index {
                                    continue; // Skip the element we're updating
                                }
                                let offset = layout.tuple_offset(tuple_ty, i).unwrap_or(i) as i32;

                                // Load from old
                                let old_elem_ptr = function
                                    .new_typed_value_id(MirType::pointer(elem_type.clone()));
                                instructions.push(
                                    Instruction::get_element_ptr(
                                        old_elem_ptr,
                                        Value::operand(old_alloca),
                                        Value::integer(offset),
                                    )
                                    .with_comment(format!("Get old element {}", i)),
                                );
                                let elem_value = function.new_typed_value_id(elem_type.clone());
                                instructions.push(
                                    Instruction::load(
                                        elem_value,
                                        elem_type.clone(),
                                        Value::operand(old_elem_ptr),
                                    )
                                    .with_comment(format!("Load old element {}", i)),
                                );

                                // Store to new
                                let new_elem_ptr = function
                                    .new_typed_value_id(MirType::pointer(elem_type.clone()));
                                instructions.push(
                                    Instruction::get_element_ptr(
                                        new_elem_ptr,
                                        Value::operand(new_alloca),
                                        Value::integer(offset),
                                    )
                                    .with_comment(format!("Get new element {} location", i)),
                                );
                                instructions.push(
                                    Instruction::store(
                                        Value::operand(new_elem_ptr),
                                        Value::operand(elem_value),
                                        elem_type.clone(),
                                    )
                                    .with_comment(format!("Copy element {}", i)),
                                );
                            }
                        }
                    }
                }

                // Now store the updated element value
                let element_type = if let MirType::Tuple(types) = tuple_ty {
                    types.get(*index).cloned().unwrap_or(MirType::Unknown)
                } else {
                    MirType::Unknown
                };

                let layout = DataLayout::new();
                let offset = layout.tuple_offset(tuple_ty, *index).unwrap_or(*index) as i32;

                let elem_ptr = function.new_typed_value_id(MirType::pointer(element_type.clone()));
                instructions.push(
                    Instruction::get_element_ptr(
                        elem_ptr,
                        Value::operand(new_alloca),
                        Value::integer(offset),
                    )
                    .with_comment(format!(
                        "Get address (offset {}) of tuple element {} for update",
                        offset, index
                    )),
                );
                instructions.push(
                    Instruction::store(Value::operand(elem_ptr), *new_value, element_type)
                        .with_comment(format!("Update tuple element {}", index)),
                );

                self.aggregate_allocas.insert(*dest, new_alloca);
                instructions.push(
                    Instruction::assign(
                        *dest,
                        Value::operand(new_alloca),
                        MirType::pointer(tuple_ty.clone()),
                    )
                    .with_comment("Alias updated tuple to alloca".to_string()),
                );

                instructions
            }

            // Keep all other instructions as-is
            _ => vec![instruction.clone()],
        }
    }
}

impl MirPass for LowerAggregatesPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // Clone instructions to avoid borrow issues
        let blocks_instructions: Vec<Vec<Instruction>> = function
            .basic_blocks
            .iter()
            .map(|b| b.instructions.clone())
            .collect();

        // Process each block's instructions
        let mut blocks_new_instructions = Vec::new();
        for block_instructions in blocks_instructions {
            let mut new_instructions = Vec::new();

            for instruction in block_instructions {
                let lowered = self.lower_instruction(&instruction, function);
                if lowered.len() != 1 || !lowered[0].kind.eq(&instruction.kind) {
                    modified = true;
                }
                new_instructions.extend(lowered);
            }

            blocks_new_instructions.push(new_instructions);
        }

        // Now apply the changes
        for (block, new_instructions) in function
            .basic_blocks
            .iter_mut()
            .zip(blocks_new_instructions)
        {
            block.instructions = new_instructions;
        }

        modified
    }

    fn name(&self) -> &'static str {
        "LowerAggregates"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminator::Terminator;

    #[test]
    fn test_tuple_lowering() {
        let mut function = MirFunction::new("test".to_string());
        let mut pass = LowerAggregatesPass::new();

        // Create a simple function with MakeTuple and ExtractTupleElement
        let elem1 = function.new_typed_value_id(MirType::felt());
        let elem2 = function.new_typed_value_id(MirType::felt());
        let tuple_val =
            function.new_typed_value_id(MirType::Tuple(vec![MirType::felt(), MirType::felt()]));
        let extracted = function.new_typed_value_id(MirType::felt());

        let block_id = function.add_basic_block();
        let block = function.get_basic_block_mut(block_id).unwrap();
        block.instructions.push(Instruction::make_tuple(
            tuple_val,
            vec![Value::operand(elem1), Value::operand(elem2)],
        ));
        block.instructions.push(Instruction::extract_tuple_element(
            extracted,
            Value::operand(tuple_val),
            0,
            MirType::felt(),
        ));
        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(extracted)],
        });

        // Run the lowering pass
        let modified = pass.run(&mut function);
        assert!(modified);

        // Check that we now have frame_alloc, stores, GEP, and load
        // The block we added is at index block_id, not necessarily the entry block
        let lowered_block = &function.basic_blocks[block_id];

        let has_frame_alloc = lowered_block
            .instructions
            .iter()
            .any(|i| matches!(i.kind, InstructionKind::FrameAlloc { .. }));
        let has_store = lowered_block
            .instructions
            .iter()
            .any(|i| matches!(i.kind, InstructionKind::Store { .. }));
        let has_gep = lowered_block
            .instructions
            .iter()
            .any(|i| matches!(i.kind, InstructionKind::GetElementPtr { .. }));
        let has_load = lowered_block
            .instructions
            .iter()
            .any(|i| matches!(i.kind, InstructionKind::Load { .. }));

        assert!(has_frame_alloc, "Should have frame_alloc after lowering");
        assert!(has_store, "Should have store operations after lowering");
        assert!(has_gep, "Should have GEP after lowering");
        assert!(has_load, "Should have load after lowering");
    }

    #[test]
    fn test_struct_lowering() {
        let mut function = MirFunction::new("test".to_string());
        let mut pass = LowerAggregatesPass::new();

        // Create a simple function with MakeStruct and ExtractStructField
        let x_val = function.new_typed_value_id(MirType::felt());
        let y_val = function.new_typed_value_id(MirType::felt());
        let struct_type = MirType::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), MirType::felt()),
                ("y".to_string(), MirType::felt()),
            ],
        };
        let struct_val = function.new_typed_value_id(struct_type.clone());
        let extracted = function.new_typed_value_id(MirType::felt());

        let block_id = function.add_basic_block();
        let block = function.get_basic_block_mut(block_id).unwrap();
        block.instructions.push(Instruction::make_struct(
            struct_val,
            vec![
                ("x".to_string(), Value::operand(x_val)),
                ("y".to_string(), Value::operand(y_val)),
            ],
            struct_type,
        ));
        block.instructions.push(Instruction::extract_struct_field(
            extracted,
            Value::operand(struct_val),
            "x".to_string(),
            MirType::felt(),
        ));
        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(extracted)],
        });

        // Run the lowering pass
        let modified = pass.run(&mut function);
        assert!(modified);

        // Check that we now have memory operations
        // The block we added is at index block_id, not necessarily the entry block
        let lowered_block = &function.basic_blocks[block_id];
        let has_frame_alloc = lowered_block
            .instructions
            .iter()
            .any(|i| matches!(i.kind, InstructionKind::FrameAlloc { .. }));
        let has_store = lowered_block
            .instructions
            .iter()
            .any(|i| matches!(i.kind, InstructionKind::Store { .. }));

        assert!(has_frame_alloc, "Should have frame_alloc for struct");
        assert!(has_store, "Should have store operations for struct fields");
    }

    #[test]
    fn test_no_lowering_without_aggregates() {
        let mut function = MirFunction::new("test".to_string());
        let mut pass = LowerAggregatesPass::new();

        // Create a simple function without aggregate operations
        let a = function.new_typed_value_id(MirType::felt());
        let b = function.new_typed_value_id(MirType::felt());
        let c = function.new_typed_value_id(MirType::felt());

        let block_id = function.add_basic_block();
        let block = function.get_basic_block_mut(block_id).unwrap();
        block.instructions.push(Instruction::binary_op(
            crate::BinaryOp::Add,
            c,
            Value::operand(a),
            Value::operand(b),
        ));
        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(c)],
        });

        // Run the lowering pass
        let modified = pass.run(&mut function);
        assert!(!modified, "Should not modify functions without aggregates");
    }
}
