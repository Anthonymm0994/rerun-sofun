# F.R.O.G. Release Build Script for Windows
# This script sets up the Visual Studio environment and builds the release binary

Write-Host "F.R.O.G. Release Build Script" -ForegroundColor Green
Write-Host "=============================" -ForegroundColor Green
Write-Host ""

# Function to find Visual Studio installation
function Find-VisualStudio {
    $vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    
    if (Test-Path $vsWhere) {
        $vsPath = & $vsWhere -latest -property installationPath
        return $vsPath
    }
    
    # Fallback to common paths
    $commonPaths = @(
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Community",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Professional",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Enterprise",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\Community",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\Professional",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\Enterprise"
    )
    
    foreach ($path in $commonPaths) {
        if (Test-Path $path) {
            return $path
        }
    }
    
    return $null
}

# Find Visual Studio
$vsPath = Find-VisualStudio
if (-not $vsPath) {
    Write-Host "ERROR: Visual Studio 2022 not found!" -ForegroundColor Red
    Write-Host "Please install Visual Studio 2022 with C++ build tools." -ForegroundColor Yellow
    exit 1
}

Write-Host "Found Visual Studio at: $vsPath" -ForegroundColor Cyan

# Set up environment
$vcvarsPath = Join-Path $vsPath "VC\Auxiliary\Build\vcvars64.bat"
if (-not (Test-Path $vcvarsPath)) {
    Write-Host "ERROR: vcvars64.bat not found!" -ForegroundColor Red
    Write-Host "Please ensure C++ build tools are installed." -ForegroundColor Yellow
    exit 1
}

Write-Host "Setting up build environment..." -ForegroundColor Yellow
cmd /c """$vcvarsPath"" && set" | ForEach-Object {
    if ($_ -match "^(.*?)=(.*)$") {
        Set-Item -Path "env:$($matches[1])" -Value $matches[2]
    }
}

Write-Host "Building F.R.O.G. in release mode..." -ForegroundColor Yellow
Write-Host ""

# Build the project
cargo build --release

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "✅ Build successful!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Executable location:" -ForegroundColor Cyan
    Write-Host "  target\release\frog.exe" -ForegroundColor White
    Write-Host ""
    Write-Host "To run F.R.O.G.:" -ForegroundColor Cyan
    Write-Host "  .\target\release\frog.exe" -ForegroundColor White
} else {
    Write-Host ""
    Write-Host "❌ Build failed!" -ForegroundColor Red
    Write-Host "Check the error messages above for details." -ForegroundColor Yellow
    exit 1
} 