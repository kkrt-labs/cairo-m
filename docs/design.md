# Cairo M design document

Cairo (i.e. Cpu AIR) M is a new _zkVM_ design aiming at being especially
efficient with

- small-field provers: STARK provers using small prime fields (e.g. M31 or
  Babybear)
- continuation: _infinitely_ long program runs should be provable out-of-the-box
- recursion: the ability to verify generated proofs directly with the prover
  framework and not with a verifier written in a high-level language
- low host memory usage

For the sake of simplicity, the remaining of this paper assumes that the
selected prime number is Mersenne31 (M31): `M31 = 2^31 - 1`. Minor adaptations
to the current design specifications should be made for other primes.

## Memory

The memory is chosen to be a 1D-addressable array, indexed directly with field
elements. Consequently, its maximum length depends on the prover's prime field.

### Commitment

The whole memory needs to be efficiently committed to for _continuation_, where
one needs to make sure that the final memory of a given stage `n` is actually
the same as the initial memory of stage `n + 1`.

A Merkle tree is chosen as the memory commitment form because:

- it allows to commit to challenge-independent quantities: the two roots
  (initial and final) of the whole memory
- it naturally handles sparse memory with partial trees: the tree is effectively
  pruned of all intermediate nodes that don't lead to used leaves in the current
  run

In order to allow for efficient recursion, the Merkle tree needs to be
implemented with a zk friendly hash function. We chose Poseidon2 in our current
implementation but this can be easily updated should any other hash function be
preferred.

Eventually, we chose `0..2**30` for the memory address space as it doesn't
require padding for the Merkle root computation and is big enough to fit
reasonably big computation. This could easily be extended to `P - 1` but we just
did not find it relevant in our use cases as big memory address space is mainly
relevant for long traces while we focus on client side proving. Actually, only
very long runs may require a memory bigger than 2^30, which in turn would
require a host machine with lots more RAM than what is usually available in
consumer devices.

The Merkle tree furthermore doesn't hash the final leaves (i.e. the memory
values) in order to save on hashes number. While leaves hashing is required to
avoid disclosing siblings values with proof of inclusion, it doesn't bring any
benefit for proving a state root and can safely be dropped. This however
requires all the memory cells to have a value, and consequently the whole memory
is implicitly initialized with 0s. This default value doesn't actually have any
impact as actual initial values are written from the VM run, being 0 or not.

### Read/Write operations

Given the fact that we want at the same time to be able to run arbitrary long
programs and have a fixed size memory with a relatively small address space
(2^30 = 1,048,576), we adopt a read-write memory model.

Read and Write operations are actually emulated with lookup arguments:

- a memory entry is modelled as a triplet `(address, clock, value)` where
  `clock` is like a timestamp that lets us record when the `address` had value
  `value`;
- this `clock` is actually a simple sequence from 0 up to the required length,
  guessed during witness generation;
- initial values from the Merkle commitment are emitted with `clock = 0`.
- read and write are actually the same operation from the lookup point of view;
  cancelling the previously emitted triplet `-(address, prev_clock, prev_value)`
  and adding a new one to the lookup sum `(address, clock, value)`. In the case
  of a read, i.e. when `value == prev_value`, the witness actually doesn't need
  to store the two (identical) values;
- the `clock` value is enforced to be strictly increasing between two
  consecutive read or write, i.e. that `clock - prev_clock > 0`. This constraint
  is enforced with a lookup to a `RangeCheck` component;
- a Clock Update component is introduced to actually update the clock when two
  consecutive memory operations are "too far", meaning that the clock difference
  would require a big range check. This `ClockUpdate` component effectively just
  updates the `clock` value of the triplet, much like in a read operation, but
  its trace is filled during witness generation and is not part of the VM
  itself. At this stage, when the adapter observe that two memory accesses are
  "too far" with respect to the biggest available `RangeCheck` component, the
  delta is divided and as many clock update are introduced as required to make
  sure that they all fit within the available range.

