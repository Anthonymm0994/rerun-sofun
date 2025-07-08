import pandas as pd
import numpy as np
from datetime import datetime, timedelta

# Set random seed for reproducibility
np.random.seed(42)

# Generate time series data
dates = pd.date_range(start='2024-01-01', periods=100, freq='H')
time_series = pd.DataFrame({
    'timestamp': dates,
    'temperature': np.random.normal(20, 5, 100) + 10 * np.sin(np.arange(100) * 0.1),
    'humidity': np.random.normal(60, 10, 100),
    'pressure': np.random.normal(1013, 5, 100),
    'wind_speed': np.abs(np.random.normal(10, 5, 100))
})
time_series.to_csv('data/time_series_test.csv', index=False)

# Generate scatter plot data
scatter_data = pd.DataFrame({
    'x': np.random.randn(200),
    'y': np.random.randn(200),
    'size': np.random.uniform(1, 10, 200),
    'category': np.random.choice(['A', 'B', 'C'], 200)
})
scatter_data.to_csv('data/scatter_test.csv', index=False)

# Generate bar chart data
categories = ['Product A', 'Product B', 'Product C', 'Product D', 'Product E']
bar_data = pd.DataFrame({
    'category': categories * 4,
    'sales': np.random.uniform(100, 1000, 20),
    'region': ['North', 'South', 'East', 'West'] * 5
})
bar_data.to_csv('data/bar_chart_test.csv', index=False)

# Generate histogram data
histogram_data = pd.DataFrame({
    'height': np.random.normal(170, 10, 500),
    'weight': np.random.normal(70, 15, 500),
    'age': np.random.gamma(2, 2, 500) * 10
})
histogram_data.to_csv('data/histogram_test.csv', index=False)

# Generate box plot data
groups = ['Control', 'Treatment A', 'Treatment B', 'Treatment C']
box_data = pd.DataFrame({
    'group': np.repeat(groups, 50),
    'value': np.concatenate([
        np.random.normal(100, 15, 50),  # Control
        np.random.normal(110, 12, 50),  # Treatment A
        np.random.normal(105, 18, 50),  # Treatment B
        np.random.normal(115, 10, 50)   # Treatment C
    ])
})
box_data.to_csv('data/box_plot_test.csv', index=False)

# Generate heatmap data
heatmap_data = pd.DataFrame({
    'x': np.repeat(range(10), 10),
    'y': np.tile(range(10), 10),
    'value': np.random.randn(100)
})
heatmap_data.to_csv('data/heatmap_test.csv', index=False)

# Generate correlation matrix data
corr_data = pd.DataFrame({
    'var1': np.random.randn(100),
    'var2': np.random.randn(100),
    'var3': np.random.randn(100),
    'var4': np.random.randn(100),
    'var5': np.random.randn(100)
})
# Add some correlations
corr_data['var2'] = corr_data['var1'] * 0.8 + np.random.randn(100) * 0.2
corr_data['var4'] = -corr_data['var3'] * 0.6 + np.random.randn(100) * 0.4
corr_data.to_csv('data/correlation_test.csv', index=False)

# Generate 3D scatter data
scatter3d_data = pd.DataFrame({
    'x': np.random.randn(150),
    'y': np.random.randn(150),
    'z': np.random.randn(150),
    'cluster': np.random.choice(['A', 'B', 'C'], 150)
})
scatter3d_data.to_csv('data/scatter3d_test.csv', index=False)

# Generate radar chart data
radar_data = pd.DataFrame({
    'category': ['Team A', 'Team B', 'Team C'] * 2,
    'speed': [80, 90, 70, 85, 88, 75],
    'accuracy': [90, 85, 95, 88, 87, 92],
    'strength': [85, 80, 75, 82, 83, 78],
    'endurance': [70, 85, 80, 75, 82, 79],
    'flexibility': [75, 70, 85, 78, 72, 83]
})
radar_data.to_csv('data/radar_test.csv', index=False)

# Generate distribution data
distribution_data = pd.DataFrame({
    'normal': np.random.normal(0, 1, 1000),
    'exponential': np.random.exponential(2, 1000),
    'uniform': np.random.uniform(-3, 3, 1000),
    'bimodal': np.concatenate([
        np.random.normal(-2, 0.5, 500),
        np.random.normal(2, 0.5, 500)
    ])
})
distribution_data.to_csv('data/distribution_test.csv', index=False)

# Generate contour plot data (grid data)
x = np.linspace(-3, 3, 50)
y = np.linspace(-3, 3, 50)
X, Y = np.meshgrid(x, y)
Z = np.sin(np.sqrt(X**2 + Y**2))
contour_data = pd.DataFrame({
    'x': X.flatten(),
    'y': Y.flatten(),
    'z': Z.flatten()
})
contour_data.to_csv('data/contour_test.csv', index=False)

# Generate parallel coordinates data
parallel_data = pd.DataFrame({
    'sample_id': range(100),
    'feature1': np.random.randn(100),
    'feature2': np.random.randn(100),
    'feature3': np.random.randn(100),
    'feature4': np.random.randn(100),
    'feature5': np.random.randn(100),
    'class': np.random.choice(['Class A', 'Class B', 'Class C'], 100)
})
parallel_data.to_csv('data/parallel_coordinates_test.csv', index=False)

print("Test data files created successfully!")
print("\nCreated files:")
print("- data/time_series_test.csv")
print("- data/scatter_test.csv")
print("- data/bar_chart_test.csv")
print("- data/histogram_test.csv")
print("- data/box_plot_test.csv")
print("- data/heatmap_test.csv")
print("- data/correlation_test.csv")
print("- data/scatter3d_test.csv")
print("- data/radar_test.csv")
print("- data/distribution_test.csv")
print("- data/contour_test.csv")
print("- data/parallel_coordinates_test.csv") 