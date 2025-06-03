use crate::ast::*;
use crate::casm::{CasmInstruction, CasmInstructionType};
use std::collections::HashMap;

pub struct Compiler {
    code_elements: Vec<CodeElement>,
    casm_instructions: Vec<CasmInstruction>,
    local_variables: HashMap<String, i32>,
    local_variables_count: u64,
    fp_offset: u64,
    label_counter: u64,
}

impl Compiler {
    pub fn new(code_elements: Vec<CodeElement>) -> Self {
        Self {
            code_elements,
            casm_instructions: Vec::new(),
            local_variables: HashMap::new(),
            local_variables_count: 0,
            fp_offset: 0,
            label_counter: 0,
        }
    }

    pub fn compile(&mut self) -> Vec<CasmInstruction> {
        for code_element in self.code_elements.clone() {
            self.compile_code_element(code_element);
        }
        self.casm_instructions.clone()
    }

    // pushes litteral on stack and returns fp offset at which it is stored
    fn compile_int_literal(&mut self, expr: Expr) -> u64 {
        assert!(matches!(expr.expr_type, ExprType::IntegerLiteral));
        let instr = CasmInstruction {
            instruction_type: CasmInstructionType::MovFpImm,
            label: None,
            arg0: self.fp_offset as i32,
            arg1: expr.token.unwrap().lexeme.parse::<i32>().unwrap(),
            arg2: 0,
        };
        self.casm_instructions.push(instr);
        self.fp_offset += 1;
        return self.fp_offset - 1;
    }

    fn compile_add(&mut self, expr: Expr) -> u64 {
        assert!(matches!(expr.expr_type, ExprType::Add));
        let left_offset = self.compile_expr(*expr.left.unwrap());
        let right_offset = self.compile_expr(*expr.right.unwrap());
        let instr = CasmInstruction {
            instruction_type: CasmInstructionType::AddFpFp,
            label: None,
            arg0: self.fp_offset as i32,
            arg1: left_offset as i32,
            arg2: right_offset as i32,
        };
        self.casm_instructions.push(instr);
        self.fp_offset += 1;
        return self.fp_offset - 1;
    }

    fn compile_sub(&mut self, expr: Expr) -> u64 {
        assert!(matches!(expr.expr_type, ExprType::Sub));
        let left_offset = self.compile_expr(*expr.left.unwrap());
        let right_offset = self.compile_expr(*expr.right.unwrap());
        let instr = CasmInstruction {
            instruction_type: CasmInstructionType::SubFpFp,
            label: None,
            arg0: self.fp_offset as i32,
            arg1: left_offset as i32,
            arg2: right_offset as i32,
        };
        self.casm_instructions.push(instr);
        self.fp_offset += 1;
        return self.fp_offset - 1;
    }

    fn compile_mul(&mut self, expr: Expr) -> u64 {
        assert!(matches!(expr.expr_type, ExprType::Mul));
        let left_offset = self.compile_expr(*expr.left.unwrap());
        let right_offset = self.compile_expr(*expr.right.unwrap());
        let instr = CasmInstruction {
            instruction_type: CasmInstructionType::MulFpFp,
            label: None,
            arg0: self.fp_offset as i32,
            arg1: left_offset as i32,
            arg2: right_offset as i32,
        };
        self.casm_instructions.push(instr);
        self.fp_offset += 1;
        return self.fp_offset - 1;
    }

    fn compile_function_call(&mut self, expr: Expr) -> u64 {
        assert!(matches!(expr.expr_type, ExprType::FunctionCall));
        let func_name = expr.ident.unwrap().token.lexeme;

        // evaluating each argument and storing offsets
        let mut arg_offsets = Vec::new();
        for arg in expr.paren_args {
            match arg {
                ExprAssignment::Expr(expr) => arg_offsets.push(self.compile_expr(expr)),
                ExprAssignment::Assign(ident, expr) => todo!(),
            };
        }

        // pushing arguments to stack
        for arg_offset in arg_offsets.iter() {
            let instr = CasmInstruction {
                instruction_type: CasmInstructionType::MovFpFp,
                label: None,
                arg0: self.fp_offset as i32,
                arg1: *arg_offset as i32,
                arg2: 0,
            };
            self.casm_instructions.push(instr);
            self.fp_offset += 1;
        }

        // calling function
        let instr = CasmInstruction {
            instruction_type: CasmInstructionType::CallLabel,
            label: Some(func_name),
            arg0: self.fp_offset as i32, // frame size
            arg1: 0,
            arg2: 0,
        };
        self.casm_instructions.push(instr);

        // return value is at top of stack
        self.fp_offset += 1;
        return self.fp_offset - 1;
    }

