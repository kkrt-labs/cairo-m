# I. Debugging Stwo(ConstraintsNotSatisfied)

## 1. A simple trace-AIR inconsistency

Let’s say you just added a component or made modifications to an existing one.
The first step is to test the constraints by running :

`RUST_BACKTRACE=1 cargo test -p cairo-m-prover test_all_opcodes_constraints -—release`

This creates a mock commitment scheme that doesn’t do any interpolation and
simply evaluates the constraints with the raw trace values. This tells you if
there are any inconsistencies between your `write_trace` and
`write_interaction_trace` functions and the AIR in the `evaluate` function. The
error messages will like this :

```rust
thread '<unnamed>' panicked at external/stwo/crates/constraint_framework/src/assert.rs:84:9:
assertion `left == right` failed: row: #4, constraint #6
  left: (1107519218 + 1556664935i) + (283364838 + 911036789i)u
 right: (0 + 0i) + (0 + 0i)u
stack backtrace:
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: core::panicking::assert_failed_inner
   3: core::panicking::assert_failed
   4: <stwo_constraint_framework::assert::AssertEvaluator as stwo_constraint_framework::EvalAtRow>::finalize_logup_batched
   5: <cairo_m_prover::components::opcodes::store_mul_fp_imm::Eval as stwo_constraint_framework::component::FrameworkEval>::evaluate
   6: rayon_core::join::join_context::{{closure}}
   7: rayon_core::join::join_context::{{closure}}
   8: rayon::iter::plumbing::bridge_producer_consumer::helper
   9: <rayon_core::job::StackJob<L,F,R> as rayon_core::job::Job>::execute
  10: rayon_core::registry::WorkerThread::wait_until_cold
  11: rayon_core::registry::ThreadBuilder::run
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

Only two pieces of information are useful here: `constraint #6` that gives the
index of the unverified constraint and at stack index 5, the name of the
component that is failing `store_mul_fp_imm` . To count constraints:

- start at 0 in the `evaluate` function of a component,
- then do +1 for each add_constraint,
- then do +0.5 for batched `add_to_relation` and +1 for non batched
  `add_to_relation` .

**This allows you to spot the exact constraint that is not satisfied !**

Here are a few common issues.

### [CASE 1] - Incorrect columns order

In `write_trace`, columns are added like so:

```rust
*row[0] = enabler;
*row[1] = pc;
*row[2] = fp;
// and so on ...
```

The order matters since these columns are fetched in the `evaluate` function
like so:

```rust
let enabler = eval.next_trace_mask();
let pc = eval.next_trace_mask();
let fp = eval.next_trace_mask();
// and so on ...

// Enabler is 1 or 0
eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));
```

So if you mix that order, constraints won’t be satisfied anymore :

```rust
let pc = eval.next_trace_mask(); // what is named `pc` is actually `enabler`
let enabler = eval.next_trace_mask(); // likewise, this is actually `pc`
let fp = eval.next_trace_mask();
// and so on ...

// Enabler is 1 or 0
eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));
```

This constraint won’t be satisfied and you will get a
`Stwo(ConstraintsNotSatisfied)` error.

### [CASE 2] - Incorrect lookup data

Let’s say you want to do the following memory lookup `+ [fp + off0, clk, value]`
but write this :

```rust
// In write_trace :
*lookup_data.memory[0] = [fp, clk, value, zero, zero, zero];

// In write_interaction_trace
let num = + PackedQM31::from(enabler_col.packed_at(i));
let denom: PackedQM31 = relations.memory.combine(memory_entry);
writer.write_frac(num, denom);

// In evaluate
eval.add_to_relation(RelationEntry::new(
    &self.relations.memory,
    - E::EF::from(enabler.clone()),
    &[
        fp + off0,
        clk,
        value,
    ],
));
```

The `add_to_relation` constraint won’t be satisfied for two reasons:

- the `memory_entry` combined in the `interaction_trace` has `fp` as address but
  the `add_to_relation` enforces a different one (`fp + off0`).
- the numerator written in the interaction trace is `+ enabler` while it’s
  `- enabler` in the AIR. Be careful to numerators that can sometimes be `one`
  (e.g for `range_checks`) or `enabler` (e.g for memory lookups).

