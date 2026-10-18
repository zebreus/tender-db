"""Band stage step 2: diff the re-read band and path-V heads against the baseline; the changed set from the pages."""
import glob, hashlib, json, os, sys

RAW = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASELINE = '/home/user/tender-db/.scratch/tender-db/568-r208-lots/drain/baseline-pre568u8-570.json'
LO, HI = 957854566, 958764334

base = json.load(open(BASELINE))

# changed set
pages = sorted(glob.glob(os.path.join(RAW, 'changes', 'p*.json')))
h = hashlib.sha256()
ev_all, gens, ops, cat = [], set(), set(), set()
for p in pages:
    b = open(p, 'rb').read()
    h.update(b)
    d = json.loads(b)
    gens.add(d.get('generation'))
    for e in d['events']:
        ev_all.append(e)
in_range = [e for e in ev_all if LO < int(e['cursor']) <= HI]
changed = {}
for e in in_range:
    changed.setdefault(e['id'], []).append(e)
summary_changes = {
    'pages': len(pages), 'generation': sorted(gens), 'events_read': len(ev_all), 'events_in_range': len(in_range),
    'distinct_ids': len(changed), 'ops': sorted({(e['op'], e['version']) for e in in_range}, key=str),
    'entities': sorted({e['entity'] for e in in_range}), 'changed_at': sorted({e['changed_at'] for e in in_range}),
    'first_cursor': min(int(e['cursor']) for e in in_range), 'last_cursor': max(int(e['cursor']) for e in in_range),
    'last_page_last_cursor': json.load(open(pages[-1]))['last_cursor'], 'last_page_more': json.load(open(pages[-1])).get('more'),
    'events_beyond_hi': len([e for e in ev_all if int(e['cursor']) > HI]),
    'sha256_of_concatenated_pages': h.hexdigest(),
}

# band now
band = {}
bpages = sorted(glob.glob(os.path.join(RAW, 'bandpages', 'b*.json')))
dups = 0
for p in bpages:
    d = json.load(open(p))
    for t in d['items']:
        if str(t['id']) in band:
            dups += 1
        band[str(t['id'])] = {'value': t['value'], 'derivation': t.get('value_derivation'), 'version': t['version']}

b0 = base['band']
entered = sorted(set(band) - set(b0), key=int)
left = sorted(set(b0) - set(band), key=int)
moved = []
for k in sorted(set(band) & set(b0), key=int):
    diffs = [f for f in ('value', 'derivation', 'version') if band[k][f] != b0[k][f]]
    if diffs:
        moved.append({'id': int(k), 'fields': diffs, 'before': b0[k], 'now': band[k]})

# path-V
pv = []
for k, before in sorted(base['path_v'].items(), key=lambda kv: int(kv[0])):
    t = json.load(open(os.path.join(RAW, 'pathv', f't{k}.json')))
    now = {'value': t.get('value'), 'derivation': t.get('value_derivation'), 'version': t.get('version')}
    diffs = [f for f in ('value', 'derivation', 'version') if now[f] != before[f]]
    pv.append({'id': int(k), 'fields': diffs, 'before': before, 'now': now, 'in_changed_set': int(k) in changed})

movers = sorted({m['id'] for m in moved} | {int(x) for x in entered} | {int(x) for x in left}
                | {p['id'] for p in pv if p['fields']})
out = {
    'changes': summary_changes,
    'band': {'baseline_n': len(b0), 'now_n': len(band), 'now_pages': len(bpages), 'dup_ids_across_pages': dups,
             'entered': [int(x) for x in entered], 'left': [int(x) for x in left], 'moved_in_band': moved},
    'path_v': pv,
    'movers': [{'id': i, 'in_changed_set': i in changed,
                'events': [{'cursor': e['cursor'], 'op': e['op'], 'version': e['version']} for e in changed.get(i, [])]}
               for i in movers],
    'band_in_changed_set': sorted(int(k) for k in band if int(k) in changed),
    'baseline_band_in_changed_set': sorted(int(k) for k in b0 if int(k) in changed),
}
json.dump(out, open(os.path.join(RAW, 'diff.json'), 'w'), indent=1)
print(json.dumps({'changes': summary_changes, 'band_n': (len(b0), len(band)), 'entered': entered, 'left': left,
                  'moved_in_band': [(m['id'], m['fields']) for m in moved],
                  'path_v_moved': [(p['id'], p['fields']) for p in pv if p['fields']],
                  'movers_not_in_changed': [i for i in movers if i not in changed],
                  'band_ids_in_changed_set': len(out['band_in_changed_set']),
                  'baseline_band_ids_in_changed_set': len(out['baseline_band_in_changed_set'])}, indent=1))
