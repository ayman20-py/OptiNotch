# OptiNotch Single-Binary Distribution & Packaging Script
$ErrorActionPreference = "Stop"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host " Building OptiNotch Standalone Executable " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

$WorkspaceRoot = Split-Path -Parent $PSScriptRoot
Set-Location $WorkspaceRoot

# 1. Clean previous dist
$DistDir = Join-Path $WorkspaceRoot "dist"
if (Test-Path $DistDir) {
    Write-Host "[1/4] Cleaning existing dist directory..." -ForegroundColor Yellow
    Remove-Item -Path $DistDir -Recurse -Force
}
New-Item -ItemType Directory -Path $DistDir | Out-Null

# 2. Generate multi-res Windows icon from SVG
Write-Host "[2/5] Ensuring assets/icon.ico is generated..." -ForegroundColor Green
& (Join-Path $PSScriptRoot "generate_icon.ps1")

# 3. Cargo build release
Write-Host "[3/5] Compiling optimized release binary with LTO, strip, and icon embedding..." -ForegroundColor Green
cargo build --release

$TargetExe = Join-Path $WorkspaceRoot "target\release\opti-notch.exe"
if (-not (Test-Path $TargetExe)) {
    # Check if named OptiNotch.exe instead
    $AltExe = Join-Path $WorkspaceRoot "target\release\OptiNotch.exe"
    if (Test-Path $AltExe) {
        $TargetExe = $AltExe
    } else {
        Write-Error "Release executable was not found at $TargetExe!"
        exit 1
    }
}

# 3. Copy to dist/
$DistExe = Join-Path $DistDir "OptiNotch.exe"
Write-Host "[3/4] Copying binary to $DistExe..." -ForegroundColor Green
Copy-Item -Path $TargetExe -Destination $DistExe -Force

# 4. Generate metadata / ZIP archive
$DistZip = Join-Path $DistDir "OptiNotch-v0.1.0-windows-x64.zip"
Write-Host "[4/4] Creating standalone distribution ZIP: $DistZip..." -ForegroundColor Green
Compress-Archive -Path $DistExe -DestinationPath $DistZip -Force

$SizeMB = [math]::Round((Get-Item $DistExe).Length / 1MB, 2)
$ZipSizeMB = [math]::Round((Get-Item $DistZip).Length / 1MB, 2)

Write-Host "`nDistribution Build Complete!" -ForegroundColor Cyan
Write-Host "Standalone .EXE : $DistExe ($SizeMB MB)" -ForegroundColor White
Write-Host "Release .ZIP    : $DistZip ($ZipSizeMB MB)" -ForegroundColor White
Write-Host "`nYou can now upload 'OptiNotch.exe' or 'OptiNotch-v0.1.0-windows-x64.zip' directly to your GitHub Releases." -ForegroundColor Green