> **💡 Note**
>
> Reminder of how the `add_to_relation` enforces a constraint.
>
> For non batched lookups, the following trace is built:
>
> $$
> \begin{align}x_0 &= \frac{+\mathrm{enabler}}{\mathrm{combine}(entry_0)} \\[6pt]x_1 &= x_0 + \frac{-\mathrm{enabler}}{\mathrm{combine}(entry_1)}\end{align}
> $$
>
> where $x_0$ is in the first column and $x_1$ in the second one. Then :
>
> ```rust
> add_to_relation(relation, enabler, entry_0);
> add_to_relation(relation, -enabler, entry_1);
>
> // Translates to :
>
> running_sum = 0
> prev_running_sum = 0
> // running sum is incremented from the add_to_relation values
> // on the other hand x_0 is from the interaction trace
> running_sum += enabler/relation.combine(entry_0);
> diff = running_sum - prev_running_sum;
> prev_running_sum = running_sum
> add_constraint(x_0.denom * diff - x_0.denom);
>
> running_sum += - enabler/relation.combine(entry_1);
> diff = running_sum - prev_running_sum;
> prev_running_sum = running_sum
> add_constraint(x_1.denom * diff - x_1.denom);
>
> ```
>
> For batched lookups, two fractions are added per column :
>
> $$
> \begin{align}x_0 &= \frac{\mathrm{enabler} \cdot \mathrm{combine}(entry_1) - \mathrm{enabler} \cdot \mathrm{combine}(entry_0)}{\mathrm{combine}(entry_0) \cdot \mathrm{combine}(entry_1)}\end{align}
> $$

### [CASE 3] - Incorrect logup batching

Batching must be consistent accross witness generation and the AIR. So if you
generate the trace like so (batching):

```rust
let mut col = interaction_trace.new_col();
(
    col.par_iter_mut(),
    &interaction_claim_data.lookup_data.memory[4],
    &interaction_claim_data.lookup_data.memory[5],
)
    .into_par_iter()
    .enumerate()
    .for_each(|(i, (writer, memory_prev, memory_new))| {
        let num_prev = -PackedQM31::from(enabler_col.packed_at(i));
        let num_new = PackedQM31::from(enabler_col.packed_at(i));
        let denom_prev: PackedQM31 = relations.memory.combine(memory_prev);
        let denom_new: PackedQM31 = relations.memory.combine(memory_new);

        let numerator = num_prev * denom_new + num_new * denom_prev;
        let denom = denom_prev * denom_new;

        writer.write_frac(numerator, denom);
    });
col.finalize_col();
```

You must use `eval.finalize_logup_in_pairs();` in the evaluate function. Note
that the last fraction doesn’t have to be batched with another one.

If you are not batching simply use `eval.finalize_logup();` .

You can also do custom batches (group certain fraction but some not):

```rust
eval.finalize_logup_batched(&vec![
    0, 0, // two first add_to_relation are grouped
    1, 1, // two next too
    2, 2, // these too
    3, 4, // but here the two following add_to_relation are separated
    5, 5, // batched again
    6,
]);
```

You might get the following error if batching is inconsistent:

```rust
itertools: .zip_eq() reached end of one iterator before the other
```

In that case verify that you are batching/pair batching consistently across
trace and AIR.

### [CASE 4] - Bad constraint

The constraint might simply not be verified for the trace because it’s a bad
constraint. Check that the expression should indeed evaluate to 0, if not you
have to change your AIR.

## 2. When constraints are still not satisfied (not good)

Once the `test_all_opcodes_constraints` test passes, it’s time to run:

`cargo test -p cairo-m-prover test_prove_and_verify_all_opcodes -—release`

This section covers the case where you still get a
`Stwo(ConstraintsNotSatisfied)` error.

There is no method to debug since there is no indication on what is going wrong
exactly but here are some cases where this error occurs.

> **💡 Note**
>
> If you get this error on a program always run `test_all_opcodes_constraints`
> with the said program before, you need to put aside the trace-AIR
> inconsistency scenario.

## [CASE 1] - Incorrect multiplicity

