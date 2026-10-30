param([ValidateSet('debug', 'release')][string]$Configuration = 'debug')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$previousDebugInfo = $env:SLINT_EMIT_DEBUG_INFO
Push-Location -LiteralPath $projectRoot
try {
    $localCargo = Join-Path $projectRoot '.tools/cargo/bin/cargo.exe'
    if (Test-Path -LiteralPath $localCargo) {
        $env:RUSTUP_HOME = Join-Path $projectRoot '.tools/rustup'
        $env:CARGO_HOME = Join-Path $projectRoot '.tools/cargo'
        $cargoExe = $localCargo
    } else {
        $cargoExe = (Get-Command cargo -ErrorAction Stop).Source
    }
    # Only this test build contains element introspection metadata. The testing
    # backend renders into memory; it never creates or controls a desktop window.
    $env:SLINT_EMIT_DEBUG_INFO = '1'
    $cargoArgs = @('test', '--locked', '--bin', 'openevo')
    if ($Configuration -eq 'release') { $cargoArgs += '--release' }
    $cargoArgs += @('headless_motion_regression', '--', '--ignored', '--nocapture')
    & $cargoExe @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw "Headless motion regression failed: $LASTEXITCODE" }
} finally {
    $env:SLINT_EMIT_DEBUG_INFO = $previousDebugInfo
    Pop-Location
}