All together, this read-write memory costs up to 5 main trace columns
(`address`, `prev_clock`, `clock`, `prev_value`, `value`) and 3 lookup ups
columns (`-(address, prev_clock, prev_value)`, `+(address, clock, value)`,
`+(clock - prev_clock -1)`) per memory access in a component, or 5T + 3L.

This is to be compared to a read-only memory which would only be 2 main trace
columns (`address`, `value`) and 1 lookup (`-(address, value)`), or 2T and 1L.

Given the fact that a lookup column uses the secure field, which is a `QM31`,
each lookup column is 4 base columns. Eventually, the overhead in terms of base
columns is up to `3T + 2L = 3 + 8 = 11` per memory access. For a STORE operation
like `dst = op0 + op1`, this is up to 33 more columns.

This overhead can be mitigated using opcodes that write in place (e.g.
`x += y`). It can also be limited by grouping the log ups columns by two,
precomputing the logup sums in pairs when the maximum constraints degree remain
low. If we consequently count only 2 column per look up, the memory access
overhead becomes `3 + 2*2 = 7`. If we furthermore consider an in place
operation, then it becomes `2*(5T + 3L / 2) = 10 + 12 = 22` columns for the
read-write memory and `3*(2T + 1L/2) = 6 + 4 = 10` for the read-only memory.

Since the read-write memory allows for a much easier control flow, reduce the
need to copy memory values with new frames, and is much easier to reason with
when developing a software, this overhead is worth it.

On the other hand, not all part of the memory need to be writable. Especially
the program and embedded constant values can stay read-only. To enforce a memory
segment to be read-only, the easiest way is to place the read-only part at the
beginning of the memory and add a range-check to the write addresses in the
`STORE` opcodes.

### Word size

If the Merkle root lets commit to up to 2^30 field elements, the VM doesn't need
to use a single felt as the base word size. In our first implementation, we used
a fixed-size word built from 4 M31 so as to easily fit all felt-based
instruction in a single read. This effectively reduce the memory size to
2^28=268,435,456.

However, there is no need to use such a fixed-size memory word. Given the fact
that the memory consistency is enforced only with lookup arguments, each address
can "consume" any number of field elements that are eventually all summed up
together with the relation's challenge coefficients. The only requirement is
eventually that an address always consume and write the same number of field
elements. One can think of this as accessing a slice of an array at a given
index, with the slice length depending on the index, instead of just a given
value:

```ignore
memory[address: address + len(address)]
```

instead of

```ignore
memory[address]
```

To make limbs of a given address available, we shall introduce the `SPLIT`
opcode. `SPLIT` would just consume the lookup term with the whole slice at
address `a` and adds one term for each of the `len` limbs at address
`a + i, i=0..len`.

The opposite operation is called `JOIN` and would let the machine gather a list
of continuous limbs into a single memory address, consuming each of the
`(a + i; v_i)` lookup terms and adding a single `(a, [v_0, ..., v_{len - 1}])`.

These opcodes don't necessarily need to be added at the VM level, much like the
`ClockUpdate`. They can be guessed during witness generation based on the
recorded memory accesses. Furthermore, one doesn't need one join and split
opcode pair per target slice `len`. Actually, one single component can do both
`JOIN` and `SPLIT` for all the lengths up to the component width. Given the
columns `address`, `limb_0`, `...`, `limb_n`, one just adds to the log up sum

```latex
sign(opcode_id) * \lbrace (address, [limb_0, ... limb_n]) - \sum_i * (address + i, limb_i) \rbrace
```

where `sign(opcode_id)` would just be `+` or `-` derived from the actual opcode
id: set the two opcode with a consecutive id, `ID` and `ID + 1`. The derived
sign is just `sign(opcode_id) = 2 * (opcode_id - ID) - 1`.

