"""519 measure: one bounded /v1/sql read through /root/sq.sh, cached by tag under a raw dir.
Stops at the first non-JSON or error answer (never retries a 408)."""
import json, os, subprocess, sys


def sql(raw_dir, tag, q):
    os.makedirs(raw_dir, exist_ok=True)
    path = os.path.join(raw_dir, tag + '.json')
    if os.path.exists(path):
        return json.load(open(path))
    r = subprocess.run(['ssh', '-o', 'BatchMode=yes', 'root@zebreus.click', '/root/sq.sh'],
                       input=q.encode(), capture_output=True)
    try:
        d = json.loads(r.stdout)
    except ValueError:
        sys.exit(f'{tag}: non-JSON answer: {r.stdout[:300]!r} {r.stderr[:300]!r}')
    if 'rows' not in d or 'columns' not in d:
        sys.exit(f'{tag}: error answer {str(d)[:300]}')
    if d.get('truncated'):
        sys.exit(f'{tag}: truncated')
    d['_sql'] = q
    with open(path + '.tmp', 'w') as f:
        json.dump(d, f)
    os.replace(path + '.tmp', path)
    return d
