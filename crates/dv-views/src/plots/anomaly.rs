//! Anomaly detection view for identifying outliers and unusual patterns

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotUi, Line, PlotPoints, Points, Legend, Corner, VLine, Polygon, MarkerShape};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, Int64Array, Array};
use serde_json::{json, Value};
use std::collections::HashSet;
use rand::prelude::*;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;
use super::utils::stats::{calculate_quartiles, detect_outliers_iqr, zscore_outliers};

/// Configuration for anomaly detection view
#[derive(Clone)]
pub struct AnomalyDetectionConfig {
    pub data_source_id: String,
    /// Column to analyze
    pub column: String,
    
    /// Time column (optional)
    pub time_column: Option<String>,
    
    /// Detection method
    pub detection_method: DetectionMethod,
    
    /// Z-score threshold
    pub zscore_threshold: f64,
    
    /// IQR multiplier
    pub iqr_multiplier: f64,
    
    /// Window size for moving statistics
    pub window_size: usize,
    
    /// Whether to show confidence bands
    pub show_confidence_bands: bool,
    
    /// Whether to show statistics
    pub show_statistics: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DetectionMethod {
    ZScore,
    IQR,
    MovingAverage,
    IsolationForest,
    LocalOutlierFactor,
    DBSCAN,
}

impl Default for AnomalyDetectionConfig {
    fn default() -> Self {
        Self {
            data_source_id: String::new(),
            column: String::new(),
            time_column: None,
            detection_method: DetectionMethod::ZScore,
            zscore_threshold: 3.0,
            iqr_multiplier: 1.5,
            window_size: 20,
            show_confidence_bands: true,
            show_statistics: true,
        }
    }
}

/// Anomaly detection view
pub struct AnomalyDetectionView {
    id: SpaceViewId,
    title: String,
    pub config: AnomalyDetectionConfig,
    
