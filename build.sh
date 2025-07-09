#!/bin/bash
# F.R.O.G. Build Script

echo "F.R.O.G. Build Script"
echo "===================="
echo ""

# Detect OS
OS="unknown"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "win32" ]]; then
    OS="windows"
fi

echo "Detected OS: $OS"
echo ""

# Build based on arguments
if [ "$1" == "release" ]; then
    echo "Building in release mode..."
    cargo build --release
    
    if [ $? -eq 0 ]; then
        echo ""
        echo "✅ Build successful!"
        echo ""
        if [ "$OS" == "windows" ]; then
            echo "Executable: target/release/frog.exe"
        else
            echo "Executable: target/release/frog"
        fi
    else
        echo ""
        echo "❌ Build failed!"
        exit 1
    fi
else
    echo "Building in debug mode..."
    cargo build
    
    if [ $? -eq 0 ]; then
        echo ""
        echo "✅ Build successful!"
        echo ""
        if [ "$OS" == "windows" ]; then
            echo "Executable: target/debug/frog.exe"
        else
            echo "Executable: target/debug/frog"
        fi
    else
        echo ""
        echo "❌ Build failed!"
        exit 1
    fi
fi

echo ""
echo "To run F.R.O.G.:"
if [ "$1" == "release" ]; then
    echo "  cargo run --release"
else
    echo "  cargo run"
fi 