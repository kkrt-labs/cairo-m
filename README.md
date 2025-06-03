# Welcome to Cairo M

Cairo M is a brand new Cpu AIR leveraging M31 as its native prime field,
unleashing the Maximum power of Starkware's S-TWO, enabling Mobile proving

## Overview

Cairo M is designed to provide efficient and secure computation using the M31
prime field. It includes several components that work together to compile, run,
and prove Cairo programs.

## Components

- **Compiler**: The `cairo-m-compiler` crate handles the compilation of Cairo
  code into an intermediate representation suitable for execution and proving.
- **Runner**: The `cairo-m-runner` crate is responsible for executing the
  compiled Cairo programs.
- **Prover**: The `cairo-m-prover` crate generates proofs of correct execution
  for the programs run using the runner.

## Getting Started

To get started with Cairo M, clone the repository and build the project using
Cargo. Make sure you have the Rust nightly toolchain installed as specified in
`rust-toolchain.toml`.
