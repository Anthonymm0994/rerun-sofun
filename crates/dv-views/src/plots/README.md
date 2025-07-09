# Plot Export Integration Guide

This guide explains how to add export functionality to your plots.

## Quick Start

1. Add the export module to your plot imports:
```rust
use crate::export::{ExportDialog, show_export_button};
```

2. Add an `ExportDialog` field to your plot struct:
```rust
pub struct MyPlot {
    // ... other fields ...
    export_dialog: ExportDialog,
}
```

3. Initialize it in your constructor:
```rust
impl MyPlot {
    pub fn new() -> Self {
        Self {
            // ... other fields ...
            export_dialog: ExportDialog::default(),
        }
    }
}
```

4. In your `ui` method, handle the export dialog and add the button:
```rust
fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
    // Handle export dialog
    if let Some((options, format)) = self.export_dialog.show(ui.ctx()) {
        // TODO: Implement actual export logic
        tracing::info!("Export requested: {:?} format with options: {:?}", format, options);
    }
    
    // Add export button to your UI
    ui.horizontal(|ui| {
        // Your existing UI elements...
        
        // Add export button at the right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if show_export_button(ui, &format!("{:?}", self.id)) {
                self.export_dialog.show = true;
            }
        });
    });
    
    // Rest of your plot UI...
}
```

## Advanced Export Implementation

To actually implement the export functionality:

1. Capture the plot's render output
2. Convert to the desired format
3. Save to file using a file dialog

Example implementation will be added once the rendering pipeline is integrated.

## Export Formats

- **PNG**: Fully implemented, raster format
- **SVG**: Placeholder, requires vector rendering
- **PDF**: Placeholder, requires PDF generation library

## Customization Options

The export dialog provides:
- Custom dimensions or preset sizes (HD, 4K, Square)
- Background color (including transparent)
- DPI settings for vector formats
- Options to include/exclude title and legend 