"""Verifier's independent re-derivation of the O1 extension walk (readback-u8/readback.md O1).
Own code: salted order, exclusions, population skip, holding rule, corrected G2 test; reads fresh into walkcache/.
Usage: python3 -I walk.py OUT.json [--offline]"""
import collections, hashlib, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import vfetch as V
VD = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
R = '/home/user/tender-db/.scratch/tender-db/568-r208-lots'
CACHE = os.path.join(VD, 'walkcache')
OFFLINE = '--offline' in sys.argv
out_path = sys.argv[1]

def salt(i):
    return hashlib.sha256(('568u8-fresh30-' + str(i)).encode()).hexdigest()

changed = V.ld(os.path.join(VD, 'changed_ids.json'))
ex = {}
for g in V.ld(f'{R}/readback/u7_sample.json')['groups']:
    for i in g['ids']: ex.setdefault(int(i), 's150')
for x in V.ld(f'{R}/readback/fresh30.json')['draw']: ex.setdefault(int(x['id']), 'f30')
for i in V.ld(f'{R}/readback/measured77.json')['served']: ex.setdefault(int(i), 'm77')
for t in V.ld(f'{R}/measurement.json')['tenders']: ex.setdefault(int(t['tender_id']), 'm77')
ex.setdefault(8327340, '8327340')
for g in V.ld(f'{R}/u8-oos/draw.json')['groups']:
    for i in g['ids']: ex.setdefault(int(i), 'oos50')
pop = set(V.ld(f'{R}/readback/population.json')['changed_null_version_ids'])
excluded_hit = collections.Counter(ex[i] for i in changed if i in ex)
order = sorted((i for i in changed if i not in ex), key=salt)

prev = V.ld(f'{R}/readback-u8/fresh568/draw.json')
pw = [w['id'] for w in prev['walked']]
assert len(pw) == 771, len(pw)
same_prefix = pw == order[:771]
keep_idx = list(range(22)) + [61, 282, 366, 449, 770]
held_prev = [i for i, w in enumerate(prev['walked']) if w.get('held')]
keep = [prev['walked'][i] for i in keep_idx]
assert all(w['held'] for w in keep)
n_out = sum(1 for w in keep if w['out'])
n_g2 = 5
held = [w['id'] for w in keep]

def listing(tid):
    prof, page, cur = {}, 0, None
    while True:
        p = os.path.join(CACHE, f'listing_{tid}_p{page}.json')
        q = f'/v1/notices?tender={tid}' + ('' if cur is None else f'&cursor={cur}')
        if OFFLINE and not os.path.exists(p): return None, 'offline'
        ok, code = V.get(q, p)
        if not ok: return None, code
        d = V.ld(p)
        for x in d['items']: prof[int(x['id'])] = (x.get('profile'), x.get('declared_version'))
        if not d.get('more'): return prof, page + 1
        cur = d.get('next_cursor'); page += 1
        if cur is None: return None, 'more without cursor'

def content(nid):
    p = os.path.join(CACHE, f'content_{nid}.json')
    if OFFLINE and not os.path.exists(p): return None
    ok, code = V.get(f'/v1/notices/{nid}/content', p)
    return V.ld(p) if ok else None

def g2_test(t):
    placed = collections.Counter(r['notice_id'] for r in t.get('lot_results', []) if r.get('lot') is not None)
    ev = {'placed_results': sum(placed.values()), 'placed_notices': sorted(placed)}
    if not placed:
        return False, ev
    prof, pages = listing(t['id'])
    if prof is None:
        return None, dict(ev, err=f'listing {pages}')
    ev['listing_pages'] = pages
    ev['placed_profiles'] = {str(n): prof.get(n, ('UNLISTED', None)) for n in sorted(placed)}
    ev['any_208_in_listing'] = sorted(n for n, (p_, dv) in prof.items() if '208' in str(p_) or '2.0.8' in str(dv))
    cands = sorted(n for n in placed if prof.get(n, (None,))[0] == 'ted-export-r208'
                   or '2.0.8' in str(prof.get(n, (None, None))[1]))
    ev['r208_placed'] = cands
    for n in cands:
        c = content(n)
        if c is None:
            return None, dict(ev, err=f'content {n}')
        kinds = collections.Counter(s.get('kind') for s in c.get('sections', []))
        lots = sum(v for k, v in kinds.items() if k in ('Lot', 'LotsGroup', 'Part'))
        ev.setdefault('r208_lot_sections', {})[str(n)] = lots
        if lots == 0:
            ev['qualifying'] = n
            ev['qualifying_placed'] = placed[n]
            return True, ev
    return False, ev

rows, stop = [], None
for idx in range(771, len(order)):
    if len(held) == 30: break
    tid = order[idx]
    need_out, need_g2 = max(0, 10 - n_out), max(0, 8 - n_g2)
    free_after = 30 - len(held) - 1
    inside = tid in pop
    row = {'index': idx, 'id': tid, 'pop': inside}
    if inside and free_after < need_g2 + max(0, need_out - 1):
        row['act'] = 'skip-unread-pop'; rows.append(row); continue
    p = os.path.join(CACHE, f'tenders_{tid}.json')
    if OFFLINE and not os.path.exists(p):
        stop = f'offline at {idx}'; break
    ok, code = V.get(f'/v1/tenders/{tid}', p)
    if not ok:
        if code == '404':
            row['act'] = 'skip-404'; rows.append(row); continue
        stop = f'tender {tid} HTTP {code}'; break
    t = V.ld(p)
    pub = str(t.get('published_at') or '')
    yr = int(pub[:4]) if pub[:4].isdigit() else None
    out_era = yr is not None and not (2012 <= yr <= 2015)
    if inside:
        g2, ev = False, {'why': 'pop'}
    else:
        g2, ev = g2_test(t)
        if g2 is None:
            stop = f'g2 test of {tid}: {ev}'; break
    row.update(year=yr, out=out_era, g2=g2, ev=ev)
    a_out, a_g2 = n_out + out_era, n_g2 + g2
    if free_after >= max(0, 10 - a_out) + max(0, 8 - a_g2):
        held.append(tid); n_out, n_g2 = a_out, a_g2; row['act'] = 'held'
    else:
        row['act'] = 'skip'
    rows.append(row)
    if len(rows) % 50 == 0:
        print(idx, len(held), V.gets, flush=True)
res = {'changed': len(changed), 'excluded_hit': dict(excluded_hit), 'candidates': len(order),
       'prefix_771_equals_fresh568_walk': same_prefix, 'fresh568_held_indices': held_prev,
       'seed': {'held': 27, 'out': n_out if False else sum(1 for w in keep if w['out']), 'g2': 5},
       'held': held, 'n_out': n_out, 'n_g2': n_g2, 'stop': stop, 'gets': V.gets,
       'walked_to': rows[-1]['index'] if rows else None, 'rows': rows}
json.dump(res, open(out_path, 'w'), indent=1)
print(json.dumps({k: v for k, v in res.items() if k != 'rows'}))
