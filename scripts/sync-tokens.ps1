#Requires -Version 7
$root = Split-Path $PSScriptRoot -Parent
$src = "A:/projects-vault/design/web/abakus.tokens.css"
$dst = Join-Path $root "src/tokens.css"
Copy-Item -LiteralPath $src -Destination $dst -Force
Write-Host "tokens synced -> $dst"

# Keep the Rail's mark (public/icon.svg) in sync with the source mark
# (assets/icon.svg, Task 2) so the app never ships a stale copy.
$markSrc = Join-Path $root "assets/icon.svg"
$markDst = Join-Path $root "public/icon.svg"
Copy-Item -LiteralPath $markSrc -Destination $markDst -Force
Write-Host "mark synced -> $markDst"