    // State
    cached_data: Option<AnomalyData>,
    last_navigation_pos: Option<NavigationPosition>,
}

/// Cached anomaly detection data
struct AnomalyData {
    values: Vec<f64>,
    indices: Vec<f64>,
    anomalies: Vec<usize>,
    statistics: AnomalyStatistics,
    confidence_bands: Option<(Vec<f64>, Vec<f64>)>,
}

struct AnomalyStatistics {
    mean: f64,
    std_dev: f64,
    median: f64,
    mad: f64, // Median Absolute Deviation
    num_anomalies: usize,
    anomaly_rate: f64,
}

impl AnomalyDetectionView {
    /// Create a new anomaly detection view
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: AnomalyDetectionConfig::default(),
            cached_data: None,
            last_navigation_pos: None,
        }
    }
    
    /// Fetch data and detect anomalies
    fn fetch_data(&mut self, ctx: &ViewerContext) -> Option<AnomalyData> {
        let data_sources = ctx.data_sources.read();
        
        // Get the specific data source for this view
        let data_source = if !self.config.data_source_id.is_empty() {
            data_sources.get(&self.config.data_source_id)
        } else {
            data_sources.values().next()
        }?;
        
        // Get navigation context
        let nav_context = ctx.navigation.get_context();
        
        // Fetch all data for analysis
        let range = dv_core::navigation::NavigationRange {
            start: dv_core::navigation::NavigationPosition::Sequential(0),
            end: dv_core::navigation::NavigationPosition::Sequential(nav_context.total_rows),
        };
        
        let data = ctx.runtime_handle.block_on(data_source.query_range(&range)).ok()?;
        
        // Extract value column
        let column = data.column_by_name(&self.config.column)?;
        let values: Vec<f64> = if let Some(float_array) = column.as_any().downcast_ref::<Float64Array>() {
            (0..float_array.len()).filter_map(|i| {
                if float_array.is_null(i) { None } else { Some(float_array.value(i)) }
            }).collect()
        } else if let Some(int_array) = column.as_any().downcast_ref::<Int64Array>() {
            (0..int_array.len()).filter_map(|i| {
                if int_array.is_null(i) { None } else { Some(int_array.value(i) as f64) }
            }).collect()
        } else {
            return None;
        };
        
        if values.is_empty() {
            return None;
        }
        
        // Generate indices (use time column if available)
        let indices: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();
        
        // Detect anomalies based on method
        let anomalies = match self.config.detection_method {
            DetectionMethod::ZScore => zscore_outliers(&values, self.config.zscore_threshold),
            DetectionMethod::IQR => detect_outliers_iqr(&values),
            DetectionMethod::MovingAverage => self.detect_moving_average_anomalies(&values),
            DetectionMethod::IsolationForest => self.detect_isolation_forest_anomalies(&values),
            DetectionMethod::LocalOutlierFactor => self.detect_lof_anomalies(&values),
            DetectionMethod::DBSCAN => self.detect_dbscan_anomalies(&values),
        };
        
        // Calculate statistics
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let std_dev = {
            let variance = values.iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>() / values.len() as f64;
            variance.sqrt()
        };
        let median = {
            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if sorted.len() % 2 == 0 {
                (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
            } else {
                sorted[sorted.len() / 2]
            }
        };
        
        // Calculate MAD
        let deviations: Vec<f64> = values.iter()
            .map(|&v| (v - median).abs())
            .collect();
        let mad = {
            let mut sorted_dev = deviations.clone();
            sorted_dev.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if sorted_dev.len() % 2 == 0 {
                (sorted_dev[sorted_dev.len() / 2 - 1] + sorted_dev[sorted_dev.len() / 2]) / 2.0
            } else {
                sorted_dev[sorted_dev.len() / 2]
            }
        };
        
        let statistics = AnomalyStatistics {
            mean,
            std_dev,
            median,
            mad,
            num_anomalies: anomalies.len(),
            anomaly_rate: anomalies.len() as f64 / values.len() as f64,
        };
        
        // Calculate confidence bands
        let confidence_bands = if self.config.show_confidence_bands {
            Some(self.calculate_confidence_bands(&values))
        } else {
            None
        };
        
        Some(AnomalyData {
            values,
            indices,
            anomalies,
            statistics,
            confidence_bands,
        })
    }
    
    fn detect_moving_average_anomalies(&self, values: &[f64]) -> Vec<usize> {
        let window = self.config.window_size.max(3);
        let mut anomalies = Vec::new();
        
        for i in window..values.len() {
            let window_values: Vec<f64> = values[i-window..i].to_vec();
            let window_mean = window_values.iter().sum::<f64>() / window_values.len() as f64;
            let window_std = window_values.iter()
                .map(|v| (v - window_mean).powi(2))
                .sum::<f64>()
                .sqrt() / (window_values.len() as f64).sqrt();
            
            let threshold = window_mean + self.config.zscore_threshold * window_std;
            if values[i] > threshold {
                anomalies.push(i);
            }
        }
        
        anomalies
    }
    
    fn detect_isolation_forest_anomalies(&self, values: &[f64]) -> Vec<usize> {
        // Simplified isolation forest for 1D data
        let mut anomalies = Vec::new();
        let num_trees = 100;
        let sample_size = (values.len() as f64).sqrt() as usize;
        
        let mut rng = thread_rng();
        
        // Calculate anomaly scores
        let mut scores = vec![0.0; values.len()];
        
        for _ in 0..num_trees {
            // Sample data
            let mut sample: Vec<f64> = values.choose_multiple(&mut rng, sample_size).cloned().collect();
            sample.sort_by(|a, b| a.partial_cmp(b).unwrap());
            
            // Calculate path lengths
            for (i, &value) in values.iter().enumerate() {
                let path_length = self.calculate_path_length(&sample, value);
                scores[i] += path_length;
            }
        }
        
        // Normalize scores
        for score in &mut scores {
            *score /= num_trees as f64;
        }
        
        // Calculate mean and std of scores
        let mean_score = scores.iter().sum::<f64>() / scores.len() as f64;
        let std_score = scores.iter()
            .map(|s| (s - mean_score).powi(2))
            .sum::<f64>()
            .sqrt() / (scores.len() as f64).sqrt();
        
        let threshold = 2.0;
        
        for (i, &score) in scores.iter().enumerate() {
            if score > mean_score + threshold * std_score {
                anomalies.push(i);
            }
        }
        
        anomalies
    }
    
    fn calculate_path_length(&self, sample: &[f64], value: f64) -> f64 {
        let mut depth = 0.0;
        let mut left = 0;
        let mut right = sample.len() - 1;
        
        while left < right {
            let mid = (left + right) / 2;
            depth += 1.0;
            
            if value < sample[mid] {
                right = mid;
            } else {
                left = mid + 1;
            }
        }
        
        depth
    }
    
    fn detect_lof_anomalies(&self, values: &[f64]) -> Vec<usize> {
        // Simplified Local Outlier Factor for 1D
        let k = 5.min(values.len() / 10).max(2);
        let mut anomalies = Vec::new();
        
        // Calculate LOF scores
        let mut lof_scores = vec![0.0; values.len()];
        
        for i in 0..values.len() {
            // Find k nearest neighbors
            let mut distances: Vec<(usize, f64)> = values.iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(j, &v)| (j, (values[i] - v).abs()))
                .collect();
            distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            distances.truncate(k);
            
            // Calculate reachability distance
            let k_distance = distances.last().unwrap().1;
            let avg_reachability: f64 = distances.iter()
                .map(|(_, d)| d.max(k_distance))
                .sum::<f64>() / k as f64;
            
            lof_scores[i] = if avg_reachability > 0.0 { 1.0 / avg_reachability } else { 0.0 };
        }
        
        // Normalize and find anomalies
        let mean_lof = lof_scores.iter().sum::<f64>() / lof_scores.len() as f64;
        let std_lof = {
            let variance = lof_scores.iter()
                .map(|s| (s - mean_lof).powi(2))
                .sum::<f64>() / lof_scores.len() as f64;
            variance.sqrt()
        };
        
        for (i, &score) in lof_scores.iter().enumerate() {
            if score > mean_lof + 2.0 * std_lof {
                anomalies.push(i);
            }
        }
        
        anomalies
    }
    
    fn detect_dbscan_anomalies(&self, values: &[f64]) -> Vec<usize> {
        // DBSCAN for 1D data
        let std_dev = {
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let variance = values.iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>() / values.len() as f64;
            variance.sqrt()
        };
        let eps = std_dev * 0.3; // epsilon radius
        let min_pts = 3;
        let mut labels = vec![-1i32; values.len()]; // -1 = noise
        let mut cluster_id = 0;
        
        for i in 0..values.len() {
            if labels[i] != -1 {
                continue;
            }
            
            // Find neighbors
            let neighbors: Vec<usize> = values.iter()
                .enumerate()
                .filter(|(_j, &v)| (values[i] - v).abs() <= eps)
                .map(|(j, _)| j)
                .collect();
            
            if neighbors.len() < min_pts {
                labels[i] = -1; // Mark as noise
            } else {
                // Start a new cluster
                labels[i] = cluster_id;
                let mut seeds = neighbors.clone();
                let mut j = 0;
                
                while j < seeds.len() {
                    let q = seeds[j];
                    if labels[q] == -1 {
                        labels[q] = cluster_id;
                    }
                    
                    if labels[q] == -1 || labels[q] > -1 {
                        j += 1;
                        continue;
                    }
                    
                    labels[q] = cluster_id;
                    
                    // Find q's neighbors
                    let q_neighbors: Vec<usize> = values.iter()
                        .enumerate()
                        .filter(|(_k, &v)| (values[q] - v).abs() <= eps)
                        .map(|(k, _)| k)
                        .collect();
                    
                    if q_neighbors.len() >= min_pts {
                        for &n in &q_neighbors {
                            if !seeds.contains(&n) {
                                seeds.push(n);
                            }
                        }
                    }
                    
                    j += 1;
                }
                
                cluster_id += 1;
            }
        }
        
        // Points with label -1 are anomalies
        labels.iter()
            .enumerate()
            .filter(|(_, &label)| label == -1)
            .map(|(i, _)| i)
            .collect()
    }
    
    fn calculate_confidence_bands(&self, values: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let window = self.config.window_size;
        let mut upper_band = Vec::new();
        let mut lower_band = Vec::new();
        
        for i in 0..values.len() {
            let start = i.saturating_sub(window / 2);
            let end = (i + window / 2 + 1).min(values.len());
            
            let window_values: Vec<f64> = values[start..end].to_vec();
            let mean = window_values.iter().sum::<f64>() / window_values.len() as f64;
            let std = window_values.iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>()
                .sqrt() / (window_values.len() as f64).sqrt();
            
            upper_band.push(mean + 2.0 * std);
            lower_band.push(mean - 2.0 * std);
        }
        
        (upper_band, lower_band)
    }
    
    fn plot_anomalies(&self, plot_ui: &mut PlotUi, values: &[f64], anomalies: &[usize]) {
        // Implementation of plot_anomalies method
    }
}

