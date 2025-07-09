# F.R.O.G. Developer Guide

## Development Setup

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Git
- Platform-specific requirements:
  - **Windows**: Visual Studio Build Tools with C++ workload
  - **macOS**: Xcode Command Line Tools
  - **Linux**: gcc, pkg-config, development libraries

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/frog-viz.git
cd frog-viz

# Build the project
cargo build

# Run tests
cargo test

# Run the application
cargo run
```

### Development Tools

```bash
# Install helpful development tools
cargo install cargo-watch    # Auto-rebuild on changes
cargo install cargo-expand   # Macro expansion
cargo install cargo-tree     # Dependency visualization
cargo install cargo-audit    # Security audit
```

## Project Structure

```
frog-viz/
├── crates/              # Modular crate workspace
│   ├── dv-app/         # Main binary
│   ├── dv-core/        # Core abstractions
│   ├── dv-data/        # Data sources
│   ├── dv-views/       # Visualizations
│   ├── dv-ui/          # UI components
│   ├── dv-render/      # Rendering layer
│   └── dv-templates/   # Dashboard templates
├── data/               # Sample data files
├── docs/               # Technical documentation
└── target/             # Build artifacts
```

## Development Workflow

### Running in Development Mode

```bash
# Auto-rebuild and run on file changes
cargo watch -x run

# Run with debug logging
RUST_LOG=debug cargo run

# Run with specific features
cargo run --features gpu-rendering
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check for common mistakes
cargo check

# Run tests with coverage
cargo tarpaulin
```

### Building for Release

```bash
# Standard release build
cargo build --release

# Windows-specific (see WINDOWS_BUILD_GUIDE.md)
./scripts/build-release.ps1
```

## Common Development Tasks

### Adding a New Visualization

1. Create new module in `crates/dv-views/src/plots/`
2. Implement the `SpaceView` trait:

```rust
pub struct MyCustomView {
    id: SpaceViewId,
    config: MyConfig,
    cached_data: Option<RecordBatch>,
}

impl SpaceView for MyCustomView {
    fn id(&self) -> SpaceViewId { self.id }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Render your visualization
    }
    
    // ... other required methods
}
```

3. Register in `plots/mod.rs`
4. Add to `ViewConfig` enum in `dv-app`

### Adding a New Data Source

1. Create module in `crates/dv-data/src/sources/`
2. Implement `DataSource` trait:

```rust
#[async_trait]
impl DataSource for MySource {
    async fn query_at(&self, position: &NavigationPosition) -> Result<RecordBatch> {
        // Load and return data
    }
    
    async fn schema(&self) -> Arc<Schema> {
        // Return Arrow schema
    }
}
```

### Working with egui

Common patterns in this codebase:

```rust
// Custom widget with state
ui.horizontal(|ui| {
    ui.label("Setting:");
    if ui.button("Click").clicked() {
        self.state.toggle();
    }
});

// Responsive layouts
let available = ui.available_rect_before_wrap();
if available.width() > 600.0 {
    ui.columns(2, |columns| {
        columns[0].label("Left");
        columns[1].label("Right");
    });
}
```

## Debugging Tips

### Enable Debug Logging

```rust
// In your code
tracing::debug!("Navigation position: {:?}", position);

// Run with
RUST_LOG=dv_core=debug,dv_views=debug cargo run
```

### Performance Profiling

```rust
// Use puffin for frame profiling
puffin::profile_scope!("expensive_operation");

// Enable in main.rs
puffin::set_scopes_on(true);
```

### Common Issues

1. **Borrow checker fights with egui**
   - Clone data before UI callbacks
   - Use `Arc<RwLock<T>>` for shared state

2. **Async in sync context**
   - Use `runtime.block_on()` carefully
   - Prefer spawning tasks

3. **Large data performance**
   - Implement pagination
   - Use data windows/sampling

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_navigation() {
        let nav = Navigation::new();
        assert_eq!(nav.position(), 0);
    }
}
```

### Integration Tests

Create files in `tests/` directory:

```rust
// tests/data_loading.rs
#[test]
fn test_csv_loading() {
    let source = CsvSource::new("data/test.csv");
    let batch = source.load().await.unwrap();
    assert_eq!(batch.num_rows(), 100);
}
```

## Code Style Guidelines

1. **Use descriptive names**: `calculate_viewport_bounds` not `calc_vp_bnds`
2. **Document public APIs**: All `pub` items need doc comments
3. **Handle errors explicitly**: Avoid `.unwrap()` in production code
4. **Keep functions focused**: Single responsibility principle
5. **Use type aliases**: For complex generic types

## Performance Guidelines

1. **Avoid cloning large data**: Use `Arc` for sharing
2. **Lazy evaluation**: Don't compute until needed
3. **Cache expensive operations**: Store results in view state
4. **Profile before optimizing**: Use benchmarks to guide decisions

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Create git tag: `git tag -a v0.1.0 -m "Release version 0.1.0"`
5. Build release artifacts
6. Create GitHub release

## Getting Help

- Check existing code for patterns
- Read egui documentation and examples
- Use `cargo doc --open` for API docs
- Search issues for similar problems

## Architecture Decisions

See [ARCHITECTURE.md](ARCHITECTURE.md) for system design.

Key principles:
- **Modularity**: Crates should be independent
- **Testability**: Dependency injection over globals
- **Performance**: Measure, don't guess
- **Simplicity**: Clear code over clever code 