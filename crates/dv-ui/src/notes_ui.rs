//! UI components for the note-taking system

use egui::*;
use dv_core::notes::{Note, NoteAttachment, NoteId, NoteManager, NoteStyle};
use std::collections::HashSet;

/// Note widget that displays a single note
pub struct NoteWidget<'a> {
    note: &'a Note,
    interactive: bool,
    max_width: f32,
}

impl<'a> NoteWidget<'a> {
    pub fn new(note: &'a Note) -> Self {
        Self {
            note,
            interactive: true,
            max_width: 300.0,
        }
    }
    
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
    
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }
    
    pub fn show(self, ui: &mut Ui) -> NoteWidgetResponse {
        let mut clicked = false;
        let mut delete_requested = false;
        let mut edit_requested = false;
        
        let bg_color = Color32::from_rgba_unmultiplied(
            self.note.style.background_color[0],
            self.note.style.background_color[1],
            self.note.style.background_color[2],
            self.note.style.background_color[3],
        );
        
        let text_color = Color32::from_rgba_unmultiplied(
            self.note.style.text_color[0],
            self.note.style.text_color[1],
            self.note.style.text_color[2],
            self.note.style.text_color[3],
        );
        
        let border_color = Color32::from_rgba_unmultiplied(
            self.note.style.border_color[0],
            self.note.style.border_color[1],
            self.note.style.border_color[2],
            self.note.style.border_color[3],
        );
        
        let frame = Frame::none()
            .fill(bg_color)
            .stroke(Stroke::new(1.0, border_color))
            .inner_margin(8.0)
            .rounding(4.0)
            .shadow(egui::epaint::Shadow::small_light());
        
        let response = frame.show(ui, |ui| {
            ui.set_max_width(self.max_width * self.note.style.size_factor);
            
            // Header with icon and controls
            ui.horizontal(|ui| {
                // Icon
                if let Some(icon) = &self.note.style.icon {
                    ui.label(RichText::new(icon).size(16.0 * self.note.style.size_factor));
                }
                
                // Title or truncated content
                let title_text = self.note.title.as_deref()
                    .unwrap_or_else(|| {
                        let content = &self.note.content;
                        if content.len() > 30 {
                            &content[..30]
                        } else {
                            content
                        }
                    });
                
                ui.label(
                    RichText::new(title_text)
                        .color(text_color)
                        .size(14.0 * self.note.style.size_factor)
                        .strong()
                );
                
                // Controls on the right
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if self.interactive {
                        // Delete button
                        if ui.small_button("❌").on_hover_text("Delete note").clicked() {
                            delete_requested = true;
                        }
                        
                        // Edit button
                        if ui.small_button("✏️").on_hover_text("Edit note").clicked() {
                            edit_requested = true;
                        }
                        
                        // Pin indicator
                        if self.note.pinned {
                            ui.label("📌");
                        }
                    }
                });
            });
            
            ui.separator();
            
            // Content
            ui.label(
                RichText::new(&self.note.content)
                    .color(text_color)
                    .size(12.0 * self.note.style.size_factor)
            );
            
            // Tags
            if !self.note.tags.is_empty() {
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    for tag in &self.note.tags {
                        ui.label(
                            RichText::new(format!("#{}", tag))
                                .color(text_color.linear_multiply(0.7))
                                .size(10.0 * self.note.style.size_factor)
                        );
                    }
                });
            }
            
            // Metadata
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "by {} • {}",
                    self.note.author,
                    self.note.created_at.format("%Y-%m-%d %H:%M")
                ))
                .color(text_color.linear_multiply(0.5))
                .size(10.0 * self.note.style.size_factor)
            );
        });
        
        if self.interactive && response.response.clicked() {
            clicked = true;
        }
        
        NoteWidgetResponse {
            clicked,
            delete_requested,
            edit_requested,
            response: response.response,
        }
    }
}

pub struct NoteWidgetResponse {
    pub clicked: bool,
    pub delete_requested: bool,
    pub edit_requested: bool,
    pub response: Response,
}

/// Note indicator - small icon shown where a note is attached
pub struct NoteIndicator {
    count: usize,
    pinned: bool,
}

impl NoteIndicator {
    pub fn new(count: usize, pinned: bool) -> Self {
        Self { count, pinned }
    }
    
