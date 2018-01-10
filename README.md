# sophon-wasm

[![Build Status](https://travis-ci.org/super-string/sophon-wasm.svg?branch=master)](https://travis-ci.org/super-string/sophon-wasm)
[![crates.io link](https://img.shields.io/crates/v/sophon-wasm.svg)](https://crates.io/crates/sophon-wasm)

[Documentation](https://nikvolf.github.io/sophon-wasm/sophon_wasm/)

## Rust WebAssembly format serializing/deserializing

along with experimental interpreter

```rust

extern crate sophon_wasm;

let module = sophon_wasm::deserialize_file("./res/cases/v1/hello.wasm");
assert_eq!(module.code_section().is_some());

let code_section = module.code_section().unwrap(); // Part of the module with functions code

println!("Function count in wasm file: {}", code_section.bodies().len());
```

## Wabt Test suite

Interpreter and decoder supports full wabt testsuite (https://github.com/WebAssembly/testsuite), To run testsuite:

- make sure you have all prerequisites to build `wabt` (since sophon-wasm builds it internally using `cmake`, see https://github.com/WebAssembly/wabt)
- checkout with submodules (`git submodule update --init --recursive`)
- run `cargo test --release --manifest-path=spec/Cargo.toml`

Decoder can be fuzzed with `cargo-fuzz` using `wasm-opt` (https://github.com/WebAssembly/binaryen):

- make sure you have all prerequisites to build `binaryen` and `cargo-fuzz` (`cmake` and a C++11 toolchain)
- checkout with submodules (`git submodule update --init --recursive`)
- install `cargo fuzz` subcommand with `cargo install cargo-fuzz`
- set rustup to use a nightly toolchain, because `cargo fuzz` uses a rust compiler plugin: `rustup override set nightly`
- run `cargo fuzz run deserialize`

# License

`sophon-wasm` is primarily distributed under the terms of both the MIT
license and the Apache License (Version 2.0), at your choice.

See LICENSE-APACHE, and LICENSE-MIT for details.
