#!/usr/bin/env python3
"""519 M6: hold the dry rekey plan to the reviewed expectation (spec §4.5 with Check fixes 1 and 3).

usage: plan_diff.py rekey-plan-dry.json review.json verdicts.json holds.json <hold cohort>
exit 0 = run the wet; 2 = holds written (POST them, re-run the dry, re-run this); 3 = STOP, read first.
(The spec's m6/verdicts.json is review.json here; its m6/post-body.json is verdicts.json.)"""
import json, sys


def cut(s, n):
    return s.encode()[:n].decode('utf-8', 'ignore')                     # bytes, not characters (Check fix 3)


rep, ver_p, body_p, out, hold_cohort = sys.argv[1:6]
r = json.load(open(rep))
plan = json.loads(r['body']) if isinstance(r.get('body'), str) else r
posted = {v['identifier']: v for v in json.load(open(body_p))['verdicts']}
m6_orgs = {v['org_id'] for v in posted.values()}                        # Check fix 1
exp = {v['identifier']: (v.get('expected') or {}) for v in json.load(open(ver_p))}
stops, holds, planned = [], [], set()
for l in plan['plan']:
    w = l['wrong']
    planned.add(w)
    v = posted.get(w)
    e = exp.get(w, {})
    if v is None:
        stops.append(f"{l['key']}: not an M6 verdict (an older cohort's key drifted into the plan): read it")
        continue
    if l['shape'] == 'move' and (v['correct_identifier'] or '').startswith('GBPPON'):
        holds.append(dict(v, confidence='medium', rationale=(cut(v['rationale'], 3700) +
                     ' | 519 M6 hold: no standing owner; a PPON move writes the bare serial (ruling 8)')))
        continue
    # Check fix 1: the pending-move second pass merges onto the literal another M6 row just moved to.
    if l['shape'] == 'merge' and e.get('shape') == 'move' and (l.get('target') or {}).get('org') in m6_orgs:
        continue
    if l['shape'] != e.get('shape'):
        stops.append(f"{l['key']}: planned {l['shape']}, reviewed {e.get('shape')}")
        continue
    got = (l.get('target') or {}).get('org')
    if l['shape'] == 'merge' and e.get('target') is not None and got != e['target']:
        stops.append(f"{l['key']}: target {got}, reviewed {e['target']}")
for l in plan['denied']:
    if l['wrong'] in posted:
        print(f"NOTE {l['wrong']}: denied {l['shape']} (residue; pending-move merges on the second pass)")
for w, v in posted.items():
    if v['confidence'] == 'high' and exp.get(w, {}).get('shape') in ('merge', 'move') and w not in planned \
            and not any(l['wrong'] == w for l in plan['denied']):
        print(f"NOTE {w}: reviewed {exp[w]['shape']}, neither planned nor denied (gone / several / unkeyed)")
print(f"plan {len(plan['plan'])} keys; holds {len(holds)}; stops {len(stops)}")
for s in stops:
    print('STOP', s)
if holds:
    json.dump({'cohort': hold_cohort, 'verdicts': holds}, open(out, 'w'), indent=1, ensure_ascii=False)
sys.exit(3 if stops else 2 if holds else 0)
