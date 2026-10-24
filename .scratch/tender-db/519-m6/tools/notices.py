#!/usr/bin/env python3
"""519 M6: the refold pairing list (spec §4.6; needed only if 583 U2 is not live).

usage: notices.py review.json verdicts.json live.json refold-notices.json [rekey-plan-final.json]

The distinct notice_id of every R2 mention whose org's shape is `merge`: from the final dry plan when
one is given (the authoritative list), else from the reviewed expectation of the rows posted at high.
Moves keep their org id and need no refold. Chunks of <= 1,000 (REFOLD_NOTICES_CAP, supervisor.rs:237)
go to refold-notices.json, refold-notices-2.json, ...
"""
import json, sys

ver_p, body_p, live_p, out = sys.argv[1:5]
plan_p = sys.argv[5] if len(sys.argv) > 5 else None
body = {v['org_id']: v for v in json.load(open(body_p))['verdicts']}
if plan_p:
    r = json.load(open(plan_p))
    plan = json.loads(r['body']) if isinstance(r.get('body'), str) else r
    merged = {l['org'] for l in plan['plan'] if l['shape'] == 'merge'}
    src = 'final dry plan'
else:
    merged = {v['org'] for v in json.load(open(ver_p))
              if (v.get('expected') or {}).get('shape') == 'merge' and body.get(v['org'], {}).get('confidence') == 'high'}
    src = 'reviewed expectation (high rows)'
notices = sorted({m[1] for m in json.load(open(live_p))['mentions'] if m[0] in merged})
chunks = [notices[i:i + 1000] for i in range(0, len(notices), 1000)] or [[]]
for i, ch in enumerate(chunks):
    path = out if i == 0 else out.replace('.json', f'-{i + 1}.json')
    json.dump({'kind': 'refold-notices', 'notices': ch}, open(path, 'w'))
    print(path, len(ch))
print(f'{src}: {len(merged)} merged orgs, {len(notices)} distinct notices, {len(chunks)} chunk(s)')
