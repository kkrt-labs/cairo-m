//! This component is used to prove the StoreAddFpFp opcode.
//! [fp + off2] = [fp + off0] + [fp + off1]
//!
//! # Columns
//!
//! - enabler
//! - pc
//! - fp
//! - clock
//! - inst_prev_clock
//! - opcode_id
//! - off0
//! - off1
//! - off2
//! - op0_prev_clock
//! - op0_val
//! - op1_prev_clock
//! - op1_val
//! - dst_prev_clock
//! - dst_prev_val
//!
//! # Constraints
//!
//! * enabler is a bool
//!   * `enabler * (1 - enabler)`
//! * registers update is regular
//!   * `- [pc, fp] + [pc + 1, fp]` in `Registers` relation
//! * read instruction from memory
//!   * `- [pc, inst_prev_clk, opcode_id, off0, off1, off2] + [pc, clk, opcode_id, off0, off1, off2]` in `Memory` relation
//!   * `- [clk - inst_prev_clk - 1]` in `RangeCheck_20` relation
//! * assert opcode id
//!   * `opcode_id - 0`
//! * read op0
//!   * `- [fp + off0, op0_prev_clk, op0_val] + [fp + off0, clk, op0_val]` in `Memory` relation
//!   * `- [clk - op0_prev_clk - 1]` in `RangeCheck_20` relation
//! * read op1
//!   * `- [fp + off1, op1_prev_clk, op1_val] + [fp + off1, clk, op1_val]` in `Memory` relation
//!   * `- [clk - op1_prev_clk - 1]` in `RangeCheck_20` relation
//! * write dst in [fp + off2]
//!   * `- [fp + off2, dst_prev_clk, dst_prev_val] + [fp + off2, clk, op0_val + op1_val]` in `Memory` relation
//!   * `- [clk - dst_prev_clk - 1]` in `RangeCheck_20` relation

use crate::components::opcodes::macros::define_opcode_component;

