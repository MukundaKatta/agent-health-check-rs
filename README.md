# agent-health-check

A tiny, dependency-free Rust library for tracking the health status of LLM agent components.

When you build an LLM agent, it typically depends on several moving parts — an LLM provider, a vector store, a database, a cache, external tools, and so on. `agent-health-check` gives you a simple in-memory registry to record the current status of each of those components and to answer questions like "is everything OK?" or "which components are down?".

## Features

- **`Status` enum** with four states: `Ok`, `Degraded(reason)`, `Down(reason)`, and `Unknown`.
- **`HealthRegistry`** — a named map of components to their statuses.
- Aggregate checks: `all_ok()` (every component is `Ok`) and `is_healthy()` (nothing is `Down` or `Unknown`; `Degraded` is tolerated).
- Querying helpers: `down_components()`, `degraded_components()`, and `components()` — all returned sorted.
- Lifecycle helpers: `set`, `get`, `remove`, `reset_all`, `len`, `is_empty`.
- Zero external dependencies (standard library only).

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
agent-health-check = { git = "https://github.com/MukundaKatta/agent-health-check-rs" }
```

## Usage

```rust
use agent_health_check::{HealthRegistry, Status};

let mut reg = HealthRegistry::new();

reg.set("llm_provider", Status::Ok);
reg.set("database", Status::Degraded("slow queries".into()));
reg.set("vector_store", Status::Down("connection refused".into()));

// all_ok() is false — not every component is Ok.
assert!(!reg.all_ok());

// is_healthy() is false — vector_store is Down.
assert!(!reg.is_healthy());

// Inspect the trouble spots (results are sorted).
assert_eq!(reg.down_components(), vec!["vector_store"]);
assert_eq!(reg.degraded_components(), vec!["database"]);

// A Degraded-only registry is still considered "healthy".
reg.set("vector_store", Status::Ok);
assert!(reg.is_healthy());
assert!(!reg.all_ok());
```

### `all_ok` vs `is_healthy`

| Method        | Returns `true` when...                                              |
|---------------|---------------------------------------------------------------------|
| `all_ok()`    | every registered component is `Status::Ok`                          |
| `is_healthy()`| no component is `Down` or `Unknown` (`Degraded` is allowed)         |

An empty registry is vacuously `all_ok()` and `is_healthy()`.

## Tech stack

- **Language:** Rust (edition 2021)
- **Dependencies:** none — only the standard library (`std::collections::HashMap`)
- **License:** MIT

## Development

Build, test, and lint locally:

```sh
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

The library ships with a full unit-test suite covering each registry operation and aggregate check.

## License

Licensed under the [MIT License](LICENSE).
