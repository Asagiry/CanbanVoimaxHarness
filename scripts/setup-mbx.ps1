# Installs pinned precompiled mr-boxington (mbx) into ~/.cargo/bin after SHA-256 verification and runs setup.
# Takes 2-3 seconds without compiling from source.

param(
    [string]$Version = "v1.29.2",
    [string]$ExpectedSha256 = "5ccfe5942c224844ffc50eb689de9c02033e401b210d3c80fffe4b94c929b0a2"
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($ExpectedSha256)) {
    throw "ExpectedSha256 must not be empty."
}

$cargoBin = "$env:USERPROFILE\.cargo\bin"
if (-not (Test-Path $cargoBin)) {
    New-Item -ItemType Directory -Force -Path $cargoBin | Out-Null
}

$url = "https://github.com/jdx/mr-boxington/releases/download/$Version/mbx-x86_64-pc-windows-msvc.zip"
$tempZip = "$env:TEMP\mbx.zip"
$tempDir = "$env:TEMP\mbx_unpacked"

Write-Host "Downloading mr-boxington (mbx) $Version from GitHub Releases..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $url -OutFile $tempZip

$actualSha256 = (Get-FileHash -Path $tempZip -Algorithm SHA256).Hash.ToLowerInvariant()
$expectedLower = $ExpectedSha256.Trim().ToLowerInvariant()
if ($actualSha256 -ne $expectedLower) {
    Remove-Item -Force $tempZip -ErrorAction SilentlyContinue
    throw "SHA-256 checksum mismatch for mbx $Version! Expected: $expectedLower, Actual: $actualSha256"
}
Write-Host "Verified SHA-256 checksum ($actualSha256)." -ForegroundColor Green

Write-Host "Extracting mbx.exe to $cargoBin..." -ForegroundColor Cyan
if (Test-Path $tempDir) {
    Remove-Item -Recurse -Force $tempDir
}
Expand-Archive -Path $tempZip -DestinationPath $tempDir -Force

Copy-Item -Force "$tempDir\mbx.exe" "$cargoBin\mbx.exe"

Remove-Item -Force $tempZip -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue

Write-Host "Configuring mbx compiler cache..." -ForegroundColor Cyan
& "$cargoBin\mbx.exe" setup --yes

Write-Host "mr-boxington (mbx) successfully installed and configured!" -ForegroundColor Green
& "$cargoBin\mbx.exe" --version
