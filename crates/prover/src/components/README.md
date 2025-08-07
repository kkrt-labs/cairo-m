# Cairo-M STARK Components

This directory contains the core STARK constraint system components for the
Cairo-M prover. Each component encapsulates the algebraic constraints, trace
generation, and proof logic for specific aspects of the virtual machine
execution.

## Component Architecture Overview

Each component in Cairo-M follows a standard structure with three key parts:

1. **Claim**: Public claims about component execution that will be inlined in
   the proof
2. **InteractionClaim**: Claims about lookup interactions that will also be
   inlined in the proof
3. **Component Implementation**: The actual proving logic including trace
   generation, interaction traces, and constraint evaluation

## Claims and Log Sizes

The `Claim` struct is crucial as it declares the size of execution and
interaction traces:

```rust
// From clock_update.rs
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Claim {
    pub log_size: u32,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace_log_sizes = vec![self.log_size; N_TRACE_COLUMNS];
        let interaction_log_sizes = vec![self.log_size; N_INTERACTION_COLUMNS];
        TreeVec::new(vec![vec![], trace_log_sizes, interaction_log_sizes])
    }
}
```

The `log_size` is essential because:

- All traces must be powers of 2 for FFT efficiency
- It determines the allocation size for
  `ComponentTrace::<N_TRACE_COLUMNS>::uninitialized(log_size)`
- Interaction traces use the same log size as execution traces.

## Writing the Execution Trace

The `write_trace` function is the core of trace generation. It transforms raw
execution data into properly formatted traces for the STARK prover. This
function follows a consistent pattern across all components with three main
responsibilities:

1. **Determine trace size and allocate memory**
2. **Pack inputs into SIMD-optimized format**
3. **Populate trace columns while collecting lookup data**

### Understanding the Write Trace Function

The `write_trace` function signature typically looks like this:

```rust
pub fn write_trace<MC: MerkleChannel>(
    inputs: &mut Vec<ExecutionBundle>,
) -> (Self, ComponentTrace<N_TRACE_COLUMNS>, InteractionClaimData)
```

**Key aspects:**

- **Input**: Raw execution data (e.g., `ExecutionBundle` for opcodes,
  `clock_update_data` for clock component)
- **Output**: A tuple containing:
  - `Self`: The claim with computed log_size
  - `ComponentTrace<N_TRACE_COLUMNS>`: The execution trace with witness values
  - `InteractionClaimData`: Lookup data for interaction trace generation (and
    `non_padded_length`)

### Step 1: Trace Size Calculation

```rust
let non_padded_length = inputs.len();
let log_size = std::cmp::max(LOG_N_LANES, inputs.len().next_power_of_two().ilog2());
```

**Why this calculation?**

- `non_padded_length`: Tracks actual execution steps (needed for enabler)
- `log_size`: Must be at least `LOG_N_LANES` for SIMD alignment
- Trace size must be a power of 2 for FFT operations

### Step 2: Memory Allocation

```rust
let (mut trace, mut lookup_data) = unsafe {
    (
        ComponentTrace::<N_TRACE_COLUMNS>::uninitialized(log_size),
        LookupData::uninitialized(log_size - LOG_N_LANES),
    )
};
```

**Important notes:**

- Uses `unsafe` for performance - avoids zeroing large memory blocks
- `ComponentTrace` size: `2^log_size` rows
- `LookupData` size: `2^(log_size - LOG_N_LANES)` because it stores packed data
- The `-LOG_N_LANES` offset accounts for SIMD packing (N_LANES values per packed
  element)

### Step 3: Input Packing

```rust
// Resize to power of 2 with default padding
inputs.resize(1 << log_size, ExecutionBundle::default());

// Pack into SIMD format
let packed_inputs: Vec<PackedExecutionBundle> = inputs
    .par_chunks_exact(N_LANES)
    .map(|chunk| {
        let array: [ExecutionBundle; N_LANES] = chunk.try_into().unwrap();
        Pack::pack(array)
    })
    .collect();

// Clear original inputs to free memory
inputs.clear();
inputs.shrink_to_fit();
```

**Packing process:**

