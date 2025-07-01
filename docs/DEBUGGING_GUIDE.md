# Debugging Guide for Data Visualization Platform

This guide helps developers troubleshoot common issues when developing new visualizations or fixing bugs in the platform.

## Common Compilation Errors

### 1. Type Mismatches (f32 vs f64)
**Problem**: Rust is strict about numeric types. Many graphics libraries use f32 while data processing often uses f64.

**Solution**:
```rust
// Convert f64 to f32
let value_f32 = value_f64 as f32;

// Use type annotations
let points: Vec<[f64; 2]> = data.iter()
    .map(|&v| [v.x as f64, v.y as f64])
    .collect();
```

### 2. Arrow Array Value Access
**Problem**: Arrow arrays don't return Option<T> from value() method in newer versions.

**Wrong**:
```rust
if let Some(val) = array.value(i) {
    // ...
}
```

**Correct**:
```rust
let val = array.value(i);
```

### 3. Missing Imports
**Problem**: Array trait not imported for StringArray.len() method.

**Solution**:
```rust
use arrow::array::{Array, Float64Array, StringArray};
```

### 4. Deprecated APIs  
**Problem**: egui APIs have been updated.

**Wrong**:
```rust
Rounding::none()
Stroke::none()
```

**Correct**:
```rust
Rounding::ZERO
Stroke::NONE
```

### 5. Response Move Issues
**Problem**: `response.on_hover_text()` consumes the Response object.

**Wrong**:
```rust
response.on_hover_text(tooltip);
response.context_menu(|ui| { ... });  // ERROR: response moved
```

**Correct**:
```rust
response.clone().on_hover_text(tooltip);
response.context_menu(|ui| { ... });  // OK: original response still available
```

### 6. Batch Clone Issues
**Problem**: RecordBatch being moved when stored then borrowed.

**Wrong**:
```rust
self.cached_data = Some(batch);
self.extract_data(&batch);  // ERROR: batch moved
```

**Correct**:
```rust
self.cached_data = Some(batch.clone());
self.extract_data(&batch);  // OK: batch cloned before move
```

### 7. PlotPoint vs PlotPoints
**Problem**: Text::new() expects PlotPoint, not PlotPoints.

**Wrong**:
```rust
plot_ui.text(Text::new(PlotPoints::new(vec![[x, y]]), "label"))
```

**Correct**:
```rust
plot_ui.text(Text::new([x, y].into(), "label"))
// or
plot_ui.text(Text::new([x, y], "label"))
```

### 8. LineStyle Variants
**Problem**: LineStyle variants need struct initialization.

**Wrong**:
```rust
.style(egui_plot::LineStyle::Dashed)
```

**Correct**:
```rust
.style(egui_plot::LineStyle::Dashed { length: 10.0 })
```

### 9. Numeric Type Ambiguity
**Problem**: Rust can't infer numeric types in some expressions.

**Wrong**:
```rust
density += (-0.5 * u * u).exp() / (2.5066282746310002 * bandwidth);
```

**Correct**:
```rust
density += (-0.5_f64 * u * u).exp() / (2.5066282746310002_f64 * bandwidth);
```

### 10. Borrow Checker Conflicts
**Problem**: Multiple mutable borrows or immutable/mutable conflicts.

**Solution Pattern**:
```rust
// Wrong - borrowing self.data while mutating self
if let Some(data) = &self.data {
    self.process_data(data);  // ERROR: cannot borrow self as mutable
}

// Correct - extract needed data first
let data_clone = self.data.clone();
if let Some(data) = data_clone {
    self.process_data(&data);  // OK: no conflicting borrows
}
```

## Quick Fix Examples

### Fix Response Moves (8 files)
Replace all instances of:
```rust
response.on_hover_text(tooltip);
```
With:
```rust
response.clone().on_hover_text(tooltip);
```

### Fix Array Access (6 files)
Replace patterns like:
```rust
if let Some(val) = array.value(i) {
    // use val
}
```
With:
```rust
let val = array.value(i);
// use val directly
```

### Fix Batch Clones (4 files)
Replace:
```rust
self.cached_data = Some(batch);
self.extract_data(&batch);
```
With:
```rust
self.cached_data = Some(batch.clone());
self.extract_data(&batch);
```

## Testing Strategy

1. **Incremental Compilation**: Fix errors in small batches
2. **Unit Tests**: Test individual plot components
3. **Integration Tests**: Test complete visualization pipelines
4. **Performance Tests**: Ensure visualizations perform well with large datasets

## Performance Considerations

1. **Avoid Cloning Large Data**: Only clone when necessary for borrow checker
2. **Use References**: Pass references instead of owned values when possible
3. **Cache Calculations**: Store expensive computations in struct fields
4. **Lazy Evaluation**: Only compute visualizations when needed

## Common Patterns

### Data Extraction Pattern
```rust
fn extract_data(&mut self, batch: &RecordBatch) {
    // Clear previous data
    self.data.clear();
    
    // Extract columns safely
    for i in 0..batch.num_rows() {
        if let Some(col) = batch.column_by_name("column_name") {
            if let Some(array) = col.as_any().downcast_ref::<Float64Array>() {
                let value = array.value(i);
                self.data.push(value);
            }
        }
    }
}
```