define_opcode_component! {
    name: store_add_fp_fp,
    opcode_id: StoreAddFpFp,

    columns: [
        enabler, pc, fp, clock, inst_prev_clock, opcode_id,
        off0, off1, off2, op0_prev_clock, op0_val, op1_prev_clock, op1_val,
        dst_prev_clock, dst_prev_val
    ],

    lookups: {
        registers: 2,
        memory: 8,
        range_check_20: 4,
    },

    write_trace: |input, lookup_data, enabler, one, zero| {
        let pc = input.pc;
        let fp = input.fp;
        let clock = input.mem0_clock;
        let inst_prev_clock = input.mem0_prev_clock;
        let opcode_id = input.mem0_value_0;
        let off0 = input.mem0_value_1;
        let off1 = input.mem0_value_2;
        let off2 = input.mem0_value_3;
        let op0_prev_clock = input.mem1_prev_clock;
        let op0_val = input.mem1_value_0;
        let op1_prev_clock = input.mem2_prev_clock;
        let op1_val = input.mem2_value_0;
        let dst_prev_val = input.mem3_prev_val_0;
        let dst_prev_clock = input.mem3_prev_clock;

        // Fill lookup data
        *lookup_data.registers[0] = [input.pc, input.fp];
        *lookup_data.registers[1] = [input.pc + one, input.fp];

        *lookup_data.memory[0] = [input.pc, inst_prev_clock, opcode_id, off0, off1, off2];
        *lookup_data.memory[1] = [input.pc, clock, opcode_id, off0, off1, off2];

        *lookup_data.memory[2] = [fp + off0, op0_prev_clock, op0_val, zero, zero, zero];
        *lookup_data.memory[3] = [fp + off0, clock, op0_val, zero, zero, zero];

        *lookup_data.memory[4] = [fp + off1, op1_prev_clock, op1_val, zero, zero, zero];
        *lookup_data.memory[5] = [fp + off1, clock, op1_val, zero, zero, zero];

        *lookup_data.memory[6] = [fp + off2, dst_prev_clock, dst_prev_val, zero, zero, zero];
        *lookup_data.memory[7] = [fp + off2, clock, op0_val + op1_val, zero, zero, zero];

        *lookup_data.range_check_20[0] = clock - inst_prev_clock - enabler;
        *lookup_data.range_check_20[1] = clock - op0_prev_clock - enabler;
        *lookup_data.range_check_20[2] = clock - op1_prev_clock - enabler;
        *lookup_data.range_check_20[3] = clock - dst_prev_clock - enabler;

        // Return trace column values in order
        (enabler, pc, fp, clock, inst_prev_clock, opcode_id,
         off0, off1, off2, op0_prev_clock, op0_val, op1_prev_clock, op1_val,
         dst_prev_clock, dst_prev_val)
    },

    evaluate: |eval, cols, one, _self| {
        // Registers update
        eval.add_to_relation(RelationEntry::new(
            &_self.registers,
            -E::EF::from(cols.enabler.clone()),
            &[cols.pc.clone(), cols.fp.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &_self.registers,
            E::EF::from(cols.enabler.clone()),
            &[cols.pc.clone() + one, cols.fp.clone()],
        ));

        // Read instruction from memory
        eval.add_to_relation(RelationEntry::new(
            &_self.memory,
            -E::EF::from(cols.enabler.clone()),
            &[
                cols.pc.clone(),
                cols.inst_prev_clock.clone(),
                cols.opcode_id.clone(),
                cols.off0.clone(),
                cols.off1.clone(),
                cols.off2.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &_self.memory,
            E::EF::from(cols.enabler.clone()),
            &[
                cols.pc,
                cols.clock.clone(),
                cols.opcode_id,
                cols.off0.clone(),
                cols.off1.clone(),
                cols.off2.clone(),
            ],
        ));

        // Read op0
        eval.add_to_relation(RelationEntry::new(
            &_self.memory,
            -E::EF::from(cols.enabler.clone()),
            &[
                cols.fp.clone() + cols.off0.clone(),
                cols.op0_prev_clock.clone(),
                cols.op0_val.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &_self.memory,
            E::EF::from(cols.enabler.clone()),
            &[cols.fp.clone() + cols.off0, cols.clock.clone(), cols.op0_val.clone()],
        ));

        // Read op1
        eval.add_to_relation(RelationEntry::new(
            &_self.memory,
            -E::EF::from(cols.enabler.clone()),
            &[
                cols.fp.clone() + cols.off1.clone(),
                cols.op1_prev_clock.clone(),
                cols.op1_val.clone(),
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &_self.memory,
            E::EF::from(cols.enabler.clone()),
            &[cols.fp.clone() + cols.off1, cols.clock.clone(), cols.op1_val.clone()],
        ));

        // Write dst
        eval.add_to_relation(RelationEntry::new(
            &_self.memory,
            -E::EF::from(cols.enabler.clone()),
            &[
                cols.fp.clone() + cols.off2.clone(),
                cols.dst_prev_clock.clone(),
                cols.dst_prev_val,
            ],
        ));
        eval.add_to_relation(RelationEntry::new(
            &_self.memory,
            E::EF::from(cols.enabler.clone()),
            &[cols.fp + cols.off2, cols.clock.clone(), cols.op0_val + cols.op1_val],
        ));

        // Range check 20
        eval.add_to_relation(RelationEntry::new(
            &_self.range_check_20,
            -E::EF::one(),
            &[cols.clock.clone() - cols.inst_prev_clock.clone() - cols.enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &_self.range_check_20,
            -E::EF::one(),
            &[cols.clock.clone() - cols.op0_prev_clock.clone() - cols.enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &_self.range_check_20,
            -E::EF::one(),
            &[cols.clock.clone() - cols.op1_prev_clock.clone() - cols.enabler.clone()],
        ));
        eval.add_to_relation(RelationEntry::new(
            &_self.range_check_20,
            -E::EF::one(),
            &[cols.clock - cols.dst_prev_clock - cols.enabler],
        ));
    }
}
