# Cairo M Design Document

Cairo (i.e., CPU AIR) M is a new _zkVM_ design aimed at being especially
efficient with

- small-field provers: STARK provers using small prime fields (e.g., M31 or
  Babybear)
- continuation: _infinitely_ long program runs should be provable out-of-the-box
- recursion: the ability to verify generated proofs directly with the prover
  framework rather than with a verifier written in a high-level language
- low host memory usage

For the sake of simplicity, the remainder of this paper assumes that the
selected prime number is Mersenne31 (M31): `M31 = 2^31 - 1`. Minor adaptations
to the current design specifications would be required for other primes.

Furthermore, this document does not describe the current state of the
implementation but rather the design decisions that were made for this v0
implementation and what could be done to improve it.

## Memory

Memory segments are chosen to be a 1D-addressable array, indexed directly with
field elements. Consequently, its maximum length depends on the prover's prime
field.

### Commitment

Each memory segment needs to be efficiently committed to for _continuation_,
where one needs to make sure that the final memory segment of a given stage `n`
is actually the same as the initial memory of stage `n + 1`.

A Merkle tree is chosen as the memory commitment form because:

- it allows commitment to challenge-independent quantities: the two roots
  (initial and final) of the memory segments
- it naturally handles sparse memory with partial trees: the tree is effectively
  pruned of all intermediate nodes that don't lead to used leaves in the current
  run

In order to allow for efficient recursion, the Merkle tree needs to be
implemented with a ZK-friendly hash function. We chose Poseidon2 in our current
implementation, but this can be easily updated should any other hash function be
preferred.

Ultimately, we chose `0..2**30` for the memory address space as it doesn't
require padding for the Merkle root computation and is large enough to
accommodate reasonably large computations. This could easily be extended to
`P - 1`, but we did not find it relevant in our use cases as large memory
address spaces are mainly relevant for long traces, while we focus on
client-side proving. In fact, only very long runs may require memory larger than
2^30, which in turn would require a host machine with significantly more RAM
than what is typically available in consumer devices.

The Merkle tree furthermore doesn't hash the final leaves (i.e., the memory
values) in order to reduce the number of hashes. While leaf hashing is required
to avoid disclosing sibling values with proofs of inclusion, it doesn't bring
any benefit for proving a state root and can safely be omitted. This, however,
requires all memory cells to have a value, and consequently, the entire memory
segment is implicitly initialized with zeros. This default value doesn't
actually have any impact, as actual initial values are written during the VM
run, whether they are zero or not.

### Read/Write operations

Given that we want to simultaneously be able to run arbitrarily long programs
and have a fixed-size memory segment with a relatively small address space (2^30
= 1,073,741,824), we adopt a read-write memory model.

Read and write operations are actually emulated using lookup arguments:

- a memory entry is modeled as a triplet `(address, clock, value)` where `clock`
  is like a timestamp that lets us record when the `address` had the value
  `value`;
- this `clock` is actually a simple sequence from 0 up to the required length,
  determined during witness generation;
- initial values from the Merkle commitment are emitted with `clock = 0`;
- read and write are actually the same operation from the lookup point of view,
  canceling the previously emitted triplet `-(address, prev_clock, prev_value)`
  and adding a new one to the lookup sum `(address, clock, value)`. In the case
  of a read, i.e., when `value == prev_value`, the witness actually doesn't need
  to store the two (identical) values;
- the `clock` value is enforced to be strictly increasing between two
  consecutive reads or writes, i.e., that `clock - prev_clock > 0`. This
  constraint is enforced with a lookup to a `RangeCheck` component;
- a Clock Update component is introduced to actually update the clock when two
  consecutive memory operations are "too far apart," meaning that the clock
  difference would require a large range check. This `ClockUpdate` component
  effectively just updates the `clock` value of the triplet, much like in a read
  operation, but its trace is filled during witness generation and is not part
  of the VM itself. At this stage, when the adapter observes that two memory
  accesses are "too far apart" with respect to the largest available
  `RangeCheck` component, the delta is divided and as many clock updates are
  introduced as required to ensure that they all fit within the available range.

Altogether, this read-write memory costs up to 5 main trace columns (`address`,
`prev_clock`, `clock`, `prev_value`, `value`) and 3 lookup columns
(`-(address, prev_clock, prev_value)`, `+(address, clock, value)`,
`+(clock - prev_clock - 1)`) per memory access in a component, or 5T + 3L.

