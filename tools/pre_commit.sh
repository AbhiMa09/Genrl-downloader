#!/usr/bin/env bash
set -eo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

START_TIME=$(date +%s)
echo "=================================================="
echo " Running Fast Pre-Commit Guardrails (Target: < 30s)"
echo "=================================================="

# 1. Formatting
T_FMT=$(date +%s)
echo "--> [1/8] Checking Rust formatting (cargo fmt)..."
cargo fmt --manifest-path core/Cargo.toml --all -- --check

echo "--> [2/8] Checking Flutter formatting (dart format)..."
dart format --output=none --set-exit-if-changed app/
echo "    Formatting OK in $(($(date +%s) - T_FMT))s"

# 2. Linting & Static Analysis
T_LINT=$(date +%s)
echo "--> [3/8] Checking Rust clippy (zero warnings)..."
cargo clippy --manifest-path core/Cargo.toml --workspace -- -D warnings

echo "--> [4/8] Checking Flutter analysis (flutter analyze)..."
(cd app && flutter analyze)
echo "    Lint & analysis OK in $(($(date +%s) - T_LINT))s"

# 3. File Size Cap Guardrail
T_CAPS=$(date +%s)
echo "--> [5/8] Checking file line caps (<= 500 lines)..."
python3 tools/check_caps.py
echo "    Line caps OK in $(($(date +%s) - T_CAPS))s"

# 4. Single-Source Contract Guardrail
T_CONTRACT=$(date +%s)
echo "--> [6/8] Checking contract synchronization..."
python3 tools/generate_contract.py --check
echo "    Contract OK in $(($(date +%s) - T_CONTRACT))s"

# 5. Design Tokens Theme Guardrail
T_THEME=$(date +%s)
echo "--> [7/8] Checking theme tokens synchronization..."
python3 tools/generate_theme.py --check
echo "    Theme OK in $(($(date +%s) - T_THEME))s"

# 6. Fast Architecture & Guardrail Tests
T_GUARD=$(date +%s)
echo "--> [8/8] Checking fast architectural boundary tests..."
(cd app && flutter test test/architecture_guardrails_test.dart test/contract_compliance_test.dart test/design_tokens_test.dart)
echo "    Fast tests OK in $(($(date +%s) - T_GUARD))s"

TOTAL_DURATION=$(($(date +%s) - START_TIME))
echo "=================================================="
echo " All fast pre-commit guardrails PASSED in ${TOTAL_DURATION}s!"
echo "=================================================="
