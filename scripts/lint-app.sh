#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo fmt --manifest-path src-tauri/lib/ajazz-sdk/Cargo.toml -- --check
cargo fmt --manifest-path plugins/com.amansprojects.starterpack.sdPlugin/Cargo.toml -- --check

cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
cargo clippy --manifest-path src-tauri/lib/ajazz-sdk/Cargo.toml --all-targets
cargo clippy --manifest-path plugins/com.amansprojects.starterpack.sdPlugin/Cargo.toml --all-targets

cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/lib/ajazz-sdk/Cargo.toml
cargo test --manifest-path plugins/com.amansprojects.starterpack.sdPlugin/Cargo.toml

bun run verify

