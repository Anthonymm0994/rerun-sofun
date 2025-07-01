# F.R.O.G. Data Visualizer 🐸

A **F**ast, **R**esponsive, **O**rganized **G**raphics data visualization platform built with Rust and egui.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Status](https://img.shields.io/badge/status-beta-yellow?style=for-the-badge)
![License](https://img.shields.io/badge/license-MIT-blue?style=for-the-badge)

## ✨ Features

### 🎯 Core Capabilities
- **Multiple Data Sources**: CSV files, SQLite databases, and more coming soon
- **Interactive Visualizations**: Time series, scatter plots, bar charts, tables, and statistical summaries
- **Smart Navigation**: Sequential, temporal, and categorical data exploration
- **Real-time Updates**: See changes across all views as you navigate
- **High Performance**: Built with Rust for speed and efficiency

### 🎨 Visualization Types
- 📈 **Time Series Plots**: Track metrics over time with multiple series
- 🎯 **Scatter Plots**: Explore correlations with optional color coding
- 📊 **Bar Charts**: Compare categorical data
- 📋 **Data Tables**: Inspect raw data with sorting and filtering
- 📊 **Summary Statistics**: Quick statistical insights

### 🚀 Key Features
- **Dashboard Builder**: Visual designer with flexible grid layouts
  - Pre-built templates for common use cases
  - Mixed layouts (e.g., 2 small views + 1 wide view)
  - Drag-and-drop column assignment
  - Real-time preview
- **Synchronized Views**: All visualizations stay in sync as you navigate
- **Interactive Legends**: Show/hide series with a click
- **Cross-View Highlighting**: Hover on one view, see related data highlighted everywhere
- **Keyboard Navigation**: Full keyboard support for power users
- **Dark Theme**: Easy on the eyes for extended analysis sessions

## 🚀 Quick Start

### Prerequisites
- [Rust](https://rustup.rs/) (1.70 or later)
- Windows, macOS, or Linux

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/rerun-sofun.git
cd rerun-sofun

# Build and run
cargo run --release
```

### First Run

1. **Load Data**: Press `Ctrl+O` or click "Open Data Source"
2. **Choose Source**: Select a CSV file or SQLite database
3. **Build Dashboard**: Press `B` to open the Dashboard Builder
4. **Select Template**: Choose from pre-built layouts or create custom
5. **Start Exploring**: Use navigation controls or keyboard shortcuts

## ⌨️ Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+O` | Open data source |
| `B` | Open Dashboard Builder |
| `Space` | Play/Pause navigation |
| `←/→` | Previous/Next item |
| `H` | Go to first item |
| `End` | Go to last item |
| `D` | Demo mode |
| `Tab` | Toggle side panels |
| `Esc` | Close dialogs |

## 🏗️ Architecture

F.R.O.G. uses a modular architecture for maintainability and extensibility:

```
crates/
├── dv-app/        # Main application
├── dv-core/       # Core types and navigation
├── dv-data/       # Data sources and caching  
├── dv-views/      # Visualization implementations
├── dv-ui/         # Reusable UI components
└── dv-render/     # Rendering abstractions
```

See [Design Principles](docs/DESIGN_PRINCIPLES.md) for detailed architecture documentation.

## 📊 Supported Data Formats

### CSV Files
- Automatic type detection
- Support for large files (streaming)
- Multiple encoding support

### SQLite Databases
- Query any table or view
- Automatic schema detection
- Efficient data loading

### Coming Soon
- JSON/JSONL files
- Parquet files
- REST API endpoints
- Real-time data streams

## 🎨 Dashboard Builder

The visual Dashboard Builder lets you create custom layouts:

### Templates
- **Time Series Dashboard**: Track metrics over time
- **Correlation Analysis**: Explore relationships
- **Mixed Layout**: Combine different view sizes
- **Vertical Split**: Side-by-side comparisons
- **Custom**: Start from scratch

### Features
- Visual grid editor
- Flexible cell sizes (1x1, 2x1, 1x2, etc.)
- Drag columns to views
- Live preview
- Save/load layouts (coming soon)

## 🔧 Configuration

Configuration files are stored in:
- Windows: `%APPDATA%/frog-viz/`
- macOS: `~/Library/Application Support/frog-viz/`
- Linux: `~/.config/frog-viz/`

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Install development dependencies
cargo install cargo-watch

# Run in development mode with auto-reload
cargo watch -x run

# Run tests
cargo test

# Check code quality
cargo clippy -- -D warnings
```

## 📈 Performance Tips

- For large datasets (>1M rows), use SQLite instead of CSV
- Enable release mode (`cargo run --release`) for best performance
- Adjust viewport bounds for scatter plots with many points
- Use sampling for initial exploration of huge datasets

## 🐛 Known Issues

- Windows: Release builds may fail with CRT linking errors
  - Workaround: Use `cargo build` without `--release` flag
- Legend colors may appear white when series names are duplicated
- Some egui_plot features are limited by the underlying library

## 📚 Resources

- [Design Principles](docs/DESIGN_PRINCIPLES.md) - Architecture and design decisions
- [User Guide](docs/USER_GUIDE.md) - Detailed usage instructions (coming soon)
- [API Documentation](https://docs.rs/frog-viz) - Code documentation (coming soon)

## 📝 License

This project is licensed under the MIT License - see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- [egui](https://github.com/emilk/egui) - Immediate mode GUI framework
- [Apache Arrow](https://arrow.apache.org/) - Columnar data format
- [Rerun](https://www.rerun.io/) - Inspiration for visualization concepts

---

Built with ❤️ by data visualization enthusiasts 