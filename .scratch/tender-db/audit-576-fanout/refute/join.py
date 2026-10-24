import json, collections, sys
cl = json.load(open('refute/clusters.json'))
par = collections.defaultdict(list)
for f in ['refute/raw/01-r1_clusters_a.json', 'refute/raw/02-r2_clusters_b.json']:
    d = json.load(open(f))
    for r in d['rows']:
        r = dict(zip(d['columns'], r)); par[r['org']].append(r)
for c in cl:
    print(f"\n### {c['n']} {c['country']} n2keys={c['n2_distinct']} :: {c['key'][:90]}")
    for oid, tpo, name in c['members']:
        ps = par.get(oid, [])
        print(f"  {oid} ({tpo}) {name[:80]!r}")
        for p in sorted(ps, key=lambda p: -p['n']):
            print(f"      parent {p['parent_org']} id={p['parent_identified']} n={p['n']} city={p['city_min']!r}{'..'+repr(p['city_max']) if p['city_max']!=p['city_min'] else ''} pname={str(p['parent_name_min'])[:60]!r}{' ~ '+repr(str(p['parent_name_max'])[:40]) if p['parent_name_max']!=p['parent_name_min'] else ''}")
