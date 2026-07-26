#!/usr/bin/env bash

cd src-tauri && cargo clippy && cargo fmt -- --check
cd .. || exit 1
bun run check
bun run lint
bun run format:check

