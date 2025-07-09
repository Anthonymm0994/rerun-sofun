# SQLite Build Issues on Windows

## Problem
When building on Windows, you might see: `error: failed to run custom build command for libsqlite3-sys`

## Solution
This project already uses the `bundled` feature for rusqlite, which should prevent this issue. If you're still experiencing problems:

### 1. Clean Build
```bash
cargo clean
cargo build
```

### 2. Update Dependencies
```bash
cargo update
```

### 3. Check Environment
If using a custom SQLite installation, ensure these environment variables are NOT set:
- `SQLITE3_LIB_DIR`
- `SQLITE3_INCLUDE_DIR`
- `SQLITE3_NO_PKG_CONFIG`

### 4. Visual Studio Build Tools
If the bundled build still fails, install:
1. Download Visual Studio 2022 Community
2. Install "Desktop development with C++"
3. Include Windows SDK

### 5. Alternative: Use vcpkg
```bash
# Install vcpkg
git clone https://github.com/Microsoft/vcpkg.git
cd vcpkg
.\bootstrap-vcpkg.bat
.\vcpkg integrate install

# Install SQLite
.\vcpkg install sqlite3:x64-windows
```

## Current Configuration
The project uses bundled SQLite in these crates:
- `dv-data`: `features = ["bundled-full", "modern_sqlite"]`
- `dv-app`: `features = ["bundled"]`
- Root workspace: `features = ["bundled-full"]`

This configuration compiles SQLite from source, avoiding system dependencies. 