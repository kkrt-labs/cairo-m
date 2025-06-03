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

The project uses [trunk.io](https://trunk.io/) for managing all the linters, so
make sure to install both the CLI and the VScode extension.
