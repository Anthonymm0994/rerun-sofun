//! Geographic plot implementation

use egui::{Ui, Color32, Pos2, Vec2, Rect, Stroke, Shape, FontId, Rounding, Align2};
use egui_plot::{Plot, PlotUi, Points, PlotPoints, Legend, PlotBounds, Text};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, StringArray, Array};
use serde_json::{json, Value};
use geo_types::{Point, LineString, Polygon as GeoPolygon};
use geojson::{GeoJson, Feature, FeatureCollection};
use rstar::{RTree, RTreeObject, AABB};
use std::collections::HashMap;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use super::utils::colors::{ColorScheme, viridis_color, plasma_color, diverging_color};

/// Geographic plot configuration
#[derive(Debug, Clone)]
pub struct GeoPlotConfig {
    pub data_source_id: Option<String>,
    pub lat_column: String,
    pub lon_column: String,
    pub value_column: Option<String>,
    pub label_column: Option<String>,
    pub group_column: Option<String>,
    
    // Map settings
    pub projection: ProjectionType,
    pub show_coastlines: bool,
    pub show_countries: bool,
    pub show_grid: bool,
    pub show_labels: bool,
    
    // Visualization type
    pub viz_type: GeoVizType,
    pub marker_size: f32,
    pub heatmap_radius: f32,
    pub color_scheme: ColorScheme,
    
    // Interactivity
    pub enable_zoom: bool,
    pub enable_pan: bool,
    pub show_tooltips: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectionType {
    Mercator,
    Robinson,
    AlbersEqualArea,
    Orthographic,
    NaturalEarth,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GeoVizType {
    Points,
    Heatmap,
    Choropleth,
    Lines,
    Hexbin,
}

impl Default for GeoPlotConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            lat_column: String::new(),
            lon_column: String::new(),
            value_column: None,
            label_column: None,
            group_column: None,
            projection: ProjectionType::Mercator,
            show_coastlines: true,
            show_countries: true,
            show_grid: true,
            show_labels: true,
            viz_type: GeoVizType::Points,
            marker_size: 5.0,
            heatmap_radius: 20.0,
            color_scheme: ColorScheme::Viridis,
            enable_zoom: true,
            enable_pan: true,
            show_tooltips: true,
        }
    }
}

/// Geographic plot view
pub struct GeoPlot {
    id: SpaceViewId,
    title: String,
    pub config: GeoPlotConfig,
    
    // State
    cached_data: Option<RecordBatch>,
    
    // Map state
    center_lat: f64,
    center_lon: f64,
    zoom_level: f64,
    
    // Cached geometry
    coastlines: Vec<Vec<Pos2>>,
    countries: Vec<(Vec<Vec<Pos2>>, String)>,
    spatial_index: Option<RTree<GeoPoint>>,
}

#[derive(Clone)]
struct GeoPoint {
    lat: f64,
    lon: f64,
    value: Option<f64>,
    label: Option<String>,
    group: Option<String>,
    index: usize,
}

impl rstar::RTreeObject for GeoPoint {
    type Envelope = AABB<[f64; 2]>;
    
    fn envelope(&self) -> Self::Envelope {
        AABB::from_point([self.lon, self.lat])
    }
}

impl GeoPlot {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        let mut plot = Self {
            id,
            title,
            config: GeoPlotConfig::default(),
            cached_data: None,
            center_lat: 0.0,
            center_lon: 0.0,
            zoom_level: 1.0,
            coastlines: Vec::new(),
            countries: Vec::new(),
            spatial_index: None,
        };
        
