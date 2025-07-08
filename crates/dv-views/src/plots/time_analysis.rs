//! Time series analysis and decomposition view

use egui::{Ui, Color32};
use egui_plot::{Plot, PlotUi, PlotPoints, Line, Legend, Points};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, TimestampSecondArray};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};
use dv_core::navigation::NavigationPosition;
use super::utils::colors::{categorical_color, ColorScheme};

/// Time analysis configuration
#[derive(Debug, Clone)]
pub struct TimeAnalysisConfig {
    pub data_source_id: Option<String>,
    
    pub time_column: String,
    pub value_columns: Vec<String>,
    
    // Analysis options
    pub show_trend: bool,
    pub show_seasonal: bool,
    pub show_residual: bool,
    pub decomposition_type: DecompositionType,
    pub period: Option<usize>, // Auto-detect if None
    
    // Smoothing options
    pub smoothing_window: usize,
    pub smoothing_type: SmoothingType,
    
    // Forecasting
    pub show_forecast: bool,
    pub forecast_periods: usize,
    pub confidence_interval: f64,
    
    // Visual options
    pub show_anomalies: bool,
    pub show_change_points: bool,
    pub show_correlogram: bool,
    pub color_scheme: ColorScheme,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecompositionType {
    Additive,
    Multiplicative,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SmoothingType {
    MovingAverage,
    ExponentialSmoothing,
    Loess,
    SavitzkyGolay,
}

impl Default for TimeAnalysisConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            
            time_column: String::new(),
            value_columns: Vec::new(),
            show_trend: true,
            show_seasonal: true,
            show_residual: false,
            decomposition_type: DecompositionType::Additive,
            period: None,
            smoothing_window: 7,
            smoothing_type: SmoothingType::MovingAverage,
            show_forecast: false,
            forecast_periods: 30,
            confidence_interval: 0.95,
            show_anomalies: true,
            show_change_points: false,
            show_correlogram: false,
            color_scheme: ColorScheme::Viridis,
        }
    }
}

/// Time series decomposition results
#[derive(Clone)]
struct Decomposition {
    trend: Vec<f64>,
    seasonal: Vec<f64>,
    residual: Vec<f64>,
    period: usize,
}

/// Time analysis view
pub struct TimeAnalysisPlot {
    id: SpaceViewId,
    title: String,
    pub config: TimeAnalysisConfig,
    
    // State
    cached_data: Option<RecordBatch>,
    decomposition_cache: HashMap<String, Decomposition>,
    anomalies: HashMap<String, Vec<usize>>,
    change_points: HashMap<String, Vec<usize>>,
}

