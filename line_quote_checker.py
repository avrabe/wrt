#!/usr/bin/env python3

with open('wrt-decoder/src/component/utils.rs', 'r') as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    quote_count = line.count('"')
    if quote_count % 2 != 0:
        print(f'Line {i+1}: Odd quotes ({quote_count}): {line.strip()}')