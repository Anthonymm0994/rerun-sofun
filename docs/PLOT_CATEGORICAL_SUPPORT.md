# Plot Types - Categorical Column Support

This document summarizes which plot types support categorical columns for grouping, coloring, or categorization, and how they use them.

## Plots with Full Categorical Support ✅

### 1. **Polar Plot**
- **Category Column**: Optional
- **Usage**: Groups points by category with different colors
- **Legend**: Yes - shows each category with its color
- **Features**: 
  - Stable color assignment using BTreeMap
  - Interactive legend (click to show/hide categories)
  - Smart angle/radius column detection

### 2. **Scatter Plot** 
- **Color Column**: Optional
- **Usage**: Colors points by category
- **Legend**: Yes - shows each category with its color
- **Features**:
  - Groups points by category for proper legend display
  - Stable color assignment
  - Maintains color on hover

### 3. **Box Plot**
- **Category Column**: Optional
- **Usage**: Creates separate box plots for each category
- **Legend**: Yes - shows each category
- **Features**:
  - Groups data by category
  - Shows statistics per category
  - Side-by-side comparison

### 4. **Violin Plot**
- **Category Column**: Optional
- **Usage**: Creates separate violin plots for each category
- **Legend**: Yes - shows each category
- **Features**:
  - Similar to box plot but shows distribution shape
  - Groups data by category

### 5. **Bar Chart**
- **Category Column**: Required
- **Usage**: X-axis categories
- **Legend**: No (categories shown on axis)
- **Features**:
  - Aggregates values by category (sum)
  - Sorted alphabetically

### 6. **Stream Graph**
- **Category Column**: Required
- **Usage**: Creates separate streams for each category
- **Legend**: Yes - shows each stream/category
- **Features**:
  - Stacked area visualization
  - Different baseline algorithms
  - Time-based flow visualization

### 7. **Heatmap**
- **X Column**: Often categorical
- **Y Column**: Often categorical
- **Usage**: Matrix visualization of categories
- **Legend**: Color scale legend
- **Features**:
  - Shows relationships between two categorical dimensions

### 8. **Radar Chart**
- **Group Column**: Optional
- **Usage**: Creates separate radar shapes for each group
- **Legend**: Yes - shows each group
- **Features**:
  - Compares multiple metrics across groups

### 9. **Sankey Diagram**
- **Source Column**: Categorical
- **Target Column**: Categorical
- **Usage**: Shows flow between categories
- **Legend**: Implicit in the diagram
- **Features**:
  - Visualizes relationships and flows

### 10. **Treemap**
- **Category Column**: Required
- **Usage**: Hierarchical categories
- **Legend**: Categories shown in rectangles
- **Features**:
  - Size represents values
  - Nested categories supported

### 11. **Sunburst**
- **Hierarchy Columns**: Multiple categorical
- **Usage**: Hierarchical categories in circular layout
- **Legend**: Categories shown in segments
- **Features**:
  - Multi-level categorization
  - Interactive drill-down

### 12. **Network Graph**
- **Source Column**: Categorical
- **Target Column**: Categorical
- **Usage**: Nodes represent categories
- **Legend**: Node labels
- **Features**:
  - Shows relationships between categories

## Plots with Limited/No Categorical Support ⚠️

### 1. **Line Plot**
- **Status**: No direct categorical support
- **Workaround**: Plot multiple Y columns with different colors
- **Potential Enhancement**: Could add grouping by category

### 2. **Histogram**
- **Status**: Single column only
- **Potential Enhancement**: Could add overlay by category

### 3. **Time Series**
- **Status**: Multiple Y columns but no grouping
- **Potential Enhancement**: Could add grouping by category

### 4. **Scatter 3D**
- **Status**: No color by category
- **Potential Enhancement**: Add color column support

### 5. **Surface 3D**
- **Status**: Continuous data only
- **Not Applicable**: Surface plots are for continuous data

### 6. **Contour Plot**
- **Status**: Continuous data only
- **Not Applicable**: Contour plots are for continuous data

### 7. **Parallel Coordinates**
- **Status**: Limited - can show categories but no grouping
- **Potential Enhancement**: Color lines by category

### 8. **Anomaly Detection**
- **Status**: Single series only
- **Potential Enhancement**: Could detect anomalies per category

### 9. **Correlation Matrix**
- **Status**: Numeric columns only
- **Not Applicable**: Correlation is for numeric data

### 10. **Distribution Plot**
- **Status**: Single column only
- **Potential Enhancement**: Could overlay distributions by category

### 11. **Time Analysis**
- **Status**: Multiple series but no categorical grouping
- **Potential Enhancement**: Could add grouping

### 12. **Geographic Plot**
- **Status**: Points only, no categorization
- **Potential Enhancement**: Color points by category

### 13. **Candlestick Chart**
- **Status**: Financial data format, no categories
- **Not Applicable**: Specific to OHLC data

## Best Practices

1. **Color Consistency**: Use stable color assignment (BTreeMap) so categories always get the same color
2. **Legend Support**: Always include legend when using categories
3. **Null Handling**: Handle null/missing categories gracefully
4. **Performance**: Consider limiting number of categories for readability
5. **Interactivity**: Support clicking legend to show/hide categories

## Implementation Pattern

For adding categorical support to a plot:

```rust
// 1. Add optional category column to config
pub struct PlotConfig {
    pub category_column: Option<String>,
    // ... other fields
}

// 2. Extract categories with stable color mapping
let mut category_map = BTreeMap::new();
for (i, cat) in unique_categories.iter().enumerate() {
    category_map.insert(cat.clone(), categorical_color(i));
}

// 3. Group data by category
let mut grouped_data: HashMap<String, Vec<DataPoint>> = HashMap::new();
// ... group logic

// 4. Plot each category separately for legend
for (category, &color) in &category_map {
    let points = /* get points for this category */;
    plot_ui.points(
        Points::new(points)
            .color(color)
            .name(category) // This adds to legend
    );
}
```

## Testing Checklist

When testing categorical support:
- [ ] Categories appear in legend
- [ ] Colors are consistent across refreshes
- [ ] Clicking legend shows/hides category
- [ ] Null/empty categories handled properly
- [ ] Performance acceptable with many categories
- [ ] Column configuration UI shows categorical options 