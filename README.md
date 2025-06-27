# Welcome to Cairo M

Cairo M is a brand new Cpu AIR leveraging M31 as its native prime field,
unleashing the Maximum power of Starkware's S-TWO to enable Mobile proving.

Cairo Max looks like Cairo Zero, just tastes better.

## Overview

Cairo M is designed to provide efficient computation on consumer hardware,
especially mobile phone. It includes several components that work together to
compile, run, and prove Cairo programs.

The main design choices are

- keep registers count at their minimum, using only `pc` (program counter) and
  `fp` (frame pointer)
- each frame is of constant deterministic size
- read-write memory
- variable-size instruction encoding (x86 style)
- support of native types (u8, u32, etc.) thanks to an heavy use of Stwo's
  component system

## Crates

- **Compiler**: The `cairo-m-compiler` crate handles the compilation of Cairo
  code into Cairo Assembly for execution.
- **Runner**: The `cairo-m-runner` crate is responsible for executing the
  compiled Cairo programs and generate the trace.
- **Prover**: The `cairo-m-prover` crate generates proofs of correct execution
  for the programs run using the runner.

## Getting Started

To get started with Cairo M, clone the repository and build the project using
Cargo. Make sure you have the Rust nightly toolchain installed as specified in
`rust-toolchain.toml`.

[Stwo](https://github.com/starkware-libs/stwo) is added as a git submodule to
allow for an easier debugging of the AIRs. Use

```bash
git submodule update --init --recursive
```

to pull the correct pinned version.

**Note for MacOS users**: You need to have `lld` installed, which can be done
easily with `brew install lld`. Also, make sure that your `clang` is the
Homebrew one (`brew install llvm`), not the Xcode one.

The project uses [trunk.io](https://trunk.io/) for managing all the linters, so
make sure to install both the CLI and the VScode extension.

## Benchmark VM

```bash
RUSTFLAGS="-C target-cpu=native" cargo bench --bench vm_benchmark -- --verbose
```

The command will run all benchmark functions from the VM and display the
throughput results in your terminal.

## Profile Prover

Samply can be used

```bash
cargo install --locked samply
```

Compile the program you want to run and launch:

```bash
samply record target/release/cairo-m-prover --entrypoint fib --arguments 100000 crates/prover/tests/test_data/fibonacci.json
```

For memory, it leverages [DHAT](https://docs.rs/dhat/latest/dhat/).

```bash
CARGO_PROFILE_RELEASE_DEBUG=true cargo test --release --package cairo-m-prover --test prover --features dhat-heap -- test_memory_profile_fibonacci_prover --nocapture
```

You can visualize with
[dh_view](https://nnethercote.github.io/dh_view/dh_view.html)
