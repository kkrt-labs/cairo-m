# Cairo M Design Document

## Introduction \label{sec:introduction} {#introduction}

The motivation behind building a new _Zero-Knowledge Virtual Machine (zkVM)_ is
strongly influenced by our experience of building a
[non-provable EVM client in Cairo Zero](https://github.com/kkrt-labs/keth), the
provable language (zkDSL) of Starkware, targeting the
[Cairo VM](https://eprint.iacr.org/2021/1063.pdf). How can a program written in
a zkDSL eventually not be provable? Not because there are any logic issues, no,
but just because of scaling issues! The Cairo VM has just not been designed to
prove billions-long traces, nor to leverage parallel proving with recursion.

Facing this hard truth made us re-evaluate the design of the Cairo VM in light
of the real needs of the current ZK ecosystem. Actually, some decisions may be
relevant when considering a given order of magnitude of execution length (around
$10^5$ steps at most), but become irrelevant when considering a much larger
execution length ($10^8$ steps at least). Furthermore, though being supposedly a
general-purpose VM, it has been designed mainly with Starknet in mind, i.e.,
with a focus on (small) transaction processing, rather than general-purpose
computation.

The original Cairo architecture, as described in the seminal paper
[Cairo – a Turing-complete STARK-friendly CPU architecture](https://eprint.iacr.org/2021/1063.pdf)
defines a general framework for building a ZK-friendly CPU, denominated as a
"CPU AIR" and known as a zero-knowledge Virtual Machine (zkVM) nowadays. This
framework is general both in terms of underlying proving scheme and base
operating prime field. However, some design decisions (like the instruction
encoding) require a prime field larger than $2^{64}$, while modern STARK provers
favor smaller prime fields like Babybear ($2^{31} - 2^{27} + 1$) or Mersenne31
($2^{31} - 1$). Consequently, even the recent
[stwo-cairo prover](https://github.com/starkware-libs/stwo-cairo) emulates the
original prime number chosen 5 years ago:
[$2^{251} + 17 \cdot 2^{192} + 1$](https://docs.starknet.io/learn/protocol/cryptography).
This emulation makes the prover up to 28x less efficient as each native field
element from the original Cairo VM is now
[up to 28 M31s](https://github.com/starkware-libs/stwo-cairo/blob/main/stwo_cairo_prover/crates/common/src/memory.rs#L1),
depending on the actual values used in the program and some optimizations.

Furthermore, the Cairo VM features a non-deterministic read-only memory model
with relocation, which creates two severe limitations:

1. a program can only make a limited number of writes to memory, so the VM
   cannot run arbitrary long (meaningful) programs;
2. the final relocation step prevents from streaming the generated trace to
   start proving chunks in parallel while the program is still running (a
   technique called [_continuation_](https://risczero.com/blog/continuations))
   as final memory addresses are only known after the program has exited.

Cairo M has been designed to overcome these limitations:

- Leverage small-field provers: STARK provers using small prime fields (e.g.,
  M31 or Babybear);
- Continuation: Arbitrarily long program runs provable out-of-the-box
- Recursion: Direct proof verification within the prover framework, eliminating
  the need for high-level language verifiers;
- Low host memory usage: Efficient memory consumption on consumer devices.

The following sections of this document assume Mersenne31 (M31: $2^{31} - 1$) as
the prime number. The design can be adapted for other primes with minor
modifications.

This document doesn't describe the current state of the
[Cairo M](https://github.com/kkrt-labs/cairo-m) nor the
[v0.1 release](https://github.com/kkrt-labs/cairo-m/releases/tag/v0.1.0) but
focuses on the decision framework and trade-offs considered when building this
first version, and potential improvements.

The design of a virtual machine mainly encompasses four decisions: the memory
model, the number of registers, the opcodes and the addressing scheme. The
remaining of this document addresses each of these questions in turn. An
Appendix section provides a succinct background about some part of STARK provers
and especially the [Stwo framework](https://github.com/starkware-libs/stwo) used
in our implementation.

## Memory \label{sec:memory} {#memory}

In the context of a Virtual Machine, the memory is the main data structure that
stores the program and the data. It is typically organized as a linear array of
addressable units (bytes, words, or field elements), where each location has a
unique address. The VM's processor reads instructions from memory, loads/stores
data values, and manages the execution state through memory operations.

Since memory segments are 1D-addressable arrays indexed by field elements, their
maximum length is determined by the prover's prime field.

### Read/Write operations \label{sec:read-write-operations} {#read-write-operations}

To support arbitrarily long programs within a fixed-size memory segment, the
design employs a read-write memory model for the RAM (Random Access Memory).

Read and write operations are implemented through [lookup arguments](#lookups):
each memory access is a lookup of a tuple
$(\text{address}, \text{clock}, \text{value})$. The $\text{clock}$ is a
monotonic counter from 0, determined during witness generation. It timestamps
when $\text{address}$ contained $\text{value}$.

To access a memory cell, one adds to the logup sum a term cancelling the
previous access, and a new term for registering the new access. As there is no
ordering in a global logup sum, the notion of "previous access" is enforced with
a range-check argument on the clock difference:
$\text{clock} - \text{prev\_clock} > 0$.

Altogether, using the notation defined in the [lookups](#lookups) section, a
memory read or write operation is implemented as follows:

$$
\begin{aligned}
&-\text{Memory}(\text{address}, \text{prev\_clock}, \text{prev\_value}) \\
&+\text{Memory}(\text{address}, \text{clock}, \text{value}) \\
&+\text{RangeCheck20}(\text{clock} - \text{prev\_clock} - 1)
\end{aligned}
$$

with $\text{address}$, $\text{prev\_clock}$, $\text{prev\_value}$,
$\text{clock}$ and $\text{value}$ being part of the main execution trace. Note
that when the memory is only read, one has $\text{prev\_value} = \text{value}$
and this simplifies to:

$$
\begin{aligned}
&-\text{Memory}(\text{address}, \text{prev\_clock}, \text{value}) \\
&+\text{Memory}(\text{address}, \text{clock}, \text{value}) \\
&+\text{RangeCheck20}(\text{clock} - \text{prev\_clock} - 1)
\end{aligned}
$$

The key point of this design based on lookup arguments is that one needs to
remove from the logup sum a term added at a point in time strictly before the
current point in time, and that one adds terms with multiplicity 1 only. Adding
terms with multiplicity greater than 1 would make it possible for the prover to
"fork" the memory at some point, accessing a value already normally updated
during the execution. The boundary conditions (initial and final memory) are
handled by the [memory commitment](#commitment) and the public memory of the
proof.

### Clock Update Component \label{sec:clock-update-component} {#clock-update-component}

The clock update component is responsible for updating the clock value when the
clock difference exceeds the capacity of the `RangeCheck` component. It is not
part of the VM specification but is part of the prover implementation. During
witness generation, the prover checks which clock updates are required. If it
encounters a clock difference exceeding the capacity of its RangeCheck
component, it performs a clock update, which essentially consists in mimicking a
read operation:

$$
\begin{aligned}
&-\text{Memory}(\text{address}, \text{prev\_clock}, \text{prev\_value}) \\
&+\text{Memory}(\text{address}, \text{prev\_clock} + \text{RC\_LIMIT}, \text{prev\_value})
\end{aligned}
$$

It then adds as many clock updates as needed to cover the clock difference. Let
us denote by $\text{RC\_LIMIT}$ the capacity of the RangeCheck component and
$\delta$ the required clock difference. The number of clock updates required is
$\lceil \delta / \text{RC\_LIMIT} \rceil$. If $\delta$ is not a multiple of
$\text{RC\_LIMIT}$, one needs to add one more clock update.

### Column Cost Analysis \label{sec:column-cost-analysis} {#column-cost-analysis}

Let us denote by $T$ a regular trace column and by $L$ a lookup operation.

**Read-write memory** (per access):

- Main trace: 4 to 5 columns (address, prev_clock, clock, prev_value, value)
- Lookup: 3
- Total: up to $5T + 3L$

**Read-only memory** (per access):

- Main trace: 2 columns (address, value)
- Lookup: 1 $-\text{Memory}(\text{address}, \text{value})$
- Total: $2T + 1L$

Since lookup columns are defined over the secure field, which is QM31 (i.e., 4
M31s), each lookup column is 4 trace columns.

- Overhead per access: $(5T + 3L) - (2T + 1L) = 3T + 2L = 3 + 8 = 11$ base
  columns
- STORE operation example ($\text{dst} = \text{op0} + \text{op1}$): up to 31
  additional columns (2 reads and 1 write)

This overhead can be mitigated using opcodes that write in place (e.g.,
`x += y`). It can also be limited by grouping the logup columns by two,
precomputing the logup sums in pairs when the maximum constraint degree remains
low (see [Cumulative Sum Structure](#cumulative-sum-structure)). If we
consequently count only 2 columns per lookup, the memory access overhead becomes
`3 + 2*2 = 7`. If we further consider an in-place operation, then it becomes
$(5T + 3L) + (4T + 3L) = 9 + 12 = 21$ columns for the read-write memory and
$3 \times (2T + 1L/2) = 6 + 8 = 14$ for the read-only memory.

Since the read-write memory allows for much easier control flow, reduces the
need to copy memory values with new frames, and is much easier to reason about
when developing software, this overhead is deemed worthwhile.

On the other hand, not all parts of the memory need to be writable. In
particular, the program with its embedded constant values can remain read-only.

The most direct way to make sure that an address space is read-only is to
range-check (or lookup) the write address in all the `STORE` opcodes. This would
however not save any columns, just enforce that some regions are unchanged. To
save on columns, one should instead add dedicated opcodes that would only
"consume" the memory value with a constant clock set to 0. These values would be
added from the commitment or the public memory with the required multiplicity.

However, range-checking the write address can become inefficient when the
address range is large. In fact, the Cairo M design embeds a RangeCheck20 (i.e.,
20-bit range-check) as the largest single range-check component (i.e., with no
limb splitting). This means that any value greater than $2^{20}$ would require
splitting (see also [the clock update section](#read-write-operations)). As a
consequence, this approach becomes inefficient when the address range is large.

Another solution is to use a dedicated read-only memory segment with its own
commitment and lookup challenges. This effectively doubles the available address
space, with half being read-only and the other half read-write. Furthermore, if
the read-only memory is used only for the program, its commitment effectively
becomes the program hash that can be used to identify the program in the proof,
without requiring the read-write memory to be initialized with zeros.

### Commitment \label{sec:commitment} {#commitment}

Memory segments require efficient commitment for continuation, ensuring the
final memory state of stage `n` matches the initial state of stage `n + 1`.

Merkle trees provide a good memory commitment mechanism due to their:

- Challenge-independent commitment through initial and final root hashes
- Natural sparse memory handling via partial tree pruning of unused intermediate
  nodes

Efficient recursion requires a ZK-friendly hash function. The current
implementation uses Poseidon2, though alternative hash functions can be
substituted as needed.

The natural memory address space spans $[0, P)$, but to avoid Merkle root
padding requirements while providing sufficient capacity for substantial
computations, we simply use the greatest power of 2 smaller than $P$, i.e.,
$2^{30}$. Extension to $P - 1$ is possible but unnecessary for client-side
proving scenarios, where memory requirements remain modest compared to
long-trace applications. Memory exceeding $2^{30}$ is only required for
extremely long runs, which would demand RAM capacity beyond typical consumer
device specifications.

Furthermore, the Merkle tree omits leaf hashing to minimize computational
overhead. Although leaf hashing prevents sibling value disclosure in inclusion
proofs, it provides no benefit for state root commitment and is therefore
omitted. This approach requires all memory cells to contain values, resulting in
implicit zero-initialization of the entire memory segment. This default zero
value has no practical impact since the VM overwrites cells with actual values
during the execution.

The Merkle commitment component is responsible for proving the $2^{30}$ leaves
from the public (initial or final) root. It does this by iteratively consuming a
root and emitting the two leaves with given multiplicity in the logup sum. The
partial underlying Merkle tree is built during witness generation. The component
only enforces via the lookup argument that the nodes and leaves actually derive
from the root, using the `Merkle` relation. It also uses the `Poseidon2`
relation to prove the Poseidon2 hash computation. Eventually, the multiplicity
at any given node can be set to 0 if the branch is actually not used, in which
case the node is pruned from the tree.

All the emitted leaves are eventually consumed by the `Memory` component to make
them available for the opcodes.

### Word size \label{sec:word-size} {#word-size}

If the Merkle root allows committing to up to $2^{30}$ field elements, the VM
doesn't need to use a single field element as the base word size. As mentioned
in the previous section, the Memory component is responsible for turning a list
of M31 leaves into memory values. These leaves can actually be grouped together
as limbs of a single memory word.

In our first implementation, we used a fixed-size word built from 4 M31 elements
to easily accommodate all field-element-based instructions in a single read.
This effectively reduces the memory size to $2^{28} = 268{,}435{,}456$.

However, there is no requirement to use such a fixed-size memory word. Given
that memory consistency is enforced only with lookup arguments, each address can
consume any number of field elements that are ultimately all summed together
with the relation's challenge coefficients. The only requirement is that
consecutive reads or writes of an address consume and write the same number of
field elements. One can think of this as accessing a slice of an array at a
given index, with the slice length depending on the index, instead of just a
single value:

```python
memory[address: address + len(address)]
```

instead of

```python
memory[address]
```

To make limbs of a given address available, we introduce the `SPLIT` opcode.
`SPLIT` simply consumes the logup term with the entire slice at address $a$ and
adds one term for each of the $\text{len}$ limbs at addresses
$a + i, i \in [0, \text{len})$:

$$
\begin{aligned}
&-\text{Memory}(a, \text{prev\_clock}, v_0, \ldots, v_{\text{len} - 1}) \\
&+\text{Memory}(a, \text{clock}, v_0) \\
&+\text{Memory}(a + 1, \text{clock}, v_1) \\
&\vdots \\
&+\text{Memory}(a + \text{len} - 1, \text{clock}, v_{\text{len} - 1})
\end{aligned}
$$

The opposite operation is called `JOIN` and allows the machine to gather a list
of contiguous limbs into a single memory address, consuming each of the
$(a + i, v_i)$ lookup terms and adding a single
$(a, [v_0, \ldots, v_{\text{len} - 1}])$ lookup term:

$$
\begin{aligned}
&-\text{Memory}(a, \text{prev\_clock}, v_0) \\
&\vdots \\
&-\text{Memory}(a + \text{len} - 1, \text{prev\_clock}, v_{\text{len} - 1}) \\
&+\text{Memory}(a, \text{clock}, v_0, \ldots, v_{\text{len} - 1})
\end{aligned}
$$

These opcodes don't necessarily need to be added at the VM level, similar to
ClockUpdate. They can be determined during witness generation based on recorded
memory accesses. Furthermore, one doesn't need separate JOIN and SPLIT opcode
pairs for each target slice length. In fact, a single component can perform both
`JOIN` and `SPLIT` for all lengths up to the component width. Given the columns
address, $\text{limb}_0, \ldots, \text{limb}_n$, one adds to the logup sum

$$\text{sign}(\text{opcode\_id}) \cdot \left\{ (\text{address}, [\text{limb}_0, \ldots, \text{limb}_n]) - \sum_i (\text{address} + i, \text{limb}_i) \right\}$$

where $\text{sign}(\text{opcode\_id})$ is simply $+$ or $-$ derived from the
actual opcode ID: set the two opcodes with consecutive IDs, $\text{ID}$ and
$\text{ID} + 1$. The derived sign is just
$\text{sign}(\text{opcode\_id}) = 2 \cdot (\text{opcode\_id} - \text{ID}) - 1$.

This variable-sized memory word pattern enables variable-sized instructions and
native handling of multi-limb types like `u32`. This will be further discussed
in the [Uint types](#uint-types) section.

## Registers \label{sec:registers} {#registers}

The original Cairo VM uses 3 registers:

- `pc`: the program counter, is the address of the current instruction;
- `fp`: the frame pointer, is a pointer to the current frame initial address;
- `ap`: the application pointer, is the free-memory pointer.

In the context of a read-write memory, the free-memory pointer becomes
unnecessary, and we can drop `ap`, leaving the VM initially with only two
registers, similar to
[Valida](https://lita.gitbook.io/lita-documentation/architecture/valida-zk-vm/technical-design-vm),
for example.

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

## Opcodes \label{sec:opcodes} {#opcodes}

The initial Cairo M design was heavily inspired by the Cairo VM. The currently
implemented opcodes reflect this origin but would not be kept as-is if the
project were started today.

### AIR basics \label{sec:air-basics} {#air-basics}

An AIR (Algebraic Intermediate Representation) represents a computation as a
collection of algebraic relationships that must be satisfied for the computation
to be considered valid.

For simplicity, let us describe an AIR as a dataframe, with columns representing
variables used in the defined constraint system (circuit) and rows representing
circuit instantiations. All constraints are eventually described as a polynomial
combination of the columns. For example, given a dataframe $\text{df}$ with
columns $a$, $b$, $c$, the constraint $a + b = c$ is described as
$\text{df}[a] + \text{df}[b] - \text{df}[c] = 0$ and applies to all rows of the
dataframe.

During proof generation, each column is interpreted as values of a given
polynomial over a base set $\{x^i\}_{i=0}^n$. This polynomial is interpolated
and evaluated over a larger domain. The prover commits to each column (each
polynomial) and then generates Merkle inclusion proofs for some evaluations of
these polynomials at random points. This means that the proof size and the
verifier complexity are directly related to the number of columns in the AIR:
the more columns, the more commitments and the greater the verifier complexity.

The Stwo framework allows defining the whole AIR of the state transition of the
virtual machine in several smaller such dataframes, called
[_components_](https://docs.starknet.io/learn/s-two/air-development/components)
(other frameworks may call them _chips_). Eventually, they are all concatenated
by the column axis to form the whole AIR.

### Design principles \label{sec:design-principles} {#design-principles}

Generally speaking, a reduced instruction set will generate more cycles, i.e.,
more rows, for a given operation than a complex instruction set (see also
[this RISC-V versus Cairo ISA comparison](https://x.com/ClementWalter/status/1896131941109506309)).
On the other hand, a complex instruction set will require more columns, i.e.,
more commitments, for a given operation than a reduced instruction set. In
short, one can think of a reduced instruction set as a long and thin dataframe,
as opposed to a complex instruction set as a short and wide one.

Notice, however, that given a component with shape $(n, m)$ ($n$ rows and $m$
columns), one can always reshape it to $(n / k, m \cdot k)$, where $k$ is an
integer, duplicating the columns and their corresponding constraints. The other
way around is not possible: one cannot keep only a partial circuit. In other
words, a reduced instruction set trace can always be reshaped to "look like" a
complex instruction set one, while the other way around is not possible. Hence,
reduced instruction sets give more flexibility.

Furthermore, long traces can be proven in batch, and even in parallel when the
program is still running (so-called
[_continuation_](https://risczero.com/blog/continuations)), and aggregated later
with recursion. This effectively boils down to splitting the dataframe into
chunks with less rows. This reduces either the proving time or the memory usage
of the host, which is
[directly proportional to the area of the AIR](https://x.com/ClementWalter/status/1964997331612488085),
(i.e., width times height).

Consequently, when designing an AIR, one tries to limit the number of columns as
much as possible. This can be done by both limiting the number of opcodes in the
instruction set, and by factorizing as much as possible several opcodes into the
same component.

The required degree of the constraints can also influence the number of columns.
Actually, the maximum degree of the constraints influences the size of the
evaluation domain, and adding constraints with a higher degree will double its
size. Hence it is always better to just add intermediate columns to reduce the
degree of the constraints.

Overall, the goal is to use as few columns as possible, and to keep the degree
of the constraints as low as possible, which can in turn require more columns.

### Minimal instruction set \label{sec:minimal-instruction-set} {#minimal-instruction-set}

We now present in this section a minimal instruction set.

**Control Flow**:

- `CallAbsImm`, `Ret`: Function call/return management
- `JmpRelImm`, `JnzFpImm`: Intra-frame jumps

**Arithmetic Operations**:

- `StoreAdd`, `StoreSub`, `StoreMul`, `StoreDiv`: Arithmetic with memory storage

**Memory Operations**:

- `StoreImm`: Store immediate (bytecode) value into memory
- `StoreDoubleDeref`: Store dereferenced value
- `StoreToDoubleDeref`: Store at dereferenced address

This proposed instruction set fits in a total of XXX columns. See the
[Opcodes columns](#opcodes-columns) section for more details.

### Extensions \label{sec:extensions} {#extensions}

If the proposed instruction set is enough to perform any kind of computation,
one may want to extend it with more opcodes. The purpose of extensions is to
make some complex operations native to the prover, i.e. to give them directly a
circuit representation. Whether extensions actually make the whole proving steps
faster depends on the context and the actual optimization they allow.

Among the most common extensions, we describe below the case of adding different
"native" types to the instruction set and built-in hash functions.

#### Uint Types \label{sec:uint-types} {#uint-types}

At the prover level, the only native type is the field element. However, at the
software level, the most common native types are `u32` or `u64`. While it is
possible
[to emulate for example a `u256` at the software level](https://github.com/starkware-libs/cairo-lang/blob/v0.14.0.1/src/starkware/cairo/common/uint256.cairo),
it may be more efficient to instead manage it at the AIR level. For example,
creating a `u32` with a struct holding two field elements would require two
memory accesses per variable use instead of one.

At the software level, the main difference between a uint of a given size and a
felt lies mainly in the division operation. In fact, at least in release mode,
uints silently overflow and wrap around, behaving like a field element over
$2^n$. On the other hand, the division for field elements is always exact (every
field element has an inverse), while the division for uints is the Euclidean
division. At the AIR level, emulating a uint mainly requires emulating
operations over the uint size, i.e., properly handling the carry, borrow, and
range-checking the values used.

Given that the current prime is $2^{31} - 1$, any uint using fewer than 31 bits
can easily be represented as a single field element. However, as mentioned
previously, every single value needs to be range-checked to ensure that it stays
within the correct boundaries. Consequently, the largest simple native uint type
that can be represented without any limb decomposition depends on the size of
the largest RangeCheck component added to the prover. Since a RangeCheck
component is just a plain enumeration of all the allowed numbers (e.g.,
$[0, 2^{20})$ for a RangeCheck20 component), this is directly related to the
size of the trace itself and so to the host memory usage and overall performance
of the prover. As a matter of fact, given some
[initial benchmarks with Stwo](https://x.com/ClementWalter/status/1927617083967234483),
we decided to keep RangeCheck20 as the largest single RangeCheck component,
consequently making `u20` the largest simple native uint type that could be
represented without any limb decomposition.

In any case, keeping the same memory segment for both felt and uint creates a
significant range-check overhead, as every read needs to be range-checked, not
just writes. For this reason, it is better to use a dedicated memory segment for
every such simple uint type, where only the write operation needs to be
range-checked.

On the other hand, given this maximum limb size, it is straightforward to derive
any uint type with limb decomposition over this base limb size with no
significant extra cost. Remember from the [Word size](#word-size) section that a
memory read is actually a memory slice read; one can read several limbs at once.

Eventually, since `u20` is not a regular base type in any software and this "20"
is strongly dependent on some internal prover configuration (the largest
available range-check component), it makes more sense to use `u16` or `u8`
instead. The question of the most optimal base between the two depends on the
context. Using `u8` would create more trace cells for `ADD` and `SUB` operations
where 16-bit limbs are fine, but would save on `MUL` and `DIV` operations where
numbers actually need to be written with 8-bit limbs since
$\text{u16} \times \text{u16} \to \text{u32} > 2^{31} - 1$.

#### Built-in functions \label{sec:built-in-functions} {#built-in-functions}

## Conclusion \label{sec:conclusion} {#conclusion}

## Appendix \label{sec:appendix} {#appendix}

### Opcodes columns \label{sec:opcodes-columns} {#opcodes-columns}

This section describes the detailed list of columns for each component. Not
mentioned is the possible need for an enabler column, which distinguishes
between the actual trace row and the padding required for the trace length to be
a power of 2.

The instruction is a variable-sized list of field elements, with the first one
always being the opcode ID. The rest is context-dependent and usually denoted as
`off_i` for address offsets or `imm_i` for immediate (i.e., constant) values.
The name $\text{op}_i$ is used to refer to the $i$-th operand, which is a memory
access at address $\text{fp} + \text{off}_i$, i.e.,
$\text{memory}[\text{fp} + \text{off}_i]$.

When several opcodes share the same set of columns, the opcode IDs are used to
select the appropriate constraints. For simplicity, it is assumed that the
opcode IDs are consecutive, so that the difference between the opcode ID and the
first opcode ID directly yields a Boolean flag. This is not required, and one
could alternatively use the constant `1 / (id_1 - id_0)` between the opcode ID
and the first opcode ID to compute the Boolean flag.

Intermediate values used below are not columns but simply variables computed
from the columns. They don't incur any cost.

Constraints are described as arithmetic formulas that should equal 0. The `= 0`
is omitted for simplicity.

#### CallAbsImm, Ret \label{sec:callabsimm-ret} {#callabsimm-ret}

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
- op0_val
- op0_plus_one_prev_clock
- op0_plus_one_prev_val
- pc_next

Intermediate columns:

- $\text{is\_ret} = \text{opcode\_id} - \text{CALL\_ABS\_IMM\_ID}$
- $\text{fp\_next} = \text{op0\_prev\_val} \cdot \text{is\_ret} + (\text{fp} + \text{off0} + 2) \cdot (1 - \text{is\_ret})$
- $\text{op0\_plus\_one\_val} = \text{op0\_plus\_one\_prev\_val} \cdot \text{is\_ret} + \text{pc} \cdot (1 - \text{is\_ret})$

Constraints:

- $\text{is\_ret} \cdot (1 - \text{is\_ret})$
- $\text{pc\_next} - \text{op0\_plus\_one\_prev\_val} \cdot \text{is\_ret} + \text{imm} \cdot (1 - \text{is\_ret})$
- $\text{op0\_val} - \text{op0\_prev\_val} \cdot \text{is\_ret} + \text{fp} \cdot (1 - \text{is\_ret})$

Lookups

- update registers
  - `-Registers(pc, fp)`
  - `+Registers(pc_next, fp_next)`
- read instruction from read-only memory
  - `-ROM(pc, opcode_id, off0)`
- read/write operands from memory
  - `-RAM(fp + off0, prev_clock, op0_prev_val)`
  - `+RAM(fp + off0, clock, op0_val)`
  - `-RAM(fp + off0 + 1, prev_clock, op0_plus_one_prev_val)`
  - `+RAM(fp + off0 + 1, clock, op0_plus_one_val)`
- range check clock difference
  - `+RangeCheck20(clock - op0_prev_clock - 1)`
  - `+RangeCheck20(clock - op0_plus_one_prev_clock - 1)`

Total: $11T + 9L$. Considering a maximum degree of 3, one can pre-sum the logup
terms using plain columns:

$$
\begin{aligned}
&= 11 + 6L + 3L \\
&= 11 + 6 \cdot 2 + 3 \cdot 4 \\
&= 35
\end{aligned}
$$

#### JmpRelImm, JnzFpImm \label{sec:jmprelimm-jnzfpimm} {#jmprelimm-jnzfpimm}

Columns:

- pc
- fp
- clock
- opcode_id
- off0
- imm
- op0_prev_clock
- op0_prev_val
- op0_plus_one_prev_clock
- op0_plus_one_prev_val

Intermediate columns:

- $\text{is\_jnz} = \text{opcode\_id} - \text{JNZ\_FP\_IMM\_ID}$
- $\text{fp\_next} = \text{op0\_prev\_val} \cdot \text{is\_jnz} + (\text{fp} + \text{off0} + 2) \cdot (1 - \text{is\_jnz})$
- $\text{pc\_next} = \text{op0\_plus\_one\_prev\_val} \cdot \text{is\_jnz} + \text{imm} \cdot (1 - \text{is\_jnz})$
- $\text{op0\_val} = \text{op0\_prev\_val} \cdot \text{is\_jnz} + \text{fp} \cdot (1 - \text{is\_jnz})$
- $\text{op0\_plus\_one\_val} = \text{op0\_plus\_one\_prev\_val} \cdot \text{is\_jnz} + \text{pc} \cdot (1 - \text{is\_jnz})$

Constraints:

- $\text{is\_jnz} \cdot (1 - \text{is\_jnz})$

Lookups:

- read instruction from read-only memory
  - `-ROM(pc, opcode_id, off0)`
- read/write operands from memory
  - `-RAM(fp + off0, prev_clock, op0_prev_val)`
  - `+RAM(fp + off0, clock, op0_val)`
  - `-RAM(fp + off0 + 1, prev_clock, op0_plus_one_prev_val)`
  - `+RAM(fp + off0 + 1, clock, op0_plus_one_val)`
- range check clock difference
  - `+RangeCheck20(clock - op0_prev_clock - 1)`
  - `+RangeCheck20(clock - op0_plus_one_prev_clock - 1)`
- registers update
  - `-Registers(pc, fp)`
  - `+Registers(pc_next, fp_next)`

#### StoreAdd, StoreSub, StoreMul, StoreDiv \label{sec:storeadd-storesub-storemul-storediv} {#storeadd-storesub-storemul-storediv}

- registers: pc | fp
- global: clock
- instruction: opcode_id | off0 | imm
- operands: memory[fp + off0]: prev_clock | prev_val
- memory[fp + off0 + 1]: prev_clock | prev_val

#### StoreDoubleDerefFpFp \label{sec:storedoubledereffpfp} {#storedoubledereffpfp}

- registers: pc | fp
- global: clock
- instruction: opcode_id | off0 | imm
- operands: memory[fp + off0]: prev_clock | prev_val
- memory[fp + off0 + 1]: prev_clock | prev_val

#### StoreDoubleDerefFp \label{sec:storedoubledereffp} {#storedoubledereffp}

- registers: pc | fp
- global: clock
- instruction: opcode_id | off0 | imm
- operands: memory[fp + off0]: prev_clock | prev_val
- memory[fp + off0 + 1]: prev_clock | prev_val

#### StoreFramePointer \label{sec:storeframepointer} {#storeframepointer}

- registers: pc | fp
- global: clock
- instruction: opcode_id | off0 | imm
- operands: memory[fp + off0]: prev_clock | prev_val
- memory[fp + off0 + 1]: prev_clock | prev_val

#### StoreImm \label{sec:storeimm} {#storeimm}

- registers: pc | fp
- global: clock
- instruction: opcode_id | off0 | imm
- operands: memory[fp + off0]: prev_clock | prev_val
- memory[fp + off0 + 1]: prev_clock | prev_val

### Lookups \label{sec:lookups} {#lookups}

A lookup argument in zero-knowledge proofs is a cryptographic primitive that
allows a prover to demonstrate that certain values in a computation trace exist
in another table, without revealing the specific values or their positions. The
prover commits to a "claimed sum" of lookup terms, and the verifier checks that
this sum equals zero, ensuring all looked-up values are valid according to the
specified relation constraints.

This entire document is drafted with Stwo's constraint framework in mind, which
uses [LogUp lookup arguments](https://eprint.iacr.org/2022/1530.pdf); see the
[logup.rs](https://github.com/starkware-libs/stwo/blob/dev/crates/constraint-framework/src/prover/logup.rs)
module for more details. Lookup and logup terms are used interchangeably in this
document to denote a relation between two components.

#### Core Concept \label{sec:core-concept} {#core-concept}

LogUp lookup arguments form a global sum of fractions that must equal zero. Each
component contributes fraction terms with three elements:

1. **Relation**: Defines alpha coefficients and z value for tuple aggregation
2. **Denominator**: Aggregated tuple value in secure field
3. **Numerator (multiplicity)**: Usage count of the tuple

#### Storage and Cost \label{sec:storage-and-cost} {#storage-and-cost}

- Terms stored in interaction trace columns
- QM31 secure field requires 4 base columns per lookup column
- Each lookup adds 4 columns to the AIR

#### Optimization Strategy \label{sec:optimization-strategy} {#optimization-strategy}

Columns can be grouped to store pre-summed terms rather than individual terms.
Pre-summing capacity depends on:

- Maximum constraint degree bound
- Variables in looked-up tuples

#### Cumulative Sum Structure \label{sec:cumulative-sum-structure} {#cumulative-sum-structure}

- Each row: cumulative sum of terms
- Last column: cumulative sum of all rows
- Bottom-right cell: total claimed sum (committed in proof)

#### Constraint Formula \label{sec:constraint-formula} {#constraint-formula}

$$\text{committed\_value} \cdot \text{current\_denominator} - \text{current\_numerator} = 0$$

Requirement:
$\text{degree}(\text{denominator}) + 1 < \text{max\_constraint\_degree}$. Given
that the resulting denominator of the sum of two fractions is the product of the
two denominators:

$$\frac{a}{b} + \frac{c}{d} = \frac{a \cdot d + c \cdot b}{b \cdot d}$$

one can, for example, pre-sum the terms by two when each denominator has a
degree of 1 and the maximum constraint degree bound is 3.

Throughout this document, we simply write
$\pm k \cdot \text{Relation}(\text{value}_0, \ldots, \text{value}_n)$ to refer
to the lookup of the tuple $(\text{value}_0, \ldots, \text{value}_n)$ for the
relation Relation with multiplicity $+k$ or $-k$, depending on the sign of the
numerator. We refer to "emitted", "yielded", or "added" values when the
multiplicity is positive, and "consumed" or "subtracted" values when the
multiplicity is negative.
