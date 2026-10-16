#!/usr/bin/env bash
set -eo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

START_TIME=$(date +%s)
echo "=================================================="
echo " Running Pre-Push Full Test Verification"
echo "=================================================="

# 1. Full Rust Test Suite (Core, CLI, Snapshots, Proptests)
T_RUST=$(date +%s)
echo "--> [1/2] Running full Rust workspace test suite..."
cargo test --manifest-path core/Cargo.toml --workspace
echo "    Rust tests PASSED in $(($(date +%s) - T_RUST))s"

# 2. Full Flutter Test Suite
T_FLUTTER=$(date +%s)
echo "--> [2/2] Running full Flutter test suite..."
(cd app && flutter test)
echo "    Flutter tests PASSED in $(($(date +%s) - T_FLUTTER))s"

TOTAL_DURATION=$(($(date +%s) - START_TIME))
echo "=================================================="
echo " All pre-push test suites PASSED in ${TOTAL_DURATION}s!"
echo "=================================================="
