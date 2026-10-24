"""readback_check.py <readback.sql output JSON ({"columns","rows"})> <cohort.json>
After wet + project: each rehome mention sits on its target, each keep on 49, none on 36."""
import json, sys
out = json.load(open(sys.argv[1])); coh = json.load(open(sys.argv[2]))
at = {(r[0], r[1]): r[2] for r in out['rows']}
bad = []
for v in coh['verdicts']:
    k = (v['notice_id'], v['section_id'])
    want = v['target_org_id'] if v['action'] == 'rehome' else v['org_id']
    if at.get(k) != want:
        bad.append((k, v['action'], 'want', want, 'sits on', at.get(k)))
print(f"{len(coh['verdicts']) - len(bad)}/{len(coh['verdicts'])} as expected")
for b in bad: print('  ', b, '(36 = re-resolved by E0 after the move: see runbook F)' if b[-1] == 36 else '')
sys.exit(0 if not bad else 1)