impl SpaceView for AnomalyDetectionView {
    fn id(&self) -> SpaceViewId {
        self.id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn display_name(&self) -> &str {
        &self.title
    }
    
    fn view_type(&self) -> &str {
        "AnomalyDetectionView"
    }
    
    fn ui(&mut self, ctx: &ViewerContext, ui: &mut Ui) {
        // Update data if navigation changed
        let nav_pos = ctx.navigation.get_context().position.clone();
        if self.last_navigation_pos.as_ref() != Some(&nav_pos) {
            self.cached_data = self.fetch_data(ctx);
            self.last_navigation_pos = Some(nav_pos);
        }
        
        // Draw the plot
        if let Some(data) = &self.cached_data {
            // Show statistics
            if self.config.show_statistics {
                ui.horizontal(|ui| {
                    ui.label(format!("Anomalies: {} ({:.1}%)", 
                        data.statistics.num_anomalies,
                        data.statistics.anomaly_rate * 100.0
                    ));
                    ui.separator();
                    ui.label(format!("Mean: {:.2}", data.statistics.mean));
                    ui.separator();
                    ui.label(format!("Std: {:.2}", data.statistics.std_dev));
                    ui.separator();
                    ui.label(format!("Median: {:.2}", data.statistics.median));
                    ui.separator();
                    ui.label(format!("MAD: {:.2}", data.statistics.mad));
                });
                ui.add_space(4.0);
            }
            
            let plot = Plot::new(format!("{:?}", self.id))
                .legend(Legend::default())
                .show_grid(true)
                .allow_zoom(true)
                .allow_drag(true)
                .allow_boxed_zoom(true)
                .x_axis_label("Index")
                .y_axis_label(&self.config.column);
            
            plot.show(ui, |plot_ui| {
                // Draw main data line
                let points: Vec<[f64; 2]> = data.indices.iter()
                    .zip(&data.values)
                    .map(|(&x, &y)| [x, y])
                    .collect();
                
                plot_ui.line(
                    Line::new(PlotPoints::new(points))
                        .color(Color32::from_rgb(100, 150, 250))
                        .width(2.0)
                        .name("Data")
                );
                
                // Draw confidence bands if enabled
                if let Some((upper, lower)) = &data.confidence_bands {
                    let upper_points: Vec<[f64; 2]> = data.indices.iter()
                        .zip(upper)
                        .map(|(&x, &y)| [x, y])
                        .collect();
                    
                    let lower_points: Vec<[f64; 2]> = data.indices.iter()
                        .zip(lower)
                        .map(|(&x, &y)| [x, y])
                        .collect();
                    
                    plot_ui.line(
                        Line::new(PlotPoints::new(upper_points))
                            .color(Color32::from_rgb(200, 200, 200))
                            .width(1.0)
                            .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                            .name("Upper Band")
                    );
                    
                    plot_ui.line(
                        Line::new(PlotPoints::new(lower_points))
                            .color(Color32::from_rgb(200, 200, 200))
                            .width(1.0)
                            .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                            .name("Lower Band")
                    );
                }
                
                // Highlight anomalies
                let anomaly_points: Vec<[f64; 2]> = data.anomalies.iter()
                    .map(|&idx| [data.indices[idx], data.values[idx]])
                    .collect();
                
                if !anomaly_points.is_empty() {
                    plot_ui.points(
                        Points::new(anomaly_points)
                            .color(Color32::from_rgb(255, 100, 100))
                            .radius(5.0)
                            .shape(MarkerShape::Circle)
                            .name("Anomalies")
                    );
                }
                
                // Draw threshold lines for some methods
                match self.config.detection_method {
                    DetectionMethod::ZScore => {
                        let upper_threshold = data.statistics.mean + self.config.zscore_threshold * data.statistics.std_dev;
                        let lower_threshold = data.statistics.mean - self.config.zscore_threshold * data.statistics.std_dev;
                        
                        plot_ui.hline(
                            egui_plot::HLine::new(upper_threshold)
                                .color(Color32::from_rgb(255, 200, 100))
                                .width(2.0)
                                .style(egui_plot::LineStyle::Dotted { spacing: 10.0 })
                        );
                        
                        plot_ui.hline(
                            egui_plot::HLine::new(lower_threshold)
                                .color(Color32::from_rgb(255, 200, 100))
                                .width(2.0)
                                .style(egui_plot::LineStyle::Dotted { spacing: 10.0 })
                        );
                    }
                    _ => {}
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
                ui.label(egui::RichText::new("Configure column to detect anomalies").weak());
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "column": self.config.column,
            "time_column": self.config.time_column,
            "detection_method": match self.config.detection_method {
                DetectionMethod::ZScore => "zscore",
                DetectionMethod::IQR => "iqr",
                DetectionMethod::MovingAverage => "moving_average",
                DetectionMethod::IsolationForest => "isolation_forest",
                DetectionMethod::LocalOutlierFactor => "lof",
                DetectionMethod::DBSCAN => "dbscan",
            },
            "zscore_threshold": self.config.zscore_threshold,
            "iqr_multiplier": self.config.iqr_multiplier,
            "window_size": self.config.window_size,
            "show_confidence_bands": self.config.show_confidence_bands,
            "show_statistics": self.config.show_statistics,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(col) = config.get("column").and_then(|v| v.as_str()) {
            self.config.column = col.to_string();
        }
        if let Some(time_col) = config.get("time_column").and_then(|v| v.as_str()) {
            self.config.time_column = Some(time_col.to_string());
        }
        if let Some(method) = config.get("detection_method").and_then(|v| v.as_str()) {
            self.config.detection_method = match method {
                "zscore" => DetectionMethod::ZScore,
                "iqr" => DetectionMethod::IQR,
                "moving_average" => DetectionMethod::MovingAverage,
                "isolation_forest" => DetectionMethod::IsolationForest,
                "lof" => DetectionMethod::LocalOutlierFactor,
                "dbscan" => DetectionMethod::DBSCAN,
                _ => DetectionMethod::ZScore,
            };
        }
        if let Some(threshold) = config.get("zscore_threshold").and_then(|v| v.as_f64()) {
            self.config.zscore_threshold = threshold;
        }
        if let Some(multiplier) = config.get("iqr_multiplier").and_then(|v| v.as_f64()) {
            self.config.iqr_multiplier = multiplier;
        }
        if let Some(window) = config.get("window_size").and_then(|v| v.as_u64()) {
            self.config.window_size = window as usize;
        }
        if let Some(show) = config.get("show_confidence_bands").and_then(|v| v.as_bool()) {
            self.config.show_confidence_bands = show;
        }
        if let Some(show) = config.get("show_statistics").and_then(|v| v.as_bool()) {
            self.config.show_statistics = show;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {
        // TODO: Highlight selected anomalies
    }
    
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {
        // Nothing to update per frame
    }
} 