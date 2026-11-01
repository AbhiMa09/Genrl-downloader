# PowerShell Fast Pre-Commit Guardrails (< 30s target)
# Parity with tools/pre_commit.sh

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

$StartTime = [System.Diagnostics.Stopwatch]::StartNew()
Write-Host "=================================================="
Write-Host " Running Fast Pre-Commit Guardrails (Target: < 30s)"
Write-Host "=================================================="

# 1. Rust Formatting
$StepWatch = [System.Diagnostics.Stopwatch]::StartNew()
Write-Host "--> [1/8] Checking Rust formatting (cargo fmt)..."
cargo fmt --manifest-path core/Cargo.toml --all -- --check
if ($LASTEXITCODE -ne 0) { throw "cargo fmt failed" }

# 2. Flutter Formatting
Write-Host "--> [2/8] Checking Flutter formatting (dart format)..."
dart format --output=none --set-exit-if-changed app/
if ($LASTEXITCODE -ne 0) { throw "dart format failed" }
Write-Host "    Formatting OK in $($StepWatch.Elapsed.TotalSeconds.ToString('F1'))s"

# 3. Rust Clippy
$StepWatch.Restart()
Write-Host "--> [3/8] Checking Rust clippy (zero warnings)..."
cargo clippy --manifest-path core/Cargo.toml --workspace -- -D warnings
if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed" }

# 4. Flutter Analyze
Write-Host "--> [4/8] Checking Flutter analysis (flutter analyze)..."
Push-Location app
try {
    flutter analyze
    if ($LASTEXITCODE -ne 0) { throw "flutter analyze failed" }
} finally {
    Pop-Location
}
Write-Host "    Lint & analysis OK in $($StepWatch.Elapsed.TotalSeconds.ToString('F1'))s"

# 5. File Size Cap Guardrail
$StepWatch.Restart()
Write-Host "--> [5/8] Checking file line caps (<= 500 lines)..."
python tools/check_caps.py
if ($LASTEXITCODE -ne 0) { throw "check_caps.py failed" }
Write-Host "    Line caps OK in $($StepWatch.Elapsed.TotalSeconds.ToString('F1'))s"

# 6. Contract Guardrail
$StepWatch.Restart()
Write-Host "--> [6/8] Checking contract synchronization..."
python tools/generate_contract.py --check
if ($LASTEXITCODE -ne 0) { throw "generate_contract.py failed" }
Write-Host "    Contract OK in $($StepWatch.Elapsed.TotalSeconds.ToString('F1'))s"

# 7. Theme Guardrail
$StepWatch.Restart()
Write-Host "--> [7/8] Checking theme tokens synchronization..."
python tools/generate_theme.py --check
if ($LASTEXITCODE -ne 0) { throw "generate_theme.py failed" }
Write-Host "    Theme OK in $($StepWatch.Elapsed.TotalSeconds.ToString('F1'))s"

# 8. Fast Architectural Guardrail Tests
$StepWatch.Restart()
Write-Host "--> [8/8] Checking fast architectural boundary tests..."
Push-Location app
try {
    flutter test test/architecture_guardrails_test.dart test/contract_compliance_test.dart test/design_tokens_test.dart
    if ($LASTEXITCODE -ne 0) { throw "architecture guardrail tests failed" }
} finally {
    Pop-Location
}
Write-Host "    Fast tests OK in $($StepWatch.Elapsed.TotalSeconds.ToString('F1'))s"

$TotalDuration = [Math]::Round($StartTime.Elapsed.TotalSeconds)
Write-Host "=================================================="
Write-Host " All fast pre-commit guardrails PASSED in ${TotalDuration}s!"
Write-Host "=================================================="
