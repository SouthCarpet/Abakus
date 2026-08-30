#Requires -Version 7
$src = "A:/projects-vault/design/web/abakus.tokens.css"
$dst = Join-Path (Split-Path $PSScriptRoot -Parent) "src/tokens.css"
Copy-Item -LiteralPath $src -Destination $dst -Force
Write-Host "tokens synced -> $dst"
