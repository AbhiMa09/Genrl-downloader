# PowerShell Pre-Push Full Test Verification
# Parity with tools/pre_push.sh

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

$StartTime = [System.Diagnostics.Stopwatch]::StartNew()
Write-Host "=================================================="
Write-Host " Running Pre-Push Full Test Verification"
Write-Host "=================================================="

# 1. Full Rust Workspace Test Suite
$StepWatch = [System.Diagnostics.Stopwatch]::StartNew()
Write-Host "--> [1/2] Running full Rust workspace test suite..."
cargo test --manifest-path core/Cargo.toml --workspace
if ($LASTEXITCODE -ne 0) { throw "Rust test suite failed" }
Write-Host "    Rust tests PASSED in $($StepWatch.Elapsed.TotalSeconds.ToString('F1'))s"

# 2. Full Flutter Test Suite (Excluding goldens)
$StepWatch.Restart()
Write-Host "--> [2/2] Running full Flutter test suite (excluding goldens)..."
Push-Location app
try {
    flutter test --exclude-tags golden
    if ($LASTEXITCODE -ne 0) { throw "Flutter test suite failed" }
} finally {
    Pop-Location
}
Write-Host "    Flutter tests PASSED in $($StepWatch.Elapsed.TotalSeconds.ToString('F1'))s"

$TotalDuration = [Math]::Round($StartTime.Elapsed.TotalSeconds)
Write-Host "=================================================="
Write-Host " All pre-push test suites PASSED in ${TotalDuration}s!"
Write-Host "=================================================="
