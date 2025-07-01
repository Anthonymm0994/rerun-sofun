# Plot Types Guide

This guide provides an overview of all available plot types in the data visualization platform, their use cases, and configuration options.

## Statistical Plots

### Box Plot
**Use Case**: Visualizing distribution and outliers in numerical data
- Shows quartiles, median, whiskers, and outliers
- Supports multiple series for comparison
- Customizable whisker calculation methods

### Violin Plot
**Use Case**: Showing probability density along with box plot statistics
- Combines box plot with kernel density estimation
- Better for understanding distribution shape
- Supports split violins for comparison

### Histogram
**Use Case**: Understanding frequency distribution of a single variable
- Multiple binning strategies (equal width, quantile, etc.)
- Overlay with density curves
- Stacked or grouped for multiple series

## Time Series Plots

### Line Plot
**Use Case**: Showing trends over time or continuous variables
- Multiple series with different styles
- Area fills for emphasis
- Interpolation options (linear, step, spline)

### Stream Graph
**Use Case**: Showing composition changes over time
- Stacked area chart with stream layout
- Emphasizes flow and relative changes
- Good for multiple time series

### Candlestick Chart
**Use Case**: Financial data visualization (OHLC)
- Shows open, high, low, close values
- Volume overlay support
- Technical indicators integration

### Time Analysis
**Use Case**: Advanced time series analysis
- Decomposition (trend, seasonal, residual)
- Anomaly detection over time
- Forecasting capabilities

## Correlation & Relationships

### Scatter Plot
**Use Case**: Showing relationships between two continuous variables
- Size and color encoding for additional dimensions
- Trend line fitting
- Density contours for large datasets

### 3D Scatter Plot
**Use Case**: Relationships between three continuous variables
- Interactive camera controls
- Depth-based coloring
- Point cloud visualization

### Correlation Matrix
**Use Case**: Pairwise relationships in multivariate data
- Pearson, Spearman, or Kendall correlation
- Heatmap visualization
- Significance testing

### Parallel Coordinates
**Use Case**: Visualizing high-dimensional data
- Interactive axis reordering
- Brushing and filtering
- Pattern detection across dimensions

## Distribution & Composition

### Bar Chart
**Use Case**: Comparing categories or discrete values
- Horizontal or vertical orientation
- Grouped or stacked variants
- Error bars support

### Treemap
**Use Case**: Hierarchical data with size relationships
- Squarified layout algorithm
- Color coding for categories
- Interactive drill-down

### Sunburst Chart
**Use Case**: Hierarchical data with radial layout
- Better for showing paths and relationships
- Interactive exploration
- Multi-level visualization

### Sankey Diagram
**Use Case**: Flow visualization between categories
- Shows magnitude of flows
- Multiple stages support
- Interactive highlighting

## Geographic Visualization

### Geographic Plot
**Use Case**: Spatial data visualization
- Point maps with customizable markers
- Heatmaps for density
- Choropleth for regions
- Multiple map projections

## Network & Graph Visualization

### Network Graph
**Use Case**: Visualizing relationships and connections
- Force-directed layout
- Node and edge customization
- Community detection
- Interactive exploration

## Specialized Plots

### Heatmap
**Use Case**: 2D data or matrix visualization
- Multiple color schemes
- Aggregation methods
- Annotations support

### Contour Plot
**Use Case**: Continuous 2D functions or density
- Filled or line contours
- Customizable levels
- 3D surface projection

### Surface 3D
**Use Case**: 3D continuous functions
- Mesh or surface rendering
- Color mapping for fourth dimension
- Interactive rotation and zoom

### Radar Chart
**Use Case**: Multivariate comparison
- Good for 3-12 variables
- Multiple series overlay
- Area or line representation

## Anomaly Detection

### Anomaly Plot
**Use Case**: Identifying outliers and unusual patterns
- Multiple detection methods:
  - Z-Score
  - Interquartile Range (IQR)
  - Moving Average
  - Isolation Forest
  - Local Outlier Factor (LOF)
  - DBSCAN clustering
- Visual highlighting of anomalies
- Confidence scoring

## Configuration Patterns

### Common Options
All plots support:
- **Title and labels**: Customizable text
- **Colors**: Theme-based or custom palettes
- **Interactivity**: Pan, zoom, selection
- **Export**: PNG, SVG, or data export

### Data Requirements

| Plot Type | Minimum Data | Optimal Data |
|-----------|--------------|--------------|
| Line | 1D numeric | Time + numeric |
| Scatter | 2D numeric | 2D numeric + categories |
| Bar | Categories + values | Sorted categories |
| Histogram | 1D numeric | Large sample size |
| Box Plot | 1D numeric | Multiple groups |
| Heatmap | 2D matrix | Regular grid |
| Network | Edges list | Nodes + edges + attributes |
| Geographic | Lat/lon pairs | Spatial + attributes |
| Time Series | Time + values | Regular intervals |

## Performance Guidelines

### Small Data (< 10K points)
- All plot types perform well
- Full interactivity enabled
- No optimization needed

### Medium Data (10K - 100K points)
- Consider aggregation for dense plots
- Use sampling for overviews
- Enable progressive rendering

### Large Data (> 100K points)
- Use specialized plots (histogram vs scatter)
- Enable GPU acceleration for 3D
- Implement level-of-detail
- Consider data indexing

## Choosing the Right Plot

### For Distributions
- **Single variable**: Histogram, Violin plot
- **Multiple groups**: Box plot, Violin plot
- **2D density**: Heatmap, Contour

### For Relationships
- **2 variables**: Scatter plot
- **3 variables**: 3D scatter, bubble chart
- **Many variables**: Parallel coordinates
- **Correlation**: Correlation matrix

### For Time Series
- **Single series**: Line plot
- **Multiple series**: Multi-line, Stream graph
- **Financial**: Candlestick
- **Anomalies**: Time analysis plot

### For Categories
- **Comparison**: Bar chart
- **Hierarchical**: Treemap, Sunburst
- **Flow**: Sankey diagram
- **Geographic**: Map visualizations

### For Networks
- **General**: Network graph
- **Hierarchical**: Tree layout
- **Flow**: Sankey diagram

## Integration Example

```rust
use dv_views::plots::{LinePlot, ScatterPlot, BoxPlot};
use polars::prelude::*;

// Load data
let df = DataFrame::read_csv("data.csv")?;

// Create line plot
let line = LinePlot::new()
    .x_column("date")
    .y_column("value")
    .title("Trend Over Time");

// Create scatter plot
let scatter = ScatterPlot::new()
    .x_column("feature1")
    .y_column("feature2")
    .color_column("category");

// Draw in UI
line.draw(ui, &line_config);
scatter.draw(ui, &scatter_config);
```

## Best Practices

1. **Match plot to data type**: Use appropriate visualizations for your data
2. **Consider audience**: Technical vs non-technical users
3. **Avoid overplotting**: Use aggregation or sampling for dense data
4. **Use color wisely**: Ensure accessibility and clarity
5. **Provide context**: Always include labels, titles, and legends
6. **Enable exploration**: Add interactivity for deeper insights

## Further Resources

- [Architecture Guide](./VISUALIZATION_ARCHITECTURE.md)
- [API Documentation](../target/doc)
- [Example Gallery](../examples)
- [Performance Tuning](./PERFORMANCE.md) 