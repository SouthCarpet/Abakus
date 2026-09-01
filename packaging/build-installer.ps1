<#
.SYNOPSIS
    Builds Abakus and packages it into a Windows installer with Inno Setup 6.

.DESCRIPTION
    Reads the app version from src-tauri\tauri.conf.json, builds the
    frontend (tsc, then vite) and the release binary (tauri-cli, via node,
    jobs=4) unless -SkipBuild, then runs ISCC against packaging\abakus.iss
    with /DAppVersion=<version>. Both the frontend build and the tauri-cli
    build call node directly on the installed package scripts instead of
    `npm run build` / `npm run tauri build`, because the .cmd shims for
    those tools return "Access is denied" on this machine (an AV lock, not a
    real build failure). The tauri-cli build merges in
    packaging\tauri.build-override.json to blank out
    tauri.conf.json's beforeBuildCommand, so tauri-cli does not try to run
    `npm run build` itself and hit the same shim lock; the frontend is
    already built by the step above. ESET real-time protection also
    sometimes locks a freshly written setup exe, so the ISCC step retries up
    to 6 times with a 15 s pause between attempts.

.PARAMETER SkipBuild
    Skip the Tauri release build. Use this when the release binary is
    already built (for example, a prior step already ran it).

.EXAMPLE
    .\packaging\build-installer.ps1

.EXAMPLE
    .\packaging\build-installer.ps1 -SkipBuild
