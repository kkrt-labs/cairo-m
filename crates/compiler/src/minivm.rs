//! Cairo-M Mini Virtual Machine
//!
//! This module implements a minimal virtual machine for executing Cairo-M assembly code.
//! The VM uses a simple stack-based architecture with frame pointer (FP) relative addressing
//! and supports basic arithmetic operations, control flow, and function calls.
//!
//! Architecture:
//! - Memory: Linear array of 32-bit words
//! - Registers: Program Counter (PC) and Frame Pointer (FP)
//! - Stack: Grows upward from the end of the program
//!
//! When a function is called, a new frame is pushed to the stack
//! - arguments are stored in [fp-4], [fp-5], ...
//! - [fp-3] is return value
//! - [fp-2] is old fp
//! - [fp-1] is return address
//!
//! When a function returns, the frame is popped from the stack
//! The return value should be stored in [fp-3]
//! TODO : support tuple return values
//!
//! The stack is used to store the return value, the old fp, and the return address.
//!
//! Instruction Format:
//! - Each instruction is 4 words (16 bytes)
//! - First word: Opcode
//! - Remaining words: Arguments (encoded as signed offsets from 0x8000)

/// A minimal virtual machine for executing Cairo-M assembly code.
///
/// The VM maintains its state in memory, with the program counter (PC) tracking
/// the current instruction and the frame pointer (FP) managing the call stack.
pub struct MiniVm {
    /// Program Counter: Points to the next instruction to execute
    pub pc: u32,
    /// Frame Pointer: Points to the current stack frame
    pub fp: u32,
    /// Memory: Array of 32-bit words containing both program and stack
    pub mem: Vec<u32>,
    /// Exit flag: Set to true when the program should terminate
    pub exit: bool,
    /// Size of the loaded program in words
    pub program_size: u32,
}

impl MiniVm {
    /// Creates a new VM instance with initialized memory and registers.
    pub fn new() -> Self {
        Self {
            pc: 0,
            fp: 0,
            mem: vec![0; 512],
            exit: false,
            program_size: 0,
        }
    }

    /// Loads a program into VM memory and initializes the frame pointer.
    ///
    /// The program is copied to the beginning of memory, and the frame pointer
    /// is set to the end of the program, where the stack will begin.
    pub fn load_program(&mut self, program: Vec<u32>) {
        // Copy program to beginning of memory
        for (i, &instruction) in program.iter().enumerate() {
            self.mem[i] = instruction;
        }
        self.program_size = program.len() as u32;
        self.fp = self.program_size;
    }

    /// Executes the loaded program until completion or error.
    ///
    /// The VM fetches and executes instructions in a loop until either:
    /// - The program reaches a return instruction at the top level
    /// - An unknown instruction is encountered
    pub fn run(&mut self) {
        while !self.exit {
            let instr = self.mem[self.pc as usize];
            //println!("pc: {}, instr: {}", self.pc, instr);
            match instr {
                0 => self.mov_fp_fp(),
                1 => self.mov_fp_imm(),
                2 => self.add_fp_fp(),
                3 => self.add_fp_imm(),
                4 => self.sub_fp_fp(),
                5 => self.sub_fp_imm(),
                6 => self.mul_fp_fp(),
                7 => self.mul_fp_imm(),
                8 => self.jmp_abs(),
                9 => self.jmp_rel(),
                10 => self.jmp_abs_if_neq(),
                11 => self.jmp_rel_if_neq(),
                12 => self.call_rel(),
                13 => self.call_abs(),
                14 => self.ret(),
                _ => panic!("Unknown instruction: {}", instr),
            }
        }
    }

    /// Decodes an instruction argument from memory.
    ///
    /// Arguments are stored as signed offsets from 0x8000 to allow for both
    /// positive and negative frame pointer offsets.
    fn get_arg(&self, offset: u32) -> i32 {
        let arg = self.mem[(self.pc + offset) as usize];
        (arg as i32) - 0x8000
    }

    /// Moves a value from one frame pointer offset to another.
    ///
    /// Format: mov_fp_fp dst_offset src_offset
    fn mov_fp_fp(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        self.mem[(self.fp as i32 + offdst) as usize] = self.mem[(self.fp as i32 + offop0) as usize];
        self.pc += 4;
    }

    /// Moves an immediate value to a frame pointer offset.
    ///
    /// Format: mov_fp_imm dst_offset immediate
    fn mov_fp_imm(&mut self) {
        let offdst = self.get_arg(1);
        let imm = self.get_arg(2);
        self.mem[(self.fp as i32 + offdst) as usize] = imm as u32;
        self.pc += 4;
    }

    /// Adds two values from frame pointer offsets and stores the result.
    ///
    /// Format: add_fp_fp dst_offset src1_offset src2_offset
    fn add_fp_fp(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        let offop1 = self.get_arg(3);
        self.mem[(self.fp as i32 + offdst) as usize] = self.mem[(self.fp as i32 + offop0) as usize]
            + self.mem[(self.fp as i32 + offop1) as usize];
        self.pc += 4;
    }

    /// Adds a value from a frame pointer offset and an immediate value.
    ///
    /// Format: add_fp_imm dst_offset src_offset immediate
    fn add_fp_imm(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        let imm = self.get_arg(3);
        self.mem[(self.fp as i32 + offdst) as usize] =
            self.mem[(self.fp as i32 + offop0) as usize] + imm as u32;
        self.pc += 4;
    }

