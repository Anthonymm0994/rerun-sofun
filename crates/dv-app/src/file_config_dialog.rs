use dv_data::config::{FileConfigManager, FileType, SerializableDataType};
use std::path::PathBuf;
use egui::{Context, Ui, RichText, Color32, ScrollArea, DragValue, Grid};
use tokio::runtime::Handle;

/// File configuration dialog
pub struct FileConfigDialog {
    /// Configuration manager
    pub config_manager: FileConfigManager,
    
    /// Show dialog
    pub show: bool,
    
    /// Null pattern input field
    null_pattern_input: String,
    
    /// Type inference results
    inference_results: Vec<String>,
    
    /// Error message
    error_message: Option<String>,
    
    /// Runtime handle
    runtime: Handle,
    
    /// Loading state
    is_loading: bool,
    
    /// Loading progress (0.0 - 1.0)
    loading_progress: f32,
    
    /// Loading message
    loading_message: String,
    
    /// Cancel loading flag
    cancel_loading: bool,
}

impl FileConfigDialog {
    /// Create a new file configuration dialog
    pub fn new(config_manager: FileConfigManager, runtime: Handle) -> Self {
        Self {
            config_manager,
            show: true,
            null_pattern_input: String::new(),
            inference_results: Vec::new(),
            error_message: None,
            runtime,
            is_loading: false,
            loading_progress: 0.0,
            loading_message: String::new(),
            cancel_loading: false,
        }
    }
    