This is to be compared with a read-only memory, which would require only 2 main
trace columns (`address`, `value`) and 1 lookup (`-(address, value)`), or 2T +
1L.

Given that a lookup column uses the secure field, which is a `QM31`, each lookup
column consists of 4 base columns. Ultimately, the overhead in terms of base
columns is up to `3T + 2L = 3 + 8 = 11` per memory access. For a STORE operation
like `dst = op0 + op1`, this is up to 33 more columns.

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

Let us first recapitulate what matters most when designing a "ZK-native" minimal
instruction set:

- an AIR can ultimately be viewed as a regular dataframe, where each column is a
  variable used in a constraint and each row is an instantiation of the inlined
  circuit defined on the column variables;
- usually, one opcode corresponds to one dataframe, but several opcodes can also
  be factorized into a single dataframe when the constraints are similar;
- each cycle of the VM adds a row to one of these dataframes;
- the fewer opcodes in the instruction set, the more cycles the VM needs to
  perform for the same task;
- all these dataframes are defined in independent "components" but are
  ultimately concatenated on axis 1 to form one single large dataframe (this was
  true with old provers requiring padding of the smallest ones; not fully
  accurate with Stwo, but for the sake of simplicity, one can continue viewing
  this as one single large table);
- given a witness, reshaping to reduce the number of rows and add more columns
  is always possible (simply duplicate the constraints), but there is a minimum
  number of columns that depends on the instruction set;
- the number of columns will impact the proof size and increase the verifier
  work as there is one commitment per column in the proof;
- the peak memory consumption of the host machine when proving is directly
  related to the total number of cells, i.e., `\sum width * length`;
- long traces can always be split into several proofs with continuation, while
  you cannot prove only half of the constraints.

In conclusion, the fewer columns in the AIR, the better, in order to achieve
more flexibility in the downstream usage of the zkVM, at the cost of generating
more cycles for end programs. This cycles-versus-AIR-width trade-off is actually
a classic CPU-versus-memory trade-off: you can always trade memory usage for
more CPU cycles, and vice versa. Since standard consumer devices typically
feature powerful processors but have very limited RAM resources, and RAM is more
expensive than compute, it makes sense to minimize the AIR width as much as
possible and reshape witnesses to fit the actual device memory availability.

Altogether, we shall keep the following opcodes for this minimal ZK-native ISA,
with one line per component:

- CallAbsImm, Ret;
  - create a new call stack and return to the calling position when done
- JmpRelImm, JnzFpImm;
  - jump to a new PC without leaving the current frame
- StoreAdd, StoreSub, StoreMul, StoreDiv;
  - store the result of the given arithmetic operation in memory
- StoreDoubleDerefFpFp, StoreToDoubleDerefFpFp;
  - store the dereferenced memory value or store at the dereferenced memory
    address.

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

At a high level, the lookup arguments with logup are simply a large sum of
fractions that must sum to 0. Each component adds terms to this large global
sum. These terms are fractions defined by:

- a `Relation` that defines the alpha coefficients and z value to be used to
  aggregate the looked-up tuple;
- a denominator, which is the aggregated value of the looked-up tuple in the
  secure field;
- a numerator, also referred to as multiplicity, which is actually the number of
  times the looked-up tuple is "used" or "emitted".

All of these terms are ultimately stored as regular columns in the trace,
referred to as the interaction trace. Because the secure field is a `QM31`, each
of these columns is actually 4 base columns. Consequently, each lookup adds 4
columns to the AIR and not only 1.

Because the only goal of all these columns is to compute the large global
cumulative sum of all the logup terms, it is possible to group these columns by
storing not only one term but the sum of several terms that need to be summed
together. The number of terms that one can "pre-sum" depends on the maximum
constraint degree bound and the variables used in the looked-up tuples.

In fact, the trace stores in each row the cumulative sum of all the terms, and
in the last column of the interaction trace, the cumulative sum of all the rows,
so that the final bottom-right cell ultimately contains the cumulative sum of
all the terms added by the component. This value is known as the "claimed sum"
and is committed to in the proof.

Given this construction, the constraint enforced for each cell in each row
ultimately becomes:

```ignore
committed_value * current_denominator - current_numerator = 0
```

which means that `degree(denominator) + 1` must remain less than the maximum
constraint degree bound of the component. Given that the resulting denominator
of the sum of two fractions is the product of the two denominators:

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
