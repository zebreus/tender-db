# Offline test of 548's top_stale lead on 524 v11 gate (d)'s 20 judged top_stale holds (plan 2562).
# Inputs (read-only): 524-v11/gates-2562/v11_gate_groups.json and rejudge2562_result.json.
# A judged "wrong" on a HOLD means the hold is HARMFUL (the change to new_name was better).
import json, collections
base = '/home/user/tender-db/.scratch/tender-db/524-v11/gates-2562/'
g = json.load(open(base + 'v11_gate_groups.json'))
r = json.load(open(base + 'rejudge2562_result.json'))['final']
items = [i for gr in g['groups'] if gr['key'].startswith('hold:top_stale') for i in gr['items']]
lab = {'wrong': 'harmful', 'right': 'right', 'neutral': 'neutral'}
t = collections.Counter()
for i in items:
    v = lab[r[str(i['org_id'])]['verdict']]
    fresher = i['rival_newest'] is not None and i['rival_newest'] > i['top_newest']
    arm = 'still held (rival fresher than top)' if fresher else 'released (top not superseded)'
    t[(arm, v)] += 1
    print(f"{i['org_id']:>9} {v:8} top_newest {i['top_newest']} rival_newest {i['rival_newest']} -> {arm}")
print()
for k in sorted(t): print(k, t[k])
