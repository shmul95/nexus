# nexus-gen

## Project Structure
Rust workspace with 4 crates: nexus-core, nexus-validate, nexus-codegen, nexus-cli.

## Build
- `cargo build` — build all crates
- `cargo test` — run all tests
- `cargo run -- <args>` — run nexus-cli (binary name: nexus-gen)
- `nix develop` — enter dev shell

## Conventions
- Use `thiserror` for error types in library crates
- Use workspace dependencies (defined in root Cargo.toml)
- All public types in nexus-core must derive Debug, Clone
- Tests go in the same file as the code they test (mod tests)

## Architecture
- nexus-core: types, parsers (.nxs via pest, network.toml via serde)
- nexus-validate: graph validation rules
- nexus-codegen: C header/impl generation via minijinja templates
- nexus-cli: thin CLI shell using clap
