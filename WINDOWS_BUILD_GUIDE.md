# Windows Build Guide for rerun-sofun

## Common Windows Build Issues

You're experiencing classic Windows MSVC linker issues with unresolved symbols for C runtime functions (`log10`, `expf`, `memset`, `memmove`, etc.). This is a common problem when building Rust projects with C dependencies on Windows.

## Solutions (Try in Order)

### 1. Use the GNU Toolchain (Recommended for Quick Fix)

The GNU toolchain often works better for projects with complex C dependencies:

```bash
# Add GNU target
rustup target add x86_64-pc-windows-gnu

# Build with GNU target
cargo clean
cargo build --release --target x86_64-pc-windows-gnu
```

The executable will be in `target/x86_64-pc-windows-gnu/release/datavis.exe`

### 2. Install Visual Studio Build Tools Properly

If you prefer MSVC (smaller binaries, better Windows integration):

1. Download Visual Studio Installer
2. Install "Desktop development with C++" workload
3. Ensure these components are selected:
   - MSVC v143 - VS 2022 C++ x64/x86 build tools
   - Windows 10/11 SDK
   - C++ CMake tools for Windows

### 3. Use the PowerShell Build Script

Run the included build script which sets up the environment:

```powershell
.\scripts\build-release.ps1
```

### 4. Manual MSVC Build

If you have Visual Studio installed:

```cmd
# Open "x64 Native Tools Command Prompt for VS 2022"
cd C:\Users\antho\source\repos\rerun-sofun
cargo clean
cargo build --release
```

## Build Configuration

The project is configured for optimal release builds:

```toml
[profile.release]
opt-level = 3      # Maximum optimization
lto = false        # Disabled to avoid linker issues
codegen-units = 16 # Parallel compilation
strip = false      # Keep symbols
debug = true       # Include debug info for troubleshooting
```

## Troubleshooting

### If you still get linker errors:

1. **Check Rust toolchain:**
   ```bash
   rustup show
   # Should show: Default host: x86_64-pc-windows-msvc
   ```

2. **Check if MSVC is in PATH:**
   ```bash
   where cl
   where link
   ```

3. **Clear all caches:**
   ```bash
   cargo clean
   rd /s /q target
   del Cargo.lock
   ```

4. **Try with minimal features:**
   ```bash
   cargo build --release --no-default-features
   ```

## Expected Performance

The release build is optimized for:
- Fast data processing (100k+ rows)
- Smooth UI rendering
- Efficient memory usage
- Native CPU optimizations

## Running the Application

After successful build:

```bash
# MSVC build
.\target\release\frog.exe

# GNU build
.\target\x86_64-pc-windows-gnu\release\frog.exe
```

## Alternative: WSL2

If native Windows builds continue to fail, consider using WSL2:

```bash
# In WSL2 Ubuntu
cargo build --release
# Then run with X11 forwarding or WSLg
``` 