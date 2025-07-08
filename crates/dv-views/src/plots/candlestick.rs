//! Candlestick chart for financial data visualization

use egui::{Ui, Color32, Rect, Pos2, Vec2, Stroke, Shape};
use egui_plot::{Plot, PlotUi, PlotPoints, Line, Legend, Corner, Text, BoxElem, BoxPlot, BoxSpread, Bar, BarChart};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, TimestampSecondArray, StringArray};
use arrow::temporal_conversions::timestamp_s_to_datetime;
use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::{SpaceView, SpaceViewId, SelectionState, ViewerContext};

/// Candlestick configuration
#[derive(Debug, Clone)]
pub struct CandlestickConfig {
    pub data_source_id: Option<String>,
    
    pub time_column: Option<String>,
    pub open_column: Option<String>,
    pub high_column: Option<String>,
    pub low_column: Option<String>,
    pub close_column: Option<String>,
    pub volume_column: Option<String>,
    
    // Visual options
    pub candle_width: f32,
    pub up_color: Color32,
    pub down_color: Color32,
    pub show_volume: bool,
    pub show_ma: Vec<usize>, // Moving average periods
    pub show_bollinger: bool,
    pub bollinger_period: usize,
    pub bollinger_std: f64,
    
    // Analysis overlays
    pub show_trend_lines: bool,
    pub show_support_resistance: bool,
    pub show_patterns: bool,
}

impl Default for CandlestickConfig {
    fn default() -> Self {
        Self {
            data_source_id: None,
            
            time_column: None,
            open_column: None,
            high_column: None,
            low_column: None,
            close_column: None,
            volume_column: None,
            candle_width: 0.8,
            up_color: Color32::from_rgb(50, 200, 100),
            down_color: Color32::from_rgb(200, 50, 50),
            show_volume: true,
            show_ma: vec![20, 50],
            show_bollinger: false,
            bollinger_period: 20,
            bollinger_std: 2.0,
            show_trend_lines: false,
            show_support_resistance: false,
            show_patterns: false,
        }
    }
}

#[derive(Clone, Debug)]
struct Candle {
    time: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: Option<f64>,
}

#[derive(Clone, Debug)]
struct Pattern {
    name: String,
    start_time: i64,
    end_time: i64,
    pattern_type: PatternType,
}

#[derive(Clone, Debug, PartialEq)]
enum PatternType {
    Bullish,
    Bearish,
    Neutral,
}

/// Candlestick chart view
pub struct CandlestickChart {
    id: SpaceViewId,
    title: String,
    pub config: CandlestickConfig,
    
    // State
    cached_data: Option<RecordBatch>,
    candles: BTreeMap<i64, Candle>,
    moving_averages: Vec<(usize, Vec<[f64; 2]>)>,
    bollinger_bands: Option<(Vec<[f64; 2]>, Vec<[f64; 2]>, Vec<[f64; 2]>)>, // upper, middle, lower
    patterns: Vec<Pattern>,
    support_resistance: Vec<(f64, String)>,
}

impl CandlestickChart {
    pub fn new(id: SpaceViewId, title: String) -> Self {
        Self {
            id,
            title,
            config: CandlestickConfig::default(),
            cached_data: None,
            candles: BTreeMap::new(),
            moving_averages: Vec::new(),
            bollinger_bands: None,
            patterns: Vec::new(),
            support_resistance: Vec::new(),
        }
    }
    
    fn extract_data(&mut self, batch: &RecordBatch) {
        self.candles.clear();
        
        // Find columns if not specified
        self.auto_detect_columns(batch);
        
        // Extract OHLC data
        let time_col = self.get_column::<TimestampSecondArray>(batch, &self.config.time_column);
        let open_col = self.get_column::<Float64Array>(batch, &self.config.open_column);
        let high_col = self.get_column::<Float64Array>(batch, &self.config.high_column);
        let low_col = self.get_column::<Float64Array>(batch, &self.config.low_column);
        let close_col = self.get_column::<Float64Array>(batch, &self.config.close_column);
        let volume_col = self.get_column::<Float64Array>(batch, &self.config.volume_column);
        
        if let (Some(time), Some(open), Some(high), Some(low), Some(close)) = 
            (time_col, open_col, high_col, low_col, close_col) {
            
            for i in 0..batch.num_rows() {
                                let t = time.value(i);
                let o = open.value(i);
                let h = high.value(i);
                let l = low.value(i);
                let c = close.value(i);
                    
                    let volume = volume_col.map(|v| v.value(i));
                    
                    self.candles.insert(t, Candle {
                        time: t,
                        open: o,
                        high: h,
                        low: l,
                        close: c,
                        volume,
                    });
            }
        }
        
        // Calculate indicators
        self.calculate_moving_averages();
        self.calculate_bollinger_bands();
        
        if self.config.show_patterns {
            self.detect_patterns();
        }
        
        if self.config.show_support_resistance {
            self.find_support_resistance();
        }
    }
    
