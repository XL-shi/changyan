#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CERT="ChangYan Dev"
RUNNER="$SCRIPT_DIR/src-tauri/scripts/sign-and-run"

# Use the runner hook: cargo calls it AFTER build, BEFORE launch.
# The runner signs the binary and then exec's it — guaranteed signed at startup.
export CARGO_TARGET_X86_64_APPLE_DARWIN_RUNNER="$RUNNER"
export CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER="$RUNNER"

npm run tauri dev
