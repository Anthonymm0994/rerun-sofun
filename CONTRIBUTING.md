# Contributing to F.R.O.G. Data Visualizer

Thank you for your interest in contributing to F.R.O.G.! This document provides guidelines and instructions for contributing to the project.

## 🎯 Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow
- Celebrate diverse perspectives

## 🚀 Getting Started

### Prerequisites

1. **Rust**: Install via [rustup](https://rustup.rs/)
2. **Git**: For version control
3. **IDE**: VS Code with rust-analyzer recommended

### Development Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/rerun-sofun.git
cd rerun-sofun

# Build the project
cargo build

# Run tests
cargo test

# Run with hot reload during development
cargo install cargo-watch
cargo watch -x run
```

## 📁 Project Structure

```
crates/
├── dv-app/        # Main application entry point
├── dv-core/       # Core abstractions and navigation
├── dv-data/       # Data sources (CSV, SQLite, etc.)
├── dv-views/      # Visualization implementations
├── dv-ui/         # Reusable UI components
└── dv-render/     # Rendering abstractions (future GPU)
```

## 🔧 Development Guidelines

### Code Style

1. **Format**: Always run `cargo fmt` before committing
2. **Linting**: Ensure `cargo clippy` passes with no warnings
3. **Naming**: Use descriptive names following Rust conventions
4. **Comments**: Document complex logic and public APIs

### Architecture Principles

1. **Modularity**: Keep crates focused and independent
2. **Performance**: Profile before optimizing
3. **Error Handling**: Use `Result` types, avoid `unwrap()` in production
4. **Testing**: Write tests for new functionality

### Adding a New View Type

1. Create a new module in `crates/dv-views/src/`
2. Implement the `SpaceView` trait
3. Add configuration struct with serde derives
4. Export from `lib.rs`
5. Add to view builder templates if appropriate

Example structure:
```rust
pub struct MyView {
    id: SpaceViewId,
    title: String,
    config: MyViewConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyViewConfig {
    // Configuration fields
}

impl SpaceView for MyView {
    // Implement required methods
}
```

### Adding a New Data Source

1. Create module in `crates/dv-data/src/sources/`
2. Implement the `DataSource` trait
3. Handle async loading appropriately
4. Add tests for edge cases

## 🧪 Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p dv-views

# Run with output
cargo test -- --nocapture
```

### Writing Tests

- Unit tests go in the same file as the code
- Integration tests go in `tests/` directory
- Use descriptive test names
- Test edge cases and error conditions

## 📝 Pull Request Process

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Commit** your changes (`git commit -m 'Add amazing feature'`)
4. **Push** to the branch (`git push origin feature/amazing-feature`)
5. **Open** a Pull Request

### PR Checklist

- [ ] Code follows style guidelines
- [ ] Tests pass locally
- [ ] Documentation updated if needed
- [ ] Commit messages are clear
- [ ] PR description explains the change

### Commit Message Format

```
type: brief description

Longer explanation if needed. Wrap at 72 characters.

Fixes #123
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

## 🐛 Reporting Issues

### Bug Reports

Include:
- OS and version
- Rust version
- Steps to reproduce
- Expected vs actual behavior
- Error messages/logs

### Feature Requests

Describe:
- The problem you're trying to solve
- Your proposed solution
- Alternative solutions considered
- Mock-ups or examples if applicable

## 📚 Documentation

- Update README.md for user-facing changes
- Update inline documentation for API changes
- Add examples for complex features
- Keep design docs current

## 🎨 UI/UX Guidelines

- Follow existing visual patterns
- Test with keyboard navigation
- Ensure color contrast for accessibility
- Keep interactions intuitive

## ⚡ Performance Guidelines

- Profile before optimizing
- Avoid premature optimization
- Document performance-critical code
- Consider memory usage for large datasets

## 🤝 Getting Help

- Check existing issues and discussions
- Ask in PR comments for code review
- Be specific about what you need help with

## 🙏 Recognition

Contributors will be:
- Listed in CONTRIBUTORS.md
- Thanked in release notes
- Part of the F.R.O.G. community!

Thank you for helping make F.R.O.G. better! 🐸 