    pub fn show(self, ui: &mut Ui, pos: Pos2) -> Response {
        let icon = if self.pinned { "📌" } else { "📝" };
        let text = if self.count > 1 {
            format!("{} ({})", icon, self.count)
        } else {
            icon.to_string()
        };
        
        let galley = ui.painter().layout_no_wrap(
            text,
            FontId::proportional(14.0),
            Color32::from_rgb(255, 255, 200),
        );
        
        let rect = Rect::from_min_size(
            pos - galley.size() / 2.0,
            galley.size(),
        ).expand(4.0);
        
        let response = ui.allocate_rect(rect, Sense::click());
        
        ui.painter().rect_filled(
            rect,
            Rounding::same(4.0),
            Color32::from_rgba_unmultiplied(0, 0, 0, 180),
        );
        
        ui.painter().galley(
            rect.center() - galley.size() / 2.0,
            galley,
        );
        
        response.on_hover_text(format!("{} note(s)", self.count))
    }
}

/// Note creation/editing dialog
pub struct NoteEditor {
    pub visible: bool,
    pub note_id: Option<NoteId>,
    pub content: String,
    pub title: String,
    pub tags: String,
    pub attachment: NoteAttachment,
    pub style: NoteStyle,
}

impl NoteEditor {
    pub fn new(attachment: NoteAttachment) -> Self {
        Self {
            visible: false,
            note_id: None,
            content: String::new(),
            title: String::new(),
            tags: String::new(),
            attachment,
            style: NoteStyle::default(),
        }
    }
    
    pub fn edit_note(note: &Note) -> Self {
        Self {
            visible: true,
            note_id: Some(note.id),
            content: note.content.clone(),
            title: note.title.clone().unwrap_or_default(),
            tags: note.tags.join(", "),
            attachment: note.attachment.clone(),
            style: note.style.clone(),
        }
    }
    
    pub fn show(&mut self, ctx: &Context, author: &str) -> Option<NoteEditorAction> {
        if !self.visible {
            return None;
        }
        
        let mut action = None;
        let mut should_close = false;
        
        Window::new(if self.note_id.is_some() { "Edit Note" } else { "New Note" })
            .open(&mut self.visible)
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Title
                    ui.label("Title (optional):");
                    ui.text_edit_singleline(&mut self.title);
                    
                    ui.add_space(8.0);
                    
                    // Content
                    ui.label("Content:");
                    ui.add(
                        TextEdit::multiline(&mut self.content)
                            .desired_rows(6)
                            .desired_width(f32::INFINITY)
                    );
                    
                    ui.add_space(8.0);
                    
                    // Tags
                    ui.label("Tags (comma-separated):");
                    ui.text_edit_singleline(&mut self.tags);
                    
                    ui.add_space(8.0);
                    
                    // Style options
                    ui.collapsing("Style", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Background:");
                            let mut bg = Color32::from_rgba_unmultiplied(
                                self.style.background_color[0],
                                self.style.background_color[1],
                                self.style.background_color[2],
                                self.style.background_color[3],
                            );
                            if ui.color_edit_button_srgba(&mut bg).changed() {
                                self.style.background_color = bg.to_array();
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Text:");
                            let mut text = Color32::from_rgba_unmultiplied(
                                self.style.text_color[0],
                                self.style.text_color[1],
                                self.style.text_color[2],
                                self.style.text_color[3],
                            );
                            if ui.color_edit_button_srgba(&mut text).changed() {
                                self.style.text_color = text.to_array();
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Icon:");
                            let mut icon = self.style.icon.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut icon).changed() {
                                self.style.icon = if icon.is_empty() { None } else { Some(icon) };
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            ui.add(Slider::new(&mut self.style.size_factor, 0.5..=2.0));
                        });
                    });
                    
                    ui.add_space(8.0);
                    
                    // Buttons
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() && !self.content.is_empty() {
                            let tags: Vec<String> = self.tags
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            
                            action = Some(NoteEditorAction::Save {
                                id: self.note_id,
                                content: self.content.clone(),
                                title: if self.title.is_empty() { None } else { Some(self.title.clone()) },
                                tags,
                                attachment: self.attachment.clone(),
                                style: self.style.clone(),
                                author: author.to_string(),
                            });
                            
                            should_close = true;
                        }
                        
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                    });
                });
            });
        
        if should_close {
            self.visible = false;
        }
        
        action
    }
}

