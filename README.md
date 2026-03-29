# nexus-gen

A code generator for declarative IPC topology. Define your inter-process communication network once in TOML, validate the communication graph, and automatically generate C headers, implementations, and Nix derivations.

## What is nexus-gen?

nexus-gen lets you:
- **Declare** your IPC topology in `network.toml` — which nodes exist, how they communicate, and what contracts they exchange
- **Define** message schemas in `.nxs` files — lightweight schema format for message structure
- **Validate** the graph to catch broken edges, missing contracts, and topology errors before deployment
- **Generate** production-ready C code and build files from your topology definition

This removes the need to manually wire up IPC code across multiple processes and keeps your topology specification in sync with your implementation.

## Quick Start

### Prerequisites
- Rust 1.70+ (via `nix develop` or installed locally)
- Nix (optional, for development shell)

### Build

**With Nix:**
```bash
nix develop
cargo build --release
```

**Without Nix:**
```bash
cargo build --release
```

The compiled binary is `nexus-gen` (located at `./target/release/nexus-gen`).

### Run

**Validate a topology:**
```bash
cargo run -- validate --config examples/sample/network.toml
```

Output:
```
Validation passed. 3 nodes, 2 contracts, 2 edges.
```

**Generate C code and Nix derivations:**
```bash
cargo run -- build --emit nix --config examples/sample/network.toml --output ./out
```

Generated files appear in `./out/`:
- C headers (`.h`)
- C implementations (`.c`)
- Nix derivations (`.nix`)

## Project Structure

The workspace contains four crates:

### nexus-core
Parses `network.toml` and `.nxs` schema files. Provides the core data structures representing nodes, contracts, and message schemas.

### nexus-validate
Validates the communication graph:
- Ensures all referenced contracts exist
- Checks for orphaned nodes
- Detects broken edges (sender/receiver mismatches)
- Validates schema consistency

### nexus-codegen
Generates C headers, implementations, and Nix derivations from a validated network. Uses Minijinja templates for flexible output.

### nexus-cli
Command-line interface. Entry point is `nexus-gen` binary with subcommands: `validate`, `build`, and `diff` (planned).

## Configuration Format

### network.toml

Define your nodes and the contracts (message types) they exchange:

```toml
[[nodes]]
name = "game_engine"
sends = [{ contract = "game_info", to = "backend" }]

[[nodes]]
name = "backend"
receives = [{ contract = "game_info", from = "game_engine" }]
sends    = [{ contract = "display",   to = "frontend" }]

[[nodes]]
name = "frontend"
receives = [{ contract = "display", from = "backend" }]

[[contracts]]
name      = "game_info"
transport = "unix_socket"
schema    = "schemas/game_info.nxs"

[[contracts]]
name      = "display"
transport = "unix_socket"
fields = [
  { name = "frame_id", type = "u32" },
  { name = "width",    type = "u32" },
  { name = "height",   type = "u32" },
]
```

**Nodes** declare the processes in your system and their send/receive edges.

**Contracts** define message types. A contract can either:
- Reference a `.nxs` schema file (`schema = "path/to/file.nxs"`)
- Inline field definitions (`fields = [...]`)

### .nxs Schema Format

Define message structure in `.nxs` files:

```nxs
struct GameInfo {
    player_id  : u32
    position_x : f32
    position_y : f32
    timestamp  : u64
}
```

Supported types: `u32`, `i32`, `f32`, `f64`, `u64`, `i64`, `bool`.

## Generated Output

When you run `build`, nexus-gen produces:

**C Headers** (`.h`)
- Type definitions for each contract message
- Function declarations for send/receive

**C Implementations** (`.c`)
- IPC transport logic (Unix socket for MVP)
- Message serialization/deserialization

**Nix Derivations** (`.nix`)
- Build specifications for each component
- Dependency management

All files are placed in the output directory with a structure matching your node names.

## Commands

### validate
Check that your `network.toml` and schemas are valid.

```bash
nexus-gen validate --config network.toml
```

### build
Generate C code and build files.

```bash
nexus-gen build \
  --emit nix \
  --config network.toml \
  --output ./out
```

- `--emit nix` — Output Nix derivations (currently the only supported format)
- `--emit cmake` — Planned for future release
- `--config` — Path to network.toml (default: `network.toml`)
- `--output` — Output directory (default: `nexus-gen-out`)

### diff
Detect breaking changes between two network.toml files.

```bash
nexus-gen diff old.toml new.toml
```

*Not yet implemented.*

## Examples

The `examples/` directory contains working configurations:

- **minimal/** — Two nodes exchanging a single message
- **sample/** — Three nodes with two contracts (game engine, backend, frontend)

Run the sample:
```bash
cargo run -- validate --config examples/sample/network.toml
cargo run -- build --emit nix --config examples/sample/network.toml --output ./out
```

## Project Status

**Phase 1 (Current MVP):**
- Unix socket transport
- Basic validation
- C code generation
- Nix derivation output

**Phase 2 (Planned):**
- gRPC transport
- HTTP transport
- iceoryx shared-memory transport
- `diff` command for breaking change detection
- Language bindings (Python, Go)
- Visual editor for topology design

## Development

Tests run with:
```bash
cargo test
```

End-to-end tests live in `tests/e2e.rs`.

## License

MIT
