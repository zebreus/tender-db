import json, os, collections, sys
sys.dont_write_bytecode = True
S = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/fresh570'
D = S + '/changes_lot'
ev = collections.defaultdict(list)
for f in sorted(os.listdir(D)):
    if f.startswith('p') and f.endswith('.json'):
        for e in json.load(open(os.path.join(D, f)))['events']:
            ev[e['id']].append((e['op'], e['version'], int(e['cursor'])))
tev = {}
for f in sorted(os.listdir(S + '/changes_tender')):
    if f.startswith('p') and f.endswith('.json'):
        for e in json.load(open(os.path.join(S + '/changes_tender', f)))['events']:
            tev[e['id']] = int(e['cursor'])
held = json.load(open(S + '/walk.json'))['held_ids']
tot = one = 0; missing = {}
for tid in held:
    t = json.load(open(f'{S}/cache/tenders_{tid}.json'))
    for l in t['lot_details']:
        tot += 1
        e = ev.get(l['id'], [])
        if len(e) == 1 and e[0][0] == 'changed' and e[0][1] is None:
            one += 1
        else:
            missing.setdefault(tid, []).append((l['lot_key'], e))
print('lots', tot, 'with exactly one changed/null', one, 'others', missing)
# contiguity for 6876932: lot cursors vs its tender cursor
t = json.load(open(f'{S}/cache/tenders_6876932.json'))
cs = sorted((ev[l['id']][0][2], l['lot_key']) for l in t['lot_details'] if ev.get(l['id']))
print('tender cursor', tev[6876932], 'lot cursors', cs[0], cs[-1], len(cs))
