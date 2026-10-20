param([Parameter(Mandatory)][string]$Name)
$ErrorActionPreference = 'Stop'
if ($Name -notmatch '^[A-Za-z0-9.-]+$') { throw 'Use a simple evidence name' }
$projectRoot = Split-Path -Parent $PSScriptRoot
$diagnosticPath = Join-Path $projectRoot "evidence/$Name.json"
$nativeExe = Join-Path $projectRoot 'target/release/openevo.exe'
$process = Start-Process -FilePath $nativeExe -ArgumentList @('--diagnostics', ('"' + $diagnosticPath + '"')) -PassThru -Wait -WindowStyle Hidden
if ($process.ExitCode -ne 0) { throw "Native diagnostics failed: $($process.ExitCode)" }
Get-Content -LiteralPath $diagnosticPath -Raw
