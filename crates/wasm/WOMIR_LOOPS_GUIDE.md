# WOMIR Loops: Implementation & Non-SSA IR Conversion Guide

## Overview

This guide explains how loops are implemented in WOMIR (Write-Once Memory IR)
and how to convert them to a traditional non-SSA IR. WOMIR's loop handling is
fundamentally different from conventional compilers due to its write-once memory
architecture.

## How WOMIR Handles Loops

### 1. **Loop Isolation in Blockless DAG**

Unlike regular blocks that get flattened, loops remain as **separate sub-DAGs**:

```rust
// From src/loader/blockless_dag.rs
BlockKind::Loop => {
    // Loops are NOT merged - they create a new frame
    let mut loop_nodes = Vec::new();

    // Push new frame context
    ctrl_stack.push_front(BlockStack {
        target_type: TargetType::FunctionOrLoop,
        frame_level: ctrl_stack[0].frame_level + 1,
    });

    // Process loop body in separate frame
    process_nodes(sub_dag.nodes, ...);

    // Create Operation::Loop with sub-DAG
    Operation::Loop {
        sub_dag: BlocklessDag { nodes: loop_nodes },
        break_targets: loop_break_targets,
    }
}
```

**Key Point**: Loops become `Operation::Loop` nodes containing their own
`BlocklessDag`, not flattened operations.

### 2. **Frame-Based Loop Architecture**

WOMIR uses a **frame-based approach** where each loop iteration gets its own
memory frame:

```rust
// From src/loader/flattening/mod.rs
Operation::Loop { sub_dag, break_targets } => {
    // Allocate dedicated loop frame
    let (mut loop_reg_gen, layout) =
        ctx.s.allocate_loop_frame_slots(needs_ret_info, saved_fps);

    // Create loop entry point
    let loop_entry = LoopStackEntry {
        label: gens.new_label(LabelType::Loop),
        layout,
        input_regs: /* registers for loop inputs */,
    };

    // Generate code to enter loop
    let enter_loop_directives = jump_into_loop(...);
}
```

### 3. **Loop Control Flow Structure**

#### **Entry Point**

- **`jump_into_loop()`** generates code to:
  - Allocate new loop frame
  - Copy outer frame pointers
  - Copy loop inputs into frame
  - Jump to loop label

#### **Iteration**

- **Each iteration calls `jump_into_loop()` again**
- **New frame allocated every time** (write-once memory requirement)
- Same setup process repeated

#### **Exit Points**

- **`jump_out_of_loop()`** handles breaking out
- Restores outer frame context
- Copies values back to caller frame

## Why This Design Exists

### **Write-Once Memory Requirements**

- **No overwriting allowed** - each iteration needs fresh memory
- **Infinite memory assumption** - frames can be allocated indefinitely
- **Explicit frame management** - no implicit variable reuse

### **Problems for Traditional IRs**

- **Memory explosion** - loops create new frames each iteration
- **No SSA** - values are copied between frames, not merged
- **Frame overhead** - significant memory allocation per iteration

## Converting to Non-SSA IR

### 1. **Basic Approach: Traditional Loop Structure**

Replace WOMIR's frame-based loops with conventional loop constructs:

```rust
// WOMIR Operation::Loop
Operation::Loop {
    sub_dag: BlocklessDag { nodes: loop_nodes },
    break_targets: loop_break_targets,
}

// Convert to traditional IR:
Loop {
    header: BasicBlock,
    body: BasicBlock,
    exit: BasicBlock,
    back_edge: BasicBlock -> header,
}
```

### 2. **Loop Header Creation**

```rust
fn create_loop_header(loop_op: &Operation) -> BasicBlock {
    let header = BasicBlock::new();

    // Add phi-like operations for loop inputs
    for input in &loop_op.inputs {
        // In non-SSA IR, use explicit assignments
        header.add_instruction(Assign {
            dest: input.clone(),
            src: /* value from previous iteration or initial */,
        });
    }

    header
}
```

### 3. **Loop Body Processing**

