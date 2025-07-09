# Changelog

All notable changes to F.R.O.G. Data Visualizer will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial beta release of F.R.O.G. Data Visualizer
- Core visualization types: time series, scatter plots, bar charts, tables
- Dashboard Builder with visual grid editor and templates
- Support for CSV files and SQLite databases
- Navigation system with sequential, temporal, and categorical modes
- Interactive legends and cross-view highlighting
- Keyboard shortcuts for power users
- Real-time data synchronization across views
- Summary statistics panel
- Demo mode with sample datasets
- Frog mascot animation 🐸

### Fixed
- Animation speeds now consistent between debug and release builds
- Windows MSVC linker issues documented with workarounds

### Known Issues
- Release builds on Windows require proper MSVC toolchain setup
- Legend colors may appear white with duplicate series names
- Some egui_plot limitations affect advanced plotting features

## [0.1.0-alpha] - 2024-01-01

### Added
- Initial project structure with modular crate architecture
- Basic egui integration
- Prototype data loading from CSV files

---

*Note: This project is in active development. Version numbers are provisional until first stable release.* 