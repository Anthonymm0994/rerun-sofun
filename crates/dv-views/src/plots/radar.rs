//! Radar/Spider chart for multivariate data comparison

use egui::{Ui, Color32, Rect, Pos2, Vec2, Stroke, FontId, Align2, Shape, Response, Sense};
use egui_plot::{PlotUi, Polygon, Text, Line, PlotPoints, Legend, Corner};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, StringArray, Int64Array};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use super::utils::{ColorScheme, categorical_color};

/// Radar chart configuration
#[derive(Debug, Clone)]
pub struct RadarConfig {
    pub data_source_id: Option<String>,
    pub value_columns: Vec<String>,
    pub group_column: Option<String>,
    
    // Scaling options
    pub scale_type: ScaleType,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub num_rings: usize,
    
    // Visual options
    pub fill_opacity: u8,
    pub line_width: f32,
    pub show_points: bool,
    pub point_size: f32,
    pub show_grid: bool,
    pub show_values: bool,
    pub show_legend: bool,
    pub color_scheme: ColorScheme,
    
    // Aggregation for multiple rows
    pub aggregation: AggregationType,
    pub show_error_bars: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScaleType {
    Linear,
    Logarithmic,
    Normalized,  // 0-1 per axis
    PercentRank, // percentile rank
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggregationType {
    Mean,
    Median,
    Sum,
    Min,
    Max,
    First,
    Last,
}

impl Default for RadarConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            value_columns: Vec::new(),
            group_column: None,
            scale_type: ScaleType::Linear,
            min_value: None,
            max_value: None,
            num_rings: 5,
            fill_opacity: 77,
            line_width: 2.0,
            show_points: true,
            point_size: 4.0,
            show_grid: true,
            show_values: false,
            show_legend: true,
            color_scheme: ColorScheme::Categorical,
            aggregation: AggregationType::Mean,
            show_error_bars: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RadarChartConfig {
    pub value_columns: Vec<String>,
    pub group_column: Option<String>,
    
    // Scaling options
    pub scale_type: ScaleType,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub num_rings: usize,
    
    // Visual options
    pub fill_opacity: u8,
    pub line_width: f32,
    pub show_points: bool,
    pub point_size: f32,
    pub show_grid: bool,
    pub show_values: bool,
    pub show_legend: bool,
    pub color_scheme: ColorScheme,
    
    // Aggregation for multiple rows
    pub aggregation: AggregationType,
    pub show_error_bars: bool,
}

#[derive(Clone, Debug)]
struct RadarSeries {
    name: String,
    values: Vec<f64>,
    std_devs: Vec<f64>,
    color: Color32,
}

/// Radar/Spider chart view
pub struct RadarChart {
    id: SpaceViewId,
    title: String,
    pub config: RadarConfig,
    
    // State
    cached_data: Option<RecordBatch>,
    series: Vec<RadarSeries>,
    axis_labels: Vec<String>,
    axis_min_max: Vec<(f64, f64)>,
}

impl RadarChart {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: RadarConfig::default(),
            cached_data: None,
            series: Vec::new(),
            axis_labels: Vec::new(),
            axis_min_max: Vec::new(),
        }
    }
    
