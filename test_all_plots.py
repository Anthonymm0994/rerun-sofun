#!/usr/bin/env python3
"""Verify all plot types are implemented in the code"""

import os
import re

def check_plots():
    plots_dir = "crates/dv-views/src/plots"
    plot_files = [
        'anomaly.rs', 'bar.rs', 'box_plot.rs', 'candlestick.rs', 
        'contour.rs', 'correlation.rs', 'distribution.rs', 'geo.rs',
        'heatmap.rs', 'histogram.rs', 'line.rs', 'network.rs',
        'parallel_coordinates.rs', 'polar.rs', 'radar.rs', 'sankey.rs',
        'scatter.rs', 'scatter3d.rs', 'stream.rs', 'sunburst.rs',
        'surface3d.rs', 'time_analysis.rs', 'treemap.rs', 'violin.rs'
    ]
    
    all_good = True
    for plot_file in plot_files:
        filepath = os.path.join(plots_dir, plot_file)
        if os.path.exists(filepath):
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()
                # Check for data_source_id field
                if 'pub data_source_id: Option<String>' in content:
                    print(f"✓ {plot_file}: Has Option<String> data_source_id")
                else:
                    print(f"✗ {plot_file}: Missing proper data_source_id")
                    all_good = False
        else:
            print(f"✗ {plot_file}: File not found")
            all_good = False
    
    return all_good

if __name__ == "__main__":
    if check_plots():
        print("\n✓ All plots have proper data_source_id configuration")
    else:
        print("\n✗ Some plots need fixes") 