    /// Show the dialog and return the configuration if confirmed
    pub fn show_dialog(&mut self, ctx: &Context) -> Option<FileConfigManager> {
        let mut result = None;
        let mut close = false;
        
        // Create a modal background by drawing a dark overlay
        let screen_rect = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(
            screen_rect,
            0.0,
            Color32::from_rgba_premultiplied(0, 0, 0, 200)
        );
        
        // Show the dialog as a window
        egui::Window::new("File Configuration")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([1200.0, 800.0])
            .show(ctx, |ui| {
                // Custom title bar
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📁").size(28.0));
                    ui.label(RichText::new("File Configuration").size(24.0).strong().color(Color32::WHITE));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("✖").size(20.0))
                            .on_hover_text("Close without loading")
                            .clicked() {
                            close = true;
                        }
                    });
                });
                
                ui.separator();
                
                // File selector
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Configure:").size(16.0).color(Color32::from_gray(200)));
                    
                    if let Some(active_path) = self.config_manager.active_file.clone() {
                        if let Some(active_config) = self.config_manager.active_config() {
                            let active_file_name = active_config.file_name();
                            
                            // Collect paths and names first
                            let file_list: Vec<(PathBuf, String)> = self.config_manager.configs
                                .iter()
                                .map(|(path, config)| (path.clone(), config.file_name()))
                                .collect();
                            
                            // Track which file was selected
                            let mut selected_path = None;
                            
                            egui::ComboBox::from_label("")
                                .selected_text(RichText::new(&active_file_name).size(16.0))
                                .show_ui(ui, |ui| {
                                    for (path, name) in file_list {
                                        let is_selected = &active_path == &path;
                                        if ui.selectable_label(is_selected, name).clicked() {
                                            selected_path = Some(path);
                                        }
                                    }
                                });
                            
                            // Update active file after UI interaction
                            if let Some(path) = selected_path {
                                self.config_manager.set_active_file(path);
                            }
                        }
                    }
                });
                
                ui.separator();
                
                // Main content area
                let available_height = ui.available_height() - 60.0; // Reserve space for buttons
                ui.allocate_ui(egui::vec2(ui.available_width(), available_height), |ui| {
                    if self.config_manager.active_file.is_some() {
                        let file_type = self.config_manager.active_config()
                            .map(|c| c.file_type)
                            .unwrap_or(FileType::Csv);
                        
                        match file_type {
                            FileType::Csv => self.show_csv_config_fullscreen(ui),
                            FileType::Sqlite => self.show_sqlite_config_redesigned(ui),
                        }
                    } else {
                        // No files message
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new("No files to configure")
                                .size(20.0)
                                .color(Color32::from_gray(150)));
                        });
                    }
                });
                
                ui.separator();
                
                // Bottom buttons
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("❌ Cancel").size(16.0))
                        .on_hover_text("Close without loading files")
                        .clicked() {
                        close = true;
                        result = None;
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Load button - only enabled when we have files
                        let can_load = !self.config_manager.configs.is_empty();
                        
                        let load_button = egui::Button::new(
                            RichText::new("✅ Load Files")
                                .size(18.0)
                                .color(Color32::WHITE)
                        )
                        .fill(Color32::from_rgb(76, 175, 80))
                        .rounding(egui::Rounding::same(6.0));
                        
                        if ui.add_enabled(can_load, load_button)
                            .on_hover_text("Load files with current configuration")
                            .clicked() {
                            result = Some(self.config_manager.clone());
                            close = true;
                        }
                        
                        ui.add_space(10.0);
                        
                        // Quick tips
                        ui.label(RichText::new("💡 Tip: You can change the detected type for each column")
                            .size(12.0)
                            .color(Color32::from_gray(150)));
                    });
                });
                
                // Error message if any
                if let Some(error) = &self.error_message {
                    ui.colored_label(Color32::from_rgb(255, 100, 100), error);
                }
            });
        
        // Show progress overlay if loading
        if self.is_loading {
            egui::Window::new("Loading")
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([300.0, 200.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new("⏳ Analyzing File").size(20.0).strong());
                        ui.add_space(10.0);
                        
                        // Progress bar
                        let progress_bar = egui::ProgressBar::new(self.loading_progress)
                            .text(format!("{:.0}%", self.loading_progress * 100.0))
                            .desired_width(250.0);
                        ui.add(progress_bar);
                        
                        ui.add_space(10.0);
                        ui.label(RichText::new(&self.loading_message).size(12.0).color(Color32::from_gray(180)));
                        
                        ui.add_space(20.0);
                        
                        if ui.button(RichText::new("❌ Cancel").size(14.0))
                            .on_hover_text("Cancel operation")
                            .clicked() {
                            self.cancel_loading = true;
                        }
                    });
                });
            
            // Request repaint while loading
            ctx.request_repaint();
        }
        
        if close {
            self.show = false;
        }
        
        result
    }
    
    /// Show redesigned CSV configuration with better layout
    fn show_csv_config_redesigned(&mut self, ui: &mut Ui) {
        let Some(active_path) = self.config_manager.active_file.clone() else { return; };
        
        // Load preview if needed
        let needs_preview = self.config_manager.configs.get(&active_path)
            .map(|c| c.preview_lines.is_none())
            .unwrap_or(false);
            
        if needs_preview {
            let mut needs_type_inference = false;
            if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                use std::fs::File;
                use std::io::BufReader;
                
                if let Ok(file) = File::open(&config.path) {
                    let reader = BufReader::new(file);
                    let mut csv_reader = ::csv::ReaderBuilder::new()
                        .has_headers(false)
                        .from_reader(reader);
                    
                    let mut lines = Vec::new();
                    
                    for (idx, record) in csv_reader.records().enumerate() {
                        if idx >= 20 {  // Show 20 lines
                            break;
                        }
                        
                        if let Ok(record) = record {
                            let cells: Vec<String> = record.iter()
                                .map(|s| s.trim().to_string())
                                .collect();
                            lines.push(cells);
                        }
                    }
                    
                    config.preview_lines = Some(lines.clone());
                    
                    // Auto-detect columns from first line
                    if let Some(first_line) = lines.get(0) {
                        config.detected_columns = first_line.clone();
                        // Select all columns by default
                        if config.selected_columns.is_empty() {
                            config.selected_columns = first_line.iter().cloned().collect();
                        }
                    }
                    
                    // Check if we need type inference
                    needs_type_inference = config.column_types.is_empty() && !config.detected_columns.is_empty();
                }
            }
            
            // Run type inference after the mutable borrow is dropped
            if needs_type_inference {
                self.run_type_inference_for_config(&active_path);
            }
        }
        
        // Two-column layout
        ui.horizontal(|ui| {
            // Left panel - Configuration options
            ui.vertical(|ui| {
                ui.set_width(500.0);
                
                // Header configuration
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("📋 Header Configuration").size(18.0).strong());
                    ui.add_space(8.0);
                    
                    ui.horizontal(|ui| {
                        ui.label("Header Line:");
                        
                        // Convert from 0-indexed to 1-indexed for display
                        let mut header_line_display = self.config_manager.configs.get(&active_path)
                            .map(|c| c.header_line + 1)
                            .unwrap_or(1);
                        
                        let max_lines = self.config_manager.configs.get(&active_path)
                            .and_then(|c| c.preview_lines.as_ref())
                            .map(|lines| lines.len())
                            .unwrap_or(20) as usize;
                        
                        ui.add_space(10.0);
                        let response = ui.add(
                            DragValue::new(&mut header_line_display)
                                .clamp_range(1..=max_lines)
                                .speed(1)
                        );
                        
                        if response.changed() {
                            let needs_inference = if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                config.header_line = header_line_display.saturating_sub(1);
                                // Force preview reload when header line changes
                                config.preview_lines = None;
                                true
                            } else {
                                false
                            };
                            
                            if needs_inference {
                                self.run_type_inference_for_config(&active_path);
                            }
                        }
                        
                        ui.label(format!("(1-{})", max_lines));
                    });
                    
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("💡 The green highlighted row in the preview is your header. Data will be read from this row down.")
                            .size(12.0)
                            .color(Color32::from_gray(150))
                    );
                });
                
                ui.add_space(12.0);
                
                // Type inference - above column selection
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Type Inference").size(16.0).strong());
                    ui.add_space(8.0);
                    
                    ui.horizontal(|ui| {
                        ui.label("Sample Size:");
                        
                        let mut sample_size = self.config_manager.configs.get(&active_path)
                            .map(|c| c.sample_size)
                            .unwrap_or(1000);
                            
                        ui.add_space(10.0);
                        let response = ui.add(
                            DragValue::new(&mut sample_size)
                                .clamp_range(100..=10000)
                                .speed(10)
                        );
                        
                        if response.changed() {
                            if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                config.sample_size = sample_size;
                            }
                        }
                        
                        ui.label("rows");
                        
                        ui.add_space(20.0);
                        
                        if ui.button("🔄 Re-detect Types")
                            .on_hover_text("Re-run automatic type detection")
                            .clicked() {
                            self.run_type_inference_for_config(&active_path);
                        }
                    });
                    

                });
                
                ui.add_space(12.0);
                
                // Column selection with types in a grid
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Column Selection").size(16.0).strong());
                    ui.add_space(8.0);
                    
                    let detected_columns = self.config_manager.configs.get(&active_path)
                        .map(|c| c.detected_columns.clone())
                        .unwrap_or_default();
                    let selected_count = self.config_manager.configs.get(&active_path)
                        .map(|c| c.selected_columns.len())
                        .unwrap_or(0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("Select All").clicked() {
                            if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                for col in &config.detected_columns {
                                    config.selected_columns.insert(col.clone());
                                }
                            }
                        }
                        if ui.button("Deselect All").clicked() {
                            if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                config.selected_columns.clear();
                            }
                        }
                        ui.label(format!("{} / {} selected", selected_count, detected_columns.len()));
                    });
                    
                    ui.separator();
                    
                    // Column grid with types
                    ScrollArea::vertical()
                        .id_source("column_selection_scroll")
                        .show(ui, |ui| {
                            Grid::new("column_grid")
                                .striped(true)
                                .spacing([8.0, 4.0])
                                .show(ui, |ui| {
                                    // Headers
                                    ui.label(RichText::new("Include").strong());
                                    ui.label(RichText::new("Column Name").strong());
                                    ui.label(RichText::new("Detected Type").strong());
                                    ui.end_row();
                                    
                                    for col in &detected_columns {
                                        let mut selected = self.config_manager.configs.get(&active_path)
                                            .map(|c| c.selected_columns.contains(col))
                                            .unwrap_or(false);
                                            
                                        if ui.checkbox(&mut selected, "").changed() {
                                            if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                                if selected {
                                                    config.selected_columns.insert(col.clone());
                                                } else {
                                                    config.selected_columns.remove(col);
                                                }
                                            }
                                        }
                                        
                                        ui.label(col);
                                        
                                        // Show detected type
                                        let detected_type = self.config_manager.configs.get(&active_path)
                                            .and_then(|c| c.column_types.get(col))
                                            .map(|t| format_serializable_type(t))
                                            .unwrap_or("String");
                                        
                                        ui.label(RichText::new(detected_type).color(Color32::from_rgb(100, 150, 200)));
                                        
                                        ui.end_row();
                                    }
                                });
                        });
                });
                
                ui.add_space(12.0);
                
                // Null handling
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Null Handling").size(16.0).strong());
                    ui.add_space(8.0);
                    
                    ui.label("Treat these values as null:");
                    
                    let null_patterns = self.config_manager.configs.get(&active_path)
                        .map(|c| c.null_config.patterns.clone())
                        .unwrap_or_default();
                    
                    ScrollArea::vertical()
                        .id_source("null_patterns_scroll")
                        .max_height(100.0)
                        .show(ui, |ui| {
                            let mut to_remove = None;
                            
                            for (idx, pattern) in null_patterns.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    if ui.small_button("×").clicked() {
                                        to_remove = Some(idx);
                                    }
                                    ui.monospace(if pattern.is_empty() { 
                                        "[empty string]" 
                                    } else { 
                                        pattern 
                                    });
                                });
                            }
                            
                            if let Some(idx) = to_remove {
                                if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                    config.null_config.patterns.remove(idx);
                                }
                            }
                        });
                    
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.null_pattern_input);
                        if ui.button("Add").clicked() && !self.null_pattern_input.is_empty() {
                            if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                config.null_config.patterns.push(self.null_pattern_input.clone());
                                self.null_pattern_input.clear();
                            }
                        }
                    });
                });
            });
            
            ui.separator();
            
            // Right panel - Preview
            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.label(RichText::new("Data Preview").size(16.0).strong());
                    ui.add_space(8.0);
                    
                    let preview_data = self.config_manager.configs.get(&active_path)
                        .and_then(|c| c.preview_lines.clone());
                    let header_line_idx = self.config_manager.configs.get(&active_path)
                        .map(|c| c.header_line)
                        .unwrap_or(0);  // 0-indexed
                        
                    if let Some(preview) = preview_data {
                        ScrollArea::both()
                            .id_source("csv_preview_scroll")
                            .show(ui, |ui| {
                                Grid::new("preview_grid")
                                    .striped(true)
                                    .spacing([12.0, 4.0])
                                    .show(ui, |ui| {
                                        // Show all rows with green highlighting for header
                                        for (row_idx, row) in preview.iter().enumerate() {
                                            // Check if this is the header line
                                            if row_idx == header_line_idx {
                                                let header_color = Color32::from_rgb(100, 200, 100);
                                                ui.label(
                                                    RichText::new((row_idx + 1).to_string())
                                                        .color(header_color)
                                                        .strong()
                                                );
                                                ui.separator();
                                                for (i, cell) in row.iter().enumerate() {
                                                    if i > 0 {
                                                        ui.separator();
                                                    }
                                                    ui.label(
                                                        RichText::new(cell)
                                                            .color(header_color)
                                                            .strong()
                                                    );
                                                }
                                                ui.end_row();
                                            } else {
                                                // Regular data row
                                                ui.label(
                                                    RichText::new((row_idx + 1).to_string())
                                                        .color(Color32::from_gray(150))
                                                );
                                                ui.separator();
                                                
                                                for (i, cell) in row.iter().enumerate() {
                                                    if i > 0 {
                                                        ui.separator();
                                                    }
                                                    ui.label(cell);
                                                }
                                                ui.end_row();
                                            }
                                        }
                                    });
                            });
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Loading preview...");
                        });
                    }
                });
            });
        });
    }
    
    /// Show redesigned SQLite configuration
    fn show_sqlite_config_redesigned(&mut self, ui: &mut Ui) {
        let Some(active_path) = self.config_manager.active_file.clone() else { return; };
        
        // Load table list if not loaded
        let needs_loading = self.config_manager.configs.get(&active_path)
            .map(|c| c.detected_columns.is_empty())
            .unwrap_or(false);
            
        if needs_loading {
            if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                if let Ok(conn) = rusqlite::Connection::open(&config.path) {
                    let query = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'";
                    
                    if let Ok(mut stmt) = conn.prepare(query) {
                        if let Ok(tables) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                            config.detected_columns = tables.filter_map(Result::ok).collect();
                        }
                    }
                }
            }
        }
        
        ui.centered_and_justified(|ui| {
            ui.group(|ui| {
                ui.set_max_width(600.0);
                
                ui.label(RichText::new("SQLite Database").size(20.0).strong());
                ui.add_space(12.0);
                
                if let Some(config) = self.config_manager.configs.get(&active_path) {
                    ui.label(RichText::new(format!("{}", config.file_name()))
                        .size(14.0)
                        .color(Color32::from_gray(180)));
                }
                
                ui.add_space(20.0);
                ui.separator();
                ui.add_space(20.0);
                
                ui.label(RichText::new("Select Tables to Load").size(16.0));
                ui.label(RichText::new("Each table will be loaded as a separate data source")
                    .size(12.0)
                    .color(Color32::from_gray(150)));
                
                ui.add_space(12.0);
                
                let detected_tables = self.config_manager.configs.get(&active_path)
                    .map(|c| c.detected_columns.clone())
                    .unwrap_or_default();
                
                if detected_tables.is_empty() {
                    ui.label(RichText::new("No tables found in database")
                        .size(14.0)
                        .color(Color32::from_gray(150)));
                } else {
                    ScrollArea::vertical()
                        .id_source("sqlite_tables_redesigned_scroll")
                        .max_height(400.0)
                        .show(ui, |ui| {
                            for table in &detected_tables {
                                let is_selected = self.config_manager.configs.get(&active_path)
                                    .map(|c| c.selected_columns.contains(table))
                                    .unwrap_or(false);
                                
                                let table_button = ui.add(
                                    egui::SelectableLabel::new(is_selected, 
                                        RichText::new(format!("{}", table))
                                            .size(14.0)
                                    )
                                );
                                
                                if table_button.clicked() {
                                    if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                        if is_selected {
                                            config.selected_columns.remove(table);
                                        } else {
                                            config.selected_columns.insert(table.clone());
                                        }
                                    }
                                }
                            }
                        });
                    
                    ui.add_space(12.0);
                    
                    let selected_count = self.config_manager.configs.get(&active_path)
                        .map(|c| c.selected_columns.len())
                        .unwrap_or(0);
                    
                    ui.label(RichText::new(format!("{} table{} selected", 
                        selected_count,
                        if selected_count == 1 { "" } else { "s" }
                    ))
                        .size(12.0)
                        .color(Color32::from_gray(150)));
                }
            });
        });
    }
    
    /// Run type inference for a specific config
    fn run_type_inference_for_config(&mut self, path: &PathBuf) {
        
        // Extract the necessary data first to avoid borrowing conflicts
        let (file_type, file_path, header_line, sample_size) = 
            if let Some(config) = self.config_manager.configs.get(path) {
                (config.file_type.clone(), config.path.clone(), config.header_line, config.sample_size)
            } else {
                return;
            };
        
        // Show loading state
        self.is_loading = true;
        self.loading_progress = 0.0;
        self.loading_message = "Analyzing file structure...".to_string();
        
        match file_type {
            FileType::Csv => {
                use std::fs::File;
                use std::io::BufReader;
                
                if let Ok(file) = File::open(&file_path) {
                    let reader = BufReader::new(file);
                    let mut csv_reader = ::csv::ReaderBuilder::new()
                        .has_headers(false)
                        .from_reader(reader);
                    
                    // Skip to header line
                    for _ in 0..header_line {
                        csv_reader.records().next();
                    }
                    
                    // Read header
                    if let Some(Ok(header_record)) = csv_reader.records().next() {
                        let headers: Vec<String> = header_record.iter()
                            .map(|s| s.to_string())
                            .collect();
                        
                        // Now sample data rows for type detection
                        let mut samples: Vec<Vec<String>> = Vec::new();
                        let sample_size = sample_size.min(10000);
                        
                        self.loading_message = format!("Sampling {} rows for type detection...", sample_size);
                        
                        for (idx, record) in csv_reader.records().enumerate() {
                            if idx >= sample_size {
                                break;
                            }
                            
                            // Update progress
                            self.loading_progress = (idx as f32) / (sample_size as f32);
                            
                                                            // Check for cancellation
                                if self.cancel_loading {
                                    self.is_loading = false;
                                    self.cancel_loading = false;
                                    return;
                                }
                            
                            if let Ok(record) = record {
                                samples.push(record.iter().map(|s| s.to_string()).collect());
                            }
                        }
                        
                        self.loading_message = "Detecting column types...".to_string();
                        self.loading_progress = 0.9;
                        
                                                    // Collect all the type inferences first
                            let mut column_types: Vec<(String, SerializableDataType)> = Vec::new();
                            
                            for (col_idx, header) in headers.iter().enumerate() {
                                let column_values: Vec<String> = samples.iter()
                                    .filter_map(|row| row.get(col_idx).cloned())
                                    .collect();
                                
                                let detected_type = self.infer_column_type(&column_values);
                                column_types.push((header.clone(), detected_type));
                            }
                            
                            // Now update the config with all the types at once
                        if let Some(config) = self.config_manager.configs.get_mut(path) {
                            // Update detected columns with new headers
                            config.detected_columns = headers.clone();
                            
                            // Clear and rebuild selected columns to only include valid columns
                            let old_selected = config.selected_columns.clone();
                            config.selected_columns.clear();
                            
                            // Re-select columns that exist in the new headers
                            for col in headers.iter() {
                                if old_selected.contains(col) || old_selected.is_empty() {
                                    // Keep previously selected columns or select all if none were selected
                                    config.selected_columns.insert(col.clone());
                                }
                            }
                            
                            // If no columns were selected (all old selections invalid), select all
                            if config.selected_columns.is_empty() {
                                for col in headers.iter() {
                                    config.selected_columns.insert(col.clone());
                                }
                            }
                            
                            // Update column types
                            config.column_types.clear();
                            for (header, dtype) in column_types {
                                config.column_types.insert(header, dtype);
                            }
                        }
                        
                        self.loading_progress = 1.0;
                        self.loading_message = "Type detection complete!".to_string();
                    }
                }
            }
            FileType::Sqlite => {
                // SQLite type inference is simpler - we can get types from schema
                self.loading_message = "Reading SQLite schema...".to_string();
                self.loading_progress = 0.5;
                
                use rusqlite::Connection;
                if let Ok(_conn) = Connection::open(&file_path) {
                    // For SQLite, we already have type information from the schema
                    self.loading_progress = 1.0;
                }
            }
        }
        
        // Hide loading state
        self.is_loading = false;
        self.loading_progress = 0.0;
        self.loading_message.clear();
    }
    
    /// Infer column type from sample values
    fn infer_column_type(&self, values: &[String]) -> SerializableDataType {
        if values.is_empty() {
            return SerializableDataType::Utf8;
        }
        
        let non_empty_values: Vec<&str> = values.iter()
            .filter(|v| !v.is_empty() && !v.trim().is_empty())
            .map(|s| s.trim())
            .collect();
        
        if non_empty_values.is_empty() {
            return SerializableDataType::Utf8;
        }
        
        // Track statistics for type inference
        let mut all_integers = true;
        let mut all_floats = true;
        let mut all_booleans = true;
        let mut has_decimal = false;
        
        for value in &non_empty_values {
            let v = value.trim();
            
            // Check for boolean patterns
            if all_booleans {
                let lower = v.to_lowercase();
                if !matches!(lower.as_str(), "true" | "false" | "1" | "0" | "yes" | "no" | "y" | "n") {
                    all_booleans = false;
                }
            }
            
            // Check for integer
            if all_integers {
                // Remove possible thousand separators and check
                let cleaned = v.replace(",", "").replace("_", "");
                if cleaned.parse::<i64>().is_err() {
                    all_integers = false;
                }
            }
            
            // Check for float
            if all_floats {
                let cleaned = v.replace(",", "").replace("_", "");
                if cleaned.contains('.') {
                    has_decimal = true;
                }
                if cleaned.parse::<f64>().is_err() {
                    all_floats = false;
                }
            }
        }
        
        // Return the most specific type that matches
        if all_booleans && non_empty_values.len() > 1 {
            return SerializableDataType::Boolean;
        }
        
        if all_integers {
            return SerializableDataType::Int64;
        }
        
        if all_floats {
            // Only return float if we actually saw decimal points
            // Otherwise integers like "123" would be detected as float
            if has_decimal {
                return SerializableDataType::Float64;
            } else {
                return SerializableDataType::Int64;
            }
        }
        
        // Check for dates (ISO format YYYY-MM-DD)
        if non_empty_values.iter().all(|v| {
            if v.len() == 10 && v.chars().nth(4) == Some('-') && v.chars().nth(7) == Some('-') {
                if let Ok(_) = chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d") {
                    return true;
                }
            }
            false
        }) {
            return SerializableDataType::Date32;
        }
        
        // Check for datetime/timestamp
        // Common formats: ISO 8601, RFC 3339, etc.
        let datetime_formats = [
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%d %H:%M:%S%.f",
            "%Y-%m-%dT%H:%M:%S%.f",
            "%Y-%m-%dT%H:%M:%SZ",
            "%Y-%m-%dT%H:%M:%S%.fZ",
            "%Y/%m/%d %H:%M:%S",
            "%d/%m/%Y %H:%M:%S",
            "%m/%d/%Y %H:%M:%S",
        ];
        
        for format in &datetime_formats {
            if non_empty_values.iter().all(|v| {
                chrono::NaiveDateTime::parse_from_str(v, format).is_ok() ||
                chrono::DateTime::parse_from_rfc3339(v).is_ok() ||
                chrono::DateTime::parse_from_rfc2822(v).is_ok()
            }) {
                return SerializableDataType::Timestamp;
            }
        }
        
        // Default to string
        SerializableDataType::Utf8
    }
    
    /// Show CSV configuration using full screen width
    fn show_csv_config_fullscreen(&mut self, ui: &mut Ui) {
        let Some(active_path) = self.config_manager.active_file.clone() else { return; };
        
        // Load preview if needed
        let needs_preview = self.config_manager.configs.get(&active_path)
            .map(|c| c.preview_lines.is_none())
            .unwrap_or(false);
            
        if needs_preview {
            let mut needs_type_inference = false;
            if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                use std::fs::File;
                use std::io::BufReader;
                
                if let Ok(file) = File::open(&config.path) {
                    let reader = BufReader::new(file);
                    let mut csv_reader = ::csv::ReaderBuilder::new()
                        .has_headers(false)
                        .from_reader(reader);
                    
                    let mut lines = Vec::new();
                    
                    for (idx, record) in csv_reader.records().enumerate() {
                        if idx >= 50 {  // Show 50 lines for better preview
                            break;
                        }
                        
                        if let Ok(record) = record {
                            let cells: Vec<String> = record.iter()
                                .map(|s| s.trim().to_string())
                                .collect();
                            lines.push(cells);
                        }
                    }
                    
                    config.preview_lines = Some(lines.clone());
                    
                    // Auto-detect columns from first line
                    if let Some(first_line) = lines.get(0) {
                        config.detected_columns = first_line.clone();
                        // Select all columns by default
                        if config.selected_columns.is_empty() {
                            config.selected_columns = first_line.iter().cloned().collect();
                        }
                    }
                    
                    // Check if we need type inference
                    needs_type_inference = config.column_types.is_empty() && !config.detected_columns.is_empty();
                }
            }
            
            // Run type inference after the mutable borrow is dropped
            if needs_type_inference {
                self.run_type_inference_for_config(&active_path);
            }
        }
        
        // Split into left panel (config) and right panel (preview)
        egui::SidePanel::left("config_panel")
            .resizable(true)
            .default_width(400.0)
            .min_width(350.0)
            .show_inside(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    // Header configuration
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("📋 Header Configuration").size(18.0).strong());
                        ui.add_space(8.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("Header Line:");
                            
                            // Convert from 0-indexed to 1-indexed for display
                            let mut header_line_display = self.config_manager.configs.get(&active_path)
                                .map(|c| c.header_line + 1)
                                .unwrap_or(1);
                            
                            let max_lines = self.config_manager.configs.get(&active_path)
                                .and_then(|c| c.preview_lines.as_ref())
                                .map(|lines| lines.len())
                                .unwrap_or(20) as usize;
                            
                            ui.add_space(10.0);
                            let response = ui.add(
                                DragValue::new(&mut header_line_display)
                                    .clamp_range(1..=max_lines)
                                    .speed(1)
                            );
                            
                            if response.changed() {
                                let needs_inference = if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                    config.header_line = header_line_display.saturating_sub(1);
                                    true
                                } else {
                                    false
                                };
                                
                                if needs_inference {
                                    self.run_type_inference_for_config(&active_path);
                                }
                            }
                            
                            ui.label(format!("(1-{})", max_lines));
                        });
                        
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("💡 The green highlighted row in the preview is your header. Data will be read from this row down.")
                                .size(12.0)
                                .color(Color32::from_gray(150))
                        );
                    });
                    
                    ui.add_space(12.0);
                    
                    // Type inference settings
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("🔍 Type Inference").size(18.0).strong());
                        ui.add_space(8.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("Sample Size:");
                            
                            let mut sample_size = self.config_manager.configs.get(&active_path)
                                .map(|c| c.sample_size)
                                .unwrap_or(1000);
                            
                            ui.add_space(10.0);
                            let response = ui.add(
                                DragValue::new(&mut sample_size)
                                    .clamp_range(100..=10000)
                                    .speed(10)
                            );
                            
                            if response.changed() {
                                if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                    config.sample_size = sample_size;
                                }
                            }
                            
                            ui.label("rows");
                            
                            ui.add_space(20.0);
                            
                            if ui.button("🔄 Re-detect Types")
                                .on_hover_text("Re-run automatic type detection")
                                .clicked() {
                                self.run_type_inference_for_config(&active_path);
                            }
                        });
                        

                    });
                    
                    ui.add_space(12.0);
                    
                    // Null handling
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("❌ Null Handling").size(18.0).strong());
                        ui.add_space(8.0);
                        
                        ui.label("Treat these values as null:");
                        
                        let null_patterns = self.config_manager.configs.get(&active_path)
                            .map(|c| c.null_config.patterns.clone())
                            .unwrap_or_default();
                        
                        ScrollArea::vertical()
                            .id_source("null_patterns_scroll")
                            .max_height(100.0)
                            .show(ui, |ui| {
                                let mut to_remove = None;
                                
                                for (idx, pattern) in null_patterns.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        if ui.small_button("×").clicked() {
                                            to_remove = Some(idx);
                                        }
                                        ui.monospace(if pattern.is_empty() { 
                                            "[empty string]" 
                                        } else { 
                                            pattern 
                                        });
                                    });
                                }
                                
                                if let Some(idx) = to_remove {
                                    if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                        config.null_config.patterns.remove(idx);
                                    }
                                }
                            });
                        
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.null_pattern_input);
                            if ui.button("Add").clicked() && !self.null_pattern_input.is_empty() {
                                if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                    config.null_config.patterns.push(self.null_pattern_input.clone());
                                    self.null_pattern_input.clear();
                                }
                            }
                        });
                    });
                    
                    ui.add_space(12.0);
                    
                    // Column Selection - will fill remaining space
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("📊 Column Selection").size(18.0).strong());
                        ui.add_space(8.0);
                        
                        let detected_columns = self.config_manager.configs.get(&active_path)
                            .map(|c| c.detected_columns.clone())
                            .unwrap_or_default();
                        let selected_count = self.config_manager.configs.get(&active_path)
                            .map(|c| c.selected_columns.len())
                            .unwrap_or(0);
                        
                        ui.horizontal(|ui| {
                            if ui.button("Select All").clicked() {
                                if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                    for col in &config.detected_columns {
                                        config.selected_columns.insert(col.clone());
                                    }
                                }
                            }
                            if ui.button("Deselect All").clicked() {
                                if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                    config.selected_columns.clear();
                                }
                            }
                            ui.label(format!("{} / {} selected", selected_count, detected_columns.len()));
                        });
                        
                        ui.separator();
                        
                        // This grid will now use all remaining vertical space
                        Grid::new("column_grid")
                            .num_columns(3)
                            .spacing([4.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(RichText::new("Include").strong());
                                ui.separator();
                                ui.label(RichText::new("Column Name").strong());
                                ui.separator();
                                ui.label(RichText::new("Detected Type").strong());
                                ui.end_row();
                                
                                for (col_idx, col_name) in detected_columns.iter().enumerate() {
                                    // Include checkbox
                                    let mut selected = self.config_manager.configs.get(&active_path)
                                        .map(|c| c.selected_columns.contains(col_name))
                                        .unwrap_or(false);
                                        
                                    if ui.checkbox(&mut selected, "").changed() {
                                        if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                            if selected {
                                                config.selected_columns.insert(col_name.clone());
                                            } else {
                                                config.selected_columns.remove(col_name);
                                            }
                                        }
                                    }
                                    
                                    ui.separator();
                                    
                                    // Column name
                                    ui.label(col_name);
                                    
                                    ui.separator();
                                    
                                    // Data type dropdown
                                    let current_type = self.config_manager.configs.get(&active_path)
                                        .and_then(|c| c.column_types.get(col_name))
                                        .cloned()
                                        .unwrap_or(SerializableDataType::Utf8);
                                    
                                    let type_label = format_serializable_type(&current_type);
                                    
                                    egui::ComboBox::from_id_source(format!("type_{}", col_idx))
                                        .selected_text(type_label)
                                        .width(100.0)
                                        .show_ui(ui, |ui| {
                                            let types = [
                                                (SerializableDataType::Utf8, "String"),
                                                (SerializableDataType::Int64, "Integer"),
                                                (SerializableDataType::Float64, "Float"),
                                                (SerializableDataType::Boolean, "Boolean"),
                                                (SerializableDataType::Date32, "Date"),
                                                (SerializableDataType::Timestamp, "DateTime"),
                                            ];
                                            
                                            for (dtype, label) in types {
                                                if ui.selectable_label(current_type == dtype, label).clicked() {
                                                    if let Some(config) = self.config_manager.configs.get_mut(&active_path) {
                                                        config.column_types.insert(col_name.clone(), dtype);
                                                    }
                                                }
                                            }
                                        });
                                    
                                    ui.end_row();
                                }
                            });
                    });
                });
            });
        
        // Right panel - Data preview uses remaining space
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.label(RichText::new("📊 Data Preview").size(18.0).strong());
            ui.add_space(8.0);
            
            let preview_data = self.config_manager.configs.get(&active_path)
                .and_then(|c| c.preview_lines.clone());
            let header_line_idx = self.config_manager.configs.get(&active_path)
                .map(|c| c.header_line)
                .unwrap_or(0);
            
            if let Some(preview) = preview_data {
                ScrollArea::both()
                    .id_source("csv_preview_scroll")
                    .show(ui, |ui| {
                        Grid::new("preview_grid")
                            .striped(true)
                            .spacing([12.0, 4.0])
                            .show(ui, |ui| {
                                // Show all rows with green highlighting for header
                                for (row_idx, row) in preview.iter().enumerate() {
                                    // Check if this is the header line
                                    if row_idx == header_line_idx {
                                        let header_color = Color32::from_rgb(100, 200, 100);
                                        ui.label(
                                            RichText::new((row_idx + 1).to_string())
                                                .color(header_color)
                                                .strong()
                                        );
                                        ui.separator();
                                        for (i, cell) in row.iter().enumerate() {
                                            if i > 0 {
                                                ui.separator();
                                            }
                                            ui.label(
                                                RichText::new(cell)
                                                    .color(header_color)
                                                    .strong()
                                            );
                                        }
                                        ui.end_row();
                                    } else {
                                        // Regular data row
                                        ui.label(
                                            RichText::new((row_idx + 1).to_string())
                                                .color(Color32::from_gray(150))
                                        );
                                        ui.separator();
                                        
                                        for (i, cell) in row.iter().enumerate() {
                                            if i > 0 {
                                                ui.separator();
                                            }
                                            ui.label(cell);
                                        }
                                        ui.end_row();
                                    }
                                }
                            });
                    });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Loading preview...");
                });
            }
        });
    }
}

/// Format serializable data type for display
fn format_serializable_type(dtype: &SerializableDataType) -> &'static str {
    match dtype {
        SerializableDataType::Boolean => "Boolean",
        SerializableDataType::Int32 => "Integer",
        SerializableDataType::Int64 => "Integer",
        SerializableDataType::Float32 => "Float",
        SerializableDataType::Float64 => "Float",
        SerializableDataType::Utf8 => "String",
        SerializableDataType::Date32 => "Date",
        SerializableDataType::Date64 => "Date",
        SerializableDataType::Timestamp => "DateTime",
    }
} 