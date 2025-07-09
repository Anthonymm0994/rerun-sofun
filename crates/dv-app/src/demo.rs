//! Demo mode for data visualizer
//! Creates synthetic data to showcase features

use std::sync::Arc;
use arrow::array::{ArrayRef, Float64Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use dv_core::data::DataSource;
use dv_core::navigation::{NavigationSpec, NavigationMode, NavigationPosition, NavigationRange};
use anyhow::Result;

/// Demo data source that generates synthetic data
pub struct DemoDataSource {
    schema: SchemaRef,
    total_rows: usize,
}

impl DemoDataSource {
    pub fn new() -> Self {
        // Create a rich schema with multiple data types
        let schema = Arc::new(Schema::new(vec![
            // Time axis
            Field::new("time", DataType::Float64, false),
            Field::new("timestamp", DataType::Int64, false),
            
            // Basic waveforms
            Field::new("sin_wave", DataType::Float64, false),
            Field::new("cos_wave", DataType::Float64, false),
            Field::new("sawtooth", DataType::Float64, false),
            Field::new("square_wave", DataType::Float64, false),
            
            // Signal decomposition
            Field::new("combined", DataType::Float64, false),
            Field::new("trend", DataType::Float64, false),
            Field::new("seasonal", DataType::Float64, false),
            Field::new("noise", DataType::Float64, false),
            
            // Physics simulation
            Field::new("position_x", DataType::Float64, false),
            Field::new("position_y", DataType::Float64, false),
            Field::new("velocity_x", DataType::Float64, false),
            Field::new("velocity_y", DataType::Float64, false),
            Field::new("acceleration", DataType::Float64, false),
            Field::new("energy", DataType::Float64, false),
            
            // Business metrics
            Field::new("revenue", DataType::Float64, false),
            Field::new("cost", DataType::Float64, false),
            Field::new("profit", DataType::Float64, false),
            Field::new("margin", DataType::Float64, false),
            
            // Assembly line metrics (new!)
            Field::new("station_1_throughput", DataType::Float64, false),
            Field::new("station_2_throughput", DataType::Float64, false),
            Field::new("station_3_throughput", DataType::Float64, false),
            Field::new("defect_rate", DataType::Float64, false),
            Field::new("efficiency", DataType::Float64, false),
            Field::new("buffer_level", DataType::Float64, false),
            
            // Network performance
            Field::new("cpu_usage", DataType::Float64, false),
            Field::new("memory_usage", DataType::Float64, false),
            Field::new("network_latency", DataType::Float64, false),
            Field::new("requests_per_sec", DataType::Float64, false),
            Field::new("error_rate", DataType::Float64, false),
            
            // Categories
            Field::new("category", DataType::Utf8, false),
            Field::new("status", DataType::Utf8, false),
        ]));
        
        Self {
            schema,
            total_rows: 1000,
        }
    }
    
    /// Generate a batch of synthetic data at the given position
    fn generate_batch(&self, start_idx: usize, count: usize) -> RecordBatch {
        let mut arrays: Vec<ArrayRef> = Vec::new();
        
        // Generate data for each row
        let mut time_values = Vec::new();
        let mut timestamp_values = Vec::new();
        let mut sin_values = Vec::new();
        let mut cos_values = Vec::new();
        let mut sawtooth_values = Vec::new();
        let mut square_values = Vec::new();
        let mut combined_values = Vec::new();
        let mut trend_values = Vec::new();
        let mut seasonal_values = Vec::new();
        let mut noise_values = Vec::new();
        let mut pos_x_values = Vec::new();
        let mut pos_y_values = Vec::new();
        let mut vel_x_values = Vec::new();
        let mut vel_y_values = Vec::new();
        let mut accel_values = Vec::new();
        let mut energy_values = Vec::new();
        let mut revenue_values = Vec::new();
        let mut cost_values = Vec::new();
        let mut profit_values = Vec::new();
        let mut margin_values = Vec::new();
        let mut station_1_values = Vec::new();
        let mut station_2_values = Vec::new();
        let mut station_3_values = Vec::new();
        let mut defect_rate_values = Vec::new();
        let mut efficiency_values = Vec::new();
        let mut buffer_level_values = Vec::new();
        let mut cpu_values = Vec::new();
        let mut memory_values = Vec::new();
        let mut latency_values = Vec::new();
        let mut rps_values = Vec::new();
        let mut error_rate_values = Vec::new();
        let mut category_values = Vec::new();
        let mut status_values = Vec::new();
        
        for i in 0..count {
            let idx = (start_idx + i) as f64;
            let t = idx * 0.01; // Time in seconds
            
            // Time values
            time_values.push(t);
            timestamp_values.push((t * 1000.0) as i64);
            
            // Basic waveforms
            sin_values.push((t * 2.0).sin());
            cos_values.push((t * 2.0).cos());
            sawtooth_values.push((t % 1.0) * 2.0 - 1.0);
            square_values.push(if (t * 2.0).sin() > 0.0 { 1.0 } else { -1.0 });
            
            // Signal decomposition (combined = trend + seasonal + noise)
            let trend = t * 0.1 + (t * 0.05).sin() * 0.5;
            let seasonal = (t * 4.0).sin() * 0.3 + (t * 8.0).sin() * 0.1;
            let noise = (idx * 12345.6789).sin() * 0.1; // Pseudo-random
            let combined = trend + seasonal + noise;
            
            combined_values.push(combined);
            trend_values.push(trend);
            seasonal_values.push(seasonal);
            noise_values.push(noise);
            
            // Physics simulation - orbital motion with decay
            let angle = t * 0.5;
            let radius = 10.0 * (1.0 - t * 0.0005).max(0.1);
            let pos_x = radius * angle.cos();
            let pos_y = radius * angle.sin();
            let vel_mag = (radius * 0.5).sqrt();
            let vel_x = -vel_mag * angle.sin();
            let vel_y = vel_mag * angle.cos();
            let acceleration = vel_mag * vel_mag / radius;
            let energy = 0.5 * vel_mag * vel_mag;
            
            pos_x_values.push(pos_x);
            pos_y_values.push(pos_y);
            vel_x_values.push(vel_x);
            vel_y_values.push(vel_y);
            accel_values.push(acceleration);
            energy_values.push(energy);
            
            // Business metrics with realistic patterns
            let day_of_week = (idx / 100.0) % 7.0;
            let is_weekend = day_of_week >= 5.0;
            let base_revenue = 10000.0 + (t * 10.0).sin() * 2000.0;
            let revenue = if is_weekend { base_revenue * 0.7 } else { base_revenue };
            let cost = 6000.0 + (t * 8.0).cos() * 500.0 + noise * 200.0;
            let profit = revenue - cost;
            let margin = if revenue > 0.0 { profit / revenue } else { 0.0 };
            
            revenue_values.push(revenue);
            cost_values.push(cost);
            profit_values.push(profit);
            margin_values.push(margin);
            
            // Assembly line simulation
            let base_throughput = 100.0;
            let station_1 = base_throughput + (t * 3.0).sin() * 10.0 + noise * 5.0;
            let station_2 = station_1 * 0.95 + (t * 4.0).cos() * 8.0; // Slightly slower
            let station_3 = station_2.min(station_1) * 0.9 + (t * 5.0).sin() * 6.0; // Bottleneck
            
            // Defect rate increases with throughput mismatch
            let throughput_variance = ((station_1 - station_2).abs() + (station_2 - station_3).abs()) / base_throughput;
            let defect_rate = (0.02 + throughput_variance * 0.1).min(0.15);
            
            // Efficiency based on utilization and defect rate
            let min_throughput = station_1.min(station_2).min(station_3);
            let efficiency = (min_throughput / base_throughput) * (1.0 - defect_rate);
            
            // Buffer level oscillates based on throughput differences
            let buffer_level = 50.0 + (station_1 - station_3) * t.sin() * 2.0;
            
            station_1_values.push(station_1);
            station_2_values.push(station_2);
            station_3_values.push(station_3);
            defect_rate_values.push(defect_rate * 100.0); // As percentage
            efficiency_values.push(efficiency * 100.0); // As percentage
            buffer_level_values.push(buffer_level.max(0.0).min(100.0));
            
            // Network performance metrics
            let base_cpu = 30.0;
            let cpu_spike = if idx % 200.0 < 10.0 { 40.0 } else { 0.0 }; // Periodic spikes
            let cpu = base_cpu + (t * 2.0).sin() * 10.0 + cpu_spike + noise * 5.0;
            
            let memory = 40.0 + t * 0.05 + (t * 1.5).cos() * 5.0; // Slowly increasing
            let latency = 10.0 + (cpu / 100.0) * 20.0 + noise * 2.0; // Correlated with CPU
            let rps = 1000.0 - (cpu - 50.0).max(0.0) * 10.0; // Drops when CPU is high
            let error_rate = ((cpu - 80.0).max(0.0) / 100.0).min(0.1);
            
            cpu_values.push(cpu.min(100.0));
            memory_values.push(memory.min(95.0));
            latency_values.push(latency);
            rps_values.push(rps.max(0.0));
            error_rate_values.push(error_rate * 100.0); // As percentage
            
            // Categories
            category_values.push(match (idx as usize / 100) % 4 {
                0 => "Production",
                1 => "Testing",
                2 => "Maintenance",
                _ => "Idle",
            });
            
            status_values.push(if efficiency > 85.0 { "Optimal" } else if efficiency > 70.0 { "Normal" } else { "Warning" });
        }
        
        // Create arrays
        arrays.push(Arc::new(Float64Array::from(time_values)));
        arrays.push(Arc::new(Int64Array::from(timestamp_values)));
        arrays.push(Arc::new(Float64Array::from(sin_values)));
        arrays.push(Arc::new(Float64Array::from(cos_values)));
        arrays.push(Arc::new(Float64Array::from(sawtooth_values)));
        arrays.push(Arc::new(Float64Array::from(square_values)));
        arrays.push(Arc::new(Float64Array::from(combined_values)));
        arrays.push(Arc::new(Float64Array::from(trend_values)));
        arrays.push(Arc::new(Float64Array::from(seasonal_values)));
        arrays.push(Arc::new(Float64Array::from(noise_values)));
        arrays.push(Arc::new(Float64Array::from(pos_x_values)));
        arrays.push(Arc::new(Float64Array::from(pos_y_values)));
        arrays.push(Arc::new(Float64Array::from(vel_x_values)));
        arrays.push(Arc::new(Float64Array::from(vel_y_values)));
        arrays.push(Arc::new(Float64Array::from(accel_values)));
        arrays.push(Arc::new(Float64Array::from(energy_values)));
        arrays.push(Arc::new(Float64Array::from(revenue_values)));
        arrays.push(Arc::new(Float64Array::from(cost_values)));
        arrays.push(Arc::new(Float64Array::from(profit_values)));
        arrays.push(Arc::new(Float64Array::from(margin_values)));
        arrays.push(Arc::new(Float64Array::from(station_1_values)));
        arrays.push(Arc::new(Float64Array::from(station_2_values)));
        arrays.push(Arc::new(Float64Array::from(station_3_values)));
        arrays.push(Arc::new(Float64Array::from(defect_rate_values)));
        arrays.push(Arc::new(Float64Array::from(efficiency_values)));
        arrays.push(Arc::new(Float64Array::from(buffer_level_values)));
        arrays.push(Arc::new(Float64Array::from(cpu_values)));
        arrays.push(Arc::new(Float64Array::from(memory_values)));
        arrays.push(Arc::new(Float64Array::from(latency_values)));
        arrays.push(Arc::new(Float64Array::from(rps_values)));
        arrays.push(Arc::new(Float64Array::from(error_rate_values)));
        arrays.push(Arc::new(StringArray::from(category_values)));
        arrays.push(Arc::new(StringArray::from(status_values)));
        
        RecordBatch::try_new(self.schema.clone(), arrays).unwrap()
    }
}

#[async_trait]
impl DataSource for DemoDataSource {
    async fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
    
    async fn navigation_spec(&self) -> Result<NavigationSpec> {
        Ok(NavigationSpec {
            mode: NavigationMode::Sequential,
            total_rows: self.total_rows,
            temporal_bounds: None,
            categories: None,
        })
    }
    
    async fn query_at(&self, position: &NavigationPosition) -> Result<RecordBatch> {
        let start_idx = position.frame_nr();
        let count = 100; // Return 100 rows for smooth visualization
        
        Ok(self.generate_batch(start_idx, count))
    }
    
    async fn query_range(&self, range: &NavigationRange) -> Result<RecordBatch> {
        let start = range.start.frame_nr();
        let end = range.end.frame_nr();
        let count = (end - start).min(1000); // Cap at 1000 rows
        
        Ok(self.generate_batch(start, count))
    }
    
    async fn row_count(&self) -> Result<usize> {
        Ok(self.total_rows)
    }
    
    async fn query_all(&self) -> Result<RecordBatch> {
        // Return all rows - for demo we limit to 1000 rows total
        Ok(self.generate_batch(0, self.total_rows))
    }
    
    fn source_name(&self) -> &str {
        "Demo: Assembly Line & Manufacturing Analytics"
    }
}

/// Create a demo data source
pub fn create_demo_data_source() -> DemoDataSource {
    DemoDataSource::new()
} 