impl TimeAnalysisPlot {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: TimeAnalysisConfig::default(),
            cached_data: None,
            decomposition_cache: HashMap::new(),
            anomalies: HashMap::new(),
            change_points: HashMap::new(),
        }
    }
    
    fn extract_time_series(&mut self, batch: &RecordBatch) -> Option<(Vec<f64>, HashMap<String, Vec<f64>>)> {
        let mut series_data = HashMap::new();
        
        // Extract time column
        let time_col = batch.column_by_name(&self.config.time_column)?;
        
        // Extract timestamps
        let timestamps = if let Some(timestamp_array) = time_col.as_any().downcast_ref::<TimestampSecondArray>() {
            (0..timestamp_array.len())
                .map(|i| timestamp_array.value(i) as f64)
                .collect()
        } else {
            return None;
        };
        
        // Extract value columns
        for col_name in &self.config.value_columns {
            if let Some(col_idx) = batch.schema().fields().iter()
                .position(|f| f.name() == col_name) {
                
                let col = batch.column(col_idx);
                if let Some(float_array) = col.as_any().downcast_ref::<Float64Array>() {
                    let values: Vec<f64> = (0..float_array.len())
                        .map(|i| float_array.value(i))
                        .collect();
                    series_data.insert(col_name.clone(), values);
                }
            }
        }
        
        Some((timestamps, series_data))
    }
    
    fn decompose_series(&mut self, name: &str, values: &[f64]) -> Decomposition {
        if let Some(cached) = self.decomposition_cache.get(name) {
            return cached.clone();
        }
        
        let period = self.config.period.unwrap_or_else(|| self.detect_period(values));
        
        // Step 1: Calculate trend using moving average
        let trend = self.calculate_trend(values, period);
        
        // Step 2: Detrend the series
        let detrended: Vec<f64> = match self.config.decomposition_type {
            DecompositionType::Additive => {
                values.iter().zip(&trend)
                    .map(|(v, t)| v - t)
                    .collect()
            }
            DecompositionType::Multiplicative => {
                values.iter().zip(&trend)
                    .map(|(v, t)| if *t != 0.0 { v / t } else { 1.0 })
                    .collect()
            }
        };
        
        // Step 3: Calculate seasonal component
        let seasonal = self.calculate_seasonal(&detrended, period);
        
        // Step 4: Calculate residual
        let residual: Vec<f64> = match self.config.decomposition_type {
            DecompositionType::Additive => {
                values.iter().zip(&trend).zip(&seasonal)
                    .map(|((v, t), s)| v - t - s)
                    .collect()
            }
            DecompositionType::Multiplicative => {
                values.iter().zip(&trend).zip(&seasonal)
                    .map(|((v, t), s)| if *t * *s != 0.0 { v / (t * s) } else { 1.0 })
                    .collect()
            }
        };
        
        let decomp = Decomposition {
            trend,
            seasonal,
            residual,
            period,
        };
        
        self.decomposition_cache.insert(name.to_string(), decomp.clone());
        decomp
    }
    
    fn detect_period(&self, values: &[f64]) -> usize {
        // Simple periodogram-based period detection
        let mut max_power = 0.0;
        let mut best_period = 7; // Default to weekly
        
        // Test periods from 2 to n/2
        for period in 2..(values.len() / 2).min(365) {
            let mut power = 0.0;
            
            // Calculate power at this frequency
            for k in 0..period {
                let mut sum = 0.0;
                let mut count = 0;
                
                for i in (k..values.len()).step_by(period) {
                    sum += values[i];
                    count += 1;
                }
                
                if count > 0 {
                    let mean = sum / count as f64;
                    for i in (k..values.len()).step_by(period) {
                        power += (values[i] - mean).powi(2);
                    }
                }
            }
            
            if power > max_power {
                max_power = power;
                best_period = period;
            }
        }
        
        best_period
    }
    
    fn calculate_trend(&self, values: &[f64], period: usize) -> Vec<f64> {
        let window = match self.config.smoothing_type {
            SmoothingType::MovingAverage => period,
            _ => self.config.smoothing_window,
        };
        
        match self.config.smoothing_type {
            SmoothingType::MovingAverage => {
                self.moving_average(values, window)
            }
            SmoothingType::ExponentialSmoothing => {
                self.exponential_smoothing(values, 2.0 / (window as f64 + 1.0))
            }
            SmoothingType::Loess => {
                // Simplified LOESS implementation
                self.loess_smoothing(values, window)
            }
            SmoothingType::SavitzkyGolay => {
                // Simplified Savitzky-Golay filter
                self.savitzky_golay(values, window, 3)
            }
        }
    }
    
    fn moving_average(&self, values: &[f64], window: usize) -> Vec<f64> {
        let mut result = vec![0.0; values.len()];
        let half_window = window / 2;
        
        for i in 0..values.len() {
            let start = i.saturating_sub(half_window);
            let end = (i + half_window + 1).min(values.len());
            let sum: f64 = values[start..end].iter().sum();
            result[i] = sum / (end - start) as f64;
        }
        
        result
    }
    
    fn exponential_smoothing(&self, values: &[f64], alpha: f64) -> Vec<f64> {
        let mut result = vec![0.0; values.len()];
        result[0] = values[0];
        
        for i in 1..values.len() {
            result[i] = alpha * values[i] + (1.0 - alpha) * result[i - 1];
        }
        
        result
    }
    
    fn loess_smoothing(&self, values: &[f64], window: usize) -> Vec<f64> {
        // Simplified LOESS using weighted regression
        let mut result = vec![0.0; values.len()];
        
        for i in 0..values.len() {
            let start = i.saturating_sub(window / 2);
            let end = (i + window / 2 + 1).min(values.len());
            
            // Calculate weights (tricube kernel)
            let mut weights = Vec::new();
            let mut x_vals = Vec::new();
            let mut y_vals = Vec::new();
            
            for j in start..end {
                let dist = ((j as f64 - i as f64) / (window as f64 / 2.0)).abs();
                let weight = if dist < 1.0 {
                    (1.0 - dist.powi(3)).powi(3)
                } else {
                    0.0
                };
                weights.push(weight);
                x_vals.push(j as f64);
                y_vals.push(values[j]);
            }
            
            // Weighted linear regression
            let sum_w: f64 = weights.iter().sum();
            let sum_wx: f64 = weights.iter().zip(&x_vals).map(|(w, x)| w * x).sum();
            let sum_wy: f64 = weights.iter().zip(&y_vals).map(|(w, y)| w * y).sum();
            let sum_wxx: f64 = weights.iter().zip(&x_vals).map(|(w, x)| w * x * x).sum();
            let sum_wxy: f64 = weights.iter().zip(&x_vals).zip(&y_vals)
                .map(|((w, x), y)| w * x * y).sum();
            
            let denom = sum_w * sum_wxx - sum_wx * sum_wx;
            if denom != 0.0 {
                let slope = (sum_w * sum_wxy - sum_wx * sum_wy) / denom;
                let intercept = (sum_wy - slope * sum_wx) / sum_w;
                result[i] = slope * i as f64 + intercept;
            } else {
                result[i] = values[i];
            }
        }
        
        result
    }
    
    fn savitzky_golay(&self, values: &[f64], window: usize, _poly_order: usize) -> Vec<f64> {
        // Simplified smoothing using moving average for now
        let half_window = window / 2;
        values.iter()
            .enumerate()
            .map(|(i, _)| {
                let start = i.saturating_sub(half_window);
                let end = (i + half_window + 1).min(values.len());
                let sum: f64 = values[start..end].iter().sum();
                sum / (end - start) as f64
            })
            .collect()
    }
    
    fn calculate_seasonal(&self, detrended: &[f64], period: usize) -> Vec<f64> {
        let mut seasonal_pattern = vec![0.0; period];
        let mut counts = vec![0; period];
        
        // Calculate average for each position in the period
        for (i, &value) in detrended.iter().enumerate() {
            let pos = i % period;
            seasonal_pattern[pos] += value;
            counts[pos] += 1;
        }
        
        // Average the values
        for i in 0..period {
            if counts[i] > 0 {
                seasonal_pattern[i] /= counts[i] as f64;
            }
        }
        
        // Extend pattern to full length
        let mut seasonal = Vec::with_capacity(detrended.len());
        for i in 0..detrended.len() {
            seasonal.push(seasonal_pattern[i % period]);
        }
        
        seasonal
    }
    
    fn detect_anomalies(&mut self, name: &str, _values: &[f64], residuals: &[f64]) {
        // Z-score based anomaly detection on residuals
        let mean = residuals.iter().sum::<f64>() / residuals.len() as f64;
        let std = {
            let variance = residuals.iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f64>() / residuals.len() as f64;
            variance.sqrt()
        };
        
        let threshold = 2.5 * std;
        let anomalies: Vec<usize> = residuals.iter()
            .enumerate()
            .filter(|(_, &r)| r.abs() > threshold)
            .map(|(i, _)| i)
            .collect();
        
        self.anomalies.insert(name.to_string(), anomalies);
    }
    
    fn detect_change_points(&mut self, name: &str, values: &[f64]) {
        // Simple change point detection using CUSUM
        let mut change_points = Vec::new();
        let n = values.len();
        
        if n < 10 {
            self.change_points.insert(name.to_string(), change_points);
            return;
        }
        
        // Calculate running mean and variance
        let window = 20.min(n / 5);
        for i in window..n - window {
            let before = &values[i - window..i];
            let after = &values[i..i + window];
            
            let mean_before: f64 = before.iter().sum::<f64>() / window as f64;
            let mean_after: f64 = after.iter().sum::<f64>() / window as f64;
            
            let var_before: f64 = before.iter()
                .map(|v| (v - mean_before).powi(2))
                .sum::<f64>() / window as f64;
            let var_after: f64 = after.iter()
                .map(|v| (v - mean_after).powi(2))
                .sum::<f64>() / window as f64;
            
            // T-test statistic
            let pooled_var = (var_before + var_after) / 2.0;
            let t_stat = (mean_after - mean_before).abs() / 
                (pooled_var * (2.0 / window as f64)).sqrt();
            
            if t_stat > 3.0 {
                change_points.push(i);
            }
        }
        
        self.change_points.insert(name.to_string(), change_points);
    }
    
    fn forecast_series(&self, values: &[f64], periods: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        // Simple exponential smoothing forecast
        let _alpha = 0.3;
        let level = values[values.len() - 1];
        let mut forecast = Vec::new();
        let mut lower_bound = Vec::new();
        let mut upper_bound = Vec::new();
        
        // Calculate prediction intervals based on historical errors
        let errors: Vec<f64> = values.windows(2)
            .map(|w| w[1] - w[0])
            .collect();
        let error_std = if !errors.is_empty() {
            let mean_error = errors.iter().sum::<f64>() / errors.len() as f64;
            let variance = errors.iter()
                .map(|e| (e - mean_error).powi(2))
                .sum::<f64>() / errors.len() as f64;
            variance.sqrt()
        } else {
            0.0
        };
        
        let z_score = 1.96; // 95% confidence interval
        
        for i in 1..=periods {
            forecast.push(level);
            let interval = z_score * error_std * (i as f64).sqrt();
            lower_bound.push(level - interval);
            upper_bound.push(level + interval);
        }
        
        (forecast, lower_bound, upper_bound)
    }
    
    fn draw_main_series(&self, plot_ui: &mut PlotUi, times: &[f64], series_data: &HashMap<String, Vec<f64>>) {
        for (idx, (name, values)) in series_data.iter().enumerate() {
            let color = categorical_color(idx);
            
            let points: PlotPoints = times.iter().zip(values)
                .map(|(&t, &v)| [t, v])
                .collect();
            
            let line = Line::new(points)
                .color(color)
                .width(2.0)
                .name(name);
            
            plot_ui.line(line);
            
            // Draw anomalies if detected
            if self.config.show_anomalies {
                if let Some(anomaly_indices) = self.anomalies.get(name) {
                    let anomaly_points: PlotPoints = anomaly_indices.iter()
                        .filter_map(|&i| {
                            if i < times.len() && i < values.len() {
                                Some([times[i], values[i]])
                            } else {
                                None
                            }
                        })
                        .collect();
                    
                    plot_ui.points(
                        Points::new(anomaly_points)
                            .color(Color32::RED)
                            .radius(5.0)
                            .name(format!("{} anomalies", name))
                    );
                }
            }
            
            // Draw change points if detected
            if self.config.show_change_points {
                if let Some(change_indices) = self.change_points.get(name) {
                    for &cp_idx in change_indices {
                        if cp_idx < times.len() {
                            plot_ui.vline(egui_plot::VLine::new(times[cp_idx])
                                .color(Color32::from_rgba_unmultiplied(255, 165, 0, 128))
                                .width(2.0));
                        }
                    }
                }
            }
        }
    }
    
    fn draw_decomposition(&self, ui: &mut Ui, times: &[f64], series_data: &HashMap<String, Vec<f64>>) {
        let available_height = ui.available_height();
        let component_height = available_height / 4.0;
        
        // Original series
        let original_plot = Plot::new("time_analysis_original")
            .height(component_height)
            .legend(Legend::default())
            .label_formatter(|_, _| String::new());
            
        original_plot.show(ui, |plot_ui| {
            self.draw_main_series(plot_ui, times, series_data);
        });
        
        // Trend component
        if self.config.show_trend {
            let trend_plot = Plot::new("time_analysis_trend")
                .height(component_height)
                .legend(Legend::default())
                .label_formatter(|_, _| String::new());
                
            trend_plot.show(ui, |plot_ui| {
                for (idx, (name, _)) in series_data.iter().enumerate() {
                    if let Some(decomp) = self.decomposition_cache.get(name) {
                        let color = categorical_color(idx);
                        let points: PlotPoints = times.iter().zip(&decomp.trend)
                            .map(|(&t, &v)| [t, v])
                            .collect();
                        
                        plot_ui.line(Line::new(points)
                            .color(color)
                            .width(2.0)
                            .name(format!("{} trend", name)));
                    }
                }
            });
        }
        
        // Seasonal component
        if self.config.show_seasonal {
            let seasonal_plot = Plot::new("time_analysis_seasonal")
                .height(component_height)
                .legend(Legend::default())
                .label_formatter(|_, _| String::new());
                
            seasonal_plot.show(ui, |plot_ui| {
                for (idx, (name, _)) in series_data.iter().enumerate() {
                    if let Some(decomp) = self.decomposition_cache.get(name) {
                        let color = categorical_color(idx);
                        let points: PlotPoints = times.iter().zip(&decomp.seasonal)
                            .map(|(&t, &v)| [t, v])
                            .collect();
                        
                        plot_ui.line(Line::new(points)
                            .color(color)
                            .width(1.0)
                            .name(format!("{} seasonal", name)));
                    }
                }
            });
        }
        
        // Residual component
        if self.config.show_residual {
            let residual_plot = Plot::new("time_analysis_residual")
                .height(component_height)
                .legend(Legend::default())
                .label_formatter(|_, _| String::new());
                
            residual_plot.show(ui, |plot_ui| {
                for (idx, (name, _)) in series_data.iter().enumerate() {
                    if let Some(decomp) = self.decomposition_cache.get(name) {
                        let color = categorical_color(idx);
                        let points: PlotPoints = times.iter().zip(&decomp.residual)
                            .map(|(&t, &v)| [t, v])
                            .collect();
                        
                        plot_ui.line(Line::new(points)
                            .color(color)
                            .width(0.5)
                            .name(format!("{} residual", name)));
                    }
                }
            });
        }
    }
    
    fn detect_change_points_in_series(&mut self, name: &str, values: &[f64]) {
        self.detect_change_points(name, values);
    }
    
    fn simple_forecast(&self) {
        // Placeholder for simple forecasting
        // Already implemented in forecast_series
    }
}

