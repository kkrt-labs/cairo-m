# Cairo M Design Document

## Introduction

The motivation behind building a new zkVM is strongly influenced by our
experience of building a
[non-provable EVM client in Cairo Zero](https://github.com/kkrt-labs/keth), the
provable language (zkDSL) of Starkware, targeting the Cairo VM. How can a
program written in a zkDSL eventually not be provable? Not because there are any
logic issues, no, but just because of scaling issues! The Cairo VM has just not
been designed to prove billions-long traces, nor to leverage parallel proving
with recursion.

Facing this hard truth made us re-evaluate the design of the Cairo VM in the
light of the real needs of the current ZK ecosystem. Actually, some decision may
be relevant when considering a given order of magnitude of program length
(around 10^5 steps at most), but become irrelevant when considering a much
larger program length (10^8 steps at least). Furthermore, though being
supposedly a general-purpose VM, it has been design mainly with Starknet in
mind, i.e., with a focus on (small) transaction processing, rather than
general-purpose computation.

The original Cairo architecture, as described in the seminal paper
[Cairo – a Turing-complete STARK-friendly CPU architecture](https://eprint.iacr.org/2021/1063.pdf)
defines a general framework for building a ZK-friendly CPU, known as a
zero-knowledge Virtual Machine (zkVM) nowadays. This framework is general both
in terms of underlying proving scheme and base operating prime field. However,
some design decisions (like the instruction encoding) require a prime field
larger that 2^64, while modern STARK provers favor smaller prime fields like
Babybear (2^31 - 2^27 + 1) or Mersenne31 (2^31 - 1). Consequently, even the
recent [stwo-cairo](https://github.com/starkware-libs/stwo-cairo) prover
emulates the original prime number chosen 5 years ago
[$2^251 + 17 * 2^192 + 1$](https://docs.starknet.io/learn/protocol/cryptography).
This emulation makes the prover up to 28x less efficient as each native field
element from the original Cairo VM is now up to 28 M31s, depending on the actual
values used in the program and some optimizations.

Furthermore, the Cairo VM features a non-deterministic read-only memory model
with relocation, which creates two severe limitations:

1. a program can only make a limited number of writes to memory, so the VM
   cannot run arbitrary long (meaningful) programs;
2. the final relocation step prevents from streaming the generated trace for
   parallel proving (a technique called
   [_continuation_](https://risczero.com/blog/continuations)) as final memory
   addresses are only known after the program has executed.

Cairo M has been designed to overcome these limitations:

- Leverage small-field provers: STARK provers using small prime fields (e.g.,
  M31 or Babybear);
- Continuation: Arbitrarily long program runs provable out-of-the-box
- Recursion: Direct proof verification within the prover framework, eliminating
  the need for high-level language verifiers;
- Low host memory usage: Efficient memory consumption on consumer devices.

This following of this document assumes Mersenne31 (M31: `2^31 - 1`) as the
prime field. The design can be adapted for other primes with minor
modifications.

This document focuses on the design decisions for the v0 implementation and
potential improvements, rather than describing the current implementation state.

The design of a virtual machine mainly encompasses the memory model, the
registers, the opcodes and the addressing scheme. The remaining of this document
addresses each of these questions in turn. An Appendix section provides general
knowledge about some part of STARK provers and especially the
[Stwo framework](https://github.com/starkware-libs/stwo), used by Cairo M.

## Memory

Memory segments are implemented as 1D-addressable arrays indexed by field
elements. Their maximum length is determined by the prover's prime field.

### Read/Write operations

To support arbitrarily long programs within a fixed-size memory segment, the
design employs a read-write memory model for the RAM (Random Access Memory).

Read and write operations are actually implemented through
[lookup arguments](#lookups): each memory access is actually a lookup of a tuple
`(address, clock, value)`. `clock` is a monotonic counter from 0, determined
during witness generation. It timestamps when `address` contained `value`.

To access a memory cell, one actually adds to the logup sum a term cancelling
the previous access, and a new term for registering the new access. As there is
no ordering in a global logup sum, the notion of "previous access" is enforced
with a range-check argument on the clock difference: `clock - prev_clock > 0`.

All together, using the notation defined in the [lookups](#lookups) section, a
memory read or write operation is implemented as follows:

- `-Memory(address, prev_clock, prev_value)`
- `+Memory(address, clock, value)`
- `+RangeCheck20(clock - prev_clock - 1)`

with `address`, `prev_clock`, `prev_value`, `clock` and `value` being part of
the main execution trace. Note that when the memory is only read, one has
`prev_value = value` and this simplifies to:

- `-Memory(address, prev_clock, value)`
- `+Memory(address, clock, value)`
- `+RangeCheck20(clock - prev_clock - 1)`

The key point of this design based on lookup arguments is that one needs to
actually remove from the logup sum a term added at a point of time strictly
before the current point of time, and that one adds terms with multiplicity 1
only. Adding terms with multiplicity greater that 1 would actually make it
possible for the prover to "fork" the memory at some point, accessing a value
already normally updated during the execution. The boundary conditions (initial
and final memory) are handled by the [memory commitment](#commitment) and the
public memory of the proof.

### Clock Update Component

The clock update component is responsible for updating the clock value when the
clock difference exceeds the capacity of the `RangeCheck` component. It is not
part of the VM specification but is part of the prover implementation. During
witness generation, the prover checks what are the required clock updates. If it
encounters a clock difference exceeding the capacity of its `RangeCheck`
component, it performs a clock update, which essentially consists in mimicking a
read operation:

- `-Memory(address, prev_clock, prev_value)`
- `+Memory(address, clock + RC_LIMIT, prev_value)`

It eventually adds as many clock updates as needed to cover the clock
difference.

### Column Cost Analysis

Let us denote by `T` a regular trace column and by `L` a lookup operation.

**Read-write memory** (per access):

- Main trace: 4 to 5 columns (`address`, `prev_clock`, `clock`, `prev_value`,
  `value`)
- Lookup: 3
- Total: up to 5T + 3L

**Read-only memory** (per access):

- Main trace: 2 columns (`address`, `value`)
- Lookup: 1 `-Memory(address, value)`
- Total: 2T + 1L

Since lookup columns are defined over the secure field, which is QM31 (i.e., 4
M31s), each lookup column is actually 4 trace columns.

- Overhead per access: `(5T + 3L) - (2T + 1L) = 3T + 2L = 3 + 8 = 11` base
  columns
- STORE operation example (`dst = op0 + op1`): up to 31 additional columns

This overhead can be mitigated using opcodes that write in place (e.g.,
`x += y`). It can also be limited by grouping the logup columns by two,
precomputing the logup sums in pairs when the maximum constraint degree remains
low (see [Cumulative Sum Structure](#cumulative-sum-structure)). If we
consequently count only 2 columns per lookup, the memory access overhead becomes
`3 + 2*2 = 7`. If we furthermore consider an in-place operation, then it becomes
`(5T + 3L) + (4T + 3L) = 9 + 12 = 21` columns for the read-write memory and
`3*(2T + 1L/2) = 6 + 8 = 14` for the read-only memory.

Since the read-write memory allows for much easier control flow, reduces the
need to copy memory values with new frames, and is much easier to reason about
when developing software, this overhead is worthwhile.

On the other hand, not all parts of the memory need to be writable. In
particular, the program with its embedded constant values can remain read-only.
Generally speaking, the easiest way to make a memory segment read-only is to:

- emit all the values with `clock = 0` and the required multiplicity from the
  commitment or the public memory;
- in the `STORE` opcodes, add a range-check to the write addresses to make sure
  that it doesn't write in these segments;
- add opcodes that perform arithmetic operations on the read-only memory, as
  long as the result is written back to the read-write memory.

However, range-checking the write address can become inefficient when the
address range is large. In fact, the Cairo M design embeds a `RangeCheck20`
(i.e. 20-bit range-check) as the largest single range-check component (i.e.,
with no limb splitting). This means that any value greater than 2^20 would
require splitting (see also [the clock update section](#readwrite-operations)).
As a consequence, this approach becomes inefficient when the address range is
large.

Another solution is to use a dedicated read-only memory segment with its own
commitment and lookup challenges. This effectively doubles the available address
space, with half being read-only and the other half read-write. Furthermore, if
the read-only memory is used only for the program, its commitment effectively
becomes the program hash that can be used to identify the program in the proof,
without requiring the read-write memory to be initialized with zeros.

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

The natural memory address space spans `0..P`, but to avoid Merkle root padding
requirements while providing sufficient capacity for substantial computations,
we simply use the greatest power of 2 smaller than `P`, i.e., `2^30`. Extension
to `P - 1` is possible but unnecessary for client-side proving scenarios, where
memory requirements remain modest compared to long-trace applications. Memory
exceeding `2^30` is only required for extremely long runs, which would demand
RAM capacity beyond typical consumer device specifications.

Furthermore, the Merkle tree omits leaf hashing to minimize computational
overhead. Although leaf hashing prevents sibling value disclosure in inclusion
proofs, it provides no benefit for state root proving and is therefore omitted.
This approach requires all memory cells to contain values, resulting in implicit
zero-initialization of the entire memory segment. The default zero value has no
practical impact since the VM overwrites cells with actual values during
execution.

The Merkle commitment component is responsible for proving the leaves from the
public (initial or final) root. It does this by iteratively consuming a root and
emitting the leaves with given multiplicity in the logup sum. The partial
underlying Merkle tree is built during witness generation. The component only
enforces via the lookup arguments that the nodes and leaves actually derive from
the root, using the `Merkle` relation. It also uses the `Poseidon2` relation to
prove the Poseidon2 hash computation. Eventually, the multiplicity at any given
node can be set to 0 if the branch is actually not used, in which case the node
is pruned from the tree.

All the emitted leaves are eventually consumed by the `Memory` component to make
them available for the opcodes.

### Word size

If the Merkle root allows committing to up to 2^30 field elements, the VM
doesn't need to use a single field element as the base word size. As said in the
previous section, the `Memory` component is responsible for turning a list of
M31 leaves into memory values. These leaves can actually be grouped together as
limbs of a single memory word.

In our first implementation, we used a fixed-size word built from 4 M31 elements
to easily accommodate all field-element-based instructions in a single read.
This effectively reduces the memory size to 2^28 = 268,435,456.

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

Regular instruction set architectures also leverage registers to store temporary
values used across multiple opcodes, acting as a fast buffer to avoid memory
accesses.

From the prover point of view:

- each register requires its own trace cell, i.e., it adds 1 column per
  component;
- a list of registers is like a memory slice (see [Word size](#word-size))
  without address nor clock: each component updates the registers' state by
  removing the current state vector and adding the constrained update to the
  logup sum, much like a stack: you can access the last state and push a new
  one.

Ultimately, there is a trade-off between adding registers to the "main register
stack" (i.e., together with `pc` and `fp`) — which adds one column per component
even if the value is not used, and is free otherwise — and adding registers to a
"secondary register stack", or even a tertiary (etc.), in order to save on
unused columns, but at the cost of performing 2 more lookups (4 columns) when
they are needed. The limiting case is to simply use one relation per register,
as if they were regular memory values with no address nor clock. Depending on
the total number of components and how the compiler leverages registers, the
optimal case may vary.

Adding registers to the main register stack may be temporarily better but is
less maintainable, as the number of components may vary drastically over time
and usage (see [Reshaping](#minimal-instruction-set) and
[Extensions](#extensions)). The current design doesn't use any secondary
register stack for the sake of simplicity. However, it would most likely be
beneficial to leverage a secondary register stack with 2 values when several
arithmetic operations are performed consecutively, or to easily factorize
arithmetic operations over read-write and read-only memory.

## Opcodes

The initial Cairo M design was heavily inspired by the Cairo VM. The currently
implemented opcodes reflect this origin but would not be kept as-is if the
project were started today.

### AIR basics

An AIR (Algebraic Intermediate Representation) is a way to represent a
computation in a way that is easy to prove. It is a set of constraints that must
be satisfied by the computation.

For the sake of simplicity, let use describe an AIR as a dataframe, with columns
representing variables used in the defined constraint system (circuit) and rows
representing circuit instantiations. All the constraints are eventually
described as a polynomial combination of the columns. For example, given a
dataframe `df` with columns `a`, `b`, `c`, the constraint `a + b = c` is
described as `df[a] + df[b] - df[c] = 0` and actually applies to all of the rows
of the dataframe.

During proof generation, each column is interpreted as values of a given
polynomial over a base set ${x^i}_{i=0}^n$. This polynomial is interpolated and
evaluated over a bigger domain. The prover commits to each column (each
polynomial) and then generate Merkle inclusion proofs for some evaluations of
these polynomials at random points. This means that the proof size and the
verifier complexity are directly related to the number of columns in the AIR:
the more columns, the more commitments and the more verifier complexity.

The Stwo framework lets define the whole AIR of the state transition of the
machine in several such dataframes, called
[_components_](https://docs.starknet.io/learn/s-two/air-development/components)
(other frameworks may call them _chips_). Eventually, they are all concatenated
by the column axis to form the whole AIR.

### Design principles

Generally speaking, a reduced instruction set will generate more cycles, i.e.
more rows, for a given operation than a complex instruction set. On the other
hand, a complex instruction set will require more columns, i.e. more
commitments, for a given operation than a reduced instruction set. In short, a
reduced instruction set is a long and thin dataframe, while a complex
instruction set is a short and wide one.

Notice however that, given a component with shape (`n`, `m`) (`n` rows and `m`
columns), one can always reshape it to (`n / k`, `m * k`), where `k` is an
integer, actually duplicating the columns and their corresponding constraints.
The other way around is not possible: one cannot keep only a partial circuit. In
other words, a reduced instruction set trace can always be reshaped to "look
like" a complex instruction set one, while the other way around is not possible.
Hence, reduced instruction sets give more flexibility.

Furthermore, long traces can be proven in parallel, even when the program is
still running (so-called
[_continuation_](https://risczero.com/blog/continuations)) and aggregated later
on with recursions, reducing either the proving time or the memory usage of the
host, which is directly proportional to the area of the AIR, (i.e. width times
height).

Consequently, when designing an AIR, one tries to limit the number of columns as
much as possible. This can be done by both limiting the number of opcodes in the
instruction set, and by factorizing as much as possible several opcodes into the
same component.

### Minimal instruction set

**Control Flow**:

- `CallAbsImm`, `Ret`: Function call/return management
- `JmpRelImm`, `JnzFpImm`: Intra-frame jumps

**Arithmetic Operations**:

- `StoreAdd`, `StoreSub`, `StoreMul`, `StoreDiv`: Arithmetic with memory storage

**Memory Operations**:

- `StoreDoubleDeref`: Store dereferenced value
- `StoreToDoubleDeref`: Store at dereferenced address

This proposed instruction set fits in a total of XXX columns. See the
[Opcodes columns](#opcodes-columns) section for more details.

### Extensions

#### Native types

#### Built-in functions

## Conclusion

## Appendix

### Opcodes columns

This described the detailed list of columns for each component. Not mentioned is
the possible need for an enabler column, which distinguishes between the actual
trace row and the padding required for the trace length to be a power of 2.

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

A lookup argument in zero-knowledge proofs is a cryptographic primitive that
allows a prover to demonstrate that certain values in a computation trace exist
in another table, without revealing the specific values or their positions. The
prover commits to a "claimed sum" of lookup terms, and the verifier checks that
this sum equals zero, ensuring all looked-up values are valid according to the
specified relation constraints.

This entire paper is drafted with Stwo's constraint framework in mind, which
uses [LogUp lookup arguments](https://eprint.iacr.org/2022/1530.pdf), see the
[logup.rs](https://github.com/starkware-libs/stwo/blob/dev/crates/constraint-framework/src/prover/logup.rs)
module for more details. Lookup and logup terms are used interchangeably in this
document to denote a relation between two components.

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

```ignore
committed_value * current_denominator - current_numerator = 0
```

Requirement: `degree(denominator) + 1 < max_constraint_degree`. Given that the
resulting denominator of the sum of two fractions is the product of the two
denominators:

```latex
\frac{a}{b} + \frac{c}{d} = \frac{a * d + c * b}{b * d}
```

one can, for example, pre-sum the terms by two when each denominator has a
degree of 1 and the maximum constraint degree bound is 3.

Through this document, we simply write `+/-k Relation(value_0, ..., value_n)` to
refer to the lookup of the tuple `(value_0, ..., value_n)` for the relation
`Relation` with multiplicity `+k` or `-k`, depending on the sign of the
numerator. We refer to "emitted," "yielded," or "added" values when the
multiplicity is positive, and "consumed" or "subtracted" values when the
multiplicity is negative.
