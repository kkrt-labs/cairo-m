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
  `clock` is like a timestamp that let us record when the `address` had value
  `value`
- this `clock` is actually a simple sequence from 0 up to the required length,
  guessed during witness generation
- initial values from the Merkle commitment are emitted with `clock = 0`.
- read and write are actually the same operation from the lookup point of view,
  cancelling the previously emitted triplet `-(address, prev_clock, prev_value)`
  and adding a new one to the lookup sum `(address, clock, value)`. In the case
  of a read, `value == prev_value` and the witness actually doesn't need to
  store the two (identical) values.
- the `clock` value is enforced to be strictly increasing between two
  consecutive read or write, i.e. that `clock - prev_clock > 0`. This constraint
  is enforced with a lookup to a `RangeCheck` component.

### Word size

If the Merkle root lets commit to up to 2^30 field elements, the VM doesn't need
to use a single-felt as the base word size. In our first implementation, we used
a fixed-size word built from 4 M31 so as to fit all felt-based instruction in a
single read. This effectively reduce the memory size to 2^28=268,435,456.

However, there is no need to use such a fixed-size memory word. Given the fact
that the memory consistency is enforced only with lookup arguments, each address
can "consume" any number of field elements that are eventually all summed up
together with the challenge coefficients. The only requirement is eventually
that an address always consume and write the same number of field elements. One
can think of this as accessing a slice of an array with starting index, instead
of just a given value:

```ignore
memory[address: address + len(address)]
```

instead of

```ignore
memory[address]
```

To make limbs of a given address available, we introduce the `SPLIT` opcode.
`SPLIT` would just consume the lookup term with the whole slice at address `a`
and adds one term for each of the `n` limbs at address `a + i, i=0..n`.

The opposition operation is called `JOIN` and let the machine gather a list of
continuous limbs into a single memory address, consuming each of the
`(a + i; v_i)` lookup terms and adding a single `(a, [v_0, ..., v_n])`.

### Read-only memory

## Opcodes

## Registers

## Native types

## Conclusion
