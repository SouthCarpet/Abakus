#Requires -Version 7
# One-time dev download of the pdfium binary (controller-run; plan 065 Task 7, amendment A7).
# Pinned to a release TAG, not 'latest'; first run pins the archive hash (TOFU on an immutable artifact).
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$dest = Join-Path $root "src-tauri/resources/pdfium"
$tag = "chromium%2F7469"
$url = "https://github.com/bblanchon/pdfium-binaries/releases/download/$tag/pdfium-win-x64.tgz"
$tmp = Join-Path $env:TEMP "pdfium-win-x64.tgz"
Invoke-WebRequest -Uri $url -OutFile $tmp
$hashFile = Join-Path $PSScriptRoot "pdfium.sha256"
$actual = (Get-FileHash -Algorithm SHA256 $tmp).Hash.ToLower()
if (Test-Path $hashFile) {
    $expected = (Get-Content $hashFile -Raw).Trim().ToLower()
    if ($expected -ne $actual) { throw "pdfium hash mismatch: expected $expected got $actual" }
    Write-Host "hash verified $actual"
} else {
    Set-Content -Path $hashFile -Value $actual
    Write-Host "pinned pdfium hash $actual (tag $tag)"
}
New-Item -ItemType Directory -Force $dest | Out-Null
tar -xzf $tmp -C $env:TEMP "bin/pdfium.dll"
Copy-Item (Join-Path $env:TEMP "bin/pdfium.dll") (Join-Path $dest "pdfium.dll") -Force
Write-Host "pdfium.dll -> $dest"
