# Rerun Code Integration Plan

## Key Rerun Modules to Adapt

### 1. **Viewport & Docking System** (Priority: HIGH)
From Rerun:
- `re_viewport/src/viewport.rs` - Main viewport container
- `re_viewport/src/viewport_blueprint.rs` - Layout persistence
- `re_ui/src/egui_dock.rs` - Custom dock integration

What to copy:
- Their `TabViewer` implementation with proper drag/drop
- Layout serialization/deserialization
- Dock styling and behavior customization

### 2. **Time Panel → Universal Navigation Panel** (Priority: HIGH)
From Rerun:
- `re_time_panel/src/lib.rs` - Complete time panel
- `re_time_panel/src/time_selection_ui.rs` - Timeline scrubber
- `re_viewer_context/src/time_control.rs` - Playback controls

Adapt for:
- Row-based navigation (CSV files)
- Category navigation (discrete values)
- Time navigation (when timestamp column exists)

### 3. **Selection & Hover System** (Priority: MEDIUM)
From Rerun:
- `re_viewer_context/src/selection_state.rs` - Multi-view selection sync
- `re_selection_panel/src/selection_panel.rs` - Selection details

Benefits:
- Crosshair synchronization across plots
- Linked axes and zoom
- Selection details panel

### 4. **UI Components** (Priority: MEDIUM)
From Rerun:
- `re_ui/src/list_item.rs` - Consistent list items
- `re_ui/src/design_tokens.rs` - Exact theme values
- `re_ui/src/collapsing_header.rs` - Better panels

### 5. **Query Cache** (Priority: LOW)
From Rerun:
- `re_query_cache/src/cache.rs` - Caching layer
- `re_query/src/range.rs` - Range queries

For performance with large CSVs.

## Implementation Strategy

### Phase 1: Core UI (What we need NOW)
1. Copy Rerun's `TabViewer` implementation wholesale
2. Adapt their dock styling exactly
3. Use their time panel as template for navigation

### Phase 2: Interaction
1. Port selection synchronization
2. Add crosshair hover across plots
3. Implement zoom linking

### Phase 3: Performance
1. Add query caching
2. Implement data decimation
3. GPU acceleration where Rerun uses it

## Specific Files to Study

For draggable panels fix:
```rust
// From re_viewport/src/viewport.rs
impl TabViewer for ViewportTabViewer {
    // Their drag/drop implementation
    fn on_drag_update(&mut self, ctx: &Context) { ... }
}
```

For better timeline:
```rust
// From re_time_panel/src/time_selection_ui.rs
pub fn timeline_ui(ui: &mut Ui, time_ctrl: &TimeControl) {
    // Their timeline rendering
}
```

## License Compatibility
Rerun is Apache 2.0/MIT dual licensed - compatible with our project! 