    fn extract_data(&mut self, batch: &RecordBatch) {
        self.series.clear();
        self.axis_labels.clear();
        self.axis_min_max.clear();
        
        // Use all numeric columns if none specified
        if self.config.value_columns.is_empty() {
            for field in batch.schema().fields() {
                if matches!(field.data_type(), arrow::datatypes::DataType::Float64 | 
                                             arrow::datatypes::DataType::Int64) {
                    self.config.value_columns.push(field.name().to_string());
                }
            }
        }
        
        // Set axis labels
        self.axis_labels = self.config.value_columns.clone();
        
        // Extract group column if specified
        let group_array = self.config.group_column.as_ref()
            .and_then(|col_name| {
                batch.schema().fields().iter()
                    .position(|f| f.name() == col_name)
                    .and_then(|idx| batch.column(idx).as_any().downcast_ref::<StringArray>())
            });
        
        // Group data by group column (or use single group)
        let mut grouped_data: HashMap<String, Vec<Vec<Option<f64>>>> = HashMap::new();
        
        for row_idx in 0..batch.num_rows() {
            let group = if let Some(group_arr) = &group_array {
                group_arr.value(row_idx).to_string()
            } else {
                "All Data".to_string()
            };
            
            let mut row_values: Vec<Option<f64>> = Vec::new();
            for col_name in &self.config.value_columns {
                if let Some(col_idx) = batch.schema().fields().iter()
                    .position(|f| f.name() == col_name) {
                    
                    let col = batch.column(col_idx);
                    if let Some(float_array) = col.as_any().downcast_ref::<Float64Array>() {
                        row_values.push(Some(float_array.value(row_idx)));
                    } else if let Some(int_array) = col.as_any().downcast_ref::<Int64Array>() {
                        row_values.push(Some(int_array.value(row_idx) as f64));
                    } else {
                        row_values.push(None);
                    }
                }
            }
            
            grouped_data.entry(group).or_insert_with(Vec::new).push(row_values);
        }
        
        // Calculate min/max for each axis
        for (i, col_name) in self.config.value_columns.iter().enumerate() {
            let mut all_values = Vec::new();
            for rows in grouped_data.values() {
                for row in rows {
                    if let Some(Some(val)) = row.get(i) {
                        all_values.push(*val);
                    }
                }
            }
            
            if !all_values.is_empty() {
                let min = all_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max = all_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                self.axis_min_max.push((min, max));
            } else {
                self.axis_min_max.push((0.0, 1.0));
            }
        }
        
        // Apply global min/max if specified
        if let (Some(min), Some(max)) = (self.config.min_value, self.config.max_value) {
            for (axis_min, axis_max) in &mut self.axis_min_max {
                *axis_min = min;
                *axis_max = max;
            }
        }
        
        // Aggregate data for each group
        let mut series_idx = 0;
        for (group_name, rows) in grouped_data {
            let mut aggregated_values = Vec::new();
            let mut std_devs = Vec::new();
            
            for col_idx in 0..self.config.value_columns.len() {
                let values: Vec<f64> = rows.iter()
                    .filter_map(|row| row.get(col_idx).and_then(|v| *v))
                    .collect();
                
                if values.is_empty() {
                    aggregated_values.push(0.0);
                    std_devs.push(0.0);
                    continue;
                }
                
                let aggregated = match self.config.aggregation {
                    AggregationType::Mean => values.iter().sum::<f64>() / values.len() as f64,
                    AggregationType::Median => {
                        let mut sorted = values.clone();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        if sorted.len() % 2 == 0 {
                            (sorted[sorted.len()/2 - 1] + sorted[sorted.len()/2]) / 2.0
                        } else {
                            sorted[sorted.len()/2]
                        }
                    }
                    AggregationType::Sum => values.iter().sum(),
                    AggregationType::Min => values.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
                    AggregationType::Max => values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
                    AggregationType::First => values.first().copied().unwrap_or(0.0),
                    AggregationType::Last => values.last().copied().unwrap_or(0.0),
                };
                
                // Calculate standard deviation for error bars
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
                let std_dev = variance.sqrt();
                
                aggregated_values.push(aggregated);
                std_devs.push(std_dev);
            }
            
            // Scale values based on scale type
            let scaled_values = aggregated_values.iter().enumerate().map(|(i, &val)| {
                let (min, max) = self.axis_min_max[i];
                match self.config.scale_type {
                    ScaleType::Linear => val,
                    ScaleType::Logarithmic => {
                        if val > 0.0 && min > 0.0 {
                            val.ln()
                        } else {
                            0.0
                        }
                    }
                    ScaleType::Normalized => {
                        if max > min {
                            (val - min) / (max - min)
                        } else {
                            0.5
                        }
                    }
                    ScaleType::PercentRank => {
                        // Simple percent rank - would need all values for proper implementation
                        if max > min {
                            (val - min) / (max - min) * 100.0
                        } else {
                            50.0
                        }
                    }
                }
            }).collect();
            
            self.series.push(RadarSeries {
                name: group_name,
                values: scaled_values,
                std_devs,
                color: categorical_color(series_idx),
            });
            series_idx += 1;
        }
    }
    
