#!/usr/bin/env python3

with open('wrt-decoder/src/resource_limits_section.rs', 'r') as f:
    content = f.read()

quotes = []
for i, char in enumerate(content):
    if char == '"':
        line_num = content[:i].count('\n') + 1
        quotes.append((i, line_num))

print(f"Found {len(quotes)} quotes")

# Check pairs
if len(quotes) % 2 != 0:
    print("ODD NUMBER OF QUOTES - UNTERMINATED STRING")
    print("Last few quotes:")
    for i, (pos, line) in enumerate(quotes[-10:]):
        start_idx = max(0, quotes[len(quotes)-10+i][0] - 20)
        end_idx = min(len(content), quotes[len(quotes)-10+i][0] + 20)
        context = content[start_idx:end_idx].replace('\n', '\\n')
        print(f"Quote #{len(quotes)-10+i+1} at line {line}, pos {pos}: ...{context}...")
    
    print(f"\nLast quote is at line {quotes[-1][1]}, position {quotes[-1][0]}")
    print("Context around last quote:")
    last_pos = quotes[-1][0]
    start = max(0, last_pos - 50)
    end = min(len(content), last_pos + 50)
    context = content[start:end]
    print(repr(context))
else:
    print("Even number of quotes - all strings properly closed")