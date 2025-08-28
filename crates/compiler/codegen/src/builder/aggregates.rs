use cairo_m_compiler_mir::{DataLayout, Literal, MirType, Value, ValueId};

use crate::layout::ValueLayout;
use crate::{CodegenError, CodegenResult};

use super::ArrayOperation;

impl super::CasmBuilder {
    /// Creates a struct by allocating consecutive registers and copying field values
    pub(crate) fn make_struct(
        &mut self,
        dest: ValueId,
        fields: &[(String, Value)],
        struct_ty: &MirType,
    ) -> CodegenResult<()> {
        let total_size = DataLayout::memory_size_of(struct_ty);

        // Allocate destination
        let base_offset = self.layout.allocate_local(dest, total_size)?;

        // Copy each field to its offset
        for (field_name, field_value) in fields {
            let field_offset =
                DataLayout::field_offset(struct_ty, field_name).ok_or_else(|| {
                    CodegenError::InvalidMir(format!(
                        "Field '{}' not found in struct type",
                        field_name
                    ))
                })?;

            let target_offset = base_offset + field_offset as i32;

            // Get the field type to determine its size
            let field_ty = struct_ty.field_type(field_name).ok_or_else(|| {
                CodegenError::InvalidMir(format!("Could not get type for field '{}'", field_name))
            })?;
            let field_size = DataLayout::memory_size_of(field_ty);

            // Copy the field value to the target offset
            self.copy_value_to_offset(field_value, target_offset, field_size)?;
        }

        Ok(())
    }

