#!/usr/bin/env python3
"""Fix data_source_id assignment issues in view_builder.rs"""

import re

def fix_view_builder():
    filepath = "crates/dv-app/src/view_builder.rs"
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Pattern 1: Fix assignments that unwrap_or_else to String but need Option<String>
    # These are the cases where it assigns: data_source_id.clone().unwrap_or_else(|| ...)
    pattern1 = r'(view\.config\.data_source_id = )(data_source_id\.clone\(\)\.unwrap_or_else\(\|\| \{[^}]+\}\))(;)'
    replacement1 = r'\1Some(\2)\3'
    content = re.sub(pattern1, replacement1, content, flags=re.MULTILINE | re.DOTALL)
    
    # Pattern 2: Fix assignments that use or_else but still return String
    # view.config.data_source_id = data_source_id.clone().or_else(|| {
    #     self.data_sources.first().map(|(id, _)| id.clone())
    # });
    # This pattern already returns Option<String> so it should be fine
    
    # Pattern 3: Make sure assignments that already use or_else are correct
    # These should already be returning Option<String>
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print(f"Fixed view_builder.rs")

if __name__ == "__main__":
    fix_view_builder() 