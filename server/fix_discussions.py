#!/usr/bin/env python3
with open('/root/mcguffin/server/src/discussions.rs', 'r') as f:
    lines = f.readlines()

# Line 197 (0-indexed 196): fix escaped backslashes and missing space
lines[196] = '                    return Json(serde_json::json!({"success": false, "message": format!("内容不能超过{} 字", CONTENT_MAX_LEN)}));\n'

# Line 257 (0-indexed 256): fix missing space before 字
lines[256] = '        return Json(serde_json::json!({"success": false, "message": format!("回复不能超过{} 字", REPLY_MAX_LEN)}));\n'

with open('/root/mcguffin/server/src/discussions.rs', 'w') as f:
    f.writelines(lines)
print("Fixed")