#>
[CmdletBinding()]
param(
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$tauriConfPath = Join-Path $repoRoot 'src-tauri\tauri.conf.json'

$tauriConf = Get-Content -LiteralPath $tauriConfPath -Raw | ConvertFrom-Json
$version = $tauriConf.version
Write-Host "Abakus version: $version"

if (-not $SkipBuild) {
    # `npm run tauri build` and `npm run build` shell out to the tsc/vite/
    # tauri .cmd shims, which return "Access is denied" on this machine (AV
    # lock on the shim, not a real build failure -- see the project's known
    # machine trap). Build the frontend by invoking the node scripts
    # directly, then build the Rust side through tauri-cli, also via node.
    # tauri-cli's own bundling step is skipped (bundle.active is false; Inno
    # does the packaging below).
    Write-Host "Building frontend (tsc + vite via node, bypassing .cmd shims)..."
    Push-Location $repoRoot
    try {
        node node_modules/typescript/bin/tsc -b
        if ($LASTEXITCODE -ne 0) {
            throw "tsc failed with exit code $LASTEXITCODE"
        }
        node node_modules/vite/bin/vite.js build
        if ($LASTEXITCODE -ne 0) {
            throw "vite build failed with exit code $LASTEXITCODE"
        }
        if (-not (Test-Path -LiteralPath (Join-Path $repoRoot 'dist\index.html'))) {
            throw "Expected dist\index.html not found after the frontend build"
        }

        Write-Host "Building release binary (tauri-cli via node, jobs=4)..."
        $env:CARGO_BUILD_JOBS = '4'
        # A bare `cargo build` (even with --features custom-protocol added by
        # hand) never runs the real tauri-cli build path, so it is fragile:
        # any future tauri-cli-only step would be silently skipped. Call
        # tauri-cli itself instead, through node, to avoid the tsc/vite .cmd
        # shim lock. `--no-bundle` skips tauri-cli's own bundling (Inno does
        # that below); `--config` merges in
        # packaging\tauri.build-override.json, which sets
        # build.beforeBuildCommand to "" -- without that, tauri-cli would run
        # `npm run build` itself (tauri.conf.json's configured
        # beforeBuildCommand), which hits the same shim lock this script
        # already avoided above by calling tsc/vite directly. tauri-cli sets
        # the `custom-protocol` cargo feature itself, so the release binary
        # embeds the frontend and does not load build.devUrl
        # (http://localhost:1420) at runtime.
        $tauriOverride = Join-Path $repoRoot 'packaging\tauri.build-override.json'
        node node_modules/@tauri-apps/cli/tauri.js build --no-bundle --config $tauriOverride
        if ($LASTEXITCODE -ne 0) {
            throw "tauri-cli build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}

# The workspace Cargo.toml at the repo root puts all members' build output in
# one shared target dir, so the binary lands in target\release, not
# src-tauri\target\release.
$exePath = Join-Path $repoRoot 'target\release\abakus.exe'
if (-not (Test-Path -LiteralPath $exePath)) {
    throw "Expected release binary not found at $exePath"
}

# A binary built by a bare `cargo build` keeps the dev URL and embeds no
# frontend, so the installed app opens WebView2 on localhost and fails. Only
# tauri-cli sets the custom-protocol feature that flips that decision. The
# packaged exe must therefore carry the vite bundle name from dist\assets;
# this guard catches the case where -SkipBuild reuses such a stale binary.
$assetsDir = Join-Path $repoRoot 'dist\assets'
$bundleName = $null
if (Test-Path -LiteralPath $assetsDir) {
    $bundle = Get-ChildItem -LiteralPath $assetsDir -Filter '*.js' | Select-Object -First 1
    if ($bundle) { $bundleName = [IO.Path]::GetFileNameWithoutExtension($bundle.Name) }
}
if ($bundleName) {
    $exeText = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($exePath))
    if (-not $exeText.Contains($bundleName)) {
        throw "The binary at $exePath does not embed the frontend bundle '$bundleName'. It was built by a bare cargo build, not by tauri-cli, so the installed app would open http://localhost:1420. Rebuild without -SkipBuild."
    }
    Write-Host "Frontend bundle '$bundleName' found in the release binary."
} else {
    Write-Warning "dist\assets holds no .js bundle, so the embedded-frontend check was skipped."
}

$pdfiumPath = Join-Path $repoRoot 'src-tauri\resources\pdfium\pdfium.dll'
if (-not (Test-Path -LiteralPath $pdfiumPath)) {
    throw "Expected pdfium.dll not found at $pdfiumPath. Run scripts\fetch-pdfium.ps1 first."
}

$isccCandidates = @()
$isccCommand = Get-Command ISCC -ErrorAction SilentlyContinue
if ($isccCommand) {
    $isccCandidates += $isccCommand.Source
}
$isccCandidates += 'C:\Users\Asus\AppData\Local\Programs\Inno Setup 6\ISCC.exe'
$isccCandidates += Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'
$isccCandidates += 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe'

$iscc = $isccCandidates | Select-Object -Unique | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1

if (-not $iscc) {
    throw "ISCC.exe (Inno Setup 6 command-line compiler) was not found. Install Inno Setup 6 " +
        "(winget install JRSoftware.InnoSetup) or add ISCC to PATH. Checked: $($isccCandidates -join ', ')"
}
Write-Host "Using ISCC: $iscc"

$issPath = Join-Path $repoRoot 'packaging\abakus.iss'
$outputDir = Join-Path $repoRoot 'packaging\output'
if (-not (Test-Path -LiteralPath $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir | Out-Null
}

# ESET real-time protection can hold an exclusive lock on the setup exe for
# tens of seconds after ISCC finishes writing it (observed up to ~40 s on
# this machine), so a plain 4x/10s loop is not always enough. Retry more
# patiently: 6 attempts, 15 s apart.
$maxAttempts = 6
$attempt = 0
$compiled = $false
while (-not $compiled -and $attempt -lt $maxAttempts) {
    $attempt++
    & $iscc "/DAppVersion=$version" $issPath
    if ($LASTEXITCODE -eq 0) {
        $compiled = $true
    } else {
        Write-Warning "ISCC attempt $attempt failed (exit $LASTEXITCODE). ESET real-time protection can lock a freshly written setup exe; retrying."
        if ($attempt -lt $maxAttempts) {
            # Delete the half-written setup before retrying. Without this the
            # loop could never recover: ISCC aborts with "The output file
            # appears to be in use (5)", leaves the partial exe behind, and
            # the scanner keeps that leftover locked, so every later attempt
            # fails on the SAME stale file rather than on a fresh write.
            # Observed 2026-08-31: 6 attempts failed in a row, then one
            # attempt after deleting the leftover succeeded immediately.
            $partial = Join-Path $outputDir "abakus-setup-$version.exe"
            if (Test-Path -LiteralPath $partial) {
                Remove-Item -LiteralPath $partial -Force -ErrorAction SilentlyContinue
            }
            Start-Sleep -Seconds 15
        }
    }
}
if (-not $compiled) {
    throw "ISCC failed after $maxAttempts attempts."
}

$outputExe = Join-Path $repoRoot "packaging\output\abakus-setup-$version.exe"
if (-not (Test-Path -LiteralPath $outputExe)) {
    throw "Expected installer not found at $outputExe"
}
Write-Host "Installer built: $outputExe"
