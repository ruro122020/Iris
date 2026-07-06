# Iris

A learning project: building a **web API in Rust** from scratch, as a way to
deeply understand how Rust works.

## What this is

Iris is a hands-on exercise in writing a web API using Rust. The goal is not only a functioning server, but a solid mental model of *why* Rust code is written the way it is: ownership,
borrowing, lifetimes, error handling, traits, and async (asynchronous)
programming.

Every non-trivial design decision is treated as a chance to learn what the
compiler does at build time (borrow checking, monomorphization, trait
resolution) and what happens at runtime (stack vs. heap allocation, when memory
is freed, static vs. dynamic dispatch).

## Goals

- **Learn Rust properly.** Understand the language under the hood, not just copy
  patterns that compile.
- **Build a working web API.** Serve HTTP requests, define routes, and return structured responses.
- **Practice idiomatic Rust.** Follow `clippy` (Rust's linter) and `rustfmt`
  (Rust's formatter) conventions; use `Result`-based error handling instead of
  `unwrap()`/`expect()` in real code paths.
- **Keep dependencies minimal and justified.** Every crate added
  to `Cargo.toml` should be to maintain a learning process.

## Current status

Early scaffold. Right now the project is a fresh Cargo binary: `main.rs` prints
`Hello, world!` and there are no dependencies yet. The web API itself is still
to be built.

## Getting started

Prerequisites: a recent Rust toolchain installed via
[rustup](https://rustup.rs/).

```bash
# Build the project
cargo build

# Run it
cargo run

# Check without producing a binary (fast feedback loop)
cargo check

# Lint and format
cargo clippy
cargo fmt
```

## Planned direction

The exact web framework is not chosen yet. Likely candidates in the Rust
ecosystem include **Axum**, **Actix Web**, and **Rocket**. The choice will be
made deliberately, weighing the learning value and trade-offs of each rather
than defaulting to the most popular option.

## Project layout

```
Iris/
├── Cargo.toml      # Package manifest: metadata + dependencies
├── Cargo.lock      # Exact resolved dependency versions
├── src/
│   └── main.rs     # Entry point (currently "Hello, world!")
└── README.md       # This file
```

