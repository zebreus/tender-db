import json, os, collections, sys
D = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/fresh570/changes_lot'
A, B = 957854566, 958764334
files = sorted(f for f in os.listdir(D) if f.startswith('p') and f.endswith('.json'))
pages = [json.load(open(os.path.join(D, f))) for f in files]
print('pages', len(pages), 'first', files[0], 'last', files[-1])
print('gens', collections.Counter(p.get('generation') for p in pages), 'reset', [p.get('reset') for p in pages if p.get('reset')], 'ignored', collections.Counter(str(p.get('ignored_filters')) for p in pages))
prev = A
gaps = 0
for i, p in enumerate(pages):
    ev = p['events']
    if ev and int(ev[0]['cursor']) <= prev:
        print('overlap at', i)
    if ev:
        prev = int(p['last_cursor'])
print('last page last_cursor', pages[-1]['last_cursor'], 'more', pages[-1].get('more'))
ev = [e for p in pages for e in p['events']]
inr = [e for e in ev if A < int(e['cursor']) <= B]
print('events', len(ev), 'in range', len(inr), 'outside', len(ev) - len(inr))
print(collections.Counter((e['entity'], e['op'], 'null' if e['version'] is None else 'v') for e in inr))
print('distinct lot ids', len({e['id'] for e in inr}))
print('removed', [e for e in inr if e['op'] == 'removed'][:5])
print('max cursor in range', max(int(e['cursor']) for e in inr), 'min', min(int(e['cursor']) for e in inr))
# cursor monotonic within pages
bad = sum(1 for p in pages for a, b in zip(p['events'], p['events'][1:]) if int(b['cursor']) <= int(a['cursor']))
print('non-monotonic', bad)
print('keys', sorted(ev[0].keys()))
