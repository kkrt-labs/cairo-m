pub struct MiniVm {
    pub pc: u32,
    pub fp: u32,
    pub mem: Vec<u32>,
    pub exit: bool,
    pub program_size: u32,
}

impl MiniVm {
    pub fn new() -> Self {
        Self {
            pc: 0,
            fp: 0,
            mem: vec![0; 512],
            exit: false,
            program_size: 0,
        }
    }

    pub fn load_program(&mut self, program: Vec<u32>) {
        // Copy program to beginning of memory
        for (i, &instruction) in program.iter().enumerate() {
            self.mem[i] = instruction;
        }
        self.program_size = program.len() as u32;
        self.fp = self.program_size;
    }

    pub fn run(&mut self) {
        while !self.exit {
            let instr = self.mem[self.pc as usize];
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

    fn get_arg(&self, offset: u32) -> i32 {
        let arg = self.mem[(self.pc + offset) as usize];
        (arg as i32) - 0x8000
    }

    fn mov_fp_fp(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        self.mem[(self.fp as i32 + offdst) as usize] = self.mem[(self.fp as i32 + offop0) as usize];
        self.pc += 4;
    }

    fn mov_fp_imm(&mut self) {
        let offdst = self.get_arg(1);
        let imm = self.get_arg(2);
        self.mem[(self.fp as i32 + offdst) as usize] = imm as u32;
        self.pc += 4;
    }

    fn add_fp_fp(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        let offop1 = self.get_arg(3);
        self.mem[(self.fp as i32 + offdst) as usize] = self.mem[(self.fp as i32 + offop0) as usize]
            + self.mem[(self.fp as i32 + offop1) as usize];
        self.pc += 4;
    }

    fn add_fp_imm(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        let imm = self.get_arg(3);
        self.mem[(self.fp as i32 + offdst) as usize] =
            self.mem[(self.fp as i32 + offop0) as usize] + imm as u32;
        self.pc += 4;
    }

    fn sub_fp_fp(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        let offop1 = self.get_arg(3);
        self.mem[(self.fp as i32 + offdst) as usize] = self.mem[(self.fp as i32 + offop0) as usize]
            - self.mem[(self.fp as i32 + offop1) as usize];
        self.pc += 4;
    }

    fn sub_fp_imm(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        let imm = self.get_arg(3);
        self.mem[(self.fp as i32 + offdst) as usize] =
            self.mem[(self.fp as i32 + offop0) as usize] - imm as u32;
        self.pc += 4;
    }

    fn mul_fp_fp(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        let offop1 = self.get_arg(3);
        self.mem[(self.fp as i32 + offdst) as usize] = self.mem[(self.fp as i32 + offop0) as usize]
            * self.mem[(self.fp as i32 + offop1) as usize];
        self.pc += 4;
    }

    fn mul_fp_imm(&mut self) {
        let offdst = self.get_arg(1);
        let offop0 = self.get_arg(2);
        let imm = self.get_arg(3);
        self.mem[(self.fp as i32 + offdst) as usize] =
            self.mem[(self.fp as i32 + offop0) as usize] * imm as u32;
        self.pc += 4;
    }

    fn jmp_abs(&mut self) {
        let address = self.get_arg(1);
        self.pc = address as u32 - 4;
        self.pc += 4;
    }

    fn jmp_rel(&mut self) {
        let offset = self.get_arg(1);
        self.pc = self.pc.wrapping_add(offset as u32);
    }

    fn jmp_abs_if_neq(&mut self) {
        let address = self.get_arg(1);
        let offset = self.get_arg(2);
        if self.mem[(self.fp as i32 + offset) as usize] != 0 {
            self.pc = address as u32;
        }
    }

    fn jmp_rel_if_neq(&mut self) {
        let offset = self.get_arg(1);
        let check_offset = self.get_arg(2);
        if self.mem[(self.fp as i32 + check_offset) as usize] != 0 {
            self.pc = self.pc.wrapping_add(offset as u32);
        }
    }

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

    pub fn print_mem(&self) {
        for i in 0..self.mem.len() {
            println!("{}: {}", i, self.mem[i]);
        }
    }
}
