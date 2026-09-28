$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repositoryRoot
try {
    & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-contracts.ps1
    if ($LASTEXITCODE -ne 0) { throw "Contract verification failed." }

    & npm.cmd run build
    if ($LASTEXITCODE -ne 0) { throw "Frontend build failed." }

    & cargo.exe fmt --manifest-path src-tauri/Cargo.toml --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "Rust formatting check failed." }

    & cargo.exe clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "Rust clippy failed." }

    & cargo.exe test --locked --manifest-path src-tauri/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw "Rust tests failed." }
}
finally {
    Pop-Location
}