This function defines the maximum degree in constraints:

```rust
fn max_constraint_log_degree_bound(&self) -> u32 {
    self.log_size() + 1
}
```

Here the maximum degree is $2^1$.

```rust
let a = eval.next_trace_mask();
let b = eval.next_trace_mask();
let c = eval.next_trace_mask();
let d = eval.next_trace_mask();

// This will work:
eval.add_constraint(a * b * c);
eval.add_constraint(42 * a * 3 * b * c)
let e = eval.add_intermediate(a + b);
eval.add_constraint(e * a * b);
// This won't:
eval.add_constraint(a * b * c * d);

// Careful to batched lookups that rapidely increase the multiplicity
// These two add_to_relations from store_mul_fp_imm can't be batched together
// because of the op0_val * off1
eval.add_to_relation(RelationEntry::new(
    &self.relations.memory,
    -E::EF::from(enabler.clone()),
    &[
        fp.clone() + off2.clone(),
        dst_prev_clock.clone(),
        dst_prev_val,
    ],
));
eval.add_to_relation(RelationEntry::new(
    &self.relations.memory,
    E::EF::from(enabler.clone()),
    &[fp + off2, clock.clone(), op0_val * off1],
));
```

## [CASE 2] - Log Size issue

See section IV.

## II. Beating Stwo(InvalidLogupSum)

You can run
`cargo test -p cairo-m-prover test_prove_and_verify_all_opcodes -—release`
without getting `Stwo(ConstraintsNotSatisfied)` but now you get
`Stwo(InvalidLogupSum)`. The root problem is that the sum of all logups sums
from every component is not equal to 0. This means that an entry for a relation
is emited but it isn’t consumed.

The first thing to do there is to run:

`cargo test -p cairo-m-prover test_prove_and_verify_all_opcodes -—release --features relation-tracker`

## [Toy example] - Understand the relation-tracker output

Say opcode `CallAbsImm` (`ID=10`) is used during the program execution, then:

```rust
// Memory component is supposed to produce the initial cell with clock 0
(1) produces: [addr, clk=0, QM31_value = (10, target, 0, 0)]

// First instance of CallAbsImm:
consumes: [addr, clk=0, QM31_value = (10, target, 0, 0)]
produces: [addr, clk_1, QM31_value = (10, target, 0, 0)]

// Next instance
consumes: [addr, clk_1, QM31_value = (10, target, 0, 0)]
produces: [addr, clk_2, QM31_value = (10, target, 0, 0)]

// Memory component is supposed to consume the final cell
(2) consumes: [addr, clk_2, QM31_value = (10, target, 0, 0)]
```

But if one removes all memory lookups from the memory component, constraints
will be satisfied but boundary memory cells won’t be produced (1) nor consumed
(2). The overall sum for the memory relation won’t be 0.

By running the test with `relation-tracker` feature, you will get for that
example:

```rust
Relation Summary:
Memory:
[addr, clk=0, 10, target] -> 2147483646 // trailing 0 are hidden
[addr, clk2, 10, target] -> 1
```

This should be interpreted as: _“a component consumed the entry (1) but no one
produced it, there is an unbalance”_. For the second line: “_a component emited
entry (2) but no one used it_”

> **💡 Note**
>
> Trailing zeros are not shown in the `relation-tracker`, so the entry `[0]`
> will be displayed as `[]` and `[42, 0]` will be `[42]`.

> **💡 Note**
>
> Note that `2147483646 = P - 1 = -1 mod(P)`.

> **💡 Note**
>
> In `Cairo-M`:
>
> - Consume ↔ use ↔ request : `add_to_relation(-fraction)`
> - Produce ↔ emit ↔ yield : `add_to_relation(+fraction)`
>
> In `Stwo-cairo`: + and - are the other way around.

## [Tips]

The example gives you the theory. But in practice it’s harder to make sense out
of the output. So here are a few tips:

- Make sure the lookups fractions have the correct sign, it happens to misplace
  a + or a -
- For the memory entry you can get useful information from the first value of
  the QM31 since it’s the `opcode_id` . This gives you a good idea of what
  opcode component could be wrong (now again the error could also come from the
  adapter, the memory component, etc.)
