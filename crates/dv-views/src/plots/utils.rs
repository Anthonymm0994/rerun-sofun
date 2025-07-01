//! Utilities for plot views

pub mod colors;
pub mod stats;

// Re-export commonly used items
pub use colors::{categorical_color, viridis_color, plasma_color, diverging_color, ColorScheme};
pub use stats::{calculate_quartiles, detect_outliers_iqr, zscore_outliers}; 