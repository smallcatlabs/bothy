#!/usr/bin/env bash
# The quality gate, kept green by every change. Run it bare and read its
# exit status; a check piped through grep reports the filter's exit, not
# the check's.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
git diff --check
./target/debug/bothy check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
