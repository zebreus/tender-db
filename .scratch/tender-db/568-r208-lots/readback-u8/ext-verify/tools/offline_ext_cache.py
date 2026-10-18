"""Apply the verifier's own corrected-G2 test, offline, to the extender's draw_cache for every row it read."""
import collections, json, os, sys
C = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/ext/draw_cache'
ext = json.load(open('/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8/ext/draw_ext.json'))
ld = lambda p: json.load(open(p, encoding='utf-8'))
res = collections.Counter(); bad = []; missing = []
for w in ext['walked']:
    if 'year' not in w:
        continue
    tid = w['id']
    t = ld(os.path.join(C, f'tenders_{tid}.json'))
    placed = collections.Counter(r['notice_id'] for r in t.get('lot_results', []) if r.get('lot') is not None)
    g2 = False
    if placed and not w['in_population']:
        prof, page = {}, 0
        while True:
            p = os.path.join(C, f'notices_tender_{tid}.json' if page == 0 else f'notices_tender_{tid}_p{page}.json')
            if not os.path.exists(p):
                missing.append((tid, 'listing', page)); prof = None; break
            d = ld(p)
            prof.update({int(x['id']): (x.get('profile'), x.get('declared_version')) for x in d['items']})
            if not d.get('more'): break
            page += 1
        if prof is not None:
            for n in sorted(placed):
                pr, dv = prof.get(n, (None, None))
                if pr == 'ted-export-r208' or '2.0.8' in str(dv):
                    cp = os.path.join(C, f'notices_{n}_content.json')
                    if not os.path.exists(cp):
                        missing.append((tid, 'content', n)); continue
                    if not any(s.get('kind') in ('Lot', 'LotsGroup', 'Part') for s in ld(cp).get('sections', [])):
                        g2 = True; break
    res[(g2, bool(w['g2']))] += 1
    if g2 != bool(w['g2']):
        bad.append((w['index'], tid, g2, w['g2']))
print('(mine, ext) counts', dict(res)); print('disagreements', bad); print('missing files', missing[:20], len(missing))
