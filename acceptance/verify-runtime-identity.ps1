param(
    [Parameter(Mandatory = $true)][string]$ReadyManifest,
    [string]$Output
)

$ErrorActionPreference = 'Stop'
$config = Get-Content -Raw -LiteralPath $ReadyManifest | ConvertFrom-Json

function First-Value([object]$Object, [string[]]$Names) {
    foreach ($name in $Names) {
        $property = $Object.PSObject.Properties[$name]
        if ($null -ne $property -and $null -ne $property.Value -and "$($property.Value)" -ne '') {
            return $property.Value
        }
    }
    throw "readiness manifest missing one of: $($Names -join ', ')"
}

$taskPid = [int](First-Value $config @('pid', 'process_id', 'app_pid'))
$artifact = [string](First-Value $config @('artifact_path', 'binary_path', 'executable_path', 'exe_path'))
$expectedHash = ([string](First-Value $config @('artifact_sha256', 'binary_sha256', 'sha256'))).ToLowerInvariant()
$process = Get-Process -Id $taskPid -ErrorAction Stop
$processPath = $process.Path
$artifactResolved = (Resolve-Path -LiteralPath $artifact).Path
$processResolved = (Resolve-Path -LiteralPath $processPath).Path
$actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $artifactResolved).Hash.ToLowerInvariant()

if ($processResolved -cne $artifactResolved) {
    throw "PID $taskPid owns '$processResolved', expected exact task artifact '$artifactResolved'"
}
if ($actualHash -cne $expectedHash) {
    throw "artifact SHA256 mismatch: expected $expectedHash, got $actualHash"
}

$result = [ordered]@{
    pid = $taskPid
    process_name = $process.ProcessName
    executable_path = $processResolved
    sha256 = $actualHash
    started_at = $process.StartTime.ToString('o')
    identity_matches_readiness_manifest = $true
}
if ($Output) {
    if (Test-Path -LiteralPath $Output) {
        throw "refusing existing identity output: $Output"
    }
    $outputDirectory = Split-Path -Parent $Output
    New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
    $result | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 -LiteralPath $Output
}
$result | ConvertTo-Json -Depth 4 -Compress