- `Cmd + F` and `Sort lines ascending` are your best friend, try to find
  patterns in the debugged entries. Say the `CallAbsImm` opcode wrongly inverted
  the 2 first QM31 values. It woud correctly consume
  `[addr, clk, value0, value1]` but would falsely produce
  `[addr, clk, value1, value0]` . The resulting relation summary would be:

```rust
Relation Summary:
Memory:
// First CallAbsImm instance uses the inital entry (emited by memory component
// But it emits the wrong entry, this is were problems begin
[addr, clk_1, QM31_value = (target, 10, 0, 0)] -> 1
// The second instance uses what should be the last emited entry but no such
// entry was emitted because of bug. And so on
[addr, clk_1, QM31_value = (10, target, 0, 0)] -> 2147483646
[addr, clk_2, QM31_value = (target, 10, 0, 0)] -> 1
// Memory component tries to consume an unexisting value: error.
[addr, clk_2, QM31_value = (10, target, 0, 0)] -> 2147483646
```

By putting these entries next to each other, the problem is way more visible. So
try to find patterns.

- Show the raw data from the `ProverInput` . For instance, you might not
  directly have the `opcode_id` in the QM31 (e.g if the memory entry is a
  write), in that case you can show the `ProverInput` and spot with the clock
  the exact problematic `ExecutionBundles` .

The main idea is to trace back the values added to the relation and check if
these are correct.

## III. "Not enough alpha powers to combine values”

For each relation there is a maximum number of entries. For instance `Memory`
lookups take at most 6 values: `[addr, clk, value0, value1, value2, value3]`.

So if one calls `combine(1, 2, 3, 4, 5, 6, 7)` , this error will be thrown. So
verify, in the `add_to_relation` and in the `write_interaction_trace` that you
are not combining too much values.

> **💡 Note**
>
> Note that combining less than the max is equivalent to combining the same
> entry padded with zeros. This is used all across the codebase:
>
> ```rust
> eval.add_to_relation(RelationEntry::new(
>     &self.relations.memory,
>     E::EF::from(enabler.clone()),
>     &[
> 	    fp + off0 + one.clone(),
>       clock.clone(),
>       pc + one
>      ],
> ));
>
> // Is the same as:
> eval.add_to_relation(RelationEntry::new(
>     &self.relations.memory,
>     E::EF::from(enabler.clone()),
>     &[
> 	    fp + off0 + one.clone(),
>       clock.clone(),
>       pc + one,
>       E::EF::zero(),
>       E::EF::zero(),
>       E::EF::zero()
>     ],
> ));
> ```

## IV. TooManyQueriedValues

In the end, all traces are written next to each other and the evaluate uses a
`TraceLocationAllocator` to associate the right columns with the right evaluate
functions. To do so the `log_sizes()` methods are used so make sure to:

- when adding a component, keep a consistent order all across the
  `components/mod.rs` file. For instance have:

```rust
/// In the Claim
pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
    let trees = vec![
        self.component1.log_sizes(),
        self.component2.log_sizes()
    ];
    TreeVec::concat_cols(trees.into_iter())
}
pub fn mix_into(&self, channel: &mut impl Channel) {
        self.component1.mix_into(channel);
        self.component2.mix_into(channel);
    }

/// In write trace:
let trace = opcodes_trace
            .into_iter()
            .chain(component1.to_evals())
            .chain(component2.to_evals());

/// In write interaction trace:
opcodes_interaction_trace
                .into_iter()
                .chain(component1)
                .chain(component2)

/// In the Components
component1::Component::new(
    location_allocator,
    component1::Eval {
        claim: claim.component1.clone(),
        relations: relations.clone(),
    },
    interaction_claim.component1.claimed_sum,
),
component2::Component::new(
    location_allocator,
    merkle::Eval {
        claim: claim.component2.clone(),
        relations: relations.clone(),
    },
    interaction_claim.component2.claimed_sum,
),
```

- Then make sure the `log_sizes` are correct in the components ie. that
  `N_TRACE_COLUMNS` is correct and `N_INTERACTION_COLUMNS` is too.
