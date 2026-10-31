$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    $exe = Join-Path $root 'target/release/PinWin.exe'
    $version = (Get-Item -LiteralPath $exe).VersionInfo.ProductVersion
    if ([string]::IsNullOrWhiteSpace($version)) { throw 'EXE version resource missing' }
    $dist = Join-Path $root 'dist'
    New-Item -ItemType Directory -Path $dist -Force | Out-Null
    $archive = Join-Path $dist "PinWin-$version-windows-x64.zip"
    Compress-Archive -LiteralPath $exe, (Join-Path $root 'README.md'), (Join-Path $root 'LICENSE') -DestinationPath $archive -Force
    Get-Item -LiteralPath $exe, $archive | Select-Object FullName, Length
    Get-FileHash -LiteralPath $archive -Algorithm SHA256
} finally {
    Pop-Location
}
