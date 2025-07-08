# Widget ID Best Practices

This document outlines best practices for managing widget IDs in egui to avoid "multiple widgets with same ID" issues.

## The Problem

In egui debug builds, creating multiple widgets with the same ID causes red debug overlays with messages like:
- "First use of ScrollArea ID 91EE"
- "Second use of ScrollArea ID 91EE"

This happens when:
1. Widgets are created in loops without unique IDs
2. Multiple instances of the same widget type use default IDs
3. Copy-pasted code reuses the same ID strings

## Solutions

### 1. Use the Widget ID Utilities

The `dv-ui` crate provides utilities in the `widget_utils` module:

```rust
use dv_ui::{WidgetId, ScrollAreaExt, widget_id};

// Using the builder pattern
ScrollArea::vertical()
    .id_builder(WidgetId::new("config").with("csv").index(idx))
    .show(ui, |ui| {
        // content
    });

// Using helper functions
ScrollArea::vertical()
    .id_source(widget_id("column_scroll", idx))
    .show(ui, |ui| {
        // content
    });
```

### 2. Widget-Specific Guidelines

#### ScrollArea
Always add an `id_source` when creating ScrollAreas:
```rust
ScrollArea::vertical()
    .id_source("unique_scroll_id")
    .max_height(200.0)
    .show(ui, |ui| {
        // content
    });
```

#### ComboBox
Use `from_id_source` with a unique identifier:
```rust
egui::ComboBox::from_id_source(format!("combo_{}", item_id))
    .selected_text(&current_value)
    .show_ui(ui, |ui| {
        // options
    });
```

#### Grid
Create grids with unique names:
```rust
Grid::new("my_unique_grid")
    .striped(true)
    .show(ui, |ui| {
        // content
    });
```

### 3. Common Patterns

#### In Loops
```rust
for (idx, item) in items.iter().enumerate() {
    // Add index to make ID unique
    ScrollArea::vertical()
        .id_source(format!("item_scroll_{}", idx))
        .show(ui, |ui| {
            // content
        });
}
```

#### In Reusable Components
```rust
impl MyComponent {
    fn show(&mut self, ui: &mut Ui, instance_id: &str) {
        ScrollArea::vertical()
            .id_source(format!("my_component_scroll_{}", instance_id))
            .show(ui, |ui| {
                // content
            });
    }
}
```

#### Nested Contexts
```rust
let base_id = "file_config";
let file_type = "csv";
let section = "columns";

ScrollArea::vertical()
    .id_source(format!("{}_{}_{}_scroll", base_id, file_type, section))
    .show(ui, |ui| {
        // content
    });
```

## Testing

To verify your widget IDs are unique:

1. Run in debug mode: `cargo run`
2. Look for red overlay boxes - these indicate ID conflicts
3. Check the console for ID conflict warnings
4. Use the egui debug UI (if enabled) to inspect widget IDs

## Quick Reference

| Widget Type | Method | Example |
|------------|--------|---------|
| ScrollArea | `id_source()` | `.id_source("my_scroll")` |
| ComboBox | `from_id_source()` | `ComboBox::from_id_source("my_combo")` |
| Grid | Constructor | `Grid::new("my_grid")` |
| Window | `id()` | `.id(Id::new("my_window"))` |
| CollapsingHeader | `id_source()` | `.id_source("my_header")` |

## Migration Guide

When fixing existing code:

1. Search for widget creation without IDs:
   - `ScrollArea::vertical().show`
   - `ComboBox::from_label`
   - `Grid::new("")`

2. Add unique IDs based on context:
   - File/component name
   - Section/purpose
   - Index (if in loop)

3. Test in debug mode to verify no conflicts remain

Remember: It's better to have overly specific IDs than to risk conflicts! 