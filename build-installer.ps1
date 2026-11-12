# Membuat installer PDF Catcher: dist\PDFCatcher-Setup-<versi>.exe
# Pakai:  powershell -ExecutionPolicy Bypass -File .\build-installer.ps1
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
. .\env.ps1

Write-Host '1/3  Menjalankan test...'
cargo test --release
if ($LASTEXITCODE -ne 0) { throw 'Test gagal, installer tidak dibuat.' }

Write-Host '2/3  Build pdfcatcher.exe (release)...'
cargo build --release
if ($LASTEXITCODE -ne 0) { throw 'Build gagal.' }

Write-Host '3/3  Kompilasi installer...'
$iscc = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 6 (ISCC.exe) tidak ditemukan.' }
& $iscc /Q installer\pdfcatcher.iss
if ($LASTEXITCODE -ne 0) { throw 'Kompilasi installer gagal.' }

Get-ChildItem dist\*.exe | Select-Object Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } }
