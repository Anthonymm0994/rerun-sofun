# Notes System Documentation

The notes system allows users to attach annotations to various parts of the visualization.

## Architecture

### Core Components

1. **Note** - The basic note structure containing:
   - Content and optional title
   - Author and timestamps
   - Tags for organization
   - Visual styling options
   - Attachment information

2. **NoteAttachment** - Defines what a note is attached to:
   - `DataPoint` - Specific data point with row/column
   - `Plot` - A visualization with optional position
   - `ScreenPosition` - Fixed screen coordinates
   - `TimeRange` - A time period
   - `General` - Unattached notes

3. **NoteManager** - Handles storage and retrieval:
   - CRUD operations
   - Indexing for fast lookups
   - Search functionality
   - Tag management

## Usage Examples

### Creating a Note

```rust
let mut manager = NoteManager::new();

// Create a note attached to a data point
let note_id = manager.create_note(
    "Anomaly detected in this data point".to_string(),
    NoteAttachment::DataPoint {
        source_id: "sales_data.csv".to_string(),
        row_index: 42,
        column: Some("revenue".to_string()),
        value: json!(12345.67),
    },
    "John Doe".to_string(),
);

// Add tags
manager.add_tag(note_id, "anomaly".to_string());
manager.add_tag(note_id, "review".to_string());
```

### Searching Notes

```rust
// Search by content
let results = manager.search_notes("anomaly");

// Get notes by tag
let anomaly_notes = manager.get_notes_by_tag("anomaly");

// Get notes for a specific data point
let point_notes = manager.get_notes_for_data_point("sales_data.csv", 42);
```

### UI Integration

The UI components in `dv-ui/src/notes_ui.rs` provide:

1. **NoteWidget** - Display individual notes
2. **NoteIndicator** - Small icons showing note locations
3. **NoteEditor** - Dialog for creating/editing notes
4. **NotesPanel** - Browse and manage all notes

## Persistence

Notes can be serialized/deserialized using serde:

```rust
// Save
let json = serde_json::to_string(&manager)?;

// Load
let manager: NoteManager = serde_json::from_str(&json)?;
manager.rebuild_indices(); // Important!
```

## Best Practices

1. **Performance**: Use indices for lookups rather than iterating all notes
2. **Tags**: Use consistent tag naming for better organization
3. **Attachments**: Choose the most specific attachment type
4. **Cleanup**: Remove orphaned notes when data sources change

## Future Enhancements

- Collaborative notes with multiple authors
- Note templates for common annotations
- Export notes to markdown/PDF
- Note versioning and history
- Rich text formatting 