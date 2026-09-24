# OptiNotch Single-Binary Distribution & Packaging Script
$ErrorActionPreference = "Stop"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host " Building OptiNotch Standalone Executable " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

$WorkspaceRoot = Split-Path -Parent $PSScriptRoot
Set-Location $WorkspaceRoot

# 1. Terminate any running instances to prevent file lock errors
Get-Process -Name "*OptiNotch*", "*opti-notch*" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 300

# 2. Extract Version from Cargo.toml
$CargoContent = Get-Content (Join-Path $WorkspaceRoot "Cargo.toml") -Raw
if ($CargoContent -match 'version\s*=\s*"([^"]+)"') {
    $Version = $matches[1]
} else {
    $Version = "0.1.0"
}
Write-Host "Target Version: v$Version" -ForegroundColor Yellow

# 3. Ensure dist directory exists
$DistDir = Join-Path $WorkspaceRoot "dist"
if (-not (Test-Path $DistDir)) {
    New-Item -ItemType Directory -Path $DistDir | Out-Null
}

# 4. Generate multi-res Windows icon from SVG
Write-Host "[1/4] Ensuring assets/icon.ico is generated..." -ForegroundColor Green
& (Join-Path $PSScriptRoot "generate_icon.ps1")

# 5. Cargo build release
Write-Host "[2/4] Compiling optimized release binary with LTO, strip, and icon embedding..." -ForegroundColor Green
cargo build --release

$TargetExe = Join-Path $WorkspaceRoot "target\release\opti-notch.exe"
if (-not (Test-Path $TargetExe)) {
    $AltExe = Join-Path $WorkspaceRoot "target\release\OptiNotch.exe"
    if (Test-Path $AltExe) {
        $TargetExe = $AltExe
    } else {
        Write-Error "Release executable was not found at $TargetExe!"
        exit 1
    }
}

# 6. Copy to dist/
$DistExe = Join-Path $DistDir "OptiNotch.exe"
Write-Host "[3/4] Copying binary to $DistExe..." -ForegroundColor Green
Copy-Item -Path $TargetExe -Destination $DistExe -Force

# 7. Generate release ZIP archive
$DistZip = Join-Path $DistDir "OptiNotch-v$Version-windows-x64.zip"
Write-Host "[4/4] Creating standalone distribution ZIP: $DistZip..." -ForegroundColor Green
if (Test-Path $DistZip) {
    Remove-Item -Path $DistZip -Force
}
Compress-Archive -Path $DistExe -DestinationPath $DistZip -Force

$SizeMB = [math]::Round((Get-Item $DistExe).Length / 1MB, 2)
$ZipSizeMB = [math]::Round((Get-Item $DistZip).Length / 1MB, 2)

Write-Host "`nDistribution Build Complete!" -ForegroundColor Cyan
Write-Host "Standalone .EXE : $DistExe ($SizeMB MB)" -ForegroundColor White
Write-Host "Release .ZIP    : $DistZip ($ZipSizeMB MB)" -ForegroundColor White
Write-Host "`nYou can now upload 'OptiNotch.exe' or 'OptiNotch-v$Version-windows-x64.zip' directly to your GitHub Releases." -ForegroundColor Green
