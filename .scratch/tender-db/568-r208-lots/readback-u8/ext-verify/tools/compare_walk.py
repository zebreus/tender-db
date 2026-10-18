"""Compare the verifier's fresh walk (walk_result.json) with the extender's draw_ext.json, row by row."""
import collections, json, sys
mine = json.load(open(sys.argv[1]))
ext = json.load(open('/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8/ext/draw_ext.json'))
er = {w['index']: w for w in ext['walked']}
mr = {r['index']: r for r in mine['rows']}
diff = []
for i in sorted(set(er) | set(mr)):
    a, b = mr.get(i), er.get(i)
    if a is None or b is None:
        diff.append((i, 'missing', a and a['id'], b and b['id'])); continue
    if a['id'] != b['id']:
        diff.append((i, 'id', a['id'], b['id'])); continue
    ma = {'held': 'held', 'skip': 'skip', 'skip-unread-pop': 'unread', 'skip-404': '404'}[a['act']]
    eb = 'held' if b.get('held') else ('unread' if 'unread' in b['why'] else ('404' if '404' in b['why'] else 'skip'))
    if ma != eb or (a.get('g2') is not None and bool(a.get('g2')) != bool(b.get('g2'))) or (a.get('out') != b.get('out') and 'out' in a):
        diff.append((i, 'act/g2', a['id'], ma, eb, a.get('g2'), b.get('g2')))
print('rows mine', len(mr), 'ext', len(er), 'diffs', len(diff))
for d in diff[:30]: print(d)
c = collections.Counter(r['act'] for r in mine['rows'])
print('mine acts', c)
prof = collections.Counter()
broad_only = []
for r in mine['rows']:
    ev = r.get('ev') or {}
    for n, (p, dv) in (ev.get('placed_profiles') or {}).items():
        prof[(p, dv)] += 1
    if ev.get('any_208_in_listing') and not r.get('g2'):
        broad_only.append((r['index'], r['id'], ev.get('any_208_in_listing')[:5], ev.get('r208_placed'), ev.get('r208_lot_sections')))
print('placed-notice profiles', prof.most_common())
print('read, not G2, but listing holds an R2.0.8 notice:', len(broad_only))
for x in broad_only[:40]: print('  ', x)
unl = [(r['index'], r['id']) for r in mine['rows'] if any(v[0] == 'UNLISTED' for v in ((r.get('ev') or {}).get('placed_profiles') or {}).values())]
print('placed notices missing from the listing:', unl)
print('multi-page listings:', [(r['index'], r['id'], r['ev'].get('listing_pages')) for r in mine['rows'] if (r.get('ev') or {}).get('listing_pages', 1) > 1][:20])
