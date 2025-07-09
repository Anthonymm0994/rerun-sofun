//! Note-taking system for data visualization
//! 
//! This module provides functionality for creating, storing, and managing notes
//! that can be attached to data points, plots, or arbitrary locations in the app.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a note
pub type NoteId = Uuid;

/// Represents a note in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// Unique identifier
    pub id: NoteId,
    
    /// Note content
    pub content: String,
    
    /// Optional title
    pub title: Option<String>,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Last modification timestamp
    pub modified_at: DateTime<Utc>,
    
    /// Author/creator of the note
    pub author: String,
    
    /// Tags for categorization
    pub tags: Vec<String>,
    
    /// Visual properties
    pub style: NoteStyle,
    
    /// What this note is attached to
    pub attachment: NoteAttachment,
    
    /// Whether the note is currently visible
    pub visible: bool,
    
    /// Whether the note is pinned (always visible)
    pub pinned: bool,
}

/// Visual style for a note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteStyle {
    /// Background color (RGBA)
    pub background_color: [u8; 4],
    
    /// Text color (RGBA)
    pub text_color: [u8; 4],
    
    /// Border color (RGBA)
    pub border_color: [u8; 4],
    
    /// Icon to display (emoji or symbol)
    pub icon: Option<String>,
    
    /// Size multiplier (1.0 = normal)
    pub size_factor: f32,
}

impl Default for NoteStyle {
    fn default() -> Self {
        Self {
            background_color: [255, 255, 200, 230], // Light yellow
            text_color: [0, 0, 0, 255], // Black
            border_color: [200, 200, 150, 255], // Darker yellow
            icon: Some("📝".to_string()),
            size_factor: 1.0,
        }
    }
}

/// What a note is attached to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoteAttachment {
    /// Attached to a specific data point
    DataPoint {
        /// Data source ID
        source_id: String,
        /// Row/record index
        row_index: usize,
        /// Optional column name for column-specific notes
        column: Option<String>,
        /// The actual data value (for display)
        value: serde_json::Value,
    },
    
    /// Attached to a plot/view
    Plot {
        /// View ID
        view_id: Uuid,
        /// Optional position within the plot (normalized 0-1 coordinates)
        position: Option<(f32, f32)>,
    },
    
    /// Attached to a specific screen position
    ScreenPosition {
        /// X coordinate (pixels)
        x: f32,
        /// Y coordinate (pixels)
        y: f32,
    },
    
    /// Attached to a time range
    TimeRange {
        /// Start time
        start: DateTime<Utc>,
        /// End time
        end: DateTime<Utc>,
    },
    
    /// General note not attached to anything specific
    General,
}

/// Note manager handles storage and retrieval of notes
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NoteManager {
    /// All notes indexed by ID
    notes: HashMap<NoteId, Note>,
    
    /// Index of notes by attachment type for faster lookup
    #[serde(skip)]
    attachment_index: HashMap<String, Vec<NoteId>>,
    
    /// Notes organized by tags
    #[serde(skip)]
    tag_index: HashMap<String, Vec<NoteId>>,
}

impl NoteManager {
    /// Create a new note manager
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a new note
    pub fn add_note(&mut self, note: Note) -> NoteId {
        let id = note.id;
        
        // Update indices
        self.index_note(&note);
        
        // Store note
        self.notes.insert(id, note);
        
        id
    }
    
    /// Create and add a new note
    pub fn create_note(
        &mut self,
        content: String,
        attachment: NoteAttachment,
        author: String,
    ) -> NoteId {
        let note = Note {
            id: Uuid::new_v4(),
            content,
            title: None,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            author,
            tags: Vec::new(),
            style: NoteStyle::default(),
            attachment,
            visible: true,
            pinned: false,
        };
        
        self.add_note(note)
    }
    
    /// Update an existing note
    pub fn update_note(&mut self, id: NoteId, content: String) -> Option<()> {
        if let Some(note) = self.notes.get_mut(&id) {
            note.content = content;
            note.modified_at = Utc::now();
            Some(())
        } else {
            None
        }
    }
    
    /// Delete a note
    pub fn delete_note(&mut self, id: NoteId) -> Option<Note> {
        if let Some(note) = self.notes.remove(&id) {
            // Remove from indices
            self.unindex_note(&note);
            Some(note)
        } else {
            None
        }
    }
    
    /// Get a note by ID
    pub fn get_note(&self, id: NoteId) -> Option<&Note> {
        self.notes.get(&id)
    }
    
    /// Get a mutable reference to a note
    pub fn get_note_mut(&mut self, id: NoteId) -> Option<&mut Note> {
        self.notes.get_mut(&id)
    }
    
    /// Get all notes
    pub fn all_notes(&self) -> impl Iterator<Item = &Note> {
        self.notes.values()
    }
    
    /// Get notes for a specific data point
    pub fn get_notes_for_data_point(
        &self,
        source_id: &str,
        row_index: usize,
    ) -> Vec<&Note> {
        self.notes
            .values()
            .filter(|note| {
                matches!(
                    &note.attachment,
                    NoteAttachment::DataPoint { source_id: s, row_index: r, .. }
                    if s == source_id && *r == row_index
                )
            })
            .collect()
    }
    
