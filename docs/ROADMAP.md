# F.R.O.G. Roadmap

## Project Vision

F.R.O.G. (Fast, Responsive, Organized Graphics) aims to be the most intuitive and performant data visualization tool for exploring CSV, SQLite, and other data formats. Inspired by Rerun's elegant interface, we're building a tool that makes data exploration as enjoyable as playing with a frog in a pond! 🐸

## Current Status (v0.1.0-beta)

### ✅ Core Features Complete

**Data Sources**
- CSV file loading with type inference
- SQLite database support with table selection
- Multiple file loading (combined CSV source)
- Automatic schema detection

**Visualizations**
- Time series plots with interactive legends
- Scatter plots (2D) with color mapping
- Bar charts for categorical data
- Data tables with sortable columns
- Summary statistics panel

**User Interface**
- Beautiful dark theme optimized for data analysis
- Dashboard Builder with visual grid editor
- Drag-and-drop column assignment
- Dockable, resizable panels
- Keyboard shortcuts throughout
- Animated F.R.O.G. mascot 🐸

**Navigation System**
- Sequential navigation (row by row)
- Temporal navigation (time-based)
- Categorical navigation (by groups)
- Playback controls with variable speed
- Cross-view synchronization

## 🚧 In Development

### Q1 2024 - Enhanced Visualizations
- [ ] Heatmaps with customizable color scales
- [ ] Box plots for distribution analysis
- [ ] Histograms with bin controls
- [ ] 3D scatter plots with rotation
- [ ] Violin plots for detailed distributions

### Q2 2024 - Advanced Features
- [ ] Data filtering and search
- [ ] Export functionality (PNG, SVG, CSV)
- [ ] Custom color schemes and themes
- [ ] Annotation tools
- [ ] Undo/redo support

### Q3 2024 - Performance & Scale
- [ ] GPU acceleration via WGPU
- [ ] Streaming for large files (>1GB)
- [ ] Smart data sampling
- [ ] Parallel data processing
- [ ] Memory-mapped file support

### Q4 2024 - Collaboration
- [ ] Save/load dashboard layouts
- [ ] Shareable view configurations
- [ ] Export interactive HTML reports
- [ ] Plugin system for custom visualizations

## 🎯 2025 Goals

### Data Sources
- **Cloud Storage**: S3, Azure Blob, GCS
- **Databases**: PostgreSQL, MySQL, MongoDB
- **APIs**: REST/GraphQL endpoints
- **Streaming**: Kafka, WebSocket support
- **File Formats**: Parquet, Arrow, Excel

### Advanced Analytics
- **Statistical Tests**: Built-in hypothesis testing
- **Machine Learning**: Basic model integration
- **Time Series**: Forecasting and decomposition
- **Geospatial**: Map visualizations
- **Network Analysis**: Graph algorithms

### Enterprise Features
- **Authentication**: User management
- **Permissions**: Role-based access
- **Audit Logs**: Track all operations
- **API Server**: Headless mode
- **Clustering**: Distributed processing

## 🏗️ Technical Debt

### High Priority
- [ ] Comprehensive test suite (target: 80% coverage)
- [ ] Performance benchmarks
- [ ] API documentation
- [ ] Error recovery improvements
- [ ] Memory usage optimization

### Medium Priority
- [ ] Refactor navigation system for extensibility
- [ ] Standardize view configuration format
- [ ] Improve async/sync boundary handling
- [ ] Add telemetry for usage analytics
- [ ] Accessibility improvements (WCAG 2.1)

### Low Priority
- [ ] Migrate to workspace dependencies
- [ ] Custom egui theme system
- [ ] Internationalization support
- [ ] Mobile/tablet responsive design
- [ ] Voice control integration

## 📊 Success Metrics

### Performance Targets
- Load 1M rows in < 1 second
- 60 FPS with 10+ active views
- < 500MB memory for typical datasets
- < 100ms response to user input

### User Experience Goals
- < 5 clicks to meaningful visualization
- Zero crashes in production use
- 90%+ user satisfaction score
- < 30 seconds to first insight

### Technical Goals
- 80% test coverage
- < 5% CPU usage when idle
- < 10MB binary size
- Cross-platform consistency

## 🌟 Dream Features

These are ambitious features we'd love to implement someday:

- **AI Assistant**: Natural language queries for data exploration
- **Collaborative Editing**: Real-time multi-user sessions
- **AR/VR Support**: Immersive data visualization
- **Voice Commands**: "Show me sales by region"
- **Predictive Analytics**: ML-powered insights
- **Data Lineage**: Track data transformations
- **Version Control**: Git-like history for analyses
- **Notebook Integration**: Jupyter/Observable compatibility

## 🤝 How to Help

While the project is in early development, we're focusing on:

1. **Testing**: Try the app with your data, report issues
2. **Feedback**: What visualizations do you need?
3. **Performance**: Help us optimize for large datasets
4. **Documentation**: Improve guides and examples

---

*"Make data exploration as fun as playing with a frog in a pond!"* 🐸

Last updated: January 2024 