    // pushes local variable on stack and returns fp offset
    fn compile_identifier(&mut self, expr: Expr) -> u64 {
        assert!(matches!(expr.expr_type, ExprType::Identifier));
        let ident = expr.ident.unwrap().token.lexeme;
        let instr = CasmInstruction {
            instruction_type: CasmInstructionType::MovFpFp,
            label: None,
            arg0: self.fp_offset as i32,
            arg1: self.local_variables[&ident] as i32,
            arg2: 0,
        };
        self.casm_instructions.push(instr);
        self.fp_offset += 1;
        return self.fp_offset - 1;
    }

    pub fn compile_expr(&mut self, expr: Expr) -> u64 {
        // compiles an expression and returns the fp offset at which the result is stored
        // operations are done at the top of the stack
        // note that for ease of implementation, integers as well as references to variables are all pushed on stack
        // which creates a lot of unecessary copies and instructions
        match expr.expr_type {
            ExprType::IntegerLiteral => self.compile_int_literal(expr),
            ExprType::Add => self.compile_add(expr),
            ExprType::Sub => self.compile_sub(expr),
            ExprType::Mul => self.compile_mul(expr),
            ExprType::FunctionCall => self.compile_function_call(expr),
            ExprType::Identifier => self.compile_identifier(expr),

            _ => todo!(),
        }
    }

    pub fn compile_function(
        &mut self,
        name: Identifier,
        args: Vec<Identifier>,
        body: Vec<CodeElement>,
    ) {
        let save_locals = self.local_variables.clone();
        self.local_variables.clear();
        let save_local_variables_count = self.local_variables_count;
        self.local_variables_count = 0;
        let save_fp_offset = self.fp_offset;
        self.fp_offset = 0;
        // counting number of local declarations
        for code_element in body.clone() {
            match code_element {
                CodeElement::LocalVar(ident, expr) => {
                    self.fp_offset += 1;
                }
                _ => {}
            }
        }

        self.casm_instructions.push(CasmInstruction {
            instruction_type: CasmInstructionType::Label,
            label: Some(name.token.lexeme),
            arg0: 0,
            arg1: 0,
            arg2: 0,
        });

        // adding arguments to local variables
        // arguments are stored in [fp-4], [fp-5], ...
        // [fp-3] is return value, [fp-2] is old fp, [fp-1] is return address
        for (i, arg) in args.iter().enumerate() {
            self.local_variables.insert(
                arg.token.lexeme.clone(),
                -(args.len() as i32 + 3) + i as i32,
            );
        }
        for code_element in body {
            self.compile_code_element(code_element);
        }

        // restoring state of previous frame
        self.fp_offset = save_fp_offset;
        self.local_variables = save_locals;
        self.local_variables_count = save_local_variables_count;
    }

    fn compile_local_var(&mut self, ident: Identifier, expr: Option<Expr>) {
        self.local_variables.insert(
            ident.token.lexeme.clone(),
            self.local_variables_count as i32,
        );
        self.local_variables_count += 1;

        match expr {
            Some(expr) => {
                let offset = self.compile_expr(expr);
                let instr = CasmInstruction {
                    instruction_type: CasmInstructionType::MovFpFp,
                    label: None,
                    arg0: self.local_variables[&ident.token.lexeme] as i32,
                    arg1: offset as i32,
                    arg2: 0,
                };
                self.casm_instructions.push(instr);
            }
            None => {}
        }
    }

    fn compile_return(&mut self, expr: Expr) {
        // calculating return value
        // it is automatically at top of stack
        let offset = self.compile_expr(expr);

        // moving return value to bottom of frame
        self.casm_instructions.push(CasmInstruction {
            instruction_type: CasmInstructionType::MovFpFp,
            label: None,
            arg0: -3,
            arg1: offset as i32,
            arg2: 0,
        });

        self.casm_instructions.push(CasmInstruction {
            instruction_type: CasmInstructionType::Ret,
            label: None,
            arg0: 0,
            arg1: 0,
            arg2: 0,
        });
    }