        // Load basic world geometry
        plot.load_world_geometry();
        plot
    }
    
    fn load_world_geometry(&mut self) {
        // For now, create simplified world boundaries
        // In a real implementation, you'd load from Natural Earth data or similar
        
        // Simple world outline
        let world_outline = vec![
            vec![
                Pos2::new(-180.0, -90.0),
                Pos2::new(180.0, -90.0),
                Pos2::new(180.0, 90.0),
                Pos2::new(-180.0, 90.0),
                Pos2::new(-180.0, -90.0),
            ]
        ];
        self.coastlines = world_outline;
        
        // Add some example country boundaries
        // North America simplified
        let north_america = vec![
            Pos2::new(-170.0, 70.0),
            Pos2::new(-50.0, 70.0),
            Pos2::new(-50.0, 25.0),
            Pos2::new(-120.0, 25.0),
            Pos2::new(-170.0, 50.0),
            Pos2::new(-170.0, 70.0),
        ];
        
        // Europe simplified
        let europe = vec![
            Pos2::new(-10.0, 35.0),
            Pos2::new(40.0, 35.0),
            Pos2::new(40.0, 70.0),
            Pos2::new(-10.0, 70.0),
            Pos2::new(-10.0, 35.0),
        ];
        
        self.countries = vec![
            (vec![north_america], "North America".to_string()),
            (vec![europe], "Europe".to_string()),
        ];
    }
    
    fn project_point(&self, lat: f64, lon: f64, rect: &Rect) -> Pos2 {
        match self.config.projection {
            ProjectionType::Mercator => {
                // Simple Mercator projection
                let x = (lon + 180.0) / 360.0;
                let lat_rad = lat.to_radians();
                let y = 0.5 - (lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / (2.0 * std::f64::consts::PI);
                
                Pos2::new(
                    rect.left() + x as f32 * rect.width(),
                    rect.top() + y as f32 * rect.height()
                )
            }
            _ => {
                // Fallback to simple equirectangular
                let x = (lon + 180.0) / 360.0;
                let y = (90.0 - lat) / 180.0;
                
                Pos2::new(
                    rect.left() + x as f32 * rect.width(),
                    rect.top() + y as f32 * rect.height()
                )
            }
        }
    }
    
    fn draw_base_map(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        
        // Background
        painter.rect_filled(rect, Rounding::ZERO, Color32::from_rgb(230, 240, 250));
        
        // Draw grid
        if self.config.show_grid {
            // Latitude lines
            for lat in (-90..=90).step_by(30) {
                let start = self.project_point(lat as f64, -180.0, &rect);
                let end = self.project_point(lat as f64, 180.0, &rect);
                painter.line_segment(
                    [start, end],
                    Stroke::new(0.5, Color32::from_gray(200))
                );
                
                if self.config.show_labels && lat % 60 == 0 {
                    painter.text(
                        Pos2::new(rect.left() + 5.0, (start.y + end.y) / 2.0),
                        Align2::LEFT_CENTER,
                        format!("{}°", lat),
                        FontId::proportional(10.0),
                        Color32::from_gray(100),
                    );
                }
            }
            
            // Longitude lines
            for lon in (-180..=180).step_by(60) {
                let start = self.project_point(90.0, lon as f64, &rect);
                let end = self.project_point(-90.0, lon as f64, &rect);
                painter.line_segment(
                    [start, end],
                    Stroke::new(0.5, Color32::from_gray(200))
                );
                
                if self.config.show_labels && lon % 120 == 0 {
                    painter.text(
                        Pos2::new((start.x + end.x) / 2.0, rect.bottom() - 5.0),
                        Align2::CENTER_BOTTOM,
                        format!("{}°", lon),
                        FontId::proportional(10.0),
                        Color32::from_gray(100),
                    );
                }
            }
        }
        
        // Draw coastlines
        if self.config.show_coastlines {
            for coastline in &self.coastlines {
                let points: Vec<Pos2> = coastline.iter()
                    .map(|p| self.project_point(p.y as f64, p.x as f64, &rect))
                    .collect();
                
                if points.len() > 1 {
                    painter.add(Shape::line(
                        points,
                        Stroke::new(1.0, Color32::from_rgb(50, 50, 150))
                    ));
                }
            }
        }
        
        // Draw countries
        if self.config.show_countries {
            for (polygons, name) in &self.countries {
                for polygon in polygons {
                    let points: Vec<Pos2> = polygon.iter()
                        .map(|p| self.project_point(p.y as f64, p.x as f64, &rect))
                        .collect();
                    
                    if points.len() > 2 {
                        painter.add(Shape::closed_line(
                            points.clone(),
                            Stroke::new(0.5, Color32::from_gray(150))
                        ));
                        
                        // Add country label
                        if self.config.show_labels {
                            let center = points.iter()
                                .fold(Pos2::ZERO, |acc, p| acc + p.to_vec2())
                                / points.len() as f32;
                            
                            painter.text(
                                center,
                                Align2::CENTER_CENTER,
                                name,
                                FontId::proportional(12.0),
                                Color32::from_gray(80),
                            );
                        }
                    }
                }
            }
        }
    }
    
    fn draw_data_points(&self, ui: &mut Ui, rect: Rect, points: &[GeoPoint]) {
        let painter = ui.painter_at(rect);
        
        match self.config.viz_type {
            GeoVizType::Points => {
                // Draw points as markers
                for point in points {
                    let pos = self.project_point(point.lat, point.lon, &rect);
                    
                    let color = if let Some(value) = point.value {
                        // Color by value
                        let normalized = (value - 0.0) / 100.0; // TODO: proper normalization
                        match self.config.color_scheme {
                            ColorScheme::Viridis => viridis_color(normalized as f32),
                            ColorScheme::Plasma => plasma_color(normalized as f32),
                            _ => Color32::from_rgb(255, 100, 100),
                        }
                    } else {
                        Color32::from_rgb(255, 100, 100)
                    };
                    
                    // Draw marker
                    painter.circle_filled(pos, self.config.marker_size, color);
                    painter.circle_stroke(pos, self.config.marker_size, Stroke::new(1.0, Color32::WHITE));
                    
                    // Draw label if available
                    if self.config.show_labels {
                        if let Some(label) = &point.label {
                            painter.text(
                                pos + Vec2::new(self.config.marker_size + 2.0, 0.0),
                                Align2::LEFT_CENTER,
                                label,
                                FontId::proportional(10.0),
                                Color32::BLACK,
                            );
                        }
                    }
                }
            }
            GeoVizType::Heatmap => {
                // Create heatmap visualization
                // For simplicity, we'll draw gradients around each point
                for point in points {
                    let pos = self.project_point(point.lat, point.lon, &rect);
                    let value = point.value.unwrap_or(1.0);
                    
                    // Draw gradient circle
                    let radius = self.config.heatmap_radius;
                    for r in (0..radius as i32).rev() {
                        let alpha = (1.0 - (r as f32 / radius)) * value as f32 / 100.0;
                        let color = match self.config.color_scheme {
                            ColorScheme::Viridis => {
                                let c = viridis_color(alpha);
                                Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (alpha * 128.0) as u8)
                            }
                            _ => Color32::from_rgba_unmultiplied(255, 0, 0, (alpha * 128.0) as u8),
                        };
                        painter.circle_filled(pos, r as f32, color);
                    }
                }
            }
            GeoVizType::Hexbin => {
                // Hexagonal binning
                let hex_size = 20.0;
                let mut hex_bins: HashMap<(i32, i32), Vec<&GeoPoint>> = HashMap::new();
                
                // Bin points into hexagons
                for point in points {
                    let pos = self.project_point(point.lat, point.lon, &rect);
                    let hex_x = (pos.x / (hex_size * 1.5)) as i32;
                    let hex_y = (pos.y / (hex_size * 0.866)) as i32;
                    hex_bins.entry((hex_x, hex_y)).or_default().push(point);
                }
                
                // Draw hexagons
                for ((hx, hy), bin_points) in hex_bins {
                    let center_x = hx as f32 * hex_size * 1.5;
                    let center_y = hy as f32 * hex_size * 0.866;
                    
                    // Calculate average value
                    let avg_value = bin_points.iter()
                        .filter_map(|p| p.value)
                        .sum::<f64>() / bin_points.len() as f64;
                    
                    let normalized = (avg_value / 100.0) as f32; // TODO: proper normalization
                    let color = match self.config.color_scheme {
                        ColorScheme::Viridis => viridis_color(normalized),
                        ColorScheme::Plasma => plasma_color(normalized),
                        _ => Color32::from_rgb(255, 100, 100),
                    };
                    
                    // Draw hexagon
                    let mut hex_points = Vec::new();
                    for i in 0..6 {
                        let angle = i as f32 * std::f32::consts::PI / 3.0;
                        hex_points.push(Pos2::new(
                            center_x + hex_size * angle.cos(),
                            center_y + hex_size * angle.sin()
                        ));
                    }
                    
                    painter.add(Shape::convex_polygon(
                        hex_points,
                        color,
                        Stroke::new(0.5, Color32::from_gray(100))
                    ));
                }
            }
            _ => {}
        }
    }
    
    fn handle_interaction(&mut self, ui: &mut Ui, rect: Rect, points: &[GeoPoint]) {
        let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
        
        // Pan
        if self.config.enable_pan && response.dragged() {
            let delta = response.drag_delta();
            self.center_lon -= (delta.x as f64 / rect.width() as f64) * 360.0 / self.zoom_level;
            self.center_lat += (delta.y as f64 / rect.height() as f64) * 180.0 / self.zoom_level;
            
            // Clamp values
            self.center_lat = self.center_lat.clamp(-90.0, 90.0);
            self.center_lon = ((self.center_lon + 180.0) % 360.0) - 180.0;
        }
        
        // Zoom
        if self.config.enable_zoom {
            let scroll_delta = ui.input(|i| i.scroll_delta.y);
            if scroll_delta != 0.0 {
                self.zoom_level *= 1.1_f64.powf(scroll_delta as f64 / 100.0);
                self.zoom_level = self.zoom_level.clamp(0.5, 10.0);
            }
        }
        
        // Tooltips
        if self.config.show_tooltips && response.hovered() {
            if let Some(hover_pos) = response.hover_pos() {
                // Find nearest point
                let hover_rel = hover_pos - rect.left_top();
                
                let mut nearest_point = None;
                let mut min_dist = f32::INFINITY;
                
                for point in points {
                    let pos = self.project_point(point.lat, point.lon, &rect);
                    let dist = (pos - hover_pos).length();
                    if dist < min_dist && dist < 20.0 {
                        min_dist = dist;
                        nearest_point = Some(point);
                    }
                }
                
                if let Some(point) = nearest_point {
                    let mut tooltip_text = format!("Lat: {:.4}, Lon: {:.4}", point.lat, point.lon);
                    if let Some(value) = point.value {
                        tooltip_text.push_str(&format!("\nValue: {:.2}", value));
                    }
                    if let Some(label) = &point.label {
                        tooltip_text.push_str(&format!("\n{}", label));
                    }
                    
                    response.on_hover_text(tooltip_text);
                }
            }
        }
    }
}

