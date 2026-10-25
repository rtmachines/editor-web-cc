#!/usr/bin/env bash
# Check syntax and reject shell tracing before the stack starts.
set -euo pipefail
umask 077

for script in deploy/ci/*.sh
do
  bash -n "$script"
done
python3 -I -B deploy/ci/test_http_probe.py
python3 -I -B - <<'PY'
from pathlib import Path
import re

paths = [Path('.github/workflows/ci.yml')]
paths.extend(Path('deploy/ci').glob('*'))
paths.extend(Path('deploy/pgtools/bin').glob('*'))
paths.extend(Path('deploy/pgtools/tests').rglob('*'))
for path in paths:
    if not path.is_file():
        continue
    data = path.read_bytes()
    if path.suffix == '.sh' and b'\r' in data:
        raise SystemExit(f'CRLF in {path}')
    # Comments can describe the ban. Only a shell command enables tracing.
    if re.search(rb'^\s*set\s+[^\n#]*(-[a-zA-Z]*x|\bxtrace\b)', data, re.M):
        raise SystemExit(f'shell tracing in {path}')
print('CI syntax and tracing checks passed.')
PY
