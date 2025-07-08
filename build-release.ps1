# Build script for F.R.O.G. - Runs cargo build in background to prevent freezing
# Usage: .\build-release.ps1

Write-Host "Starting F.R.O.G. Release Build..." -ForegroundColor Green

# Check if cargo is available
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Error: Cargo not found. Please install Rust from https://rustup.rs/" -ForegroundColor Red
    exit 1
}

# Clean previous builds
Write-Host "Cleaning previous builds..." -ForegroundColor Yellow
cargo clean

# Create build job that runs in background
$buildJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    $env:RUST_BACKTRACE = "1"
    
    # Build with optimizations suitable for Windows
    cargo build --release 2>&1
}

# Show progress while building
$spinChars = @('⠋','⠙','⠹','⠸','⠼','⠴','⠦','⠧','⠇','⠏')
$spinIndex = 0
$startTime = Get-Date

while ($buildJob.State -eq 'Running') {
    $elapsed = ((Get-Date) - $startTime).ToString("mm\:ss")
    Write-Host -NoNewline "`r$($spinChars[$spinIndex]) Building... [$elapsed]" -ForegroundColor Cyan
    $spinIndex = ($spinIndex + 1) % $spinChars.Count
    Start-Sleep -Milliseconds 100
}

# Get build output
$output = Receive-Job -Job $buildJob
Remove-Job -Job $buildJob

# Check if build succeeded
if ($LASTEXITCODE -eq 0) {
    Write-Host "`n✓ Build completed successfully!" -ForegroundColor Green
    
    # Show binary location
    $exePath = Join-Path $PWD "target\release\datavis.exe"
    if (Test-Path $exePath) {
        $size = (Get-Item $exePath).Length / 1MB
        Write-Host "Binary location: $exePath" -ForegroundColor Cyan
        Write-Host "Binary size: $([math]::Round($size, 2)) MB" -ForegroundColor Cyan
        
        # Ask if user wants to run it
        $response = Read-Host "`nRun the application now? (Y/N)"
        if ($response -eq 'Y' -or $response -eq 'y') {
            Start-Process $exePath
        }
    }
} else {
    Write-Host "`n✗ Build failed!" -ForegroundColor Red
    Write-Host "`nBuild output:" -ForegroundColor Yellow
    Write-Host $output
}

Write-Host ""
Write-Host "Done!" -ForegroundColor Green