impl SpaceView for GeoPlot {
    fn id(&self) -> SpaceViewId { self.id }
    fn title(&self) -> &str {
        &self.title
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn display_name(&self) -> &str { &self.title }
    fn view_type(&self) -> &str { "GeoPlotView" }
    
    fn set_data_source(&mut self, source_id: String) {
        self.config.data_source_id = Some(source_id);
        self.cached_data = None; // Clear cache when source changes
    }
    
    fn data_source_id(&self) -> Option<&str> {
        self.config.data_source_id.as_deref()
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if needed
        if self.cached_data.is_none() {
            let data_sources = ctx.data_sources.read();

            let data_source = if let Some(source_id) = &self.config.data_source_id {
                data_sources.get(source_id)
            } else {
                data_sources.values().next()
            };
            if let Some(source) = data_source.as_ref() {
                let nav_pos = ctx.navigation.get_context().position.clone();
                if let Ok(batch) = ctx.runtime_handle.block_on(source.query_at(&nav_pos)) {
                    self.cached_data = Some(batch);
                }
            }
        }
        
        if let Some(batch) = &self.cached_data {
            // Extract lat/lon data
            let points = self.extract_geo_points(batch);
            if points.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("No geographic data available. Please configure latitude and longitude columns.");
                });
                return;
            }
            
