import json, gzip, sys, csv
d = json.load(gzip.open(sys.argv[1])); b = d['body'] if 'body' in d else d
if isinstance(b, str): b = json.loads(b)
skip = set()
with open(sys.argv[2]) as f:
    for row in csv.DictReader(f, delimiter='\t'): skip.add(int(row['org_id']))
SALT = 2780005964
key = lambda o: ((((o ^ SALT) * 2654435761) % 2**32), o)
CUT = '2025-10-16'   # as_of 2026-10-16 less 365 days; a stored day reads as its first instant
assert b['as_of_date'] == '2026-10-16' and b['epoch'] == '524u2-v11'
planned = {e['org_id']: e for e in b['planned']}
held = {e['org_id']: e for e in b['held']}
def stale(e): return e.get('top_newest') is None or e['top_newest'] <= CUT
def rival(e): return e['clusters'] > 1 + (1 if e['head'] > 0 else 0)
def droppable(e): return e['rule'] == 'R1' and not e.get('recent_released') and stale(e)   # exclude_stale_top
def applies(e): return e['rule'] != 'R2' and e.get('recent_released') != 'unfit'          # every wet run
out = {}
# (a)
A = sorted([e for e in planned.values() if e['rule'] == 'R1' and not e.get('recent_released') and stale(e) and not rival(e)], key=lambda e: key(e['org_id']))
pool_a = len(A)
out['a'] = [e['org_id'] for e in A if e['org_id'] not in skip][:30]
# (b)
samp = [s['org_id'] if isinstance(s, dict) else s for s in b['samples'].get('change:R1', [])]
rest = sorted([o for o, e in planned.items() if e['rule'] == 'R1'], key=key)
order_b = list(dict.fromkeys(samp + rest))
def draw(order, n=30):
    take, take_nd = [], []
    for o in order:
        e = planned[o]
        if o in skip or not applies(e): continue
        if len(take) < n: take.append(o)
        if not droppable(e) and len(take_nd) < n: take_nd.append(o)
        if len(take) >= n and len(take_nd) >= n: break
    return take, take_nd
out['b'], out['b_nd'] = draw(order_b)
# (c)
scope = sorted(o for o, e in planned.items() if applies(e))
cut_ids = scope[:2000]
out['c_cut'] = cut_ids[-1]
out['c'], out['c_nd'] = draw(sorted(cut_ids, key=key))
# (d)
def hold_draw(reason, n, pred=lambda e: True):
    L = sorted([e for e in held.values() if e['reason'] == reason and pred(e)], key=lambda e: key(e['org_id']))
    return [e['org_id'] for e in L if e['org_id'] not in skip][:n]
def half_differs(e):
    import re
    n = (e.get('new_name') or '')
    w = re.findall(r'\w+', n.casefold()); h = len(w) // 2
    half = ' '.join(w[:h]); head = ' '.join(re.findall(r'\w+', (e.get('old_name') or '').casefold()))
    return half != head
out['d_top_stale'] = hold_draw('top_stale', 20)
out['d_seat_dropped'] = hold_draw('seat_dropped', 5)
out['d_doubled'] = hold_draw('doubled', 5, half_differs)
allids = list(dict.fromkeys(out['a'] + out['b'] + out['b_nd'] + out['c'] + out['c_nd'] + out['d_top_stale'] + out['d_seat_dropped'] + out['d_doubled']))
print('pool (a)', pool_a, '| sizes', {k: len(v) if isinstance(v, list) else v for k, v in out.items()}, '| distinct items', len(allids))
print('(b) droppable among first 30:', sum(droppable(planned[o]) for o in out['b']), '| (c) droppable among first 30:', sum(droppable(planned[o]) for o in out['c']))
items = []
for o in allids:
    e = planned.get(o) or held.get(o)
    gates = [g for g in ('a', 'b', 'b_nd', 'c', 'c_nd', 'd_top_stale', 'd_seat_dropped', 'd_doubled') if o in out[g]]
    keep = {k: e.get(k) for k in ('org_id', 'identifier', 'country', 'old_name', 'new_name', 'verdict', 'rule', 'reason', 'recent_released',
            'n', 'voting', 'top', 'head', 'second', 'clusters', 'spellings', 'head_procs', 'head_in_window', 'head_newest', 'top_newest',
            'rival_newest', 'newest_is_head')}
    keep['gates'] = gates; keep['stale_top'] = stale(e) if e['verdict'] == 'change' else None
    items.append(keep)
json.dump({'draw': out, 'items': items}, open('v11_gate_items.json', 'w'), ensure_ascii=False, indent=1)
