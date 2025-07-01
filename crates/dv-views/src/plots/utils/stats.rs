//! Statistical utilities for plots

use ndarray::{Array1, s};

/// Calculate quartiles using linear interpolation
pub fn calculate_quartiles(values: &[f64]) -> (f64, f64, f64) {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let n = sorted.len();
    if n == 0 {
        return (0.0, 0.0, 0.0);
    }
    
    let q1_idx = (n - 1) as f64 * 0.25;
    let q2_idx = (n - 1) as f64 * 0.5;
    let q3_idx = (n - 1) as f64 * 0.75;
    
    let q1 = interpolate(&sorted, q1_idx);
    let q2 = interpolate(&sorted, q2_idx);
    let q3 = interpolate(&sorted, q3_idx);
    
    (q1, q2, q3)
}

fn interpolate(sorted: &[f64], idx: f64) -> f64 {
    let lower = idx.floor() as usize;
    let upper = idx.ceil() as usize;
    
    if lower == upper || upper >= sorted.len() {
        sorted[lower]
    } else {
        let fraction = idx - lower as f64;
        sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
    }
}

/// Detect outliers using IQR method
pub fn detect_outliers_iqr(values: &[f64]) -> Vec<usize> {
    let (q1, _, q3) = calculate_quartiles(values);
    let iqr = q3 - q1;
    let lower_fence = q1 - 1.5 * iqr;
    let upper_fence = q3 + 1.5 * iqr;
    
    values.iter()
        .enumerate()
        .filter(|(_, &v)| v < lower_fence || v > upper_fence)
        .map(|(i, _)| i)
        .collect()
}

/// Detect outliers using z-score method
pub fn zscore_outliers(values: &[f64], threshold: f64) -> Vec<usize> {
    if values.is_empty() {
        return Vec::new();
    }
    
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter()
        .map(|v| (v - mean).powi(2))
        .sum::<f64>() / values.len() as f64;
    let std_dev = variance.sqrt();
    
    if std_dev == 0.0 {
        return Vec::new();
    }
    
    values.iter()
        .enumerate()
        .filter(|(_, &v)| ((v - mean) / std_dev).abs() > threshold)
        .map(|(i, _)| i)
        .collect()
} 