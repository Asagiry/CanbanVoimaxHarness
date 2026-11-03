# Installs precompiled mr-boxington (mbx) into ~/.cargo/bin and runs setup.
# Takes 2-3 seconds without compiling from source.

$ErrorActionPreference = "Stop"

$cargoBin = "$env:USERPROFILE\.cargo\bin"
if (-not (Test-Path $cargoBin)) {
    New-Item -ItemType Directory -Force -Path $cargoBin | Out-Null
}

$url = "https://github.com/jdx/mr-boxington/releases/latest/download/mbx-x86_64-pc-windows-msvc.zip"
$tempZip = "$env:TEMP\mbx.zip"
$tempDir = "$env:TEMP\mbx_unpacked"

Write-Host "Downloading precompiled mr-boxington (mbx) from official GitHub Releases..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $url -OutFile $tempZip

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
