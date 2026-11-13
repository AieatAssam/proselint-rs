# Proselint Rust

A native Rust implementation of [Proselint](https://github.com/amperser/proselint), the English prose linter. This repository contains the Rust crate and CLI; the original project and its Python package remain at [amperser/proselint](https://github.com/amperser/proselint).

The Rust implementation currently covers a selected subset of Proselint's rules and command-line behavior. It is not yet a drop-in replacement for the full Python project. See the [compatibility status](rust/docs/COMPATIBILITY.md) for verified coverage and known gaps.

## Build and run

Build the Rust CLI from the crate manifest:

```sh
cargo build --release --manifest-path rust/Cargo.toml
```

The binary is named `proselint`. Without a configuration file, the CLI uses its bundled defaults. To override them, save a JSON configuration such as this as `proselint.json` to enable the `misc.suddenly` rule:

```json
{
  "max_errors": 1000,
  "checks": {
    "misc": {
      "suddenly": true
    }
  }
}
```

Then run it on a file:

```sh
./rust/target/release/proselint check --config proselint.json --output-format json text.md
```

The Rust library crate is `proselint_rs`, defined alongside the CLI in [`rust/Cargo.toml`](rust/Cargo.toml).

## Project information

- [Compatibility status](rust/docs/COMPATIBILITY.md)
- [Pinned Python baseline](rust/docs/BASELINE.md)
- [Python and Rust benchmark results](rust/docs/BENCHMARKS.md)
- [License](LICENSE.md)

For the original Python package, its integrations, and its full documentation, visit [the upstream Proselint project](https://github.com/amperser/proselint).
