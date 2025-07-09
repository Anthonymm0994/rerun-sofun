# F.R.O.G. Project Information

## Project Name
**F.R.O.G.** - Fast, Responsive, Organized Graphics

## Repository Structure
```
frog-viz/                    # Main project directory
├── crates/                  # Rust workspace crates
│   ├── dv-app/             # Main application (binary: frog)
│   ├── dv-core/            # Core abstractions
│   ├── dv-data/            # Data sources
│   ├── dv-views/           # Visualizations
│   ├── dv-ui/              # UI components
│   ├── dv-render/          # Rendering (future GPU)
│   └── dv-templates/       # Dashboard templates
├── data/                    # Sample data files
├── docs/                    # Technical documentation
├── scripts/                 # Build and utility scripts
├── build.sh                 # Unix build script
├── run.sh                   # Unix run script
├── run.bat                  # Windows run script
└── target/                  # Build output
    └── release/
        └── frog[.exe]       # Main executable
```

## Binary Names
- **Executable**: `frog` (or `frog.exe` on Windows)
- **Package**: `dv-app` (internal crate name)
- **Display Name**: F.R.O.G. Data Visualizer

## Build Commands
```bash
# Debug build
cargo build
./target/debug/frog

# Release build
cargo build --release
./target/release/frog

# Run directly
cargo run
cargo run --release

# Windows release build with MSVC
./scripts/build-release.ps1
```

## Key Files
- `Cargo.toml` - Workspace configuration
- `README.md` - User-facing documentation
- `ARCHITECTURE.md` - System design
- `DEV_GUIDE.md` - Developer guide
- `RUST_PATTERNS.md` - Advanced Rust patterns guide
- `CHANGELOG.md` - Version history
- `LICENSE` - MIT license

## Naming Conventions
- **Project**: frog-viz (repository/folder name)
- **Binary**: frog (executable name)
- **Display**: F.R.O.G. (marketing name)
- **Crates**: dv-* (internal module prefix)

## Quick Start
```bash
# Clone and run
git clone https://github.com/yourusername/frog-viz.git
cd frog-viz
cargo run

# Or use the convenience scripts
./run.sh        # Unix/Mac
run.bat         # Windows
``` 