    fn plot_radar(&self, plot_ui: &mut PlotUi) {
        let num_axes = self.axis_labels.len();
        if num_axes < 3 {
            return; // Need at least 3 axes for a radar chart
        }
        
        let angle_step = 2.0 * std::f64::consts::PI / num_axes as f64;
        
        // Determine radar scale
        let radar_max = match self.config.scale_type {
            ScaleType::Normalized => 1.0,
            ScaleType::PercentRank => 100.0,
            _ => {
                // Find max value across all series
                self.series.iter()
                    .flat_map(|s| &s.values)
                    .fold(0.0_f64, |a, &b| a.max(b.abs()))
                    .max(1.0)
            }
        };
        
        // Draw grid
        if self.config.show_grid {
            // Draw concentric rings
            for ring in 1..=self.config.num_rings {
                let radius = (ring as f64 / self.config.num_rings as f64) * radar_max;
                let mut ring_points = Vec::new();
                
                for i in 0..=num_axes {
                    let angle = i as f64 * angle_step - std::f64::consts::PI / 2.0;
                    ring_points.push([radius * angle.cos(), radius * angle.sin()]);
                }
                
                plot_ui.line(Line::new(PlotPoints::new(ring_points))
                    .color(Color32::from_gray(100))
                    .width(0.5));
            }
            
            // Draw axes
            for i in 0..num_axes {
                let angle = i as f64 * angle_step - std::f64::consts::PI / 2.0;
                let end_x = radar_max * angle.cos();
                let end_y = radar_max * angle.sin();
                
                plot_ui.line(Line::new(PlotPoints::new(vec![[0.0, 0.0], [end_x, end_y]]))
                    .color(Color32::from_gray(100))
                    .width(0.5));
                
                // Add axis labels
                let label_distance = radar_max * 1.15;
                let label_x = label_distance * angle.cos();
                let label_y = label_distance * angle.sin();
                
                plot_ui.text(Text::new([label_x, label_y].into(), &self.axis_labels[i])
                    .color(Color32::from_gray(200))
                    .anchor(Align2::CENTER_CENTER));
            }
            
            // Draw ring labels
            for ring in 1..=self.config.num_rings {
                let value = (ring as f64 / self.config.num_rings as f64) * radar_max;
                let label = match self.config.scale_type {
                    ScaleType::Normalized => format!("{:.1}", value),
                    ScaleType::PercentRank => format!("{:.0}%", value),
                    _ => format!("{:.1}", value),
                };
                
                plot_ui.text(Text::new([0.0, -value].into(), label)
                    .color(Color32::from_gray(150))
                    .anchor(Align2::LEFT_CENTER));
            }
        }
        
        // Draw data series
        for series in &self.series {
            let mut polygon_points = Vec::new();
            let mut line_points = Vec::new();
            
            for (i, &value) in series.values.iter().enumerate() {
                let angle = i as f64 * angle_step - std::f64::consts::PI / 2.0;
                let x = value * angle.cos();
                let y = value * angle.sin();
                polygon_points.push([x, y]);
                line_points.push([x, y]);
            }
            
            // Close the polygon
            if let Some(first) = polygon_points.first() {
                line_points.push(*first);
            }
            
            // Draw filled polygon
            let fill_color = Color32::from_rgba_unmultiplied(
                series.color.r(),
                series.color.g(),
                series.color.b(),
                self.config.fill_opacity
            );
            
            plot_ui.polygon(Polygon::new(PlotPoints::new(polygon_points.clone()))
                .fill_color(fill_color)
                .stroke(Stroke::new(self.config.line_width, series.color))
                .name(&series.name));
            
            // Draw points
            if self.config.show_points {
                for point in &polygon_points {
                    plot_ui.points(egui_plot::Points::new(PlotPoints::new(vec![*point]))
                        .color(series.color)
                        .radius(self.config.point_size)
                        .name(&series.name));
                }
            }
            
            // Draw values
            if self.config.show_values {
                for (i, (&value, point)) in series.values.iter().zip(&polygon_points).enumerate() {
                    let (min, max) = self.axis_min_max.get(i).copied().unwrap_or((0.0, 1.0));
                    let original_value = match self.config.scale_type {
                        ScaleType::Normalized => value * (max - min) + min,
                        _ => value,
                    };
                    
                    plot_ui.text(Text::new((*point).into(), format!("{:.1}", original_value))
                        .color(series.color)
                        .anchor(Align2::CENTER_BOTTOM));
                }
            }
            
            // Draw error bars if enabled
            if self.config.show_error_bars && self.config.aggregation == AggregationType::Mean {
                for (i, (&value, &std_dev)) in series.values.iter().zip(&series.std_devs).enumerate() {
                    let angle = i as f64 * angle_step - std::f64::consts::PI / 2.0;
                    let cos_angle = angle.cos();
                    let sin_angle = angle.sin();
                    
                    let inner = (value - std_dev).max(0.0);
                    let outer = value + std_dev;
                    
                    plot_ui.line(Line::new(PlotPoints::new(vec![
                        [inner * cos_angle, inner * sin_angle],
                        [outer * cos_angle, outer * sin_angle]
                    ]))
                    .color(series.color)
                    .width(3.0));
                }
            }
        }
    }
}