impl SpaceView for TimeAnalysisPlot {
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
    fn view_type(&self) -> &str { "TimeAnalysisView" }
    
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
            if let Some(source) = data_sources.values().next() {
                let nav_pos = ctx.navigation.get_context().position.clone();
                if let Ok(batch) = ctx.runtime_handle.block_on(source.query_at(&nav_pos)) {
                    self.cached_data = Some(batch);
                }
            }
        }
        
        if let Some(batch) = &self.cached_data {
            let batch_clone = batch.clone();
            if let Some((times, series_data)) = self.extract_time_series(&batch_clone) {
                // Process decomposition for each series first
                for (name, values) in &series_data {
                    if !self.decomposition_cache.contains_key(name) {
                        let decomp = self.decompose_series(name, values);
                        self.decomposition_cache.insert(name.clone(), decomp);
                        self.detect_change_points_in_series(name, values);
                    }
                }
                
                // Then detect anomalies using cached decompositions
                for (name, values) in &series_data {
                    if let Some(decomp) = self.decomposition_cache.get(name).cloned() {
                        self.detect_anomalies(name, values, &decomp.residual);
                    }
                }
                
                // Draw based on selected view
                if self.config.show_trend || self.config.show_seasonal || self.config.show_residual {
                    self.draw_decomposition(ui, &times, &series_data);
                } else {
                    // Simple time series plot
                    let plot = Plot::new("time_analysis_main")
                        .legend(Legend::default())
                        .label_formatter(|name, value| {
                            format!("{}: Time={:.1}, Value={:.2}", name, value.x, value.y)
                        });
                    
                    plot.show(ui, |plot_ui| {
                        self.draw_main_series(plot_ui, &times, &series_data);
                        
                        // Draw forecast if enabled
                        if self.config.show_forecast {
                            for (idx, (name, values)) in series_data.iter().enumerate() {
                                let color = categorical_color(idx);
                                let last_time = times.last().copied().unwrap_or(0.0);
                                
                                let (forecast, lower, upper) = self.forecast_series(
                                    values, 
                                    self.config.forecast_periods
                                );
                                
                                // Forecast times
                                let forecast_times: Vec<f64> = (1..=self.config.forecast_periods)
                                    .map(|i| last_time + i as f64)
                                    .collect();
                                
                                // Draw forecast line
                                let forecast_points: PlotPoints = forecast_times.iter()
                                    .zip(&forecast)
                                    .map(|(&t, &v)| [t, v])
                                    .collect();
                                
                                plot_ui.line(Line::new(forecast_points)
                                    .color(color)
                                    .width(2.0)
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                    .name(format!("{} forecast", name)));
                                
                                // Draw confidence band
                                let mut band_points = Vec::new();
                                for i in 0..forecast_times.len() {
                                    band_points.push([forecast_times[i], upper[i]]);
                                }
                                for i in (0..forecast_times.len()).rev() {
                                    band_points.push([forecast_times[i], lower[i]]);
                                }
                                
                                plot_ui.polygon(egui_plot::Polygon::new(band_points)
                                    .fill_color(Color32::from_rgba_unmultiplied(
                                        color.r(), color.g(), color.b(), 50
                                    ))
                                    .name(format!("{} confidence", name)));
                            }
                        }
                    });
                }
                
                // Info panel
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Time Analysis:");
                    
                    for (name, _) in &series_data {
                        if let Some(decomp) = self.decomposition_cache.get(name) {
                            ui.label(format!("{}: Period={}", name, decomp.period));
                        }
                    }
                    
                    if self.config.show_anomalies {
                        let total_anomalies: usize = self.anomalies.values()
                            .map(|v| v.len())
                            .sum();
                        ui.label(format!("Anomalies: {}", total_anomalies));
                    }
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No time series data available. Please configure time and value columns.");
                });
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Loading time series data...");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "time_column": self.config.time_column,
            "value_columns": self.config.value_columns,
            "show_trend": self.config.show_trend,
            "show_seasonal": self.config.show_seasonal,
            "show_residual": self.config.show_residual,
            "decomposition_type": format!("{:?}", self.config.decomposition_type),
            "period": self.config.period,
            "smoothing_window": self.config.smoothing_window,
            "smoothing_type": format!("{:?}", self.config.smoothing_type),
            "show_forecast": self.config.show_forecast,
            "forecast_periods": self.config.forecast_periods,
            "confidence_interval": self.config.confidence_interval,
            "show_anomalies": self.config.show_anomalies,
            "show_change_points": self.config.show_change_points,
            "color_scheme": format!("{:?}", self.config.color_scheme),
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(time) = config.get("time_column").and_then(|v| v.as_str()) {
            self.config.time_column = time.to_string();
        }
        if let Some(values) = config.get("value_columns").and_then(|v| v.as_array()) {
            self.config.value_columns = values.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
        }
        if let Some(show) = config.get("show_trend").and_then(|v| v.as_bool()) {
            self.config.show_trend = show;
        }
        if let Some(show) = config.get("show_seasonal").and_then(|v| v.as_bool()) {
            self.config.show_seasonal = show;
        }
        if let Some(show) = config.get("show_residual").and_then(|v| v.as_bool()) {
            self.config.show_residual = show;
        }
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 