            // Build spatial index if needed
            if self.spatial_index.is_none() && !points.is_empty() {
                self.spatial_index = Some(RTree::bulk_load(points.clone()));
            }
            
            // Main plot area
            let available_rect = ui.available_rect_before_wrap();
            let plot_rect = Rect::from_min_size(
                available_rect.left_top() + Vec2::new(10.0, 10.0),
                available_rect.size() - Vec2::new(20.0, 20.0)
            );
            
            // Draw base map
            self.draw_base_map(ui, plot_rect);
            
            // Draw data
            self.draw_data_points(ui, plot_rect, &points);
            
            // Handle interactions
            self.handle_interaction(ui, plot_rect, &points);
            
            // Draw legend and controls
            self.draw_controls(ui);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Loading geographic data...");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "data_source_id": self.config.data_source_id,
            "lat_column": self.config.lat_column,
            "lon_column": self.config.lon_column,
            "value_column": self.config.value_column,
            "label_column": self.config.label_column,
            "group_column": self.config.group_column,
            "projection": format!("{:?}", self.config.projection),
            "viz_type": format!("{:?}", self.config.viz_type),
            "marker_size": self.config.marker_size,
            "heatmap_radius": self.config.heatmap_radius,
            "color_scheme": format!("{:?}", self.config.color_scheme),
            "show_coastlines": self.config.show_coastlines,
            "show_countries": self.config.show_countries,
            "show_grid": self.config.show_grid,
            "show_labels": self.config.show_labels,
            "enable_zoom": self.config.enable_zoom,
            "enable_pan": self.config.enable_pan,
            "show_tooltips": self.config.show_tooltips,
            "center_lat": self.center_lat,
            "center_lon": self.center_lon,
            "zoom_level": self.zoom_level,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(data_source_id) = config.get("data_source_id").and_then(|v| v.as_str()) {
            self.config.data_source_id = Some(data_source_id.to_string());
        }
        if let Some(lat) = config.get("lat_column").and_then(|v| v.as_str()) {
            self.config.lat_column = lat.to_string();
        }
        if let Some(lon) = config.get("lon_column").and_then(|v| v.as_str()) {
            self.config.lon_column = lon.to_string();
        }
        if let Some(value) = config.get("value_column").and_then(|v| v.as_str()) {
            self.config.value_column = Some(value.to_string());
        }
        if let Some(label) = config.get("label_column").and_then(|v| v.as_str()) {
            self.config.label_column = Some(label.to_string());
        }
        if let Some(group) = config.get("group_column").and_then(|v| v.as_str()) {
            self.config.group_column = Some(group.to_string());
        }
        if let Some(center_lat) = config.get("center_lat").and_then(|v| v.as_f64()) {
            self.center_lat = center_lat;
        }
        if let Some(center_lon) = config.get("center_lon").and_then(|v| v.as_f64()) {
            self.center_lon = center_lon;
        }
        if let Some(zoom) = config.get("zoom_level").and_then(|v| v.as_f64()) {
            self.zoom_level = zoom;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
}

impl GeoPlot {
    fn extract_geo_points(&self, batch: &RecordBatch) -> Vec<GeoPoint> {
        let mut points = Vec::new();
        
        // Find columns
        let lat_col_idx = batch.schema().fields().iter()
            .position(|f| f.name() == &self.config.lat_column);
        let lon_col_idx = batch.schema().fields().iter()
            .position(|f| f.name() == &self.config.lon_column);
            
        if lat_col_idx.is_none() || lon_col_idx.is_none() {
            return points;
        }
        
        let lat_col = batch.column(lat_col_idx.unwrap());
        let lon_col = batch.column(lon_col_idx.unwrap());
        
        // Extract lat/lon values
        if let (Some(lat_array), Some(lon_array)) = (
            lat_col.as_any().downcast_ref::<Float64Array>(),
            lon_col.as_any().downcast_ref::<Float64Array>()
        ) {
            // Extract optional value column
            let value_array = self.config.value_column.as_ref()
                .and_then(|col_name| {
                    batch.schema().fields().iter()
                        .position(|f| f.name() == col_name)
                        .and_then(|idx| batch.column(idx).as_any().downcast_ref::<Float64Array>())
                });
            
            // Extract optional label column
            let label_array = self.config.label_column.as_ref()
                .and_then(|col_name| {
                    batch.schema().fields().iter()
                        .position(|f| f.name() == col_name)
                        .and_then(|idx| batch.column(idx).as_any().downcast_ref::<StringArray>())
                });
                
            // Extract optional group column
            let group_array = self.config.group_column.as_ref()
                .and_then(|col_name| {
                    batch.schema().fields().iter()
                        .position(|f| f.name() == col_name)
                        .and_then(|idx| batch.column(idx).as_any().downcast_ref::<StringArray>())
                });
            
            for i in 0..lat_array.len() {
                let (lat, lon) = (lat_array.value(i), lon_array.value(i));
                points.push(GeoPoint {
                    lat,
                    lon,
                    value: value_array.and_then(|arr| Some(arr.value(i))),
                    label: label_array.and_then(|arr| arr.value(i).to_string().into()),
                    group: group_array.and_then(|arr| arr.value(i).to_string().into()),
                    index: i,
                });
            }
        }
        
        points
    }
    
    fn draw_controls(&self, ui: &mut Ui) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Map Controls:");
            ui.label(format!("Center: {:.2}, {:.2}", self.center_lat, self.center_lon));
            ui.label(format!("Zoom: {:.1}x", self.zoom_level));
            
            if ui.button("Reset View").clicked() {
                // Reset will be handled in next frame
            }
            
            ui.separator();
            
            // Visualization type selector
            ui.label("Type:");
            let viz_type_text = format!("{:?}", self.config.viz_type);
            ui.label(&viz_type_text);
        });
    }
} 