impl SpaceView for RadarChart {
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
    fn view_type(&self) -> &str { "RadarChartView" }
    
    fn set_data_source(&mut self, source_id: String) {
        self.config.data_source_id = Some(source_id);
        // Clear any cached data
        if let Some(cache_field) = self.as_any_mut().downcast_mut::<Self>() {
            // Reset cached data if the plot has any
        }
    }
    
    fn data_source_id(&self) -> Option<&str> {
        self.config.data_source_id.as_deref()
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if needed
        if self.cached_data.is_none() {
            let data_sources = ctx.data_sources.read();
            
            // Get the specific data source for this view
            let data_source = if let Some(source_id) = &self.config.data_source_id {
            data_sources.get(source_id)
            } else {
                (if let Some(source_id) = &self.config.data_source_id {
        data_sources.get(source_id)
    } else {
        data_sources.values().next()
    })
            };
            
            if let Some(source) = data_source {
                let nav_pos = ctx.navigation.get_context().position.clone();
                if let Ok(batch) = ctx.runtime_handle.block_on(source.query_at(&nav_pos)) {
                    self.cached_data = Some(batch.clone());
                    self.extract_data(&batch);
                }
            }
        }
        
        if self.cached_data.is_some() && !self.series.is_empty() {
            // Configuration UI
            ui.horizontal(|ui| {
                ui.label("Aggregation:");
                ui.selectable_value(&mut self.config.aggregation, AggregationType::Mean, "Mean");
                ui.selectable_value(&mut self.config.aggregation, AggregationType::Median, "Median");
                ui.selectable_value(&mut self.config.aggregation, AggregationType::Sum, "Sum");
                
                ui.separator();
                
                ui.checkbox(&mut self.config.show_grid, "Grid");
                ui.checkbox(&mut self.config.show_points, "Points");
                ui.checkbox(&mut self.config.show_values, "Values");
                if self.config.aggregation == AggregationType::Mean {
                    ui.checkbox(&mut self.config.show_error_bars, "Error Bars");
                }
            });
            
            // Main plot
            let plot = egui_plot::Plot::new(format!("radar_{:?}", self.id))
                .data_aspect(1.0)
                .allow_boxed_zoom(false)
                .allow_scroll(false)
                .show_axes(false)
                .show_grid(false);
            
            let plot = if self.config.show_legend {
                plot.legend(Legend::default().position(Corner::RightTop))
            } else {
                plot
            };
            
            plot.show(ui, |plot_ui| {
                self.plot_radar(plot_ui);
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No numeric data available for radar chart. Select columns with numeric values.");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "value_columns": self.config.value_columns,
            "group_column": self.config.group_column,
            "scale_type": format!("{:?}", self.config.scale_type),
            "aggregation": format!("{:?}", self.config.aggregation),
            "fill_opacity": self.config.fill_opacity,
            "show_grid": self.config.show_grid,
            "show_points": self.config.show_points,
            "show_values": self.config.show_values,
            "show_legend": self.config.show_legend,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(columns) = config.get("value_columns").and_then(|v| v.as_array()) {
            self.config.value_columns = columns.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 