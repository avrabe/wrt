#!/usr/bin/env python3

import os
import glob

files_with_odd_quotes = []
for rust_file in glob.glob('**/*.rs', recursive=True):
    try:
        with open(rust_file, 'r', encoding='utf-8') as f:
            content = f.read()
        quote_count = content.count('"')
        if quote_count % 2 == 1:
            files_with_odd_quotes.append((rust_file, quote_count))
    except Exception as e:
        continue

print(f'Files with odd quotes: {len(files_with_odd_quotes)}')
for file, count in sorted(files_with_odd_quotes):
    print(f'{file}: {count} quotes (ODD)')