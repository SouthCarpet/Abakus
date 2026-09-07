param(
  [Parameter(Mandatory = $true)]
  [string]$PageManifest,
  [Parameter(Mandatory = $true)]
  [string]$OutputDirectory,
  [ValidateRange(1, 40)]
  [int]$PagesPerSheet = 20,
  [ValidateRange(1, 8)]
  [int]$Columns = 4
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-Descendant {
  param([string]$Parent, [string]$Child, [string]$Label)
  $parentFull = [IO.Path]::GetFullPath($Parent).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
  $childFull = [IO.Path]::GetFullPath($Child)
  if (-not $childFull.StartsWith($parentFull, [StringComparison]::OrdinalIgnoreCase)) {
    throw "$Label must stay inside $Parent`: $childFull"
  }
}

function New-Sheet {
  param([object[]]$Pages, [string]$PageDirectory, [string]$Target)
  $margin = 12
  $labelHeight = 28
  $thumbWidth = 260
  $thumbHeight = 370
  $cellWidth = $thumbWidth + $margin
  $cellHeight = $labelHeight + $thumbHeight + $margin
  $rows = [int][Math]::Ceiling($Pages.Count / [double]$Columns)
  $bitmap = [Drawing.Bitmap]::new($margin * 2 + $cellWidth * $Columns, $margin * 2 + $cellHeight * $rows)
  $graphics = [Drawing.Graphics]::FromImage($bitmap)
  $font = [Drawing.Font]::new('Arial', 12, [Drawing.FontStyle]::Bold, [Drawing.GraphicsUnit]::Pixel)
  $brush = [Drawing.Brushes]::Black
  try {
    $graphics.Clear([Drawing.Color]::White)
    $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    for ($index = 0; $index -lt $Pages.Count; $index++) {
      $page = $Pages[$index]
      $sourcePath = Join-Path $PageDirectory ([string]$page.png_file)
      if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) { throw "Missing rendered page: $sourcePath" }
      $actualHash = (Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash.ToLowerInvariant()
      if ($actualHash -ne ([string]$page.png_sha256).ToLowerInvariant()) { throw "Rendered-page hash mismatch: $sourcePath" }
      $source = [Drawing.Image]::FromFile($sourcePath)
      try {
        $scale = [Math]::Min($thumbWidth / [double]$source.Width, $thumbHeight / [double]$source.Height)
        $width = [int][Math]::Max(1, [Math]::Round($source.Width * $scale))
        $height = [int][Math]::Max(1, [Math]::Round($source.Height * $scale))
        $column = $index % $Columns
        $row = [int][Math]::Floor($index / [double]$Columns)
        $x = $margin + $column * $cellWidth + [int][Math]::Floor(($thumbWidth - $width) / 2)
        $y = $margin + $row * $cellHeight
        $graphics.DrawString("Page $($page.page_number)", $font, $brush, $x, $y)
        $graphics.DrawImage($source, $x, $y + $labelHeight, $width, $height)
      } finally {
        $source.Dispose()
      }
    }
    $bitmap.Save($Target, [Drawing.Imaging.ImageFormat]::Png)
  } finally {
    $font.Dispose()
    $graphics.Dispose()
    $bitmap.Dispose()
  }
}

Add-Type -AssemblyName System.Drawing
$acceptanceRoot = [IO.Path]::GetFullPath($PSScriptRoot)
$manifestPath = (Resolve-Path -LiteralPath $PageManifest).Path
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
Assert-Descendant -Parent $acceptanceRoot -Child $manifestPath -Label 'Page manifest'
Assert-Descendant -Parent $acceptanceRoot -Child $outputPath -Label 'Contact-sheet output'
if (Test-Path -LiteralPath $outputPath) { throw "Refusing existing output directory: $outputPath" }

$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$pages = @($manifest.pages)
if ($pages.Count -ne [int]$manifest.page_count -or $pages.Count -eq 0) { throw 'Page manifest count is invalid.' }
$expectedNumbers = @(1..$pages.Count)
$actualNumbers = @($pages | ForEach-Object { [int]$_.page_number })
if ((ConvertTo-Json -Compress $actualNumbers) -ne (ConvertTo-Json -Compress $expectedNumbers)) { throw 'Page manifest numbers are not continuous.' }

$parent = Split-Path -Parent $outputPath
if (-not (Test-Path -LiteralPath $parent -PathType Container)) { [IO.Directory]::CreateDirectory($parent) | Out-Null }
$stage = Join-Path $parent ('.' + [IO.Path]::GetFileName($outputPath) + '.partial-' + $PID)
if (Test-Path -LiteralPath $stage) { throw "Refusing existing partial directory: $stage" }
[IO.Directory]::CreateDirectory($stage) | Out-Null

try {
  $sheetRows = @()
  for ($offset = 0; $offset -lt $pages.Count; $offset += $PagesPerSheet) {
    $last = [Math]::Min($offset + $PagesPerSheet - 1, $pages.Count - 1)
    $chunk = @($pages[$offset..$last])
    $name = 'contact-{0:D4}-{1:D4}.png' -f $chunk[0].page_number, $chunk[-1].page_number
    $target = Join-Path $stage $name
    New-Sheet -Pages $chunk -PageDirectory (Split-Path -Parent $manifestPath) -Target $target
    $sheetRows += [ordered]@{
      file = $name
      sha256 = (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash.ToLowerInvariant()
      bytes = (Get-Item -LiteralPath $target).Length
      pages = @($chunk | ForEach-Object { [int]$_.page_number })
    }
  }
  $covered = @($sheetRows | ForEach-Object { $_.pages } | ForEach-Object { $_ })
  if ((ConvertTo-Json -Compress $covered) -ne (ConvertTo-Json -Compress $expectedNumbers)) { throw 'Contact sheets do not cover every page exactly once.' }
  $result = [ordered]@{
    page_manifest = $manifestPath
    page_manifest_sha256 = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
    input_pdf_sha256 = [string]$manifest.input_sha256
    page_count = $pages.Count
    pages_per_sheet = $PagesPerSheet
    columns = $Columns
    sheets = $sheetRows
  }
  $resultJson = ($result | ConvertTo-Json -Depth 8) + "`n"
  [IO.File]::WriteAllText((Join-Path $stage 'manifest.json'), $resultJson, [Text.UTF8Encoding]::new($false))
  [IO.Directory]::Move($stage, $outputPath)
  $result | ConvertTo-Json -Compress -Depth 8
} catch {
  if (Test-Path -LiteralPath $stage) {
    $resolvedStage = (Resolve-Path -LiteralPath $stage).Path
    Assert-Descendant -Parent $acceptanceRoot -Child $resolvedStage -Label 'Contact-sheet partial cleanup'
    [IO.Directory]::Delete($resolvedStage, $true)
  }
  throw
}
