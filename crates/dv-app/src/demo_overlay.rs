//! Demo Mode overlay showing all available examples

use egui::{Ui, Context, Vec2, Color32};
use crate::DemoExample;

/// Demo overlay state
pub struct DemoOverlay {
    /// Show the overlay
    pub show: bool,
    
    /// Selected example
    pub selected: Option<DemoExample>,
    
    /// Animation timer
    animation_timer: f32,
}

impl DemoOverlay {
    pub fn new() -> Self {
        Self {
            show: false,
            selected: None,
            animation_timer: 0.0,
        }
    }
    
    /// Show the demo overlay
    pub fn show(&mut self, ctx: &Context) -> Option<DemoExample> {
        if !self.show {
            return None;
        }
        
        let mut result = None;
        
        // Full-screen dark overlay - MORE OPAQUE for better visibility
        egui::Area::new("demo_overlay_bg")
            .fixed_pos([0.0, 0.0])
            .show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();
                ui.painter().rect_filled(
                    screen_rect,
                    0.0,
                    egui::Color32::from_black_alpha(200), // Increased from 150 to 200
                );
            });
        
        // Update animation with slower speed
        self.animation_timer += ctx.input(|i| i.stable_dt) * 0.5; // Reduced speed
        
        // Center content
        let content_size = Vec2::new(900.0, 600.0);
        let content_pos = ctx.screen_rect().center() - content_size * 0.5;
        
        egui::Area::new("demo_overlay_content")
            .fixed_pos(content_pos)
            .show(ctx, |ui| {
                // Background panel - make it brighter
                egui::Frame::none()
                    .fill(egui::Color32::from_gray(50)) // Increased from 35 to 50
                    .rounding(12.0)
                    .inner_margin(32.0)
                    .shadow(egui::epaint::Shadow::big_dark())
                    .show(ui, |ui| {
                        ui.set_max_width(content_size.x);
                        ui.set_max_height(content_size.y);
                        
                        // Header
                        ui.horizontal(|ui| {
                            ui.heading(egui::RichText::new("🐸 F.R.O.G. Demo Mode").size(28.0));
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("✖").on_hover_text("Close").clicked() {
                                    self.show = false;
                                }
                            });
                        });
                        
                        ui.label(egui::RichText::new("Choose an example dataset to explore").size(16.0).weak());
                        ui.add_space(20.0);
                        
                        // Examples grid
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.columns(2, |columns| {
                                for (idx, example) in [
                                    DemoExample::AssemblyLine,
                                    DemoExample::SensorNetwork,
                                    DemoExample::FinancialDashboard,
                                    DemoExample::SignalAnalysis,
                                ].iter().enumerate() {
                                    let col = &mut columns[idx % 2];
                                    
                                    if self.show_example_card(col, *example) {
                                        result = Some(*example);
                                        self.show = false;
                                    }
                                    
                                    col.add_space(16.0);
                                }
                            });
                            
                            ui.add_space(20.0);
                            
                            // File examples section
                            ui.separator();
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("📁 Load Sample Files").size(18.0).strong());
                            ui.add_space(10.0);
                            
                            ui.columns(2, |columns| {
                                // CSV files
                                columns[0].label(egui::RichText::new("CSV Files").size(14.0).strong());
                                for (name, desc, _path) in [
                                    ("💼 Sales Data", "Revenue & profit trends", "data/sales_data.csv"),
                                    ("🌡️ Sensor Readings", "Temperature, humidity data", "data/sensor_readings.csv"),
                                    ("💹 Stock Prices", "OHLCV market data", "data/stock_prices.csv"),
                                    ("⚙️ Assembly Line", "Manufacturing metrics", "data/assembly_line.csv"),
                                    ("🌐 Network Traffic", "Server performance", "data/network_traffic.csv"),
                                ] {
                                    if columns[0].button(name).on_hover_text(desc).clicked() {
                                        // TODO: Return file path to load
                                        self.show = false;
                                    }
                                }
                                
                                // SQLite tables
                                columns[1].label(egui::RichText::new("SQLite Tables").size(14.0).strong());
                                for (name, desc) in [
                                    ("📡 Sensor Telemetry", "IoT device data"),
                                    ("💳 Transactions", "Financial history"),
                                    ("⚙️ Production Metrics", "Manufacturing KPIs"),
                                ] {
                                    if columns[1].button(name).on_hover_text(desc).clicked() {
                                        // TODO: Return SQLite table to load
                                        self.show = false;
                                    }
                                }
                            });
                        });
                    });
            });
        
        // Handle escape key
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show = false;
        }
        
        result
    }
    
    /// Show an example card
    fn show_example_card(&self, ui: &mut Ui, example: DemoExample) -> bool {
        let mut clicked = false;
        
        let (icon, color) = match example {
            DemoExample::AssemblyLine => ("🏭", Color32::from_rgb(100, 150, 200)),
            DemoExample::SensorNetwork => ("📡", Color32::from_rgb(150, 200, 100)),
            DemoExample::FinancialDashboard => ("📈", Color32::from_rgb(200, 150, 100)),
            DemoExample::SignalAnalysis => ("📊", Color32::from_rgb(150, 100, 200)),
        };
        
        // Create the card frame
        let response = egui::Frame::none()
            .fill(color.linear_multiply(0.2))
            .rounding(8.0)
            .inner_margin(20.0)
            .show(ui, |ui| {
                ui.set_min_height(150.0);
                ui.vertical(|ui| {
                    // Icon and title
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(icon).size(32.0));
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(example.name()).size(18.0).strong());
                            ui.label(egui::RichText::new(example.description()).size(12.0).weak());
                        });
                    });
                    
                    ui.add_space(10.0);
                    
                    // Preview info
                    let preview_text = match example {
                        DemoExample::AssemblyLine => "• Station throughput monitoring\n• Efficiency & defect tracking\n• Real-time performance metrics",
                        DemoExample::SensorNetwork => "• Environmental sensor data\n• Network latency analysis\n• Geospatial visualization",
                        DemoExample::FinancialDashboard => "• Revenue & profit trends\n• Market cap tracking\n• Business KPI monitoring",
                        DemoExample::SignalAnalysis => "• Signal decomposition\n• Frequency analysis\n• Waveform visualization",
                    };
                    
                    ui.label(egui::RichText::new(preview_text).size(11.0).color(Color32::from_gray(200)));
                    
                    ui.add_space(10.0);
                    
                    // Load button
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() - 80.0);
                        if ui.button(egui::RichText::new("Load →").size(14.0)).clicked() {
                            clicked = true;
                        }
                    });
                });
            })
            .response;
        
        // Make entire card interactive and show hover effect
        let response = response.interact(egui::Sense::hover());
        if response.hovered() {
            ui.painter().rect(
                response.rect,
                8.0,
                Color32::TRANSPARENT,
                egui::Stroke::new(2.0, color.linear_multiply(0.6))
            );
        }
        
        if response.interact(egui::Sense::click()).clicked() {
            clicked = true;
        }
        
        clicked
    }
} 