    fn compile_if(&mut self, expr: Expr, body: Vec<CodeElement>, else_body: Vec<CodeElement>) {
        match expr.expr_type {
            ExprType::Neq => {
                let offset = self.compile_expr(Expr::new_binary(
                    ExprType::Sub,
                    *expr.left.unwrap(),
                    *expr.right.unwrap(),
                ));
                self.casm_instructions.push(CasmInstruction {
                    instruction_type: CasmInstructionType::JmpLabelIfNeq,
                    label: Some(format!("if{}", self.label_counter)),
                    arg0: 0,
                    arg1: offset as i32,
                    arg2: 0,
                });
                for code_element in else_body {
                    self.compile_code_element(code_element);
                }
                self.casm_instructions.push(CasmInstruction {
                    instruction_type: CasmInstructionType::JmpLabel,
                    label: Some(format!("end{}", self.label_counter)),
                    arg0: 0,
                    arg1: 0,
                    arg2: 0,
                });
                self.casm_instructions.push(CasmInstruction {
                    instruction_type: CasmInstructionType::Label,
                    label: Some(format!("if{}", self.label_counter)),
                    arg0: 0,
                    arg1: 0,
                    arg2: 0,
                });
                for code_element in body {
                    self.compile_code_element(code_element);
                }
                self.casm_instructions.push(CasmInstruction {
                    instruction_type: CasmInstructionType::Label,
                    label: Some(format!("end{}", self.label_counter)),
                    arg0: 0,
                    arg1: 0,
                    arg2: 0,
                });
                self.label_counter += 1;
            }
            ExprType::Eq => {
                let offset = self.compile_expr(Expr::new_binary(
                    ExprType::Sub,
                    *expr.left.unwrap(),
                    *expr.right.unwrap(),
                ));
                self.casm_instructions.push(CasmInstruction {
                    instruction_type: CasmInstructionType::JmpLabelIfNeq,
                    label: Some(format!("else{}", self.label_counter)),
                    arg0: 0,
                    arg1: offset as i32,
                    arg2: 0,
                });
                for code_element in body {
                    self.compile_code_element(code_element);
                }
                self.casm_instructions.push(CasmInstruction {
                    instruction_type: CasmInstructionType::JmpLabel,
                    label: Some(format!("end{}", self.label_counter)),
                    arg0: 0,
                    arg1: 0,
                    arg2: 0,
                });
                self.casm_instructions.push(CasmInstruction {
                    instruction_type: CasmInstructionType::Label,
                    label: Some(format!("else{}", self.label_counter)),
                    arg0: 0,
                    arg1: 0,
                    arg2: 0,
                });
                for code_element in else_body {
                    self.compile_code_element(code_element);
                }
                self.casm_instructions.push(CasmInstruction {
                    instruction_type: CasmInstructionType::Label,
                    label: Some(format!("end{}", self.label_counter)),
                    arg0: 0,
                    arg1: 0,
                    arg2: 0,
                });
                self.label_counter += 1;
            }
            _ => panic!("Invalid expression type for if statement"),
        }
    }

    fn compile_assert_equal(&mut self, instr: Instruction) {
        assert!(matches!(instr.instruction_type, InstructionType::AssertEq));
        let left = instr.args[0].clone();
        let right = instr.args[1].clone();

        if matches!(left.expr_type, ExprType::Identifier) {
            let ident = left.ident.clone().unwrap().token.lexeme;
            if self.local_variables.contains_key(&ident) {
                // If identifier exists, assign new value
                let _ = self.compile_expr(right);
                let instr = CasmInstruction {
                    instruction_type: CasmInstructionType::MovFpFp,
                    label: None,
                    arg0: self.local_variables[&ident],
                    arg1: (self.fp_offset - 1) as i32,
                    arg2: 0,
                };
                self.casm_instructions.push(instr);
            } else {
                // If identifier doesn't exist, create new local variable
                self.compile_local_var(left.ident.unwrap(), Some(right));
            }
        } else {
            panic!("Can't assign to non-identifier");
        }
    }

    fn compile_instruction(&mut self, instr: Instruction) {
        match instr.instruction_type {
            InstructionType::Ret => self.casm_instructions.push(CasmInstruction {
                instruction_type: CasmInstructionType::Ret,
                label: None,
                arg0: 0,
                arg1: 0,
                arg2: 0,
            }),
            InstructionType::AssertEq => self.compile_assert_equal(instr),
            _ => todo!(),
        }
    }

    pub fn compile_code_element(&mut self, code_element: CodeElement) {
        match code_element {
            CodeElement::LocalVar(ident, expr) => self.compile_local_var(ident, expr),
            CodeElement::Return(expr) => self.compile_return(expr),
            CodeElement::Function(name, args, body) => self.compile_function(name, args, body),
            CodeElement::If(expr, body, else_body) => self.compile_if(expr, body, else_body),
            CodeElement::Instruction(instr) => self.compile_instruction(instr),
            CodeElement::AllocLocals => (), // locals are already allocated by default at the beginning of a function
            _ => todo!(),
        }
    }
}
