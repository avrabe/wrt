"""Quick helper to explode the single‑file scaffold into real files.
Run from repo root:
    python tools/split_scaffold.py Wrt\ Ai\ Full\ Scaffold.txt
"""
import sys, pathlib, re, textwrap

if len(sys.argv) < 2:
    print("usage: split_scaffold.py <scaffold-file>"); sys.exit(1)
scaffold = pathlib.Path(sys.argv[1]).read_text().splitlines()
current_path, buf = None, []
for line in scaffold:
    m = re.match(r"=== PATH: (.+?) ===", line)
    if m:
        if current_path:
            pathlib.Path(current_path).parent.mkdir(parents=True, exist_ok=True)
            pathlib.Path(current_path).write_text("\n".join(buf).rstrip("\n") + "\n")
        current_path, buf = m.group(1), []
    else:
        buf.append(line)
if current_path:
    pathlib.Path(current_path).parent.mkdir(parents=True, exist_ok=True)
    pathlib.Path(current_path).write_text("\n".join(buf).rstrip("\n") + "\n")
print("Scaffold split complete ✂️") 