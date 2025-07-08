#!/usr/bin/env python3
"""Fix type issues with data_source_id in plots"""

import os
import re

def fix_data_source_id_type(filepath):
    """Fix data_source_id type issues and add missing fields"""
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    plot_name = os.path.basename(filepath)
    original_content = content
    
    # First check if file already has data_source_id
    has_data_source_id = bool(re.search(r'pub data_source_id:', content))
    
    # Files that need data_source_id field added (if not already present)
    missing_field_plots = [
        'sankey.rs', 'treemap.rs', 'sunburst.rs', 'network.rs', 
        'time_analysis.rs', 'candlestick.rs', 'stream.rs'
    ]
    
    if plot_name in missing_field_plots and not has_data_source_id:
        # Find the config struct
        config_pattern = r'(pub struct \w+Config\s*\{)'
        match = re.search(config_pattern, content)
        if match:
            # Add data_source_id as first field
            insert_pos = match.end()
            content = content[:insert_pos] + '\n    pub data_source_id: Option<String>,' + content[insert_pos:]
            
            # Also update Default implementation
            default_pattern = r'(impl Default for \w+Config\s*\{[^}]*fn default\(\) -> Self\s*\{[^{]*Self\s*\{)'
            match = re.search(default_pattern, content, re.DOTALL)
            if match:
                insert_pos = match.end()
                content = content[:insert_pos] + '\n            data_source_id: None,' + content[insert_pos:]
    
    # Remove duplicate data_source_id fields
    # Count occurrences
    field_pattern = r'pub data_source_id: Option<String>,'
    occurrences = len(re.findall(field_pattern, content))
    if occurrences > 1:
        # Keep only the first occurrence
        first_occurrence = content.find(field_pattern)
        if first_occurrence != -1:
            # Remove all subsequent occurrences
            remaining = content[first_occurrence + len(field_pattern):]
            remaining = remaining.replace(field_pattern, '')
            content = content[:first_occurrence + len(field_pattern)] + remaining
    
    # Remove duplicate data_source_id in Default impl
    default_field_pattern = r'data_source_id: None,'
    # Find all occurrences in Default impl
    default_impl_match = re.search(r'impl Default for \w+Config\s*\{[^}]*fn default\(\) -> Self\s*\{[^}]*\}', content, re.DOTALL)
    if default_impl_match:
        impl_content = default_impl_match.group(0)
        occurrences = impl_content.count(default_field_pattern)
        if occurrences > 1:
            # Keep only the first occurrence within the impl
            first_idx = impl_content.find(default_field_pattern)
            if first_idx != -1:
                before = impl_content[:first_idx + len(default_field_pattern)]
                after = impl_content[first_idx + len(default_field_pattern):].replace(default_field_pattern, '')
                new_impl = before + after
                content = content.replace(impl_content, new_impl)
    
    # Fix plots that have String instead of Option<String>
    # Change field type
    content = re.sub(
        r'pub data_source_id: String([,\s])',
        r'pub data_source_id: Option<String>\1',
        content
    )
    
    # Fix default values
    content = re.sub(
        r'data_source_id: String::new\(\)',
        'data_source_id: None',
        content
    )
    
    # Fix usage patterns where code expects String but now has Option<String>
    # Fix: self.config.data_source_id.is_empty() -> self.config.data_source_id.is_none()
    content = re.sub(
        r'!self\.config\.data_source_id\.is_empty\(\)',
        'self.config.data_source_id.is_some()',
        content
    )
    
    # Fix: data_sources.get(&self.config.data_source_id) -> data_sources.get(self.config.data_source_id.as_ref()?)
    # First pattern: in if blocks
    pattern1 = r'let data_source = if self\.config\.data_source_id\.is_some\(\) \{\s*data_sources\.get\(&self\.config\.data_source_id\)'
    replacement1 = 'let data_source = if let Some(source_id) = &self.config.data_source_id {\n            data_sources.get(source_id)'
    content = re.sub(pattern1, replacement1, content, flags=re.MULTILINE)
    
    # Alternative pattern with different spacing
    pattern2 = r'let data_source = if self\.config\.data_source_id\.is_some\(\) \{\s*data_sources\.get\(&self\.config\.data_source_id\)'
    replacement2 = 'let data_source = if let Some(source_id) = &self.config.data_source_id {\n                data_sources.get(source_id)'
    content = re.sub(pattern2, replacement2, content, flags=re.MULTILINE)
    
    # Fix set_data_source implementations
    content = re.sub(
        r'self\.config\.data_source_id = source_id;',
        'self.config.data_source_id = Some(source_id);',
        content
    )
    
    # Fix load_config patterns
    content = re.sub(
        r'self\.config\.data_source_id = data_source_id\.to_string\(\);',
        'self.config.data_source_id = Some(data_source_id.to_string());',
        content
    )
    
    # Fix polar.rs specific issue - missing comma after fields
    if plot_name == 'polar.rs':
        # Find the struct definition and ensure proper comma placement
        struct_pattern = r'(pub struct PolarPlot\s*\{[^}]*config: PolarPlotConfig)'
        match = re.search(struct_pattern, content, re.DOTALL)
        if match and not match.group(0).rstrip().endswith(','):
            content = content.replace(match.group(0), match.group(0) + ',')
    
    if content != original_content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        return True
    return False

def main():
    plots_dir = "crates/dv-views/src/plots"
    
    # Get all plot files
    plot_files = [f for f in os.listdir(plots_dir) if f.endswith('.rs') and f != 'mod.rs' and f != 'utils.rs']
    
    fixed_count = 0
    for plot_file in plot_files:
        filepath = os.path.join(plots_dir, plot_file)
        print(f"Processing {plot_file}...")
        
        if fix_data_source_id_type(filepath):
            print(f"  ✓ Fixed type issues")
            fixed_count += 1
    
    print(f"\nFixed {fixed_count} files")

if __name__ == "__main__":
    main() 