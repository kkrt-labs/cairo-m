//!
//! Cairo-M Lowering to CASM
//!
//! This module implements the lowering (compilation) of the Cairo-M Abstract Syntax Tree (AST)
//! into a sequence of CASM (Cairo Assembly) instructions. The `Compiler` struct is responsible
//! for traversing the parsed code elements and generating the corresponding low-level instructions
//! that can be executed by the Cairo-M virtual machine or assembler.
//!
//! Only the most basic features are supported for now :
//! - Integer literals
//! - Addition, subtraction, multiplication
//! - Function calls
//! - Local variable declarations and assignments
//! - If/else statements
//! - Return statements

use crate::ast::*;
use crate::casm::{CasmInstruction, CasmInstructionType};
use std::collections::HashMap;

/// The main compiler/lowering struct for converting AST code elements into CASM instructions.
///
/// The `Compiler` maintains state for local variables, stack frame offsets, and label generation.
/// It provides methods for compiling expressions, statements, and functions, emitting CASM instructions
/// as it traverses the AST.
pub struct Compiler {
    /// The list of top-level code elements (functions, statements, etc.) to compile
    code_elements: Vec<CodeElement>,
    /// The list of generated CASM instructions
    casm_instructions: Vec<CasmInstruction>,
    /// Mapping from local variable names to their frame pointer offsets
    local_variables: HashMap<String, i32>,
    /// The number of local variables currently allocated
    local_variables_count: u64,
    /// The current frame pointer offset (top of stack)
    fp_offset: u64,
    /// Counter for generating unique labels (for control flow)
    label_counter: u64,
}

impl Compiler {
    /// Create a new compiler instance from a list of code elements (AST root).
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

    /// Compile all code elements into a vector of CASM instructions.
    ///
    /// This is the main entry point for lowering a program.
    pub fn compile(&mut self) -> Vec<CasmInstruction> {
        for code_element in self.code_elements.clone() {
            self.compile_code_element(code_element);
        }
        self.casm_instructions.clone()
    }

    /// Compile an integer literal expression.
    ///
    /// Pushes the literal onto the stack and returns the frame pointer offset where it is stored.
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

    /// Compile an addition expression.
    ///
    /// Evaluates both operands, emits an add instruction, and returns the result offset.
    fn compile_add(&mut self, expr: Expr) -> u64 {
        assert!(matches!(expr.expr_type, ExprType::Add));
        let left_offset = self.compile_expr(*expr.left.unwrap());
        let right_offset = self.compile_expr(*expr.right.unwrap());
        let instr = CasmInstruction {
            instruction_type: CasmInstructionType::AddFpFp,
            label: None,
            arg0: left_offset as i32,
            arg1: left_offset as i32,
            arg2: right_offset as i32,
        };
        self.casm_instructions.push(instr);
        self.fp_offset = left_offset + 1;
        return left_offset;
    }

    /// Compile a subtraction expression.
    ///
    /// Evaluates both operands, emits a sub instruction, and returns the result offset.
    fn compile_sub(&mut self, expr: Expr) -> u64 {
        assert!(matches!(expr.expr_type, ExprType::Sub));
        let left_offset = self.compile_expr(*expr.left.unwrap());
        let right_offset = self.compile_expr(*expr.right.unwrap());
        let instr = CasmInstruction {
            instruction_type: CasmInstructionType::SubFpFp,
            label: None,
            arg0: left_offset as i32,
            arg1: left_offset as i32,
            arg2: right_offset as i32,
        };
        self.casm_instructions.push(instr);
        self.fp_offset = left_offset + 1;
        return left_offset;
    }

    /// Compile a multiplication expression.
    ///
    /// Evaluates both operands, emits a mul instruction, and returns the result offset.
    fn compile_mul(&mut self, expr: Expr) -> u64 {
        assert!(matches!(expr.expr_type, ExprType::Mul));
        let left_offset = self.compile_expr(*expr.left.unwrap());
        let right_offset = self.compile_expr(*expr.right.unwrap());
        let instr = CasmInstruction {
            instruction_type: CasmInstructionType::MulFpFp,
            label: None,
            arg0: left_offset as i32,
            arg1: left_offset as i32,
            arg2: right_offset as i32,
        };
        self.casm_instructions.push(instr);
        self.fp_offset = left_offset + 1;
        return left_offset;
    }

    /// Compile a function call expression.
    ///
    /// Evaluates all arguments, pushes them to the stack, emits a call instruction,
    /// and returns the offset of the return value.
    fn compile_function_call(&mut self, expr: Expr) -> u64 {
        assert!(matches!(expr.expr_type, ExprType::FunctionCall));
        let func_name = expr.ident.unwrap().token.lexeme;

        // evaluating each argument and storing offsets
        let mut arg_offsets = Vec::new();
        for arg in expr.paren_args {
            match arg {
                ExprAssignment::Expr(expr) => arg_offsets.push(self.compile_expr(expr)),
                ExprAssignment::Assign(_ident, _expr) => todo!(),
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

    /// Compile an identifier expression (variable reference).
    ///
    /// Pushes the value of the local variable onto the stack and returns its offset.
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

    /// Compile an expression and return the frame pointer offset where the result is stored.
    ///
    /// This method dispatches to the appropriate compile_* method based on the expression type.
    ///
    /// Note that for ease of implementation, integers as well as references to variables are all pushed on stack
    /// which creates a lot of unecessary copies and instructions
    pub fn compile_expr(&mut self, expr: Expr) -> u64 {
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

    /// Compile a function definition.
    ///
    /// Sets up the local variable environment, emits a label, and compiles the function body.
    /// Restores the previous compiler state after the function is compiled.
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
                CodeElement::LocalVar(_ident, _expr) => {
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

    /// Compile a local variable declaration.
    ///
    /// Allocates a new local variable and optionally initializes it with an expression.
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

    /// Compile a return statement.
    ///
    /// Moves the return value to the appropriate frame slot and emits a return instruction.
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

    /// Compile an if/else statement.
    ///
    /// Emits conditional and unconditional jumps and labels for the if/else branches.
    fn compile_if(&mut self, expr: Expr, body: Vec<CodeElement>, else_body: Vec<CodeElement>) {
        match expr.expr_type {
            ExprType::Neq => {
                // in the case of an inequality, the structure of the assembly is as follows:
                // jnz <if_label>
                // <else_body>
                // jmp <end_label>
                // <if_label>
                // <body>
                // <end_label>
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
                // in the case of an equality, the structure of the assembly is as follows:
                // jnz <else_label>
                // <if_body>
                // jmp <end_label>
                // <else_label>
                // <else_body>
                // <end_label>
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

    /// Compile an assert-equal instruction (assignment or assertion).
    ///
    /// Handles assignment to identifiers and local variable creation if needed.
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

    /// Compile a single instruction (statement).
    ///
    /// Dispatches to the appropriate handler based on the instruction type.
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

    /// Compile a top-level code element (function, statement, etc.).
    ///
    /// This is the main entry point for compiling each AST node.
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
