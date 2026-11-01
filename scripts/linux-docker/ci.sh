#!/usr/bin/env bash
# Passi del job "linux" della CI, eseguiti dentro il container (vedi scripts/test-linux-docker.sh).
set -euo pipefail
cd /src
step() { echo "=== $*"; }
step npm ci;   npm ci --prefix app/ui
step ui build; npm --prefix app/ui run build
step fmt;      cargo fmt --all -- --check
step clippy;   cargo clippy --workspace --all-targets -- -D warnings
step test
cargo test --workspace --no-fail-fast 2>&1 | tee /tmp/test.log | grep -E "test result|FAILED|panicked"
test "${PIPESTATUS[0]}" -eq 0
if [ -z "${SKIP_PACKAGE:-}" ]; then
  step package; scripts/package-linux.sh
fi
step OK
