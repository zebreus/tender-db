# usage: python3 -I r2_score.py — R2 (decision.md "Read bars"): each tender must be in the listed ?buyer= lists and
# absent from the excluded ones; lists read in full at limit=1000 (every page saved under u2/bodies/). Writes r2.json.
import json, os, glob
D = os.path.dirname(os.path.abspath(__file__))
ROWS = [  # tender, must include [(org, label)], must exclude [(org, label)]
    (8641682, [(30953692, 'NHC')], [(30914707, 'Birmingham')]),
    (8636304, [(30953692, 'NHC')], [(30914393, 'Darlington')]),
    (8677040, [(30913798, 'Welland')], [(30915335, 'Rutland')]),
    (8698064, [(30913738, 'NWSSP')], [(30917293, 'HEIW')]),
    (8596944, [(30913722, 'Gibbs Laidler')], [(2291108, 'English Rural HA (provisional)')]),
    (8655231, [(30917275, 'Trafford (host)'), (30916230, 'Rochdale')], []),
]
def lst(org):
    files = sorted(glob.glob(D + '/bodies/v1_tenders_buyer_%d_limit_1000*.json' % org))
    ids = []; pages = []
    for f in files:
        b = json.load(open(f)); ids += [i['id'] for i in b['items']]
        pages.append(dict(file=os.path.basename(f), items=len(b['items']), more=b['more'], next_cursor=b['next_cursor'], ignored_filters=b['ignored_filters']))
    complete = bool(pages) and pages[-1]['more'] is False and all(p['ignored_filters'] == [] for p in pages)
    return set(ids), dict(org=org, pages=pages, tenders=len(set(ids)), complete=complete, dupes=len(ids) - len(set(ids)))
lists = {}
out = dict(rows=[], lists={})
ok = 0
for t, inc, exc in ROWS:
    checks = []
    for o, lab in inc:
        s, meta = lists.setdefault(o, lst(o)); checks.append(dict(org=o, label=lab, must='include', present=t in s, list_complete=meta['complete'], pass_=t in s and meta['complete']))
    for o, lab in exc:
        s, meta = lists.setdefault(o, lst(o)); checks.append(dict(org=o, label=lab, must='exclude', present=t in s, list_complete=meta['complete'], pass_=(t not in s) and meta['complete']))
    passed = all(c['pass_'] for c in checks)
    ok += passed
    out['rows'].append(dict(tender=t, checks=checks, score='PASS' if passed else 'W'))
out['lists'] = {str(o): m for o, (s, m) in lists.items()}
out['tally'] = dict(ok=ok, of=len(ROWS), W=len(ROWS) - ok)
json.dump(out, open(D + '/r2.json', 'w'), indent=1)
print(out['tally'])
for r in out['rows']: print(r['tender'], r['score'], [(c['label'], c['must'], 'present' if c['present'] else 'absent', 'complete' if c['list_complete'] else 'INCOMPLETE') for c in r['checks']])
for o, m in out['lists'].items(): print('list', o, m['tenders'], 'tenders', len(m['pages']), 'page(s)', 'complete' if m['complete'] else 'INCOMPLETE', 'dupes', m['dupes'])
