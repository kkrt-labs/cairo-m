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

        // Try to read the destination tuple type to infer sizes for literal elements
        let dest_tuple_types: Option<Vec<MirType>> =
            function.value_types.get(&dest).and_then(|ty| match ty {
                MirType::Tuple(ts) => Some(ts.clone()),
                _ => None,
            });

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
                Value::Literal(_) => {
                    // Prefer tuple element type if available; otherwise assume single slot
                    if let Some(ts) = &dest_tuple_types {
                        let idx = element_offsets.len() - 1; // current element index
                        if let Some(elem_ty) = ts.get(idx) {
                            DataLayout::memory_size_of(elem_ty)
                        } else {
                            1
                        }
                    } else {
                        1
                    }
                }
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
            // Skip writing zeroes in memory.
            if element == &Value::Literal(Literal::Integer(0)) {
                continue;
            }
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
                        src_offset + i as i32,
                        base_offset,
                        element_offset + i as i32,
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
                        temp_offset,
                        base_offset,
                        element_offset,
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
                            tmp + i,
                            base_offset,
                            element_offset + i,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        builder::CasmBuilder,
        layout::FunctionLayout,
        test_support::{exec, Mem},
    };
    use cairo_m_common::instruction::{
        STORE_ADD_FP_IMM, STORE_DOUBLE_DEREF_FP, STORE_DOUBLE_DEREF_FP_FP, STORE_FRAME_POINTER,
        STORE_IMM, STORE_MUL_FP_IMM, STORE_TO_DOUBLE_DEREF_FP_IMM, U32_STORE_IMM,
    };
    use cairo_m_compiler_mir::{MirFunction, MirType, Value, ValueId};
    use proptest::prelude::*;

    // =========================================================================
    // Test Setup Helpers
    // =========================================================================

    fn mk_builder_with_struct_type() -> (CasmBuilder, MirFunction) {
        let layout = FunctionLayout::new_for_test();
        let builder = CasmBuilder::new(layout, 0);
        // Minimal function context for type lookups
        let function = MirFunction::new("test".to_string());
        (builder, function)
    }

    fn mk_builder_with_tuple_type() -> (CasmBuilder, MirFunction) {
        let layout = FunctionLayout::new_for_test();
        let builder = CasmBuilder::new(layout, 0);
        // Minimal function context for type lookups
        let function = MirFunction::new("test".to_string());
        (builder, function)
    }

    fn mk_builder_with_array() -> (CasmBuilder, ValueId) {
        let mut layout = FunctionLayout::new_for_test();
        let array_id = ValueId::from_raw(1);
        // Arrays are stored as pointers (1 slot)
        layout.allocate_value(array_id, 1).unwrap();
        let mut builder = CasmBuilder::new(layout, 0);

        // Create an array pointer at fp+0 pointing to fp+10 (where array data would be)
        builder.store_fp_plus_imm(10, 0, "[fp + 0] = fp + 10".to_string());
        (builder, array_id)
    }

    // =========================================================================
    // Struct Tests
    // =========================================================================

    #[test]
    fn test_make_struct_simple() {
        let (mut b, _function) = mk_builder_with_struct_type();
        let dest = ValueId::from_raw(10);

        // Create struct type
        let struct_ty = MirType::Struct {
            name: "TestStruct".to_string(),
            fields: vec![
                ("x".to_string(), MirType::Felt),
                ("y".to_string(), MirType::Felt),
            ],
        };

        // Create struct with literal values
        let fields = vec![
            ("x".to_string(), Value::integer(42)),
            ("y".to_string(), Value::integer(100)),
        ];

        b.make_struct(dest, &fields, &struct_ty).unwrap();

        // Dest layout must be 2 contiguous slots
        let base = b.layout.get_offset(dest).unwrap();
        match b.layout.value_layouts.get(&dest) {
            Some(ValueLayout::MultiSlot { offset, size }) => {
                assert_eq!(*offset, base);
                assert_eq!(*size, 2);
            }
            _ => panic!("expected MultiSlot layout for struct"),
        }
        // Execute and verify memory contents
        let mut mem = Mem::new(32);
        exec(&mut mem, &b.instructions).unwrap();
        assert_eq!(mem.get(base).0, 42);
        assert_eq!(mem.get(base + 1).0, 100);
    }

    #[test]
    fn test_make_struct_with_u32_field() {
        let (mut b, _function) = mk_builder_with_struct_type();
        let dest = ValueId::from_raw(10);

        // Create struct type with u32 field
        let struct_ty = MirType::Struct {
            name: "TestStruct".to_string(),
            fields: vec![
                ("id".to_string(), MirType::U32),
                ("flag".to_string(), MirType::Felt),
            ],
        };

        // Create struct with values
        let fields = vec![
            ("id".to_string(), Value::integer(0x12345678)),
            ("flag".to_string(), Value::integer(1)),
        ];

        b.make_struct(dest, &fields, &struct_ty).unwrap();

        let base = b.layout.get_offset(dest).unwrap();
        match b.layout.value_layouts.get(&dest) {
            Some(ValueLayout::MultiSlot { offset, size }) => {
                assert_eq!(*offset, base);
                assert_eq!(*size, 3);
            }
            _ => panic!("expected MultiSlot layout for struct with u32"),
        }
        let mut mem = Mem::new(32);
        exec(&mut mem, &b.instructions).unwrap();
        assert_eq!(mem.get_u32(base), 0x1234_5678);
        assert_eq!(mem.get(base + 2).0, 1);
    }

    #[test]
    fn test_extract_struct_field() {
        let (mut b, mut function) = mk_builder_with_struct_type();

        // First create a struct
        let struct_id = ValueId::from_raw(10);
        let struct_ty = MirType::Struct {
            name: "TestStruct".to_string(),
            fields: vec![
                ("x".to_string(), MirType::Felt),
                ("y".to_string(), MirType::Felt),
            ],
        };

        // Manually allocate struct at known location
        b.layout.allocate_value(struct_id, 2).unwrap();
        function.value_types.insert(struct_id, struct_ty);

        // Extract field "y"
        let dest = ValueId::from_raw(20);
        let field_ty = MirType::Felt;

        b.extract_struct_field(dest, Value::operand(struct_id), "y", &field_ty, &function)
            .unwrap();

        // Dest should be mapped to the field's offset
        assert!(b.layout.value_layouts.contains_key(&dest));
        match b.layout.value_layouts.get(&dest) {
            Some(ValueLayout::Slot { offset }) => {
                // "y" is at offset 1 from struct base
                assert_eq!(*offset, 1);
            }
            _ => panic!("Expected Slot layout for extracted field"),
        }
    }

    #[test]
    fn test_insert_struct_field() {
        let (mut b, _function) = mk_builder_with_struct_type();

        // Create struct
        let struct_id = ValueId::from_raw(10);
        let struct_ty = MirType::Struct {
            name: "TestStruct".to_string(),
            fields: vec![
                ("x".to_string(), MirType::Felt),
                ("y".to_string(), MirType::Felt),
            ],
        };

        b.layout.allocate_value(struct_id, 2).unwrap();

        // Insert new value into field "x"
        let dest = ValueId::from_raw(20);
        let new_value = Value::integer(999);

        b.insert_struct_field(dest, Value::operand(struct_id), "x", new_value, &struct_ty)
            .unwrap();

        // Execute and confirm write at struct base offset
        let base = b.layout.get_offset(struct_id).unwrap();
        let mut mem = Mem::new(32);
        exec(&mut mem, &b.instructions).unwrap();
        assert_eq!(mem.get(base).0, 999);
        // Dest should map to same tuple/struct base (in-place update maps dest to struct location)
        match b.layout.value_layouts.get(&dest) {
            Some(ValueLayout::MultiSlot { offset, size }) => {
                assert_eq!(*offset, base);
                assert_eq!(*size, 2);
            }
            Some(ValueLayout::Slot { offset }) => assert_eq!(*offset, base),
            _ => panic!("unexpected layout for dest after insert"),
        }
    }

    // =========================================================================
    // Tuple Tests
    // =========================================================================

    #[test]
    fn test_make_tuple_simple() {
        let (mut b, function) = mk_builder_with_tuple_type();
        let dest = ValueId::from_raw(10);

        // Create tuple with two felt elements
        let elements = vec![Value::integer(42), Value::integer(100)];

        b.make_tuple(dest, &elements, &function).unwrap();

        let base = b.layout.get_offset(dest).unwrap();
        match b.layout.value_layouts.get(&dest) {
            Some(ValueLayout::MultiSlot { offset, size }) => {
                assert_eq!(*offset, base);
                assert_eq!(*size, 2);
            }
            _ => panic!("expected MultiSlot for 2-element tuple"),
        }
        // Execute and verify both elements were stored
        let mut mem = Mem::new(32);
        exec(&mut mem, &b.instructions).unwrap();
        assert_eq!(mem.get(base).0, 42);
        assert_eq!(mem.get(base + 1).0, 100);
    }

    #[test]
    fn test_make_tuple_mixed_types() {
        let (mut b, mut function) = mk_builder_with_tuple_type();
        let dest = ValueId::from_raw(10);

        // Create elements with known types
        let elem1_id = ValueId::from_raw(1);
        let elem2_id = ValueId::from_raw(2);

        function.value_types.insert(elem1_id, MirType::U32);
        function.value_types.insert(elem2_id, MirType::Felt);

        b.layout.allocate_value(elem1_id, 2).unwrap();
        b.layout.allocate_value(elem2_id, 1).unwrap();

        let elements = vec![Value::operand(elem1_id), Value::operand(elem2_id)];

        b.make_tuple(dest, &elements, &function).unwrap();

        // Prepare memory with source operand values and execute
        let mut mem = Mem::new(64);
        let u32_src_off = b.layout.get_offset(elem1_id).unwrap();
        let felt_src_off = b.layout.get_offset(elem2_id).unwrap();
        mem.set_u32(u32_src_off, 0xCAFE_BABE);
        mem.set(felt_src_off, stwo_prover::core::fields::m31::M31::from(77));
        exec(&mut mem, &b.instructions).unwrap();

        let base = b.layout.get_offset(dest).unwrap();
        match b.layout.value_layouts.get(&dest) {
            Some(ValueLayout::MultiSlot { size, .. }) => assert_eq!(*size, 3),
            _ => panic!("expected MultiSlot for mixed tuple"),
        }
        assert_eq!(mem.get_u32(base), 0xCAFE_BABE);
        assert_eq!(mem.get(base + 2).0, 77);
    }

    #[test]
    fn test_extract_tuple_element() {
        let (mut b, mut function) = mk_builder_with_tuple_type();

        // Create a tuple (felt, felt)
        let tuple_id = ValueId::from_raw(10);
        let tuple_ty = MirType::Tuple(vec![MirType::Felt, MirType::Felt]);

        b.layout.allocate_value(tuple_id, 2).unwrap();
        function.value_types.insert(tuple_id, tuple_ty);

        // Extract element at index 1
        let dest = ValueId::from_raw(20);
        let element_ty = MirType::Felt;

        b.extract_tuple_element(dest, Value::operand(tuple_id), 1, &element_ty, &function)
            .unwrap();

        // Dest should be mapped to element's offset relative to tuple base
        assert!(b.layout.value_layouts.contains_key(&dest));
        match b.layout.value_layouts.get(&dest) {
            Some(ValueLayout::Slot { offset }) => {
                assert_eq!(*offset, b.layout.get_offset(tuple_id).unwrap() + 1);
            }
            _ => panic!("Expected Slot layout for extracted element"),
        }
    }

    #[test]
    fn test_insert_tuple_element() {
        let (mut b, _function) = mk_builder_with_tuple_type();

        // Create tuple
        let tuple_id = ValueId::from_raw(10);
        let tuple_ty = MirType::Tuple(vec![MirType::Felt, MirType::Felt]);

        b.layout.allocate_value(tuple_id, 2).unwrap();

        // Insert new value at index 0
        let dest = ValueId::from_raw(20);
        let new_value = Value::integer(777);

        b.insert_tuple_element(dest, Value::operand(tuple_id), 0, new_value, &tuple_ty)
            .unwrap();

        // Execute and confirm write at tuple base
        let base = b.layout.get_offset(tuple_id).unwrap();
        let mut mem = Mem::new(32);
        exec(&mut mem, &b.instructions).unwrap();
        assert_eq!(mem.get(base).0, 777);

        // Dest should map to same location as source (in-place update)
        match b.layout.value_layouts.get(&dest) {
            Some(ValueLayout::MultiSlot { offset, size }) => {
                assert_eq!(*offset, base);
                assert_eq!(*size, 2);
            }
            _ => panic!("Expected MultiSlot layout for inserted element"),
        }
    }

    #[test]
    fn test_extract_tuple_element_mixed_types() {
        let (mut b, mut function) = mk_builder_with_tuple_type();

        // Tuple (u32, felt)
        let tuple_id = ValueId::from_raw(10);
        let tuple_ty = MirType::Tuple(vec![MirType::U32, MirType::Felt]);

        b.layout.allocate_value(tuple_id, 3).unwrap();
        function.value_types.insert(tuple_id, tuple_ty);

        // Extract index 0 (u32)
        let dest0 = ValueId::from_raw(20);
        b.extract_tuple_element(dest0, Value::operand(tuple_id), 0, &MirType::U32, &function)
            .unwrap();
        match b.layout.value_layouts.get(&dest0) {
            Some(ValueLayout::MultiSlot { offset, size }) => {
                assert_eq!(*offset, b.layout.get_offset(tuple_id).unwrap());
                assert_eq!(*size, 2);
            }
            _ => panic!("Expected MultiSlot for u32 extraction"),
        }

        // Extract index 1 (felt)
        let dest1 = ValueId::from_raw(21);
        b.extract_tuple_element(
            dest1,
            Value::operand(tuple_id),
            1,
            &MirType::Felt,
            &function,
        )
        .unwrap();
        match b.layout.value_layouts.get(&dest1) {
            Some(ValueLayout::Slot { offset }) => {
                assert_eq!(*offset, b.layout.get_offset(tuple_id).unwrap() + 2);
            }
            _ => panic!("Expected Slot for felt extraction"),
        }
    }

    #[test]
    fn test_insert_tuple_element_u32_second() {
        let (mut b, _function) = mk_builder_with_tuple_type();

        // Tuple (felt, u32)
        let tuple_id = ValueId::from_raw(10);
        let tuple_ty = MirType::Tuple(vec![MirType::Felt, MirType::U32]);

        b.layout.allocate_value(tuple_id, 3).unwrap();

        let dest = ValueId::from_raw(20);
        let new_val = 0xDEAD_BEEFu32;

        b.insert_tuple_element(
            dest,
            Value::operand(tuple_id),
            1,
            Value::integer(new_val),
            &tuple_ty,
        )
        .unwrap();

        // Execute and check both slots written
        let base = b.layout.get_offset(tuple_id).unwrap();
        let mut mem = Mem::new(64);
        exec(&mut mem, &b.instructions).unwrap();
        assert_eq!(mem.get_u32(base + 1), new_val);

        // Dest should cover the whole tuple layout
        match b.layout.value_layouts.get(&dest) {
            Some(ValueLayout::MultiSlot { offset, size }) => {
                assert_eq!(*offset, base);
                assert_eq!(*size, 3);
            }
            _ => panic!("Expected MultiSlot for tuple after insert"),
        }
    }

    // =========================================================================
    // Array Tests
    // =========================================================================

    #[test]
    fn test_make_fixed_array_empty() {
        let layout = FunctionLayout::new_for_test();
        let mut b = CasmBuilder::new(layout, 0);
        let dest = ValueId::from_raw(10);

        // Create empty array
        let elements = vec![];
        let element_ty = MirType::Felt;

        b.make_fixed_array(dest, &elements, &element_ty).unwrap();

        // Should allocate pointer slot
        assert!(b.layout.value_layouts.contains_key(&dest));
        // Should store array pointer (at least one instruction)
        assert!(!b.instructions.is_empty());
    }

    #[test]
    fn test_make_fixed_array_felt_elements() {
        let layout = FunctionLayout::new_for_test();
        let mut b = CasmBuilder::new(layout, 0);
        let dest = ValueId::from_raw(10);

        // Create array with 3 felt elements
        let elements = vec![Value::integer(10), Value::integer(20), Value::integer(30)];
        let element_ty = MirType::Felt;

        b.make_fixed_array(dest, &elements, &element_ty).unwrap();

        // One pointer store + one store per element
        let ptr_stores = b
            .instructions
            .iter()
            .filter(|i| i.inner_instr().opcode_value() == STORE_FRAME_POINTER)
            .count();
        let imm_stores = b
            .instructions
            .iter()
            .filter(|i| i.inner_instr().opcode_value() == STORE_IMM)
            .count();
        assert_eq!(ptr_stores, 1);
        assert_eq!(imm_stores, 3);
    }

    #[test]
    fn test_make_fixed_array_u32_elements() {
        let layout = FunctionLayout::new_for_test();
        let mut b = CasmBuilder::new(layout, 0);
        let dest = ValueId::from_raw(10);

        // Create array with u32 elements
        let elements = vec![Value::integer(0x1000), Value::integer(0x2000)];
        let element_ty = MirType::U32;

        b.make_fixed_array(dest, &elements, &element_ty).unwrap();

        // One pointer store + two u32 immediate stores
        let ptr_stores = b
            .instructions
            .iter()
            .filter(|i| i.inner_instr().opcode_value() == STORE_FRAME_POINTER)
            .count();
        let u32_imm_stores = b
            .instructions
            .iter()
            .filter(|i| i.inner_instr().opcode_value() == U32_STORE_IMM)
            .count();
        assert_eq!(ptr_stores, 1);
        assert_eq!(u32_imm_stores, 2);
    }

    #[test]
    fn test_array_load_static_index() {
        let (mut b, array_id) = mk_builder_with_array();
        let function = MirFunction::new("test".to_string());

        // Load element at index 2
        let dest = ValueId::from_raw(20);
        let index = Value::integer(2);
        let element_ty = MirType::Felt;

        let orig_len = b.instructions.len();
        b.array_operation(
            Value::operand(array_id),
            index,
            &element_ty,
            ArrayOperation::Load { dest },
            &function,
        )
        .unwrap();

        // Expect one double-deref load instruction appended
        assert_eq!(
            b.instructions[orig_len].inner_instr().opcode_value(),
            STORE_DOUBLE_DEREF_FP
        );
    }

    #[test]
    fn test_array_store_static_index() {
        let (mut b, array_id) = mk_builder_with_array();
        let function = MirFunction::new("test".to_string());

        // Store value at index 1
        let dest = ValueId::from_raw(20);
        let index = Value::integer(1);
        let value = Value::integer(999);
        let element_ty = MirType::Felt;

        b.array_operation(
            Value::operand(array_id),
            index,
            &element_ty,
            ArrayOperation::Store { dest, value },
            &function,
        )
        .unwrap();

        // Expect one pointer copy + one double-deref store
        assert!(b
            .instructions
            .iter()
            .any(|i| i.inner_instr().opcode_value() == STORE_ADD_FP_IMM));
        assert!(b
            .instructions
            .iter()
            .any(|i| i.inner_instr().opcode_value() == STORE_TO_DOUBLE_DEREF_FP_IMM));
    }

    #[test]
    fn test_array_load_dynamic_index() {
        let (mut b, array_id) = mk_builder_with_array();
        let mut function = MirFunction::new("test".to_string());

        // Create index variable
        let index_id = ValueId::from_raw(5);
        b.layout.allocate_value(index_id, 1).unwrap();
        function.value_types.insert(index_id, MirType::Felt);

        // Load element at dynamic index
        let dest = ValueId::from_raw(20);
        let index = Value::operand(index_id);
        let element_ty = MirType::Felt;

        b.array_operation(
            Value::operand(array_id),
            index,
            &element_ty,
            ArrayOperation::Load { dest },
            &function,
        )
        .unwrap();

        // Expect one dynamic double-deref load
        assert!(b
            .instructions
            .iter()
            .any(|i| i.inner_instr().opcode_value() == STORE_DOUBLE_DEREF_FP_FP));
    }

    #[test]
    fn test_array_dynamic_index_scaling_u32() {
        let (mut b, array_id) = mk_builder_with_array();
        let mut function = MirFunction::new("test".to_string());

        // Create index variable
        let index_id = ValueId::from_raw(5);
        b.layout.allocate_value(index_id, 1).unwrap();
        function.value_types.insert(index_id, MirType::Felt);

        // Load u32 element at dynamic index (requires scaling by 2)
        let dest = ValueId::from_raw(20);
        let index = Value::operand(index_id);
        let element_ty = MirType::U32;

        b.array_operation(
            Value::operand(array_id),
            index,
            &element_ty,
            ArrayOperation::Load { dest },
            &function,
        )
        .unwrap();

        // For u32 load: expect scale (mul by 2), one load for slot 0, adjust offset, one load for slot 1
        let mul_count = b
            .instructions
            .iter()
            .filter(|i| i.inner_instr().opcode_value() == STORE_MUL_FP_IMM)
            .count();
        let load_count = b
            .instructions
            .iter()
            .filter(|i| i.inner_instr().opcode_value() == STORE_DOUBLE_DEREF_FP_FP)
            .count();
        let adjust_count = b
            .instructions
            .iter()
            .filter(|i| i.inner_instr().opcode_value() == STORE_ADD_FP_IMM)
            .count();
        assert_eq!(mul_count, 1);
        assert_eq!(load_count, 2);
        assert!(adjust_count >= 1);
    }

    #[test]
    fn test_array_reject_non_felt_index() {
        let (mut b, array_id) = mk_builder_with_array();
        let mut function = MirFunction::new("test".to_string());

        // Create u32 index (invalid)
        let index_id = ValueId::from_raw(5);
        b.layout.allocate_value(index_id, 2).unwrap(); // u32 uses 2 slots
        function.value_types.insert(index_id, MirType::U32);

        let dest = ValueId::from_raw(20);
        let index = Value::operand(index_id);
        let element_ty = MirType::Felt;

        let result = b.array_operation(
            Value::operand(array_id),
            index,
            &element_ty,
            ArrayOperation::Load { dest },
            &function,
        );

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Array index must be a felt"));
    }

    // =========================================================================
    // Property Tests
    // =========================================================================

    proptest! {
        #[test]
        fn prop_struct_field_roundtrip(x in 0u32..1000, y in 0u32..1000) {
            let (mut b, mut function) = mk_builder_with_struct_type();

            // Create struct
            let struct_id = ValueId::from_raw(10);
            let struct_ty = MirType::Struct {
                name: "S".to_string(),
                fields: vec![
                    ("x".to_string(), MirType::Felt),
                    ("y".to_string(), MirType::Felt),
                ],
            };

            let fields = vec![
                ("x".to_string(), Value::integer(x)),
                ("y".to_string(), Value::integer(y)),
            ];

            b.make_struct(struct_id, &fields, &struct_ty).unwrap();
            function.value_types.insert(struct_id, struct_ty.clone());

            // Extract each field
            let x_extracted = ValueId::from_raw(20);
            let y_extracted = ValueId::from_raw(21);

            b.extract_struct_field(
                x_extracted,
                Value::operand(struct_id),
                "x",
                &MirType::Felt,
                &function,
            ).unwrap();

            b.extract_struct_field(
                y_extracted,
                Value::operand(struct_id),
                "y",
                &MirType::Felt,
                &function,
            ).unwrap();

            // Verify extraction maps to correct offsets
            prop_assert!(b.layout.value_layouts.contains_key(&x_extracted));
            prop_assert!(b.layout.value_layouts.contains_key(&y_extracted));
        }

        #[test]
        fn prop_tuple_size_calculation(n_felts in 0usize..10, n_u32s in 0usize..5) {
            let (mut b, mut function) = mk_builder_with_tuple_type();
            let dest = ValueId::from_raw(10);

            // Build elements and type list
            let mut elements = Vec::new();
            let mut types = Vec::new();
            let mut expected_size = 0;

            for i in 0..n_felts {
                elements.push(Value::integer(i as u32));
                types.push(MirType::Felt);
                expected_size += 1;
            }

            for i in 0..n_u32s {
                elements.push(Value::integer((1000 + i) as u32));
                types.push(MirType::U32);
                expected_size += 2; // u32 uses 2 slots
            }

            if elements.is_empty() {
                // Can't create empty tuple
                return Ok(());
            }

            let tuple_ty = MirType::Tuple(types);
            function.value_types.insert(dest, tuple_ty);

            b.make_tuple(dest, &elements, &function).unwrap();

            // Verify correct total allocation
            match b.layout.value_layouts.get(&dest) {
                Some(ValueLayout::MultiSlot { size, .. }) => {
                    prop_assert_eq!(*size, expected_size);
                }
                Some(ValueLayout::Slot { .. }) if expected_size == 1 => {
                    // Single slot is ok for size 1
                }
                _ if expected_size == 1 => {
                    // Ok for single slot
                }
                _ => {
                    prop_assert!(false, "Unexpected layout for tuple");
                }
            }
        }

        #[test]
        fn prop_array_size_correct(n_elements in 0usize..20) {
            let layout = FunctionLayout::new_for_test();
            let mut b = CasmBuilder::new(layout, 0);
            let dest = ValueId::from_raw(10);

            // Create array with n felt elements
            let elements: Vec<Value> = (0..n_elements)
                .map(|i| Value::integer(i as u32))
                .collect();
            let element_ty = MirType::Felt;

            b.make_fixed_array(dest, &elements, &element_ty).unwrap();

            // Should allocate 1 slot for pointer
            prop_assert!(b.layout.value_layouts.contains_key(&dest));

            // Pointer store must exist
            prop_assert!(!b.instructions.is_empty());

            // If there is at least one non-zero element, we expect element stores
            let has_non_zero = elements.iter().any(|v| matches!(v, Value::Literal(Literal::Integer(n)) if *n != 0));
            if has_non_zero {
                prop_assert!(b.instructions.len() > 1);
            }
        }
    }
}