This variable-sized memory word pattern enables variable-sized instruction and
native handling of multi-limbs types like `u32`. This will be further discussed
in the [Native types](#native-types) section.

## Registers

The original Cairo VM uses 3 registers:

- `pc`: the program counter, is the address of the current instruction;
- `fp`: the frame pointer, is a pointer to the current frame initial address;
- `ap`: the application pointer, is the free-memory pointer.

In the context of a read-write memory, the free-memory pointer becomes useless
and we can drop `ap`, leaving the VM initially with only two registers, like
Valida for example.

Regular instruction sets also leverage registers to store temporary values used
in several opcodes, like a fast buffer to avoid memory accesses.

From the prover point of view:

- each register requires its own trace cell, i.e. that it adds 1 column per
  component;
- a list of registers is like a memory slice (see [Word size](#word-size))
  without address nor clock: each component updates the registers' state by
  removing the current state vector and adding the constrained update to the log
  up sum, much like a stack: you can access the last state and push a new one.

Eventually, there is a trade-off between adding registers to the "main register
stack" (i.e. together with `pc` and `fp`) — which adds one column per component
even if the value is not used, and is free otherwise —, and adding registers to
a "secondary register stack", or even a tertiary (etc.) in order to save on
unused columns, but at the cost of doing 2 more lookups (4 columns) when one
needs them. The limit case is to just use one relation per register, as it they
were regular memory values. Depending on the total number of components and how
the compiler leverages register, the answer may vary.

Adding register to the main register stack may be temporarily better but is less
maintainable as the number of component may vary drastically over time and usage
(see [Reshaping](#minimal-instruction-set) and [Extensions](#extensions)). The
current design doesn't use any secondary register stack for the sake of
simplicity. However, it is most probably useful to leverage a secondary register
stack with 2 values when several arithmetic operations are performed in a row.

## Opcodes

The initial Cairo M design was greatly inspired by the Cairo VM. The currently
implemented opcodes reflect this root but would not be kept as is should the
project be start today.

### Minimal instruction set

Let us recapitulate first what matters the most when designing a "zk native"
minimal instruction set:

- an AIR can eventually be viewed as a regular dataframe, where each column is a
  variable used in a constraint and each row is a usage of the inlined circuit
  defined on the column variables;
- usually, one opcodes is one dataframe, but several opcode can also be
  factorized into on single dataframe when the constraints are similar;
- each cycle of the VM adds a row to one of these dataframes;
- the less opcode in the instruction set, the more cycle the VM needs to do for
  the same task;
- all these dataframes are defined in independent "components" but are
  eventually concatenated on axis 1 to form one single big dataframe (this was
  true with old provers requiring to pad the smallest ones, not fully accurate
  with stwo, but for the sake of simplicity one can keep viewing this as one
  single big table);
- given a witness, reshaping to reduce the number of rows and add more columns
  is always possible (just duplicate the constraints), but there is a minimal
  number of columns that depends on the instruction set;
- the number of columns will impact the proof size and increase the verifier
  work as there is one commitment per column in the proof;
- the peak memory consumption of the host machine when proving is directly
  related to the total number of cells, i.e. `\sum width * length`.
- long traces can always be split into several proofs with continuation, while
  you cannot prove only half of the constraints.

In conclusion, the fewest columns in the AIR the better in order to get more
flexibility in the downstream usage of the zkVM, at the cost of generating more
cycles for end programs. This cycles versus AIR width trade-off is actually a
classic CPU versus Memory one: you can always trade some memory usage for more
CPU, and vice versa. Since standard consumer devices usually embed powerful
chips but have very limited RAM resources, and RAM is more expensive than
compute, it makes sense to really limit at maximum the AIR width and to reshape
witnesses to fit the actual device memory availability.

All together, we shall keep for this minimal zk native ISA the following
opcodes, with one line per component:

- CallAbsImm, Ret;
  - create a new call stack, and return to the calling position when done
- JmpRelImm, JnzFpImm;
  - jump to a new pc without leaving the current frame
- StoreAdd, StoreSub, StoreMul, StoreDiv;
  - store the result of the given arithmetic operation in memory
- StoreDoubleDerefFpFp, StoreToDoubleDerefFpFp;
  - store the dereferenced memory value, or at the dereference memory address.

### Extensions

## Native types

## Conclusion
