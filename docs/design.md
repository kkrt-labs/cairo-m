# Cairo M Design Document

Cairo M (CPU AIR) is a zero-knowledge virtual machine design optimized for

- Small-field provers: STARK provers using small prime fields (e.g., M31 or
  Babybear)
- Continuation: Arbitrarily long program runs provable out-of-the-box
- Recursion: Direct proof verification within the prover framework, eliminating
  the need for high-level language verifiers
- Low host memory usage: Efficient memory consumption on consumer devices

This document assumes Mersenne31 (M31: `2^31 - 1`) as the prime field. The
design can be adapted for other primes with minor modifications.

This document focuses on the design decisions for the v0 implementation and
potential improvements, rather than describing the current implementation state.

## Memory

Memory segments are implemented as 1D-addressable arrays indexed by field
elements. The maximum length is determined by the prover's prime field.

### Commitment

Memory segments require efficient commitment for continuation, ensuring the
final memory state of stage `n` matches the initial state of stage `n + 1`.

Merkle trees provide the memory commitment mechanism due to their:

- Challenge-independent commitment through initial and final root hashes
- Natural sparse memory handling via partial tree pruning of unused intermediate
  nodes

Efficient recursion requires a ZK-friendly hash function. The current
implementation uses Poseidon2, though alternative hash functions can be
substituted as needed.

The memory address space spans `0..2**30`, eliminating Merkle root padding
requirements while providing sufficient capacity for substantial computations.
Extension to `P - 1` is possible but unnecessary for client-side proving
scenarios, where memory requirements remain modest compared to long-trace
applications. Memory exceeding 2^30 is only required for extremely long runs,
which would demand RAM capacity beyond typical consumer device specifications.

The Merkle tree omits leaf hashing to minimize computational overhead. Although
leaf hashing prevents sibling value disclosure in inclusion proofs, it provides
no benefit for state root proving and is therefore omitted. This approach
requires all memory cells to contain values, resulting in implicit
zero-initialization of the entire memory segment. The default zero value has no
practical impact since the VM overwrites cells with actual values during
execution.

### Read/Write operations

To support arbitrarily long programs within a fixed-size memory segment (2^30 =
1,073,741,824 addresses), the design employs a read-write memory model.

Read and write operations are implemented through lookup arguments:

#### Memory Model

- **Memory entries**: triplets `(address, clock, value)` where `clock`
  timestamps when `address` contained `value`
- **Clock sequence**: monotonic counter from 0, determined during witness
  generation
- **Initial values**: emitted from Merkle commitment with `clock = 0`

#### Operation Mechanics

- **Read/write unification**: Both operations cancel
  `-(address, prev_clock, prev_value)` and add `(address, clock, value)` to the
  lookup sum
- **Read optimization**: When `value == prev_value`, duplicate storage is
  avoided
- **Clock monotonicity**: Enforced via `RangeCheck` component
  (`clock - prev_clock > 0`)

#### Clock Update Component

- **Purpose**: Bridges large temporal gaps between memory operations
- **Trigger**: When clock difference exceeds `RangeCheck` capacity -
  **Implementation**: Updates clock values similarly to read operations
- **Generation**: Trace filled during witness creation, outside VM execution
- **Strategy**: Divides large deltas into range-check-compliant segments

#### Column Cost Analysis

**Read-write memory** (per access):

- Main trace: 5 columns (`address`, `prev_clock`, `clock`, `prev_value`,
  `value`)
- Lookup: 3 columns (subtract previous, add current, range check)
- Total: 5T + 3L

**Read-only memory** (per access):

- Main trace: 2 columns (`address`, `value`)
- Lookup: 1 column
- Total: 2T + 1L

#### Base Column Overhead

Since lookup columns use QM31 (secure field), each represents 4 base columns:

- Overhead per access: `3T + 2L = 3 + 8 = 11` base columns
- STORE operation example (`dst = op0 + op1`): up to 33 additional columns

This overhead can be mitigated using opcodes that write in place (e.g.,
`x += y`). It can also be limited by grouping the logup columns by two,
precomputing the logup sums in pairs when the maximum constraint degree remains
low. If we consequently count only 2 columns per lookup, the memory access
overhead becomes `3 + 2*2 = 7`. If we furthermore consider an in-place
operation, then it becomes `2*(5T + 3L/2) = 10 + 12 = 22` columns for the
read-write memory and `3*(2T + 1L/2) = 6 + 4 = 10` for the read-only memory.

