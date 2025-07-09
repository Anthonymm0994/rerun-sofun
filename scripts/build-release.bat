@echo off
REM F.R.O.G. Release Build Script for Windows (Batch version)

echo F.R.O.G. Release Build Script
echo =============================
echo.

echo Setting up Visual Studio 2022 environment...
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" 2>nul
if errorlevel 1 (
    call "C:\Program Files (x86)\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" 2>nul
    if errorlevel 1 (
        echo ERROR: Could not find Visual Studio 2022!
        echo Please install Visual Studio 2022 with C++ build tools.
        exit /b 1
    )
)

echo.
echo Building F.R.O.G. in release mode...
cargo build --release

echo.
if exist target\release\frog.exe (
    echo SUCCESS: Build completed!
    echo Executable at: target\release\frog.exe
) else (
    echo ERROR: Build failed!
    exit /b 1
) 