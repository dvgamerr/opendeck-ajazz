param(
  [string]$ProjectRoot = (Get-Location).Path
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ProjectRoot = (Resolve-Path -LiteralPath $ProjectRoot).Path
$sourceIcon = Join-Path $ProjectRoot "assets\app-icon.svg"
$outputDir = Join-Path $ProjectRoot "src-tauri\icons"

if (-not (Test-Path -LiteralPath $sourceIcon -PathType Leaf)) {
  throw "Icon source not found: $sourceIcon"
}

New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

Push-Location $ProjectRoot
try {
  & bun run tauri icon $sourceIcon --output $outputDir
  if ($LASTEXITCODE -ne 0) {
    throw "Tauri icon generation failed with exit code $LASTEXITCODE"
  }
} finally {
  Pop-Location
}

$requiredFiles = @(
  "32x32.png",
  "128x128.png",
  "128x128@2x.png",
  "icon.png",
  "icon.ico",
  "icon.icns"
)

foreach ($file in $requiredFiles) {
  $path = Join-Path $outputDir $file
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Expected generated icon was not created: $path"
  }
}

Write-Host "Generated Tauri icon assets from assets/app-icon.svg"