    /// Extracts a field from a struct by mapping the destination to the field's offset
    pub(crate) fn extract_struct_field(
        &mut self,
        dest: ValueId,
        struct_val: Value,
        field_name: &str,
        field_ty: &MirType,
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        // Get struct base offset and ID
        let (struct_offset, struct_id) = match struct_val {
            Value::Operand(id) => (self.layout.get_offset(id)?, id),
            _ => {
                return Err(CodegenError::InvalidMir(
                    "ExtractStructField requires operand source".to_string(),
                ))
            }
        };

        // Get the struct type from the function's value_types
        let struct_ty = function.value_types.get(&struct_id).ok_or_else(|| {
            CodegenError::InvalidMir(format!("No type found for struct value {:?}", struct_id))
        })?;

        // Calculate field offset within the struct
        let field_offset = DataLayout::field_offset(struct_ty, field_name).ok_or_else(|| {
            CodegenError::InvalidMir(format!("Field '{}' not found in struct", field_name))
        })?;

        let field_size = DataLayout::memory_size_of(field_ty);
        let absolute_offset = struct_offset + field_offset as i32;

        // Map destination to the field's location
        if field_size == 1 {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::Slot {
                    offset: absolute_offset,
                },
            );
        } else {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::MultiSlot {
                    offset: absolute_offset,
                    size: field_size,
                },
            );
        }

        Ok(())
    }

    /// Inserts a new value into a struct field (in-place update)
    pub(crate) fn insert_struct_field(
        &mut self,
        dest: ValueId,
        struct_val: Value,
        field_name: &str,
        new_value: Value,
        struct_ty: &MirType,
    ) -> CodegenResult<()> {
        // Get struct base offset
        let struct_offset = match struct_val {
            Value::Operand(id) => self.layout.get_offset(id)?,
            _ => {
                return Err(CodegenError::InvalidMir(
                    "InsertField requires operand source".to_string(),
                ))
            }
        };

        // Calculate field offset
        let field_offset = DataLayout::field_offset(struct_ty, field_name).ok_or_else(|| {
            CodegenError::InvalidMir(format!("Field '{}' not found in struct", field_name))
        })?;

        // Get field type and size
        let field_ty = struct_ty.field_type(field_name).ok_or_else(|| {
            CodegenError::InvalidMir(format!("Could not get type for field '{}'", field_name))
        })?;
        let field_size = DataLayout::memory_size_of(field_ty);

        // Calculate target offset for the field
        let target_offset = struct_offset + field_offset as i32;

        // Overwrite the field with the new value
        self.copy_value_to_offset(&new_value, target_offset, field_size)?;

        // Map the destination to the same location as the source struct
        // (since it's an in-place update)
        let struct_size = DataLayout::memory_size_of(struct_ty);
        if struct_size == 1 {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::Slot {
                    offset: struct_offset,
                },
            );
        } else {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::MultiSlot {
                    offset: struct_offset,
                    size: struct_size,
                },
            );
        }

        Ok(())
    }
    /// Creates a tuple by allocating consecutive registers and copying element values
    pub(crate) fn make_tuple(
        &mut self,
        dest: ValueId,
        elements: &[Value],
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        // Determine the types of elements to calculate sizes
        let mut total_size = 0;
        let mut element_offsets = Vec::new();
        let mut element_sizes = Vec::new();

        for element in elements {
            element_offsets.push(total_size);

            // Determine element size from type information
            let element_size = match element {
                Value::Operand(id) => {
                    if let Some(ty) = function.value_types.get(id) {
                        DataLayout::memory_size_of(ty)
                    } else {
                        self.layout.get_value_size(*id)
                    }
                }
                Value::Literal(_) => 1, // Literals are always single-slot for now
                _ => 1,
            };

            element_sizes.push(element_size);
            total_size += element_size;
        }

        // Allocate destination
        let base_offset = self.layout.allocate_local(dest, total_size)?;

        // Copy each element to its offset
        for (i, element) in elements.iter().enumerate() {
            let target_offset = base_offset + element_offsets[i] as i32;
            let element_size = element_sizes[i];

            self.copy_value_to_offset(element, target_offset, element_size)?;
        }

        Ok(())
    }

    /// Extracts an element from a tuple by mapping the destination to the element's offset
    pub(crate) fn extract_tuple_element(
        &mut self,
        dest: ValueId,
        tuple: Value,
        index: usize,
        element_ty: &MirType,
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        // Get tuple base offset and ID
        let (tuple_offset, tuple_id) = match tuple {
            Value::Operand(id) => (self.layout.get_offset(id)?, id),
            _ => {
                return Err(CodegenError::InvalidMir(
                    "ExtractTupleElement requires operand source".to_string(),
                ))
            }
        };

        // Get the tuple type from the function's value_types
        let tuple_ty = function.value_types.get(&tuple_id).ok_or_else(|| {
            CodegenError::InvalidMir(format!("No type found for tuple value {:?}", tuple_id))
        })?;

        // Calculate element offset within the tuple
        let element_offset = DataLayout::tuple_offset(tuple_ty, index).ok_or_else(|| {
            CodegenError::InvalidMir(format!("Tuple index {} out of bounds", index))
        })?;

        let element_size = DataLayout::memory_size_of(element_ty);
        let absolute_offset = tuple_offset + element_offset as i32;

        // Map destination to the element's location
        if element_size == 1 {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::Slot {
                    offset: absolute_offset,
                },
            );
        } else {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::MultiSlot {
                    offset: absolute_offset,
                    size: element_size,
                },
            );
        }

        Ok(())
    }

    /// Inserts a new value into a tuple element (in-place update)
    pub(crate) fn insert_tuple_element(
        &mut self,
        dest: ValueId,
        tuple_val: Value,
        index: usize,
        new_value: Value,
        tuple_ty: &MirType,
    ) -> CodegenResult<()> {
        // Get tuple base offset
        let tuple_offset = match tuple_val {
            Value::Operand(id) => self.layout.get_offset(id)?,
            _ => {
                return Err(CodegenError::InvalidMir(
                    "InsertTuple requires operand source".to_string(),
                ))
            }
        };

        // Calculate element offset
        let element_offset = DataLayout::tuple_offset(tuple_ty, index).ok_or_else(|| {
            CodegenError::InvalidMir(format!("Tuple index {} out of bounds", index))
        })?;

        // Get element type and size
        let element_ty = tuple_ty.tuple_element_type(index).ok_or_else(|| {
            CodegenError::InvalidMir(format!("Could not get type for tuple element {}", index))
        })?;
        let element_size = DataLayout::memory_size_of(element_ty);

        // Calculate target offset for the element
        let target_offset = tuple_offset + element_offset as i32;

        // Overwrite the element with the new value
        self.copy_value_to_offset(&new_value, target_offset, element_size)?;

        // Map the destination to the same location as the source tuple
        // (since it's an in-place update)
        let tuple_size = DataLayout::memory_size_of(tuple_ty);
        if tuple_size == 1 {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::Slot {
                    offset: tuple_offset,
                },
            );
        } else {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::MultiSlot {
                    offset: tuple_offset,
                    size: tuple_size,
                },
            );
        }

        Ok(())
    }

    /// Create a fixed-size array from elements
    /// Materializes elements in contiguous locals and returns a pointer (fp + base)
    pub(crate) fn make_fixed_array(
        &mut self,
        dest: ValueId,
        elements: &[Value],
        element_ty: &MirType,
    ) -> CodegenResult<()> {
        // Calculate per-element size and total size needed for the array
        let element_size = DataLayout::value_size_of(element_ty);
        let total_size = element_size * elements.len();

        // Reserve space for the array elements (anonymous region)
        let base_offset = if total_size > 0 {
            self.layout.reserve_stack(total_size)
        } else {
            // Zero-sized array: still produce a pointer to the current top (valid but unused)
            self.layout.current_frame_usage()
        };

        // Copy each element to its position in the array
        for (index, element) in elements.iter().enumerate() {
            let target_offset = base_offset + (index * element_size) as i32;
            self.copy_value_to_offset(element, target_offset, element_size)?;
        }

        // Allocate a single-slot destination for the array pointer
        let dest_offset = self.layout.allocate_local(dest, 1)?;
        // Store the address (fp + base_offset) into the destination slot
        self.store_fp_plus_imm(
            base_offset,
            dest_offset,
            format!("[fp + {dest_offset}] = fp + {base_offset}"),
        );

        Ok(())
    }

    /// Unified array operation handler that dispatches based on index type and operation
    pub(crate) fn array_operation(
        &mut self,
        array: Value,
        index: Value,
        element_ty: &MirType,
        operation: ArrayOperation,
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        // Get array base pointer (arrays are always stored as pointers)
        let array_offset = match array {
            Value::Operand(id) => self.layout.get_offset(id)?,
            _ => {
                return Err(CodegenError::InvalidMir(
                    "Array must be an operand (pointer)".to_string(),
                ))
            }
        };

        // Calculate element size
        let element_size = DataLayout::value_size_of(element_ty);

        // Handle based on index type
        match index {
            Value::Literal(Literal::Integer(idx)) => {
                // Static index - compile-time offset calculation
                let element_offset = (idx as i32) * (element_size as i32);

                match operation {
                    ArrayOperation::Load { dest } => {
                        self.load_from_memory_static(
                            dest,
                            array_offset,
                            element_offset,
                            element_ty,
                        )?;
                    }
                    ArrayOperation::Store { dest, value } => {
                        self.store_to_memory_static(
                            dest,
                            value,
                            array_offset,
                            element_offset,
                            element_ty,
                            function,
                        )?;
                    }
                }
            }
            Value::Operand(idx_id) => {
                // Dynamic index - runtime offset calculation
                let _idx_offset = self.layout.get_offset(idx_id)?;
                let idx_value_layout = self.layout.value_layouts.get(&idx_id).unwrap().clone();

                // Enforce: indexing is only valid with felt (single-slot) values.
                // This avoids accidental use of multi-slot types (e.g., u32) as an index.
                if let Some(idx_ty) = function.value_types.get(&idx_id) {
                    if !matches!(idx_ty, MirType::Felt) {
                        return Err(CodegenError::InvalidMir(format!(
                            "Array index must be a felt; got {:?}",
                            idx_ty
                        )));
                    }
                }

                let indexing_value_offset = match idx_value_layout {
                    ValueLayout::Slot { offset } => offset,
                    _ => {
                        return Err(CodegenError::InternalError(
                            "Invalid index value layout".to_string(),
                        ));
                    }
                };

                match operation {
                    ArrayOperation::Load { dest } => {
                        self.load_from_memory_dynamic(
                            dest,
                            array_offset,
                            indexing_value_offset,
                            element_ty,
                        )?;
                    }
                    ArrayOperation::Store { dest, value } => {
                        self.store_to_memory_dynamic(
                            dest,
                            value,
                            array_offset,
                            indexing_value_offset,
                            element_ty,
                            function,
                        )?;
                    }
                }
            }
            _ => {
                return Err(CodegenError::InvalidMir(
                    "Array index must be literal or operand".to_string(),
                ))
            }
        }

        Ok(())
    }

    /// Helper for static loads - arrays store pointers so we load from computed address
    fn load_from_memory_static(
        &mut self,
        dest: ValueId,
        base_offset: i32,
        element_offset: i32,
        ty: &MirType,
    ) -> CodegenResult<()> {
        // Arrays store pointers to their data, so we need to dereference:
        // element N is at memory address [[fp + base_offset] + element_offset]
        let elem_size = DataLayout::value_size_of(ty);
        let dest_off = self.layout.allocate_local(dest, elem_size)?;

        // For static offsets, use StoreDoubleDerefFpImm
        // Load slot 0: [fp + dest_off] = [[fp + base_offset] + element_offset]
        self.store_from_double_deref_fp_imm(
            base_offset,
            element_offset,
            dest_off,
            format!(
                "[fp + {}] = [[fp + {}] + {}] (load array element)",
                dest_off, base_offset, element_offset
            ),
        );

        // For multi-slot elements (like u32), load additional slots
        for s in 1..elem_size {
            let slot_offset = element_offset + s as i32;
            let dst_slot = dest_off + s as i32;
            self.store_from_double_deref_fp_imm(
                base_offset,
                slot_offset,
                dst_slot,
                format!(
                    "[fp + {}] = [[fp + {}] + {}] (load array element slot {})",
                    dst_slot, base_offset, slot_offset, s
                ),
            );
        }

        Ok(())
    }

    /// Helper for dynamic loads - use STORE_DOUBLE_DEREF_FP_FP to load from memory
    fn load_from_memory_dynamic(
        &mut self,
        dest: ValueId,
        base_offset: i32,
        indexing_value_offset: i32,
        ty: &MirType,
    ) -> CodegenResult<()> {
        let elem_size = DataLayout::value_size_of(ty);
        let dest_off = self.layout.allocate_local(dest, elem_size)?;

        // If the elem_size is N, then, the value we have to retrieved is located at [fp + base_ptr + [fp + scaled_offset] * N]
        // e.g. for a array of u32, index 1, the value is located at [fp + base_ptr + (1 * 2)]

        let scaled_offset = if elem_size != 1 {
            // First, multiply the index by the element size
            let scaled_offset_ = self.layout.reserve_stack(1);
            self.felt_mul_fp_imm(indexing_value_offset, elem_size as i32, scaled_offset_, format!("[fp + {scaled_offset_}] = [fp + {indexing_value_offset}] * {elem_size} - Scale index by element size"));
            scaled_offset_
        } else {
            indexing_value_offset
        };

        // Load slot 0
        self.store_from_double_deref_fp_fp(
            base_offset,
            scaled_offset,
            dest_off,
            format!(
                "[fp + {}] = [[fp + {}] + [fp + {}]]",
                dest_off, base_offset, scaled_offset
            ),
        );

        // Additional slots if element spans multiple words (e.g., U32)
        for s in 1..elem_size {
            // temp_index = scaled_offset + s
            let tmp_idx = self.layout.reserve_stack(1);
            self.felt_add_fp_imm(
                scaled_offset,
                s as i32,
                tmp_idx,
                format!(
                    "[fp + {}] = [fp + {}] + {} (offset for slot {})",
                    tmp_idx, scaled_offset, s, s
                ),
            );

            let dst_slot = dest_off + s as i32;
            self.store_from_double_deref_fp_fp(
                base_offset,
                tmp_idx,
                dst_slot,
                format!(
                    "[fp + {}] = [[fp + {}] + [fp + {}]] (slot {})",
                    dst_slot, base_offset, tmp_idx, s
                ),
            );
        }

        Ok(())
    }

    /// Helper for static stores using StoreToDoubleDerefFpImm
    fn store_to_memory_static(
        &mut self,
        dest: ValueId,
        value: Value,
        base_offset: i32,
        element_offset: i32,
        ty: &MirType,
        _function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        // For static stores, we just copy the pointer (pointer-semantics for arrays)
        // The dest gets the same pointer as the original array
        let dest_offset = self.layout.allocate_local(dest, 1)?;

        // Store the array pointer to dest
        self.store_copy_single(
            base_offset,
            dest_offset,
            format!(
                "[fp + {}] = [fp + {}] + 0 (copy array pointer)",
                dest_offset, base_offset
            ),
        );

        // Now store the value to the array element
        match value {
            Value::Operand(src_id) => {
                let src_offset = self.layout.get_offset(src_id)?;
                let elem_size = DataLayout::value_size_of(ty);
                for i in 0..elem_size {
                    self.store_to_double_deref_fp_imm(
                        base_offset,
                        element_offset + i as i32,
                        src_offset + i as i32,
                        format!(
                            "[[fp + {}] + {}] = [fp + {}] (slot {})",
                            base_offset,
                            element_offset + i as i32,
                            src_offset + i as i32,
                            i
                        ),
                    );
                }
            }
            Value::Literal(Literal::Integer(val)) => {
                // Multi-slot immediates require staging in temporaries.
                let elem_size = DataLayout::value_size_of(ty);
                if elem_size == 1 {
                    // Single-slot element: stage in one temp then store.
                    let temp_offset = self.layout.reserve_stack(1);
                    self.store_immediate(
                        val,
                        temp_offset,
                        format!("[fp + {}] = {}", temp_offset, val),
                    );
                    self.store_to_double_deref_fp_imm(
                        base_offset,
                        element_offset,
                        temp_offset,
                        format!(
                            "[[fp + {}] + {}] = [fp + {}]",
                            base_offset, element_offset, temp_offset
                        ),
                    );
                } else if matches!(ty, MirType::U32) && elem_size == 2 {
                    // Stage u32 immediate into a 2-slot temp and store both slots.
                    let tmp = self.layout.reserve_stack(2);
                    self.store_u32_immediate(
                        val,
                        tmp,
                        format!("[fp + {}], [fp + {}] = u32({val})", tmp, tmp + 1),
                    );
                    for i in 0..2 {
                        self.store_to_double_deref_fp_imm(
                            base_offset,
                            element_offset + i,
                            tmp + i,
                            format!(
                                "[[fp + {}] + {}] = [fp + {}] (u32 slot {})",
                                base_offset,
                                element_offset + i,
                                tmp + i,
                                i
                            ),
                        );
                    }
                } else {
                    return Err(CodegenError::UnsupportedInstruction(
                        "Storing immediate into multi-slot element is unsupported".to_string(),
                    ));
                }
            }
            Value::Literal(Literal::Boolean(b)) => {
                // Boolean literal only valid for single-slot element types
                if DataLayout::value_size_of(ty) != 1 {
                    return Err(CodegenError::InvalidMir(
                        "Boolean literal store into multi-slot element".to_string(),
                    ));
                }
                let val = if b { 1 } else { 0 };
                let temp_offset = self.layout.reserve_stack(1);
                self.store_immediate(
                    val,
                    temp_offset,
                    format!("[fp + {}] = {}", temp_offset, val),
                );
                self.store_to_double_deref_fp_imm(
                    base_offset,
                    element_offset,
                    temp_offset,
                    format!(
                        "[[fp + {}] + {}] = [fp + {}]",
                        base_offset, element_offset, temp_offset
                    ),
                );
            }
            _ => {
                return Err(CodegenError::InvalidMir(
                    "Invalid value for array store".to_string(),
                ))
            }
        }
        Ok(())
    }

    /// Helper for dynamic stores using StoreToDoubleDerefFpFp
    fn store_to_memory_dynamic(
        &mut self,
        dest: ValueId,
        value: Value,
        base_offset: i32,
        indexing_value_offset: i32,
        ty: &MirType,
        _function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        // For dynamic stores, we just copy the pointer (pointer-semantics for arrays)
        // The dest gets the same pointer as the original array
        let elem_size = DataLayout::value_size_of(ty);
        let dest_offset = self.layout.allocate_local(dest, 1)?;

        let scaled_offset = if elem_size != 1 {
            // First, multiply the index by the element size
            let scaled_offset_ = self.layout.reserve_stack(1);
            self.felt_mul_fp_imm(indexing_value_offset, elem_size as i32, scaled_offset_, format!("[fp + {scaled_offset_}] = [fp + {indexing_value_offset}] * {elem_size} - Scale index by element size"));
            scaled_offset_
        } else {
            indexing_value_offset
        };

        // Store the array pointer to dest
        self.store_copy_single(
            base_offset,
            dest_offset,
            format!(
                "[fp + {}] = [fp + {}] + 0 (copy array pointer)",
                dest_offset, base_offset
            ),
        );

        // Now store the value to the array element
        match value {
            Value::Operand(src_id) => {
                let src_offset = self.layout.get_offset(src_id)?;

                for i in 0..elem_size {
                    // Compute adjusted offset for multi-slot elements
                    let off_slot = if i == 0 {
                        scaled_offset
                    } else {
                        let adjusted = self.layout.reserve_stack(1);
                        self.felt_add_fp_imm(
                            scaled_offset,
                            i as i32,
                            adjusted,
                            format!(
                                "[fp + {}] = [fp + {}] + {} (adjust for slot {})",
                                adjusted, scaled_offset, i, i
                            ),
                        );
                        adjusted
                    };

                    self.store_to_double_deref_fp_fp(
                        base_offset,
                        off_slot,
                        src_offset + i as i32,
                        format!(
                            "[[fp + {}] + [fp + {}]] = [fp + {}] (slot {})",
                            base_offset,
                            off_slot,
                            src_offset + i as i32,
                            i
                        ),
                    );
                }
            }
            Value::Literal(Literal::Integer(val)) => {
                if elem_size == 1 {
                    // Stage single-slot immediate in temp
                    let temp_offset = self.layout.reserve_stack(1);
                    self.store_immediate(
                        val,
                        temp_offset,
                        format!("[fp + {}] = {}", temp_offset, val),
                    );

                    self.store_to_double_deref_fp_fp(
                        base_offset,
                        scaled_offset,
                        temp_offset,
                        format!(
                            "[[fp + {}] + [fp + {}]] = [fp + {}]",
                            base_offset, scaled_offset, temp_offset
                        ),
                    );
                } else if matches!(ty, MirType::U32) && elem_size == 2 {
                    // Stage u32 immediate in 2-slot temp and store both slots
                    let tmp = self.layout.reserve_stack(2);
                    self.store_u32_immediate(
                        val,
                        tmp,
                        format!("[fp + {}], [fp + {}] = u32({val})", tmp, tmp + 1),
                    );

                    for i in 0..2 {
                        let off_slot = if i == 0 {
                            scaled_offset
                        } else {
                            let adjusted = self.layout.reserve_stack(1);
                            self.felt_add_fp_imm(
                                scaled_offset,
                                i,
                                adjusted,
                                format!(
                                    "[fp + {}] = [fp + {}] + {} (adjust for slot {})",
                                    adjusted, scaled_offset, i, i
                                ),
                            );
                            adjusted
                        };

                        self.store_to_double_deref_fp_fp(
                            base_offset,
                            off_slot,
                            tmp + i,
                            format!(
                                "[[fp + {}] + [fp + {}]] = [fp + {}] (u32 slot {})",
                                base_offset,
                                off_slot,
                                tmp + i,
                                i
                            ),
                        );
                    }
                } else {
                    return Err(CodegenError::UnsupportedInstruction(
                        "Storing immediate into multi-slot element is unsupported".to_string(),
                    ));
                }
            }
            Value::Literal(Literal::Boolean(b)) => {
                if elem_size != 1 {
                    return Err(CodegenError::InvalidMir(
                        "Boolean literal store into multi-slot element".to_string(),
                    ));
                }
                let val = if b { 1 } else { 0 };
                let temp_offset = self.layout.reserve_stack(1);
                self.store_immediate(
                    val,
                    temp_offset,
                    format!("[fp + {}] = {}", temp_offset, val),
                );
                self.store_to_double_deref_fp_fp(
                    base_offset,
                    scaled_offset,
                    temp_offset,
                    format!(
                        "[[fp + {}] + [fp + {}]] = [fp + {}]",
                        base_offset, scaled_offset, temp_offset
                    ),
                );
            }
            _ => {
                return Err(CodegenError::InvalidMir(
                    "Invalid value for array store".to_string(),
                ))
            }
        }
        Ok(())
    }
}
