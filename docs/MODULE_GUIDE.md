# F.R.O.G. Module Guide

Detailed documentation for each crate in the F.R.O.G. workspace.

## dv-core

**Purpose**: Core abstractions and shared types for the entire system.

### Key Components

- **Navigation System** (`navigation/`)
  - `NavigationPosition`: Enum representing current position (Sequential, Temporal, Categorical)
  - `Navigation`: Main navigation controller with history and bounds
  - `NavigationSubscriber`: Trait for components that react to navigation changes

- **State Management** (`state/`)
  - `ViewerContext`: Central state container with data sources, navigation, and time control
  - `SelectionState`: Tracks selected items across views
  - `TimeControl`: Playback speed, looping, play/pause state

- **Event System** (`events/`)
  - Event bus for decoupled communication
  - Common event types (DataLoaded, ViewportChanged, etc.)

### Usage Example

```rust
use dv_core::{Navigation, NavigationPosition};

let mut nav = Navigation::new();
nav.set_bounds(0, 1000);
nav.seek_to(NavigationPosition::Sequential(500));
```

## dv-data

**Purpose**: Data source abstraction and implementation.

### Key Components

- **Source Trait** (`sources/mod.rs`)
  ```rust
  #[async_trait]
  pub trait DataSource: Send + Sync {
      async fn query_at(&self, position: &NavigationPosition) -> Result<RecordBatch>;
      async fn schema(&self) -> Arc<Schema>;
      async fn row_count(&self) -> Result<usize>;
  }
  ```

- **CSV Source** (`sources/csv_source.rs`)
  - Streaming parser for large files
  - Automatic type inference
  - Configurable parsing options

- **SQLite Source** (`sources/sqlite_source.rs`)
  - Query any table or view
  - Prepared statement caching
  - Type mapping from SQL to Arrow

- **Cache Layer** (`cache/`)
  - LRU cache for query results
  - Configurable size limits
  - Automatic invalidation

### Configuration

```rust
use dv_data::config::{FileConfig, CsvConfig};

let config = FileConfig::Csv(CsvConfig {
    has_headers: true,
    delimiter: b',',
    null_values: vec!["NA".to_string()],
});
```

## dv-views

**Purpose**: All visualization implementations.

### Key Components

- **SpaceView Trait** (`space_view.rs`)
  ```rust
  pub trait SpaceView: Any + Send + Sync {
      fn id(&self) -> SpaceViewId;
      fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui);
      fn on_selection_change(&mut self, ctx: &ViewerContext, selection: &SelectionState);
  }
  ```

- **Plot Types** (`plots/`)
  - `LineChart`: Time series and continuous data
  - `ScatterPlot`: 2D/3D point clouds with color mapping
  - `BarChart`: Categorical comparisons
  - `Histogram`: Distribution analysis
  - `HeatMap`: 2D density visualization
  - And many more...

- **Statistical Views** (`stats/`)
  - Summary statistics panel
  - Correlation matrices
  - Distribution analysis

- **Table View** (`tables/`)
  - Sortable columns
  - Filterable rows
  - Cell formatting

### Creating a Custom View

```rust
pub struct MyView {
    id: SpaceViewId,
    config: MyConfig,
    cached_data: Option<RecordBatch>,
}

impl SpaceView for MyView {
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Render your visualization
        ui.label("My Custom View");
        
        if let Some(data) = &self.cached_data {
            // Draw visualization
        }
    }
}
```

## dv-ui

**Purpose**: Reusable UI components and widgets.

### Key Components

- **Panels** (`panels/`)
  - `NavigationPanel`: Timeline and playback controls
  - `PropertyPanel`: View configuration UI
  - `DataPanel`: Data source selection

- **Widgets** (`widgets/`)
  - Custom egui widgets
  - Color pickers
  - Range sliders
  - Multi-select lists

- **Layout System** (`layout/`)
  - Grid-based layout manager
  - Docking support (via egui_dock)
  - Responsive design helpers

- **Theme** (`theme/`)
  - Consistent color schemes
  - Typography settings
  - Spacing constants

### Widget Example

```rust
use dv_ui::widgets::color_picker;

let mut color = Color32::BLUE;
if color_picker(ui, &mut color).changed() {
    // Handle color change
}
```

## dv-app

**Purpose**: Main application binary that orchestrates all components.

### Key Components

- **Application State** (`main.rs`)
  - `FrogApp`: Main application struct
  - Window management
  - Event loop integration

- **View Builder** (`view_builder.rs`)
  - Visual dashboard designer
  - Template system
  - Grid layout editor

- **Demo Mode** (`demo.rs`)
  - Sample data generation
  - Preset layouts
  - Interactive tutorials

- **File Handling** (`file_config_dialog.rs`)
  - Multi-file selection
  - Configuration UI
  - Format detection

### Application Flow

1. User opens file → `FileConfigDialog`
2. Configuration complete → Create `DataSource`
3. Open dashboard builder → `ViewBuilderDialog`
4. Select template/layout → Create `SpaceView` instances
5. Navigation changes → All views update

## dv-render

**Purpose**: Rendering abstraction for future GPU acceleration.

### Current State

- Placeholder for WGPU integration
- Will provide GPU-accelerated rendering for large datasets
- Custom shaders for specialized visualizations

### Future API

```rust
pub trait GpuRenderer {
    fn prepare(&mut self, device: &wgpu::Device);
    fn render(&mut self, pass: &mut wgpu::RenderPass);
}
```

## dv-templates

**Purpose**: Pre-built dashboard layouts and smart template selection.

### Key Components

- **Template Registry** (`lib.rs`)
  - Catalog of available templates
  - Metadata for each template

- **Auto-Selection** (`auto_select.rs`)
  - Analyzes data schema
  - Suggests appropriate visualizations
  - Creates initial layout

- **Template Definitions**
  - Time series dashboard
  - Correlation analysis
  - Statistical overview
  - Custom layouts

### Template Example

```rust
pub fn time_series_template(schema: &Schema) -> DashboardLayout {
    DashboardLayout {
        grid: Grid::new(2, 2),
        cells: vec![
            Cell::new(0, 0, 2, 1, ViewConfig::TimeSeries { ... }),
            Cell::new(0, 1, 1, 1, ViewConfig::Stats { ... }),
            Cell::new(1, 1, 1, 1, ViewConfig::Table { ... }),
        ],
    }
}
```

## Inter-Crate Communication

### Data Flow
```
dv-app → dv-data → Arrow RecordBatch → dv-views → egui → Screen
```

### Event Flow
```
User Input → dv-app → dv-core Events → All Components
```

### State Sharing
```
ViewerContext (Arc<RwLock>) → Shared across all views
```

## Best Practices

1. **Keep crates focused**: Each crate should have a single, clear purpose
2. **Minimize dependencies**: Avoid circular dependencies between crates
3. **Use traits for abstraction**: Define interfaces in lower-level crates
4. **Document public APIs**: All `pub` items need documentation
5. **Test in isolation**: Each crate should have its own test suite

## Adding New Features

### New Visualization Type
1. Add to `dv-views/src/plots/`
2. Implement `SpaceView` trait
3. Register in `ViewConfig` enum
4. Add to dashboard builder

### New Data Format
1. Add to `dv-data/src/sources/`
2. Implement `DataSource` trait
3. Add configuration type
4. Update file dialogs

### New UI Component
1. Add to `dv-ui/src/widgets/`
2. Follow egui patterns
3. Use theme constants
4. Add examples/tests 