### Response Handling Pattern
```rust
fn handle_interaction(&mut self, ui: &mut Ui, rect: Rect) -> Response {
    let response = ui.allocate_rect(rect, Sense::click_and_drag());
    
    // Handle hover (clone first to avoid move)
    if let Some(hover_pos) = response.hover_pos() {
        if let Some(tooltip) = self.get_tooltip_at(hover_pos) {
            response.clone().on_hover_text(tooltip);
        }
    }
    
    // Handle context menu (original response still available)
    response.context_menu(|ui| {
        // menu items
    })
}
```

## Plot Development Workflow

### 1. Start with a Simple Implementation
```rust
pub struct MyPlot {
    id: SpaceViewId,
    title: String,
    pub config: MyPlotConfig,
}

impl MyPlot {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: MyPlotConfig::default(),
        }
    }
}
```

### 2. Implement SpaceView Trait
Required methods:
- `id()` - Unique identifier
- `display_name()` - Human-readable name
- `view_type()` - Type identifier
- `ui()` - Main rendering method
- `save_config()` - Serialize configuration
- `load_config()` - Deserialize configuration

### 3. Extract and Validate Data
```rust
fn extract_data(&self, batch: &RecordBatch) -> Option<MyData> {
    // Find columns
    let col_idx = batch.schema().fields().iter()
        .position(|f| f.name() == &self.config.column)?;
    
    // Get array
    let array = batch.column(col_idx)
        .as_any()
        .downcast_ref::<Float64Array>()?;
    
    // Extract values
    let values: Vec<f64> = (0..array.len())
        .map(|i| array.value(i))
        .collect();
    
    Some(MyData { values })
}
```

### 4. Handle Edge Cases
Always check for:
- Empty data
- Missing columns
- Invalid values (NaN, Inf)
- Out of bounds indices

## Debugging Tools

### Cargo Check Output
```powershell
# Check specific package
cargo check --package dv-views 2>&1

# Filter errors only
cargo check --package dv-views 2>&1 | Select-String "error"

# With context
cargo check --package dv-views 2>&1 | Select-String "error" -Context 3
```

### Common Patterns

#### Statistical Calculations
```rust
// Calculate mean without external crates
let mean = values.iter().sum::<f64>() / values.len() as f64;

// Calculate standard deviation
let variance = values.iter()
    .map(|v| (v - mean).powi(2))
    .sum::<f64>() / values.len() as f64;
let std_dev = variance.sqrt();
```

#### Color Schemes
```rust
use crate::plots::utils::{categorical_color, viridis_color, plasma_color};

// Categorical colors
let color = categorical_color(index);

// Continuous color mapping
let normalized = (value - min) / (max - min);
let color = viridis_color(normalized as f32);
```

## Performance Debugging

### Profiling
```rust
// Simple timing
let start = std::time::Instant::now();
// ... operation ...
println!("Operation took: {:?}", start.elapsed());
```

### Memory Usage
- Use `Arc` for shared data
- Clone only when necessary
- Use iterators instead of collecting when possible

### Rendering Performance
- Limit points rendered (sampling/aggregation)
- Use simpler rendering for large datasets
- Cache computed values

## Common Plot Issues

### 1. Plot Not Showing
- Check if data is being extracted correctly
- Verify column names match
- Ensure plot area has size

### 2. Incorrect Scaling
- Check data ranges
- Verify axis bounds
- Consider log scales for wide ranges

### 3. Missing Interactivity
- Ensure proper event handling
- Check if response is being used
- Verify interaction is enabled in config

## Testing Strategies

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_data_extraction() {
        // Create test data
        // Verify extraction
    }
}
```

### Integration Tests
- Test with real CSV files
- Verify plot renders without panicking
- Check configuration save/load

### Visual Tests
- Render to image for comparison
- Check specific pixel values
- Verify layout calculations

## Troubleshooting Checklist

1. **Compilation Errors**
   - [ ] All imports added?
   - [ ] Type conversions correct?
   - [ ] Brace matching?
   - [ ] Lifetime annotations needed?

2. **Runtime Errors**
   - [ ] Null checks in place?
   - [ ] Array bounds checked?
   - [ ] Division by zero handled?
   - [ ] NaN/Inf values handled?

3. **Visual Issues**
   - [ ] Coordinate transformations correct?
   - [ ] Color values in range?
   - [ ] Layout calculations correct?
   - [ ] Z-order/layering correct?

4. **Performance Issues**
   - [ ] Data structures optimal?
   - [ ] Unnecessary cloning?
   - [ ] Calculations cached?
   - [ ] Rendering optimized?

## Getting Help

1. **Check existing plots**: Look at similar implementations
2. **Read egui docs**: https://docs.rs/egui
3. **Review Arrow docs**: https://docs.rs/arrow
4. **Use Rust compiler**: It often suggests fixes
5. **Add debug prints**: `dbg!()` macro is your friend

## Best Practices Summary

1. **Start simple**: Get basic version working first
2. **Handle errors gracefully**: Use `Option` and `Result`
3. **Document assumptions**: Add comments for complex logic
4. **Test edge cases**: Empty data, single point, large datasets
5. **Profile before optimizing**: Measure, don't guess
6. **Keep consistent style**: Follow project conventions
7. **Reuse utilities**: Check `utils` module first 