Since the read-write memory allows for much easier control flow, reduces the
need to copy memory values with new frames, and is much easier to reason about
when developing software, this overhead is worthwhile.

On the other hand, not all parts of the memory need to be writable. In
particular, the program with its embedded constant values can remain read-only.
Generally speaking, the easiest way to make a memory segment read-only is to:

- emit all the values with `clock = 0` and the required multiplicity;
- in the `STORE` opcodes, add a range-check to the write addresses to make sure
  that it doesn't write in these segments.

However, range-checking the write address can become inefficient when the
address range is large. In fact, the Cairo M design embeds a RangeCheck20 as the
largest range-check component. This means that any value greater than 2^20 would
require splitting (see also [the clock update section](#readwrite-operations)).
As a consequence, this approach becomes inefficient when the address range is
large.

Another solution is to use a dedicated read-only memory segment with its own
commitment and lookup challenges. This effectively doubles the available address
space, with half being read-only and the other half read-write. Furthermore, if
the read-only memory is used only for the program, its commitment effectively
becomes the program hash that can be used to identify the program in the proof,
without requiring the read-write memory to be initialized with zeros.

### Word size

If the Merkle root allows committing to up to 2^30 field elements, the VM
doesn't need to use a single field element as the base word size. In our first
implementation, we used a fixed-size word built from 4 M31 elements to easily
accommodate all field-element-based instructions in a single read. This
effectively reduces the memory size to 2^28 = 268,435,456.

However, there is no requirement to use such a fixed-size memory word. Given
that memory consistency is enforced only with lookup arguments, each address can
"consume" any number of field elements that are ultimately all summed together
with the relation's challenge coefficients. The only requirement is that an
address always consumes and writes the same number of field elements. One can
think of this as accessing a slice of an array at a given index, with the slice
length depending on the index, instead of just a single value:

```ignore
memory[address: address + len(address)]
```

instead of

```ignore
memory[address]
```

To make limbs of a given address available, we shall introduce the `SPLIT`
opcode. `SPLIT` would simply consume the lookup term with the entire slice at
address `a` and add one term for each of the `len` limbs at addresses
`a + i, i = 0..len`.

The opposite operation is called `JOIN` and would allow the machine to gather a
list of contiguous limbs into a single memory address, consuming each of the
`(a + i; v_i)` lookup terms and adding a single `(a, [v_0, ..., v_{len - 1}])`.

These opcodes don't necessarily need to be added at the VM level, similar to
`ClockUpdate`. They can be determined during witness generation based on
recorded memory accesses. Furthermore, one doesn't need separate JOIN and SPLIT
opcode pairs for each target slice length. In fact, a single component can
perform both `JOIN` and `SPLIT` for all lengths up to the component width. Given
the columns `address`, `limb_0`, `...`, `limb_n`, one just adds to the log up
sum

```latex
sign(opcode_id) * \lbrace (address, [limb_0, ... limb_n]) - \sum_i * (address + i, limb_i) \rbrace
```

where `sign(opcode_id)` would simply be `+` or `-` derived from the actual
opcode ID: set the two opcodes with consecutive IDs, `ID` and `ID + 1`. The
derived sign is just `sign(opcode_id) = 2 * (opcode_id - ID) - 1`.

This variable-sized memory word pattern enables variable-sized instructions and
native handling of multi-limb types like `u32`. This will be further discussed
in the [Native types](#native-types) section.

## Registers

The original Cairo VM uses 3 registers:

- `pc`: the program counter, is the address of the current instruction;
- `fp`: the frame pointer, is a pointer to the current frame initial address;
- `ap`: the application pointer, is the free-memory pointer.

In the context of a read-write memory, the free-memory pointer becomes
unnecessary, and we can drop `ap`, leaving the VM initially with only two
registers, similar to Valida, for example.

Regular instruction sets also leverage registers to store temporary values used
across multiple opcodes, acting as a fast buffer to avoid memory accesses.

From the prover point of view:

- each register requires its own trace cell, i.e., it adds 1 column per
  component;
- a list of registers is like a memory slice (see [Word size](#word-size))
  without address or clock: each component updates the registers' state by
  removing the current state vector and adding the constrained update to the
  logup sum, much like a stack: you can access the last state and push a new
  one.

Ultimately, there is a trade-off between adding registers to the "main register
stack" (i.e., together with `pc` and `fp`) — which adds one column per component
even if the value is not used, and is free otherwise — and adding registers to a
"secondary register stack," or even a tertiary (etc.), in order to save on
unused columns, but at the cost of performing 2 more lookups (4 columns) when
they are needed. The limiting case is to simply use one relation per register,
as if they were regular memory values. Depending on the total number of
components and how the compiler leverages registers, the answer may vary.

Adding registers to the main register stack may be temporarily better but is
less maintainable, as the number of components may vary drastically over time
and usage (see [Reshaping](#minimal-instruction-set) and
[Extensions](#extensions)). The current design doesn't use any secondary
register stack for the sake of simplicity. However, it would most likely be
beneficial to leverage a secondary register stack with 2 values when several
arithmetic operations are performed consecutively.

## Opcodes

The initial Cairo M design was heavily inspired by the Cairo VM. The currently
implemented opcodes reflect this origin but would not be kept as-is if the
project were started today.

### Minimal instruction set

#### Design Principles

**AIR Structure**:

- Columns = constraint variables
- Rows = circuit instantiations
- Each VM cycle adds one row

**Opcode Architecture**:

- One opcode typically maps to one dataframe
- Similar constraints allow opcode factorization
- Fewer opcodes = more cycles for equivalent tasks

**Resource Constraints**:

- Column count affects proof size (one commitment per column)
- Memory usage: `∑(width × length)` total cells
- Witness reshaping trades rows for columns (minimum column count fixed by ISA)

**Continuation Properties**:

- Long traces can split into multiple proofs
- Partial constraint proving is impossible

#### Trade-off Analysis

Minimizing AIR columns maximizes zkVM flexibility at the cost of increased cycle
count. This classic CPU-memory trade-off favors computation over memory usage,
aligning with consumer device constraints (powerful CPUs, limited RAM). The
strategy: minimize AIR width and reshape witnesses to fit available memory.

#### Minimal Opcode Set

**Control Flow**:

- `CallAbsImm`, `Ret`: Function call/return management
- `JmpRelImm`, `JnzFpImm`: Intra-frame jumps

**Arithmetic Operations**:

- `StoreAdd`, `StoreSub`, `StoreMul`, `StoreDiv`: Arithmetic with memory storage

**Memory Operations**:

- `StoreDoubleDerefFpFp`: Store dereferenced value
- `StoreToDoubleDerefFpFp`: Store at dereferenced address

### Extensions

## Native types

## Conclusion

## Appendix

### Opcodes columns

The following is a list of columns for each opcode. Not mentioned is the need
for the enabler column, which distinguishes between the actual trace row and the
padding required for the trace length to be a power of 2.

The instruction is a variable-sized list of field elements, with the first one
always being the opcode ID. The rest is context-dependent and usually denoted as
`off_i` for address offsets or `imm_i` for immediate (i.e., constant) values.
The name `op_i` is used to refer to the `i`-th operand, which is a memory access
at address `fp + off_i`, i.e., `memory[fp + off_i]`.

When several opcodes share the same set of columns, the opcode IDs are used to
select the appropriate constraints. For the sake of simplicity, it is assumed
that the opcode IDs are consecutive, so that the difference between the opcode
ID and the first opcode ID directly yields a Boolean flag. This is not required,
and one could alternatively use the constant `1 / (id_1 - id_0)` between the
opcode ID and the first opcode ID to compute the Boolean flag.

Intermediate values used below are not columns but simply variables computed
from the columns. They don't cost anything.

Constraints are described as arithmetic formulas that should equal 0. The `= 0`
is omitted for the sake of simplicity.

#### CallAbsImm, Ret

Columns:

- pc
- fp
- clock
- inst_prev_clock
- opcode_id
- off0
- imm
- op0_prev_clock
- op0_prev_val
- op0_plus_one_prev_clock
- op0_plus_one_prev_val

Intermediate columns:

- `is_ret = opcode_id - CALL_ABS_IMM_ID`
- `pc_next = imm * (1 - is_ret) + op0_plus_one_prev_val * is_ret`
- `fp_next = (fp + off0 + 2) * (1 - is_ret) + op0_prev_val * is_ret`
- `op0_val = fp * (1 - is_ret) + op0_prev_val * is_ret`
- `op0_plus_one_val = pc * (1 - is_ret) + op0_plus_one_prev_val * is_ret`

Constraints:

- `is_ret * (1 - is_ret)`

Lookups

- update registers
  - `-Registers(pc, fp)`
  - `+Registers(pc_next, fp_next)`
- read instruction from read-only memory
  - `-Memory(pc, 0, opcode_id, off0, off1)`
- read/write operands from memory
  - `-Memory(fp + off0, prev_clock, op0_prev_val)`
  - `+Memory(fp + off0, clock, op0_val)`
  - `-Memory(fp + off0 + 1, prev_clock, op0_plus_one_prev_val)`
  - `+Memory(fp + off0 + 1, clock, op0_plus_one_val)`
- range check clock difference
  - `+RangeCheck20(clock - inst_prev_clock - 1)`
  - `+RangeCheck20(clock - op0_prev_clock - 1)`
  - `+RangeCheck20(clock - op0_plus_one_prev_clock - 1)`

#### JmpRelImm, JnzFpImm

- registers: pc | fp
- global: clock
- instruction: opcode_id | off0 | imm
- operands: memory[fp + off0]: prev_clock | prev_val
- memory[fp + off0 + 1]: prev_clock | prev_val

#### StoreAdd, StoreSub, StoreMul, StoreDiv

- registers: pc | fp
- global: clock
- instruction: opcode_id | off0 | imm
- operands: memory[fp + off0]: prev_clock | prev_val
- memory[fp + off0 + 1]: prev_clock | prev_val

#### StoreDoubleDerefFpFp

- registers: pc | fp
- global: clock
- instruction: opcode_id | off0 | imm
- operands: memory[fp + off0]: prev_clock | prev_val
- memory[fp + off0 + 1]: prev_clock | prev_val

#### StoreDoubleDerefFp

- registers: pc | fp
- global: clock
- instruction: opcode_id | off0 | imm
- operands: memory[fp + off0]: prev_clock | prev_val
- memory[fp + off0 + 1]: prev_clock | prev_val

#### StoreFramePointer

- registers: pc | fp
- global: clock
- instruction: opcode_id | off0 | imm
- operands: memory[fp + off0]: prev_clock | prev_val
- memory[fp + off0 + 1]: prev_clock | prev_val

#### StoreImm

- registers: pc | fp
- global: clock
- instruction: opcode_id | off0 | imm
- operands: memory[fp + off0]: prev_clock | prev_val
- memory[fp + off0 + 1]: prev_clock | prev_val

### Lookups

This entire paper is drafted with Stwo's constraint framework in mind,
particularly the
[logup.rs](https://github.com/starkware-libs/stwo/blob/dev/crates/constraint-framework/src/prover/logup.rs)
module.

#### Core Concept

Logup lookup arguments form a global sum of fractions that must equal zero. Each
component contributes fraction terms with three elements:

1. **Relation**: Defines alpha coefficients and z value for tuple aggregation
2. **Denominator**: Aggregated tuple value in secure field
3. **Numerator** (multiplicity): Usage count of the tuple

#### Storage and Cost

- Terms stored in interaction trace columns
- QM31 secure field requires 4 base columns per lookup column
- Each lookup adds 4 columns to the AIR

#### Optimization Strategy

Columns can be grouped to store pre-summed terms rather than individual terms.
Pre-summing capacity depends on:

- Maximum constraint degree bound
- Variables in looked-up tuples

#### Cumulative Sum Structure

- Each row: cumulative sum of terms
- Last column: cumulative sum of all rows
- Bottom-right cell: total "claimed sum" (committed in proof)

#### Constraint Formula

```
committed_value * current_denominator - current_numerator = 0
```

Requirement: `degree(denominator) + 1 < max_constraint_degree` Given that the
resulting denominator of the sum of two fractions is the product of the two
denominators:

```latex
\frac{a}{b} + \frac{c}{d} = \frac{a * d + c * b}{b * d}
```

one can, for example, pre-sum the terms by two when each denominator has a
degree of 1 and the maximum constraint degree bound is 3.

In this paper, we simply write informally `+/-k Relation(value_0, ..., value_n)`
to refer to the lookup of the tuple `(value_0, ..., value_n)` for the relation
`Relation` with multiplicity `+k` or `-k`, depending on the sign of the
numerator. We refer to "emitted," "yielded," or "added" values when the
multiplicity is positive, and "consumed" or "subtracted" values when the
multiplicity is negative.
