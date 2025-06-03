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
            mem: vec![0; 64],
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
                8 => self.jmp(),
                9 => self.jnz(),
                10 => self.call(),
                11 => self.ret(),
                _ => panic!("Unknown instruction: {}", instr),
            }
            self.pc += 4;
        }
    }

    fn mov_fp_fp(&mut self) {
        let offdst = self.mem[(self.pc + 1) as usize];
        let offop0 = self.mem[(self.pc + 2) as usize];
        self.mem[(self.fp + offdst) as usize] = self.mem[(self.fp + offop0) as usize];
    }

    fn mov_fp_imm(&mut self) {
        let offdst = self.mem[(self.pc + 1) as usize];
        let imm = self.mem[(self.pc + 2) as usize];
        self.mem[(self.fp + offdst) as usize] = imm;
    }

    fn add_fp_fp(&mut self) {
        let offdst = self.mem[(self.pc + 1) as usize];
        let offop0 = self.mem[(self.pc + 2) as usize];
        let offop1 = self.mem[(self.pc + 3) as usize];
        self.mem[(self.fp + offdst) as usize] =
            self.mem[(self.fp + offop0) as usize] + self.mem[(self.fp + offop1) as usize];
    }

    fn add_fp_imm(&mut self) {
        let offdst = self.mem[(self.pc + 1) as usize];
        let offop0 = self.mem[(self.pc + 2) as usize];
        let imm = self.mem[(self.pc + 3) as usize];
        self.mem[(self.fp + offdst) as usize] = self.mem[(self.fp + offop0) as usize] + imm;
    }

    fn sub_fp_fp(&mut self) {
        let offdst = self.mem[(self.pc + 1) as usize];
        let offop0 = self.mem[(self.pc + 2) as usize];
        let offop1 = self.mem[(self.pc + 3) as usize];
        self.mem[(self.fp + offdst) as usize] =
            self.mem[(self.fp + offop0) as usize] - self.mem[(self.fp + offop1) as usize];
    }

    fn sub_fp_imm(&mut self) {
        let offdst = self.mem[(self.pc + 1) as usize];
        let offop0 = self.mem[(self.pc + 2) as usize];
        let imm = self.mem[(self.pc + 3) as usize];
        self.mem[(self.fp + offdst) as usize] = self.mem[(self.fp + offop0) as usize] - imm;
    }

    fn mul_fp_fp(&mut self) {
        let offdst = self.mem[(self.pc + 1) as usize];
        let offop0 = self.mem[(self.pc + 2) as usize];
        let offop1 = self.mem[(self.pc + 3) as usize];
        self.mem[(self.fp + offdst) as usize] =
            self.mem[(self.fp + offop0) as usize] * self.mem[(self.fp + offop1) as usize];
    }

    fn mul_fp_imm(&mut self) {
        let offdst = self.mem[(self.pc + 1) as usize];
        let offop0 = self.mem[(self.pc + 2) as usize];
        let imm = self.mem[(self.pc + 2) as usize];
        self.mem[(self.fp + offdst) as usize] = self.mem[(self.fp + offop0) as usize] * imm;
    }

    fn jmp(&mut self) {
        let offset = self.mem[(self.pc + 1) as usize] * 4;
        self.pc = offset;
    }

    fn jnz(&mut self) {
        let address = self.mem[(self.pc + 1) as usize] * 4;
        let offset = self.mem[(self.pc + 2) as usize];
        if self.mem[(self.fp + offset) as usize] != 0 {
            self.pc = address - 4; // Subtract 4 because the main loop will add 4
        }
    }

    fn call(&mut self) {
        let func_addr = self.mem[(self.pc + 1) as usize];
        let frame_size = self.mem[(self.pc + 2) as usize];
        // push old fp
        self.mem[(self.fp + frame_size) as usize] = self.fp;
        // push return adress
        self.mem[(self.fp + frame_size + 1) as usize] = self.pc + 4;
        // set new fp
        self.fp = self.fp + frame_size + 2;
        // jump to function
        self.pc = func_addr;
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
