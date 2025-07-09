# F.R.O.G. Architecture Overview

## System Design

F.R.O.G. (Fast, Responsive, Organized Graphics) is a modular data visualization platform built with Rust and egui. The architecture emphasizes performance, extensibility, and clean separation of concerns.

## Crate Structure

```
┌─────────────┐
│   dv-app    │  Main application binary
└──────┬──────┘
       │ depends on
┌──────┴──────┬──────────────┬──────────────┬──────────────┐
│  dv-views   │    dv-ui     │   dv-data    │   dv-core    │
├─────────────┼──────────────┼──────────────┼──────────────┤
│Visualizations│ UI Components│ Data Sources │Core Types    │
│• Line plots │ • Panels     │ • CSV reader │• Navigation  │
│• Bar charts │ • Controls   │ • SQLite     │• State mgmt  │
│• Tables     │ • Layout     │ • Caching    │• Events      │
└─────────────┴──────────────┴──────────────┴──────────────┘
       │                              │
┌──────┴──────┐            ┌─────────┴──────┐
│ dv-render   │            │ dv-templates   │
├─────────────┤            ├────────────────┤
│GPU Rendering│            │Layout Templates│
└─────────────┘            └────────────────┘
```

## Core Components

### dv-core
The foundation crate providing:
- **Navigation System**: Manages position in data (sequential, temporal, categorical)
- **Event System**: Pub/sub for inter-component communication
- **State Management**: Centralized application state
- **Type Definitions**: Common types used across crates

### dv-data
Data source abstraction layer:
- **Source Traits**: Common interface for all data sources
- **CSV Source**: Streaming CSV parser with type inference
- **SQLite Source**: Database queries with schema detection
- **Caching Layer**: LRU cache for query results
- **Schema Management**: Dynamic type detection and conversion

### dv-views
Visualization implementations:
- **SpaceView Trait**: Common interface for all visualizations
- **Plot Types**: Line, scatter, bar, histogram, etc.
- **Statistical Views**: Summary stats, correlations
- **Table View**: Sortable, filterable data grid
- **View State**: Per-view configuration and state

### dv-ui
Reusable UI components:
- **Panels**: Navigation, control, property panels
- **Widgets**: Custom egui widgets
- **Layout Manager**: Docking and grid systems
- **Theme System**: Consistent styling

### dv-app
Main application orchestrating all components:
- **Application State**: Global app state and lifecycle
- **View Management**: Creating and destroying views
- **Dashboard Builder**: Visual layout designer
- **File Handling**: Loading and configuration
- **Demo Mode**: Sample data and layouts

## Data Flow

```
User Input → Navigation System → Data Source Query → Cache Check
                                                         ↓
View Update ← Render ← Transform ← Arrow RecordBatch ←─┘
```

1. **User navigates** (keyboard, mouse, playback)
2. **Navigation system** updates position
3. **Data sources** query for current position
4. **Cache** returns existing or fetches new data
5. **Views** transform Arrow data to visual representation
6. **Renderer** draws to screen via egui

## Key Design Patterns

### 1. Trait-Based Abstraction
```rust
pub trait DataSource: Send + Sync {
    async fn query_at(&self, position: &NavigationPosition) -> Result<RecordBatch>;
    async fn schema(&self) -> Arc<Schema>;
}
```

### 2. Arc<RwLock<T>> for Shared State
```rust
pub struct ViewerContext {
    pub data_sources: Arc<RwLock<HashMap<String, Arc<dyn DataSource>>>>,
    pub navigation: Arc<Navigation>,
    pub time_control: Arc<RwLock<TimeControl>>,
}
```

### 3. Message Passing for Decoupling
```rust
pub enum ViewportEvent {
    ViewAdded(SpaceViewId),
    ViewRemoved(SpaceViewId),
    LayoutChanged,
}
```

### 4. Builder Pattern for Complex Objects
```rust
ViewBuilder::new()
    .with_title("My View")
    .with_data_source(source_id)
    .with_config(config)
    .build()
```

## Performance Considerations

1. **Async Data Loading**: Non-blocking I/O for responsive UI
2. **Arrow Format**: Zero-copy columnar data processing
3. **View Caching**: Avoid recomputing unchanged visualizations
4. **Lazy Evaluation**: Only process visible data ranges
5. **GPU Rendering**: Planned optimization for large datasets

## Extension Points

### Adding New Data Sources
1. Implement `DataSource` trait in `dv-data`
2. Add source type to `FileConfig` enum
3. Update file dialog filters in `dv-app`

### Adding New Visualizations
1. Implement `SpaceView` trait in `dv-views`
2. Add view type to `ViewConfig` enum
3. Update Dashboard Builder templates

### Custom UI Components
1. Create widget in `dv-ui`
2. Follow egui patterns for immediate mode
3. Use consistent theme tokens

## Threading Model

- **Main Thread**: UI rendering and event handling
- **Tokio Runtime**: Async data loading and processing
- **Background Tasks**: File watching, cache management

## Error Handling

- **Result<T, anyhow::Error>**: For recoverable errors
- **Logging**: `tracing` crate for diagnostics
- **User Feedback**: Toast notifications for errors

## Future Directions

1. **Plugin System**: Dynamic loading of visualizations
2. **GPU Compute**: WGPU integration for large datasets
3. **Collaborative Features**: Multi-user sessions
4. **Cloud Data Sources**: S3, databases, APIs
5. **Export System**: Static reports, animations 