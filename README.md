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
- support of native types (felt, u32, etc.) thanks to an heavy use of Stwo's
  component system

## Crates

- **Compiler**: The `cairo-m-compiler` crate handles the compilation of Cairo
  code into Cairo Assembly for execution.
- **Runner**: The `cairo-m-runner` crate is responsible for executing the
  compiled Cairo programs and generate the trace.
- **Prover**: The `cairo-m-prover` crate generates proofs of correct execution
  for the programs run using the runner.

## Developing with Cairo M

To get started with Cairo M, read the
[getting-started](./docs/getting-started.md).

To learn the language, you can run the interactive tutorial `CairoMlings`,
inspired by Rustlings - see the
[CairoMlings README](./tutorials/cairomlings/README.md) for more details.

```bash
cargo install --path tutorials/cairomlings
```

Then, you can initialize an exercise directory with `cairomlings init`.

### Creating a New Cairo-M Project

Cairo-M provides a CLI tool to quickly scaffold new projects with integrated
testing:

```bash
# Build and install cargo-cairo-m
cargo install --path crates/cargo-cairo-m

# Create a new Cairo-M project
cargo-cairo-m init my-project

# Navigate to your project and run tests
cd my-project
cargo test
```

The generated project includes:

- A sample Cairo-M fibonacci implementation
- Rust tests that use `run_cairo_program` for differential testing
- Pre-configured dependencies on cairo-m crates
- Automatic RUSTFLAGS configuration for optimal performance
- A README with common commands

This setup allows you to write Cairo-M code and test it against Rust reference
implementations using familiar Rust testing tools.

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

Note also that `.worktrees` is added to `.gitignore` so that you can start
working with work tress from the root of the project

```bash
git worktree add .worktrees/<branch-name>
```

The project uses [trunk.io](https://trunk.io/) for managing all the linters, so
make sure to install both the CLI and the VScode extension.

### Note for MacOS users

You need to have LLVM and their LLD linker installed.

```bash
brew install llvm
brew install lld
```

#### Troubleshooting

Make sure that you are using the Homebrew version of `clang` and `llvm`, not the
Xcode (Apple) one.

```bash
echo 'export CC=/opt/homebrew/opt/llvm/bin/clang' >> ~/.zshrc
echo 'export CXX=/opt/homebrew/opt/llvm/bin/clang++' >> ~/.zshrc
echo 'export AR=/opt/homebrew/opt/llvm/bin/llvm-ar' >> ~/.zshrc
echo 'export RANLIB=/opt/homebrew/opt/llvm/bin/llvm-ranlib' >> ~/.zshrc
```

## Benchmark VM

### MacOS

```bash
RUSTFLAGS="-C link-arg=-fuse-ld=/opt/homebrew/bin/ld64.lld -C target-cpu=native" cargo bench --bench vm_benchmark -- --verbose
```

### Other Platforms

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

Compile the program you want to run and launch samply:

```bash
CARGO_PROFILE_RELEASE_DEBUG=true cargo build -r
target/release/cairo-m-compiler --input test_data/functions/fibonacci_loop.cm --output crates/prover/tests/test_data/fibonacci_loop.json
samply record target/release/cairo-m-prover crates/prover/tests/test_data/fibonacci.json --entrypoint fibonacci_loop --arguments 100000
```

For memory, it leverages [DHAT](https://docs.rs/dhat/latest/dhat/).

```bash
CARGO_PROFILE_RELEASE_DEBUG=true cargo test --release --package cairo-m-prover --test prover --features dhat-heap -- test_memory_profile_fibonacci_prover --nocapture
```

You can visualize with
[dh_view](https://nnethercote.github.io/dh_view/dh_view.html)