    fn auto_detect_columns(&mut self, batch: &RecordBatch) {
        for field in batch.schema().fields() {
            let name_lower = field.name().to_lowercase();
            
            if self.config.time_column.is_none() && 
               (name_lower.contains("time") || name_lower.contains("date")) {
                self.config.time_column = Some(field.name().to_string());
            }
            
            if self.config.open_column.is_none() && name_lower.contains("open") {
                self.config.open_column = Some(field.name().to_string());
            }
            
            if self.config.high_column.is_none() && name_lower.contains("high") {
                self.config.high_column = Some(field.name().to_string());
            }
            
            if self.config.low_column.is_none() && name_lower.contains("low") {
                self.config.low_column = Some(field.name().to_string());
            }
            
            if self.config.close_column.is_none() && name_lower.contains("close") {
                self.config.close_column = Some(field.name().to_string());
            }
            
            if self.config.volume_column.is_none() && name_lower.contains("volume") {
                self.config.volume_column = Some(field.name().to_string());
            }
        }
    }
    
    fn get_column<'a, T: 'static>(&self, batch: &'a RecordBatch, column_name: &Option<String>) -> Option<&'a T> {
        column_name.as_ref()
            .and_then(|name| batch.schema().fields().iter()
                .position(|f| f.name() == name))
            .and_then(|idx| batch.column(idx).as_any().downcast_ref::<T>())
    }
    
    fn calculate_moving_averages(&mut self) {
        self.moving_averages.clear();
        
        for &period in &self.config.show_ma {
            let mut ma_points = Vec::new();
            let prices: Vec<(i64, f64)> = self.candles.iter()
                .map(|(&time, candle)| (time, candle.close))
                .collect();
            
            if prices.len() >= period {
                for i in (period - 1)..prices.len() {
                    let sum: f64 = prices[(i + 1 - period)..=i]
                        .iter()
                        .map(|(_, price)| price)
                        .sum();
                    let avg = sum / period as f64;
                    ma_points.push([prices[i].0 as f64, avg]);
                }
            }
            
            self.moving_averages.push((period, ma_points));
        }
    }
    
    fn calculate_bollinger_bands(&mut self) {
        if !self.config.show_bollinger {
            self.bollinger_bands = None;
            return;
        }
        
        let period = self.config.bollinger_period;
        let std_dev_multiplier = self.config.bollinger_std;
        
        let prices: Vec<(i64, f64)> = self.candles.iter()
            .map(|(&time, candle)| (time, candle.close))
            .collect();
        
        if prices.len() >= period {
            let mut upper = Vec::new();
            let mut middle = Vec::new();
            let mut lower = Vec::new();
            
            for i in (period - 1)..prices.len() {
                let window: Vec<f64> = prices[(i + 1 - period)..=i]
                    .iter()
                    .map(|(_, price)| *price)
                    .collect();
                
                let mean = window.iter().sum::<f64>() / period as f64;
                let variance = window.iter()
                    .map(|x| (x - mean).powi(2))
                    .sum::<f64>() / period as f64;
                let std_dev = variance.sqrt();
                
                let time = prices[i].0 as f64;
                middle.push([time, mean]);
                upper.push([time, mean + std_dev_multiplier * std_dev]);
                lower.push([time, mean - std_dev_multiplier * std_dev]);
            }
            
            self.bollinger_bands = Some((upper, middle, lower));
        }
    }
    
    fn detect_patterns(&mut self) {
        self.patterns.clear();
        
        let candles: Vec<(&i64, &Candle)> = self.candles.iter().collect();
        
        // Detect simple patterns
        for window in candles.windows(3) {
            if let [prev, curr, next] = window {
                // Hammer pattern
                if curr.1.close > curr.1.open && 
                   (curr.1.high - curr.1.close) < (curr.1.close - curr.1.open) * 0.3 &&
                   (curr.1.open - curr.1.low) > (curr.1.close - curr.1.open) * 2.0 {
                    self.patterns.push(Pattern {
                        name: "Hammer".to_string(),
                        start_time: *curr.0,
                        end_time: *curr.0,
                        pattern_type: PatternType::Bullish,
                    });
                }
                
                // Shooting star pattern
                if curr.1.open > curr.1.close &&
                   (curr.1.high - curr.1.open) > (curr.1.open - curr.1.close) * 2.0 &&
                   (curr.1.close - curr.1.low) < (curr.1.open - curr.1.close) * 0.3 {
                    self.patterns.push(Pattern {
                        name: "Shooting Star".to_string(),
                        start_time: *curr.0,
                        end_time: *curr.0,
                        pattern_type: PatternType::Bearish,
                    });
                }
                
                // Engulfing patterns
                if prev.1.close < prev.1.open && curr.1.close > curr.1.open &&
                   curr.1.open <= prev.1.close && curr.1.close >= prev.1.open {
                    self.patterns.push(Pattern {
                        name: "Bullish Engulfing".to_string(),
                        start_time: *prev.0,
                        end_time: *curr.0,
                        pattern_type: PatternType::Bullish,
                    });
                }
            }
        }
    }
    
    fn find_support_resistance(&mut self) {
        self.support_resistance.clear();
        
        // Simple support/resistance based on local minima/maxima
        let prices: Vec<(i64, f64, f64)> = self.candles.iter()
            .map(|(&time, candle)| (time, candle.high, candle.low))
            .collect();
        
        let window_size = 10;
        
        for i in window_size..(prices.len() - window_size) {
            let current_high = prices[i].1;
            let current_low = prices[i].2;
            
            // Check for resistance (local maximum)
            let is_resistance = prices[(i - window_size)..i].iter()
                .chain(prices[(i + 1)..(i + window_size + 1)].iter())
                .all(|(_, high, _)| *high <= current_high);
            
            if is_resistance {
                self.support_resistance.push((current_high, "Resistance".to_string()));
            }
            
            // Check for support (local minimum)
            let is_support = prices[(i - window_size)..i].iter()
                .chain(prices[(i + 1)..(i + window_size + 1)].iter())
                .all(|(_, _, low)| *low >= current_low);
            
            if is_support {
                self.support_resistance.push((current_low, "Support".to_string()));
            }
        }
        
        // Remove duplicates
        self.support_resistance.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        self.support_resistance.dedup_by(|a, b| (a.0 - b.0).abs() < 0.01);
    }
    
    fn plot_candlesticks(&self, plot_ui: &mut PlotUi) {
        for candle in self.candles.values() {
            let x = candle.time as f64;
            let is_bullish = candle.close >= candle.open;
            let color = if is_bullish { self.config.up_color } else { self.config.down_color };
            
            // Draw wick (high-low line)
            plot_ui.line(Line::new(vec![[x, candle.low], [x, candle.high]])
                .color(color)
                .width(1.0));
            
            // Draw body (open-close rectangle)
            let body_top = candle.open.max(candle.close);
            let body_bottom = candle.open.min(candle.close);
            
            // Use box plot element for the body
            let box_elem = BoxElem::new(x, BoxSpread::new(
                body_bottom,  // min
                body_bottom,  // q1 (bottom of box)
                (body_top + body_bottom) / 2.0,  // median
                body_top,     // q3 (top of box)
                body_top      // max
            ))
            .box_width((self.config.candle_width * 86400.0) as f64) // Assuming daily data
            .fill(color)
            .stroke(Stroke::new(1.0, color));
            
            plot_ui.box_plot(BoxPlot::new(vec![box_elem]));
        }
    }
    
    fn plot_indicators(&self, plot_ui: &mut PlotUi) {
        // Moving averages
        for (period, points) in &self.moving_averages {
            plot_ui.line(Line::new(PlotPoints::new(points.clone()))
                .color(match period {
                    20 => Color32::from_rgb(255, 200, 100),
                    50 => Color32::from_rgb(100, 200, 255),
                    _ => Color32::from_rgb(200, 200, 200),
                })
                .width(2.0)
                .name(&format!("MA{}", period)));
        }
        
        // Bollinger bands
        if let Some((upper, middle, lower)) = &self.bollinger_bands {
            plot_ui.line(Line::new(PlotPoints::new(upper.clone()))
                .color(Color32::from_rgba_unmultiplied(150, 150, 255, 128))
                .width(1.0)
                .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                .name("BB Upper"));
            
            plot_ui.line(Line::new(PlotPoints::new(middle.clone()))
                .color(Color32::from_rgba_unmultiplied(150, 150, 255, 128))
                .width(1.0)
                .name("BB Middle"));
            
            plot_ui.line(Line::new(PlotPoints::new(lower.clone()))
                .color(Color32::from_rgba_unmultiplied(150, 150, 255, 128))
                .width(1.0)
                .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                .name("BB Lower"));
        }
        
        // Support/Resistance lines
        if self.config.show_support_resistance {
            for (level, level_type) in &self.support_resistance {
                let color = if level_type == "Resistance" {
                    Color32::from_rgba_unmultiplied(255, 100, 100, 100)
                } else {
                    Color32::from_rgba_unmultiplied(100, 255, 100, 100)
                };
                
                plot_ui.hline(egui_plot::HLine::new(*level)
                    .color(color)
                    .width(1.0)
                    .style(egui_plot::LineStyle::Dotted { spacing: 10.0 }));
            }
        }
        
        // Pattern markers
        if self.config.show_patterns {
            for pattern in &self.patterns {
                if let Some(candle) = self.candles.get(&pattern.start_time) {
                    let color = match pattern.pattern_type {
                        PatternType::Bullish => Color32::from_rgb(100, 255, 100),
                        PatternType::Bearish => Color32::from_rgb(255, 100, 100),
                        PatternType::Neutral => Color32::from_rgb(200, 200, 200),
                    };
                    
                    plot_ui.text(Text::new(
                        [pattern.start_time as f64, candle.high * 1.02].into(),
                        &pattern.name
                    )
                    .color(color));
                }
            }
        }
    }
}

impl SpaceView for CandlestickChart {
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
    fn view_type(&self) -> &str { "CandlestickView" }
    
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
                    self.cached_data = Some(batch.clone());
                    self.extract_data(&batch);
                }
            }
        }
        
        if !self.candles.is_empty() {
            // Configuration UI
            ui.horizontal(|ui| {
                ui.label("Indicators:");
                ui.checkbox(&mut self.config.show_bollinger, "Bollinger Bands");
                ui.checkbox(&mut self.config.show_support_resistance, "Support/Resistance");
                ui.checkbox(&mut self.config.show_patterns, "Patterns");
                
                ui.separator();
                ui.label("MA:");
                for period in [20, 50, 200] {
                    let mut show = self.config.show_ma.contains(&period);
                    if ui.checkbox(&mut show, &period.to_string()).changed() {
                        if show {
                            self.config.show_ma.push(period);
                        } else {
                            self.config.show_ma.retain(|&p| p != period);
                        }
                        self.calculate_moving_averages();
                    }
                }
            });
            
            // Main candlestick plot
            let plot = Plot::new(format!("candlestick_{:?}", self.id))
                .legend(Legend::default().position(Corner::LeftTop))
                .x_axis_formatter(|val, _range, _specs| {
                    if let Some(dt) = timestamp_s_to_datetime(val as i64) {
                        dt.format("%m/%d").to_string()
                    } else {
                        format!("{:.0}", val)
                    }
                })
                .auto_bounds_x()
                .auto_bounds_y();
            
            plot.show(ui, |plot_ui| {
                self.plot_candlesticks(plot_ui);
                self.plot_indicators(plot_ui);
            });
            
            // Volume chart (if enabled)
            if self.config.show_volume && self.candles.values().any(|c| c.volume.is_some()) {
                ui.separator();
                ui.label("Volume");
                
                let volume_plot = Plot::new(format!("volume_{:?}", self.id))
                    .height(100.0)
                    .x_axis_formatter(|val, _range, _specs| {
                        if let Some(dt) = timestamp_s_to_datetime(val as i64) {
                            dt.format("%m/%d").to_string()
                        } else {
                            format!("{:.0}", val)
                        }
                    })
                    .auto_bounds_x()
                    .auto_bounds_y()
                    .show_axes([false, true]);
                
                volume_plot.show(ui, |plot_ui| {
                    for candle in self.candles.values() {
                        if let Some(volume) = candle.volume {
                            let color = if candle.close >= candle.open {
                                self.config.up_color
                            } else {
                                self.config.down_color
                            };
                            
                            let bar = Bar::new(candle.time as f64, volume)
                                .width((self.config.candle_width * 86400.0) as f64)
                                .fill(color);
                            
                            plot_ui.bar_chart(egui_plot::BarChart::new(vec![bar]));
                        }
                    }
                });
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Select OHLC columns (Open, High, Low, Close) to create candlestick chart.");
            });
        }
    }
    
    fn save_config(&self) -> Value {
        json!({
            "time_column": self.config.time_column,
            "open_column": self.config.open_column,
            "high_column": self.config.high_column,
            "low_column": self.config.low_column,
            "close_column": self.config.close_column,
            "volume_column": self.config.volume_column,
            "show_volume": self.config.show_volume,
            "show_ma": self.config.show_ma,
            "show_bollinger": self.config.show_bollinger,
            "show_patterns": self.config.show_patterns,
        })
    }
    
    fn load_config(&mut self, config: Value) {
        if let Some(col) = config.get("time_column").and_then(|v| v.as_str()) {
            self.config.time_column = Some(col.to_string());
        }
        // Load other columns...
    }
    
    fn on_selection_change(&mut self, _ctx: &ViewerContext, _selection: &SelectionState) {}
    fn on_frame_update(&mut self, _ctx: &ViewerContext, _dt: f32) {}
} 