- Resizes input to exact power of 2 size
- Groups `N_LANES` consecutive elements
- Packs them into SIMD registers (`PackedM31`)
- Clears original data to reduce memory pressure

### Trace Population with Lookup Data Collection

The trace is populated while simultaneously collecting lookup data for later use
in interaction traces. From `clock_update.rs`:

```rust
(
    trace.par_iter_mut(),
    packed_inputs.into_par_iter(),
    lookup_data.par_iter_mut(),
)
    .into_par_iter()
    .enumerate()
    .for_each(|(row_index, (mut row, input, lookup_data))| {
        let enabler = enabler_col.packed_at(row_index);
        let address = input[0];
        let prev_clk = input[1];
        let value0 = input[2];
        let value1 = input[3];
        let value2 = input[4];
        let value3 = input[5];

        // Write to trace columns
        *row[0] = enabler;
        *row[1] = address;
        *row[2] = prev_clk;
        *row[3] = value0;
        *row[4] = value1;
        *row[5] = value2;
        *row[6] = value3;

        // Store lookup data for interaction traces
        *lookup_data.memory[0] = [address, prev_clk, value0, value1, value2, value3];
        *lookup_data.memory[1] = [
            address,
            prev_clk + PackedM31::broadcast(M31::from(RC20_LIMIT)),
            value0,
            value1,
            value2,
            value3,
        ];
    });
```

## Writing the Interaction Trace

The interaction trace implements the lookup argument protocol by writing
fractions that represent the lookup relations.

### Batching Two Lookups Per Column

This pattern combines two lookup relations into one fraction, requiring
`SECURE_EXTENSION_DEGREE` columns. From `clock_update.rs`:

```rust
pub fn write_interaction_trace(
    relations: &Relations,
    interaction_claim_data: &InteractionClaimData,
) -> (Self, impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>)
{
    let log_size = interaction_claim_data.lookup_data.memory[0].len().ilog2() + LOG_N_LANES;
    let mut interaction_trace = LogupTraceGenerator::new(log_size);
    let enabler_col = Enabler::new(interaction_claim_data.non_padded_length);

    let mut col = interaction_trace.new_col();
    (
        col.par_iter_mut(),
        &interaction_claim_data.lookup_data.memory[0],
        &interaction_claim_data.lookup_data.memory[1],
    )
        .into_par_iter()
        .enumerate()
        .for_each(|(i, (writer, value0, value1))| {
            let num0: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
            let denom0: PackedQM31 = relations.memory.combine(value0);
            let num1: PackedQM31 = PackedQM31::from(enabler_col.packed_at(i));
            let denom1: PackedQM31 = relations.memory.combine(value1);

            let numerator = num0 * denom1 + num1 * denom0;
            let denom = denom0 * denom1;

            writer.write_frac(numerator, denom);
        });
    col.finalize_col();
```

This batching pattern explains why
`N_INTERACTION_COLUMNS = div_ceil(num_lookups, 2) * SECURE_EXTENSION_DEGREE`.
Each column can handle two lookups batched together.

## Constraint Evaluation

The `evaluate` function defines the algebraic constraints that must hold for
valid execution. From `call_abs_imm.rs`:

```rust
fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
    let one = E::F::from(M31::one());
    let opcode_constant = E::F::from(M31::from(CALL_ABS_IMM));

    let enabler = eval.next_trace_mask();
    let pc = eval.next_trace_mask();
    let fp = eval.next_trace_mask();
    let clock = eval.next_trace_mask();
    // ... continue getting all columns via next_trace_mask()

    // Add algebraic constraints
    eval.add_constraint(
        enabler.clone() * (one.clone() - enabler.clone())
    );

    // Add lookup relations
    eval.add_to_relation(&[
        RelationEntry::new(
            &self.relations.registers,
            -enabler.clone(),
            &[pc.clone(), fp.clone()],
        ),
        RelationEntry::new(
            &self.relations.registers,
            enabler.clone(),
            &[off1.clone(), fp.clone() + off0.clone() + E::F::from(M31::from(2))],
        ),
    ]);
```

The evaluation process:

1. Retrieves column values using `next_trace_mask()`
2. Defines algebraic constraints with `add_constraint()`
3. Adds lookup relations with `add_to_relation()`
