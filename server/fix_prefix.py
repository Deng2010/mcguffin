#!/usr/bin/env python3
with open('/root/mcguffin/server/src/discussions.rs', 'r') as f:
    content = f.read()

# Fix all remaining "{}字" -> "{} 字" (add space before 字 to avoid Rust 2021 prefix parsing)
content = content.replace('{}字', '{} 字')

with open('/root/mcguffin/server/src/discussions.rs', 'w') as f:
    f.write(content)
print("Fixed all {}字 -> {} 字")
