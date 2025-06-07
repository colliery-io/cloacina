#!/usr/bin/env python3
"""
Script to convert @cloaca.workflow decorators to context manager pattern.
"""

import re
import sys

def convert_workflow_decorators(content):
    """Convert @cloaca.workflow decorators to context manager pattern."""
    
    # Pattern to match workflow decorator and function definition
    pattern = r'@cloaca\.workflow\("([^"]+)"(?:,\s*"([^"]+)")?\)\s*\n\s*def\s+(\w+)\(\):\s*\n(\s+)builder\s*=\s*cloaca\.WorkflowBuilder\("([^"]+)"\)\s*\n((?:\s+builder\.[^\n]+\n)*)\s*return\s+builder\.build\(\)'
    
    def replacement(match):
        workflow_name = match.group(1)
        description = match.group(2) or ""
        function_name = match.group(3)
        indent = match.group(4)
        builder_name = match.group(5)
        builder_calls = match.group(6)
        
        # Build the context manager version
        result = f'# Create workflow using context manager\n{indent[:-4]}with cloaca.WorkflowBuilder("{workflow_name}") as builder:'
        if description:
            result += f'\n{indent}builder.description("{description}")'
        
        # Add the builder calls (remove the extra indentation)
        if builder_calls.strip():
            for line in builder_calls.strip().split('\n'):
                if line.strip():
                    result += f'\n{indent}{line.strip()}'
        
        return result
    
    # Apply the replacement
    content = re.sub(pattern, replacement, content, flags=re.MULTILINE | re.DOTALL)
    
    return content

def main():
    if len(sys.argv) != 2:
        print("Usage: python fix_workflow_decorators.py <filename>")
        return
    
    filename = sys.argv[1]
    
    with open(filename, 'r') as f:
        content = f.read()
    
    new_content = convert_workflow_decorators(content)
    
    with open(filename, 'w') as f:
        f.write(new_content)
    
    print(f"Updated {filename}")

if __name__ == "__main__":
    main()