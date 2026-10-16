#!/usr/bin/env bash
set -eo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=================================================="
echo " Running Pre-Commit Verification Guardrails"
echo "=================================================="

# 1. Formatting
echo "--> Checking Rust formatting (cargo fmt)..."
cargo fmt --manifest-path core/Cargo.toml --all -- --check

echo "--> Checking Flutter/Dart formatting (dart format)..."
dart format --output=none --set-exit-if-changed app/

# 2. Linting & Static Analysis
echo "--> Checking Rust clippy (zero warnings)..."
cargo clippy --manifest-path core/Cargo.toml --workspace -- -D warnings

echo "--> Checking Flutter analysis (flutter analyze)..."
(cd app && flutter analyze)

# 3. Size Cap Guardrail
echo "--> Checking file line caps (<= 500 lines)..."
python3 tools/check_caps.py

# 4. Single-Source Contract Guardrail
echo "--> Checking contract synchronization..."
python3 tools/generate_contract.py --check

# 5. Design Tokens Theme Guardrail
echo "--> Checking theme tokens synchronization..."
python3 tools/generate_theme.py --check

# 6. Architecture & Bridge Boundary Guardrail
echo "--> Checking architectural boundary tests..."
(cd app && flutter test test/architecture_guardrails_test.dart test/contract_compliance_test.dart test/design_tokens_test.dart)

# 7. Fast Test Suite
echo "--> Running Rust test suite..."
cargo test --manifest-path core/Cargo.toml --workspace

echo "--> Running Flutter test suite (including goldens)..."
(cd app && flutter test)

echo "=================================================="
echo " All pre-commit guardrails PASSED successfully!"
echo "=================================================="