#[derive(Debug)]
pub enum NoteEditorAction {
    Save {
        id: Option<NoteId>,
        content: String,
        title: Option<String>,
        tags: Vec<String>,
        attachment: NoteAttachment,
        style: NoteStyle,
        author: String,
    },
}

/// Notes panel showing all notes
pub struct NotesPanel {
    search_query: String,
    selected_tags: HashSet<String>,
    show_general: bool,
    show_data_points: bool,
    show_plots: bool,
}

impl Default for NotesPanel {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            selected_tags: HashSet::new(),
            show_general: true,
            show_data_points: true,
            show_plots: true,
        }
    }
}

impl NotesPanel {
    pub fn show(&mut self, ui: &mut Ui, note_manager: &NoteManager) -> Option<NotesPanelAction> {
        let mut action = None;
        
        ui.heading("Notes");
        ui.separator();
        
        // Search bar
        ui.horizontal(|ui| {
            ui.label("🔍");
            if ui.text_edit_singleline(&mut self.search_query)
                .on_hover_text("Search notes")
                .changed() {
                // Search is reactive
            }
            
            if ui.button("Clear").clicked() {
                self.search_query.clear();
            }
        });
        
        ui.add_space(4.0);
        
        // Filters
        ui.horizontal(|ui| {
            ui.label("Show:");
            ui.checkbox(&mut self.show_general, "General");
            ui.checkbox(&mut self.show_data_points, "Data Points");
            ui.checkbox(&mut self.show_plots, "Plots");
        });
        
        // Get all unique tags
        let all_tags: HashSet<String> = note_manager
            .all_notes()
            .flat_map(|note| note.tags.iter().cloned())
            .collect();
        
        if !all_tags.is_empty() {
            ui.add_space(4.0);
            ui.label("Filter by tags:");
            ui.horizontal_wrapped(|ui| {
                for tag in &all_tags {
                    let mut selected = self.selected_tags.contains(tag);
                    if ui.toggle_value(&mut selected, format!("#{}", tag)).clicked() {
                        if selected {
                            self.selected_tags.insert(tag.clone());
                        } else {
                            self.selected_tags.remove(tag);
                        }
                    }
                }
            });
        }
        
        ui.separator();
        
        // List notes
        ScrollArea::vertical().show(ui, |ui| {
            let notes: Vec<&Note> = if self.search_query.is_empty() {
                note_manager.all_notes().collect()
            } else {
                note_manager.search_notes(&self.search_query)
            };
            
            for note in notes {
                // Filter by type
                let show = match &note.attachment {
                    NoteAttachment::General => self.show_general,
                    NoteAttachment::DataPoint { .. } => self.show_data_points,
                    NoteAttachment::Plot { .. } => self.show_plots,
                    _ => true,
                };
                
                if !show {
                    continue;
                }
                
                // Filter by tags
                if !self.selected_tags.is_empty() {
                    let has_selected_tag = note.tags.iter()
                        .any(|tag| self.selected_tags.contains(tag));
                    if !has_selected_tag {
                        continue;
                    }
                }
                
                let response = NoteWidget::new(note).show(ui);
                
                if response.clicked {
                    action = Some(NotesPanelAction::SelectNote(note.id));
                }
                
                if response.edit_requested {
                    action = Some(NotesPanelAction::EditNote(note.id));
                }
                
                if response.delete_requested {
                    action = Some(NotesPanelAction::DeleteNote(note.id));
                }
                
                ui.add_space(4.0);
            }
        });
        
        action
    }
}

#[derive(Debug)]
pub enum NotesPanelAction {
    SelectNote(NoteId),
    EditNote(NoteId),
    DeleteNote(NoteId),
}

/// Context menu for creating notes
pub fn show_note_context_menu(
    ui: &mut Ui,
    _pos: Pos2,
    _attachment: NoteAttachment,
) -> bool {
    let mut create_note = false;
    
    ui.menu_button("📝 Add Note", |ui| {
        if ui.button("Quick Note").clicked() {
            create_note = true;
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("With Style...").clicked() {
            create_note = true;
            ui.close_menu();
        }
    });
    
    create_note
} 