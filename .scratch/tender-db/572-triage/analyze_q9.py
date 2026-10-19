# usage: python3 -I analyze_q9.py — joins q9a (each candidate identified org and its own current ?buyer= count)
# and q9b (routed / routed_absent per principal) into the per-principal share, critic amendment B2.
# share = routed_absent / own(dominant identified row): the routed current tenders missing from the principal's
# own `?buyer=` list, as a fraction of that list (reopen trigger 2(d)'s definition). The dominant row is the
# identified org with the largest own list; split principals (HEIW, ABUHB, Hywel Dda, C&V, Karbon, ...) are
# flagged. Writes q9_share.summary.json and prints the table.
import json, os
D = os.path.dirname(os.path.abspath(__file__))
spec = json.load(open(D + '/q9_principal_spec.json'))
a = json.load(open(D + '/q9a_principal_orgs.out.json')); A = [dict(zip(a['columns'], r)) for r in a['rows']]
b = json.load(open(D + '/q9b_routed.out.json')); B = {r[0]: dict(zip(b['columns'], r)) for r in b['rows']}
found = json.load(open(D + '/q9b_found_orgs.json'))
KIND = {30953692: 'housing consortium (NHC)', 30913738: 'NHS Wales (NWSSP)'}
def kind(label, agent):
    if agent == 30913738: return 'health'
    if agent == 30916628: return 'health'
    if agent in (30930308, 30968739, 30914820): return 'school/trust'
    if agent == 30913722: return 'housing'
    l = label.lower()
    if any(w in l for w in ('ha', 'housing', 'homes', 'group', 'bernicia', 'manchester', 'l&q', 'guinness', 'karbon', 'leger', 'pioneer', 'ch')) and not any(w in l for w in ('council', ' cc', ' bc', ' dc', 'mbc')) and label not in ('Gateshead','Newham','Bury','Rochdale','RBKC','Kirklees','Thurrock','Wigan','North Tyneside','South Tyneside','North Northamptonshire','East Riding','East Suffolk','City of Lincoln','City of York','Stockton-on-Tees','Stoke-on-Trent'):
        return 'housing'
    return 'council'
out = []
for s in spec:
    lab = s['label']; ids = found[lab]
    rows = {r['id']: r for r in A if r['k'] in ('norm', 'id') and r['id'] in ids}
    dom = max(rows.values(), key=lambda r: r['own']) if rows else None
    rb = B[lab]
    rec = dict(principal=lab, kind=kind(lab, s['agent']), agent=s['agent'], routed=rb['routed'], routed_absent=rb['routed_absent'],
               identified_rows=[(r['id'], r['identifier'], r['own']) for r in sorted(rows.values(), key=lambda r: -r['own'])],
               dominant=(dom['id'], dom['identifier'], dom['own']) if dom else None,
               split=len([r for r in rows.values() if r['own'] > 0]) > 1)
    if dom and dom['own'] > 0:
        rec['share'] = round(rb['routed_absent'] / dom['own'], 4)
        rec['share_of_total'] = round(rb['routed_absent'] / (dom['own'] + rb['routed_absent']), 4)
    else:
        rec['share'] = None
    out.append(rec)
json.dump(out, open(D + '/q9_share.summary.json', 'w'), indent=1, ensure_ascii=False)
out.sort(key=lambda r: -(r['share'] if r['share'] is not None else 9))
print('%-28s %-12s %6s %6s %8s %7s %s' % ('principal', 'kind', 'routed', 'absent', 'own(dom)', 'share', 'dominant / split'))
for r in out:
    d = r['dominant']
    print('%-28s %-12s %6d %6d %8s %7s %s' % (r['principal'], r['kind'], r['routed'], r['routed_absent'], d[2] if d else '-',
          ('%.1f%%' % (100 * r['share'])) if r['share'] is not None else 'n/a', (str(d[0]) + ' ' + str(d[1]) + (' SPLIT' if r['split'] else '')) if d else 'no identified row'))
m = [r for r in out if r['share'] is not None and r['routed_absent'] > 0]
print('measured principals with routed_absent>0:', len(m), '; >10%:', sum(r['share'] > 0.10 for r in m),
      '; total routed', sum(r['routed'] for r in out), 'absent', sum(r['routed_absent'] for r in out))
from collections import defaultdict
k = defaultdict(list)
for r in m: k[r['kind']].append(r['share'])
for kk, v in k.items():
    v.sort(); print(kk, 'n=%d' % len(v), 'median %.1f%%' % (100 * v[len(v)//2]), 'min %.1f%% max %.1f%%' % (100*v[0], 100*v[-1]), '>10%%: %d' % sum(x > 0.10 for x in v))
