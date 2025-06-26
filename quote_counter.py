#!/usr/bin/env python3

with open('wrt-decoder/src/component/utils.rs', 'r') as f:
    content = f.read()

quote_count = 0
for i, char in enumerate(content):
    if char == '"':
        quote_count += 1
        line_num = content[:i].count('\n') + 1
        print(f'Quote #{quote_count} at line {line_num}, position {i}')

print(f'Total quotes: {quote_count}')
if quote_count % 2 != 0:
    print('ERROR: Odd number of quotes detected!')
else:
    print('OK: Even number of quotes')