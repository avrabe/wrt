#!/usr/bin/env python3

import re

# List of strings that need space before closing quote to fix Rust 2021 prefix errors
error_patterns = [
    r'"([^"]*?)(specified)"',
    r'"([^"]*?)(execution)"',
    r'"([^"]*?)(overflow)"',
    r'"([^"]*?)(short)"', 
    r'"([^"]*?)(version)"',
    r'"([^"]*?)(truncated)"',
    r'"([^"]*?)(value)"',
    r'"([^"]*?)(u32)"',
    r'"([^"]*?)(length)"',
    r'"([^"]*?)(data)"',
    r'"([^"]*?)(string)"',
    r'"([^"]*?)(count)"',
]

with open('wrt-decoder/src/resource_limits_section.rs', 'r') as f:
    content = f.read()

# Apply all replacements
for pattern in error_patterns:
    content = re.sub(pattern, r'"\1\2 "', content)

with open('wrt-decoder/src/resource_limits_section.rs', 'w') as f:
    f.write(content)

print("Fixed prefix errors")