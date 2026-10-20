param([string]$Version = '0.3.1')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if ($Version -notmatch '^[0-9A-Za-z.-]+$') {throw 'Invalid package version'}
& (Join-Path $PSScriptRoot 'build.ps1') -Configuration release
$destination = Join-Path $projectRoot "dist/OpenEvo-$Version-windows-x64"
New-Item -ItemType Directory -Path $destination -Force | Out-Null
$sourceExe = Join-Path $projectRoot 'target/release/openevo.exe'
$destinationExe = Join-Path $destination 'OpenEvo.exe'
# Identical executables can stay open during a documentation-only repack.
if (!(Test-Path -LiteralPath $destinationExe) -or
    (Get-FileHash -LiteralPath $sourceExe).Hash -ne (Get-FileHash -LiteralPath $destinationExe).Hash) {
    Copy-Item -LiteralPath $sourceExe -Destination $destinationExe
}
foreach ($name in @('README.md','CHANGELOG.md','NATIVE_README.md','LICENSE','THIRD_PARTY_NOTICES.md','RELEASE_STATUS.md','PROTOCOL.md','FEASIBILITY.md','EXECUTION.md')) {
    Copy-Item -LiteralPath (Join-Path $projectRoot $name) -Destination $destination
}
if (Test-Path -LiteralPath (Join-Path $projectRoot 'docs/images')) {
    $docsTarget = Join-Path $destination 'docs'
    New-Item -ItemType Directory -Path $docsTarget -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $projectRoot 'docs/images') -Destination $docsTarget -Recurse -Force
    Copy-Item -LiteralPath (Join-Path $projectRoot 'docs/validation') -Destination $docsTarget -Recurse -Force
}
$sourceTarget = Join-Path $destination 'source'
New-Item -ItemType Directory -Path $sourceTarget -Force | Out-Null
foreach ($name in @('src','ui','.cargo')) {
    Copy-Item -LiteralPath (Join-Path $projectRoot $name) -Destination $sourceTarget -Recurse -Force
}
foreach ($name in @('Cargo.toml','Cargo.lock','rust-toolchain.toml','build.rs','LICENSE','README.md')) {
    Copy-Item -LiteralPath (Join-Path $projectRoot $name) -Destination $sourceTarget
}
Copy-Item -LiteralPath (Join-Path $projectRoot 'NATIVE_README.md') -Destination $sourceTarget
New-Item -ItemType Directory -Path (Join-Path $sourceTarget 'tools') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'build.ps1') -Destination (Join-Path $sourceTarget 'tools')
$manifestPath = Join-Path $destination 'SHA256SUMS.txt'
$manifest = Get-ChildItem -LiteralPath $destination -File -Recurse -Force |
    Where-Object { $_.FullName -ne $manifestPath } | Sort-Object FullName |
    ForEach-Object {
        $relative = $_.FullName.Substring($destination.Length + 1).Replace('\','/')
        '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $relative
    }
Set-Content -LiteralPath $manifestPath -Value $manifest -Encoding utf8
$archive = "$destination.zip"
Compress-Archive -LiteralPath $destination -DestinationPath $archive -Force
Get-FileHash -LiteralPath $archive -Algorithm SHA256 | Select-Object Path,Hash