    /// Get notes for a specific plot/view
    pub fn get_notes_for_plot(&self, view_id: Uuid) -> Vec<&Note> {
        self.notes
            .values()
            .filter(|note| {
                matches!(
                    &note.attachment,
                    NoteAttachment::Plot { view_id: v, .. } if *v == view_id
                )
            })
            .collect()
    }
    
    /// Get notes by tag
    pub fn get_notes_by_tag(&self, tag: &str) -> Vec<&Note> {
        self.tag_index
            .get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.notes.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Search notes by content
    pub fn search_notes(&self, query: &str) -> Vec<&Note> {
        let query_lower = query.to_lowercase();
        self.notes
            .values()
            .filter(|note| {
                note.content.to_lowercase().contains(&query_lower)
                    || note.title
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect()
    }
    
    /// Toggle note visibility
    pub fn toggle_visibility(&mut self, id: NoteId) -> Option<bool> {
        self.notes.get_mut(&id).map(|note| {
            note.visible = !note.visible;
            note.visible
        })
    }
    
    /// Toggle note pinned state
    pub fn toggle_pinned(&mut self, id: NoteId) -> Option<bool> {
        self.notes.get_mut(&id).map(|note| {
            note.pinned = !note.pinned;
            note.pinned
        })
    }
    
    /// Add a tag to a note
    pub fn add_tag(&mut self, id: NoteId, tag: String) -> Option<()> {
        if let Some(note) = self.notes.get_mut(&id) {
            if !note.tags.contains(&tag) {
                note.tags.push(tag.clone());
                self.tag_index.entry(tag).or_default().push(id);
            }
            Some(())
        } else {
            None
        }
    }
    
    /// Remove a tag from a note
    pub fn remove_tag(&mut self, id: NoteId, tag: &str) -> Option<()> {
        if let Some(note) = self.notes.get_mut(&id) {
            note.tags.retain(|t| t != tag);
            if let Some(ids) = self.tag_index.get_mut(tag) {
                ids.retain(|&i| i != id);
            }
            Some(())
        } else {
            None
        }
    }
    
    /// Rebuild indices after loading from storage
    pub fn rebuild_indices(&mut self) {
        self.attachment_index.clear();
        self.tag_index.clear();
        
        let notes_to_index: Vec<Note> = self.notes.values().cloned().collect();
        for note in notes_to_index {
            self.index_note(&note);
        }
    }
    
    /// Index a note for faster lookup
    fn index_note(&mut self, note: &Note) {
        // Index by attachment type
        let attachment_key = match &note.attachment {
            NoteAttachment::DataPoint { source_id, row_index, .. } => {
                format!("data:{}:{}", source_id, row_index)
            }
            NoteAttachment::Plot { view_id, .. } => format!("plot:{}", view_id),
            NoteAttachment::ScreenPosition { .. } => "screen".to_string(),
            NoteAttachment::TimeRange { .. } => "time".to_string(),
            NoteAttachment::General => "general".to_string(),
        };
        
        self.attachment_index
            .entry(attachment_key)
            .or_default()
            .push(note.id);
        
        // Index by tags
        for tag in &note.tags {
            self.tag_index
                .entry(tag.clone())
                .or_default()
                .push(note.id);
        }
    }
    
    /// Remove a note from indices
    fn unindex_note(&mut self, note: &Note) {
        // Remove from attachment index
        let attachment_key = match &note.attachment {
            NoteAttachment::DataPoint { source_id, row_index, .. } => {
                format!("data:{}:{}", source_id, row_index)
            }
            NoteAttachment::Plot { view_id, .. } => format!("plot:{}", view_id),
            NoteAttachment::ScreenPosition { .. } => "screen".to_string(),
            NoteAttachment::TimeRange { .. } => "time".to_string(),
            NoteAttachment::General => "general".to_string(),
        };
        
        if let Some(ids) = self.attachment_index.get_mut(&attachment_key) {
            ids.retain(|&id| id != note.id);
        }
        
        // Remove from tag index
        for tag in &note.tags {
            if let Some(ids) = self.tag_index.get_mut(tag) {
                ids.retain(|&id| id != note.id);
            }
        }
    }
}

/// Note event for UI updates
#[derive(Debug, Clone)]
pub enum NoteEvent {
    /// A note was created
    Created(NoteId),
    
    /// A note was updated
    Updated(NoteId),
    
    /// A note was deleted
    Deleted(NoteId),
    
    /// Note visibility changed
    VisibilityChanged(NoteId, bool),
    
    /// Note pinned state changed
    PinnedChanged(NoteId, bool),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_note_creation() {
        let mut manager = NoteManager::new();
        
        let id = manager.create_note(
            "Test note".to_string(),
            NoteAttachment::General,
            "Test User".to_string(),
        );
        
        assert!(manager.get_note(id).is_some());
        assert_eq!(manager.all_notes().count(), 1);
    }
    
    #[test]
    fn test_note_search() {
        let mut manager = NoteManager::new();
        
        manager.create_note(
            "This is a test note".to_string(),
            NoteAttachment::General,
            "User".to_string(),
        );
        
        manager.create_note(
            "Another note".to_string(),
            NoteAttachment::General,
            "User".to_string(),
        );
        
        let results = manager.search_notes("test");
        assert_eq!(results.len(), 1);
    }
} 