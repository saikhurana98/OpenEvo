param(
    [ValidateSet('debug', 'release')][string]$Configuration = 'release',
    [switch]$Test
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $projectRoot
try {
    $localCargo = Join-Path $projectRoot '.tools\cargo\bin\cargo.exe'
    if (Test-Path -LiteralPath $localCargo) {
        $env:RUSTUP_HOME = Join-Path $projectRoot '.tools\rustup'
        $env:CARGO_HOME = Join-Path $projectRoot '.tools\cargo'
        $cargoExe = $localCargo
    } else {
        $cargoExe = (Get-Command cargo -ErrorAction Stop).Source
    }
    $cargoArgs = @('build', '--locked')
    if ($Test) { $cargoArgs[0] = 'test' }
    if ($Configuration -eq 'release') { $cargoArgs += '--release' }
    & $cargoExe @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw "Cargo exited with status $LASTEXITCODE" }
} finally { Pop-Location }