    /// Subtracts two values from frame pointer offsets and stores the result.
    ///
    /// Format: sub_fp_fp dst_offset src1_offset src2_offset
    fn sub_fp_fp(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        let offop1 = self.get_arg(3);
        self.mem[(self.fp as i32 + offdst) as usize] = self.mem[(self.fp as i32 + offop0) as usize]
            - self.mem[(self.fp as i32 + offop1) as usize];
        self.pc += 4;
    }

    /// Subtracts an immediate value from a frame pointer offset value.
    ///
    /// Format: sub_fp_imm dst_offset src_offset immediate
    fn sub_fp_imm(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        let imm = self.get_arg(3);
        self.mem[(self.fp as i32 + offdst) as usize] =
            self.mem[(self.fp as i32 + offop0) as usize] - imm as u32;
        self.pc += 4;
    }

    /// Multiplies two values from frame pointer offsets and stores the result.
    ///
    /// Format: mul_fp_fp dst_offset src1_offset src2_offset
    fn mul_fp_fp(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        let offop1 = self.get_arg(3);
        self.mem[(self.fp as i32 + offdst) as usize] = self.mem[(self.fp as i32 + offop0) as usize]
            * self.mem[(self.fp as i32 + offop1) as usize];
        self.pc += 4;
    }

    /// Multiplies a value from a frame pointer offset by an immediate value.
    ///
    /// Format: mul_fp_imm dst_offset src_offset immediate
    fn mul_fp_imm(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        let imm = self.get_arg(3);
        self.mem[(self.fp as i32 + offdst) as usize] =
            self.mem[(self.fp as i32 + offop0) as usize] * imm as u32;
        self.pc += 4;
    }

    /// Performs an absolute jump to the specified address.
    ///
    /// Format: jmp_abs address
    fn jmp_abs(&mut self) {
        let address = self.get_arg(1);
        self.pc = address as u32;
    }

    /// Performs a relative jump by adding an offset to the program counter.
    ///
    /// Format: jmp_rel offset
    fn jmp_rel(&mut self) {
        let offset = self.get_arg(1);
        self.pc = self.pc.wrapping_add(offset as u32);
    }

    /// Performs an absolute jump if the value at the specified offset is non-zero.
    ///
    /// Format: jmp_abs_if_neq address check_offset
    fn jmp_abs_if_neq(&mut self) {
        let address = self.get_arg(1);
        let offset = self.get_arg(2);
        if self.mem[(self.fp as i32 + offset) as usize] != 0 {
            self.pc = address as u32;
        } else {
            self.pc += 4;
        }
    }

    /// Performs a relative jump if the value at the specified offset is non-zero.
    ///
    /// Format: jmp_rel_if_neq offset check_offset
    fn jmp_rel_if_neq(&mut self) {
        let offset = self.get_arg(1);
        let check_offset = self.get_arg(2);
        if self.mem[(self.fp as i32 + check_offset) as usize] != 0 {
            self.pc = self.pc.wrapping_add(offset as u32);
        } else {
            self.pc += 4;
        }
    }

    /// Performs a relative function call.
    ///
    /// Sets up a new stack frame by:
    /// 1. Saving the old frame pointer
    /// 2. Saving the return address
    /// 3. Updating the frame pointer
    /// 4. Jumping to the function (relative offset)
    /// Format: call_rel offset frame_size
    fn call_rel(&mut self) {
        let offset = self.get_arg(1);
        let frame_size = self.get_arg(2);
        // push old fp
        self.mem[(self.fp as i32 + frame_size) as usize] = self.fp;
        // push return address
        self.mem[(self.fp as i32 + frame_size + 1) as usize] = self.pc + 4;
        // set new fp
        self.fp = self.fp + frame_size as u32 + 2;
        // jump to function (relative)
        self.pc = self.pc.wrapping_add(offset as u32);
    }

    /// Performs an absolute function call.
    ///
    /// Sets up a new stack frame by:
    /// 1. Reserving space for return value
    /// 2. Saving the old frame pointer
    /// 3. Saving the return address
    /// 4. Updating the frame pointer
    /// 5. Jumping to the function (absolute address)
    ///
    /// Format: call_abs address frame_size
    fn call_abs(&mut self) {
        let address = self.get_arg(1);
        let frame_size = self.get_arg(2);
        // frame size + 0 is reserved for return value
        self.mem[(self.fp as i32 + frame_size + 1) as usize] = self.fp;
        // push return address
        self.mem[(self.fp as i32 + frame_size + 2) as usize] = self.pc + 4;
        // set new fp
        self.fp = self.fp + frame_size as u32 + 3;
        // jump to function (absolute)
        self.pc = address as u32;
    }

    /// Returns from a function call.
    ///
    /// Restores the previous stack frame by:
    /// 1. Popping the return address
    /// 2. Restoring the old frame pointer
    /// 3. Jumping to the return address
    ///
    /// If returning from the top-level frame, sets the exit flag.
    fn ret(&mut self) {
        if self.fp == self.program_size {
            self.exit = true;
            return;
        }
        // pop return adress
        let return_addr = self.mem[(self.fp - 1) as usize];
        // pop old fp
        self.fp = self.mem[(self.fp - 2) as usize];
        // jump to return address
        self.pc = return_addr;
    }

    /// Prints the current state of VM memory.
    ///
    /// Useful for debugging and understanding program execution.
    pub fn print_mem(&self) {
        for i in 0..self.mem.len() {
            println!("{}: {}", i, self.mem[i]);
        }
    }
}