```rust
fn process_loop_body(sub_dag: &BlocklessDag) -> BasicBlock {
    let body = BasicBlock::new();

    // Convert sub-DAG nodes to instructions
    for node in &sub_dag.nodes {
        match &node.operation {
            Operation::WASMOp(op) => {
                body.add_instruction(convert_wasm_op(op));
            },
            Operation::Br(target) => {
                if is_loop_back_edge(target) {
                    body.add_instruction(Jump(loop_header));
                } else {
                    body.add_instruction(Jump(loop_exit));
                }
            },
            // Handle other operations...
        }
    }

    body
}
```

### 4. **Handling Loop-Carried Values**

Since you're not using SSA, handle loop-carried values with explicit
assignments:

```rust
// For each loop-carried value:
fn handle_loop_carried_value(header: &mut BasicBlock, body: &mut BasicBlock) {
    // In header: assign initial value
    header.add_instruction(Assign {
        dest: "loop_var".to_string(),
        src: "initial_value".to_string(),
    });

    // In body: update value for next iteration
    body.add_instruction(Assign {
        dest: "loop_var".to_string(),
        src: "updated_value".to_string(),
    });

    // Copy back to header for next iteration
    body.add_instruction(Copy {
        from: "loop_var".to_string(),
        to: "loop_var_next".to_string(),
    });
}
```

### 5. **Break Target Resolution**

```rust
fn resolve_break_targets(break_targets: &[(u32, Vec<TargetType>)]) -> Vec<BreakTarget> {
    let mut targets = Vec::new();

    for (depth, target_types) in break_targets {
        for target_type in target_types {
            match target_type {
                TargetType::FunctionOrLoop => {
                    // Function return or outer loop iteration
                    targets.push(BreakTarget::Function);
                },
                TargetType::Label(id) => {
                    // Break to specific label
                    targets.push(BreakTarget::Label(*id));
                }
            }
        }
    }

    targets
}
```

## Implementation Strategy

### **Phase 1: Loop Detection**

1. Find all `Operation::Loop` nodes in blockless DAG
2. Identify loop inputs and outputs
3. Map break targets to their destinations

### **Phase 2: Structure Creation**

1. Create loop header block
2. Create loop body block
3. Create loop exit block
4. Establish control flow edges

### **Phase 3: Value Handling**

1. Convert loop inputs to explicit assignments
2. Handle loop-carried values with copy operations
3. Resolve break/continue semantics

### **Phase 4: Control Flow**

1. Replace WOMIR frame operations with jumps
2. Create back edges for loop iteration
3. Handle break/continue to outer scopes

## Example Conversion

### **WOMIR Loop**

```rust
Operation::Loop {
    sub_dag: BlocklessDag {
        nodes: [
            Node { operation: Operation::WASMOp(Add), inputs: [a, b], outputs: [c] },
            Node { operation: Operation::BrIf(target), inputs: [c] },
        ]
    },
    break_targets: [(0, [TargetType::FunctionOrLoop])],
}
```

### **Non-SSA IR**

```rust
Loop {
    header: BasicBlock {
        instructions: [
            Assign { dest: "a", src: "initial_a" },
            Assign { dest: "b", src: "initial_b" },
        ]
    },
    body: BasicBlock {
        instructions: [
            Add { dest: "c", src1: "a", src2: "b" },
            BranchIf { cond: "c", true: "loop_exit", false: "loop_header" },
        ]
    },
    exit: BasicBlock { /* handle loop exit */ },
}
```

## Key Benefits of This Approach

1. **No SSA complexity** - explicit assignments and copies
2. **Traditional loop structure** - familiar to most IRs
3. **Memory efficient** - no frame explosion
4. **Standard optimizations** - loop unrolling, vectorization, etc.

## Challenges to Consider

1. **Copy elimination** - need to optimize redundant copies
2. **Register allocation** - handle loop-carried values efficiently
3. **Break target mapping** - complex nested loop scenarios
4. **Performance** - ensure loop overhead is minimal

This approach gives you the benefits of WOMIR's analysis while producing a
standard, optimizable IR structure.
