# usage: python3 -I m0_score.py — scores m0_monitor.out.json (monitor M0, 572 unit 2's close, 2026-10-24) against the frozen
# lists (../triage_frozen_lists.json, dated 2026-10-19). Writes m0.json; prints the M0 table. Names are keyed as
# build_frozen.py keys them (org_name_norm: lowercase, whitespace collapsed, trailing . , ; stripped).
import json, os, re, sys
D = os.path.dirname(os.path.abspath(__file__))
F = json.load(open(D + '/../triage_frozen_lists.json'))
A = F['list_a_reviewed_names_on_agent_accounts']
body = json.load(open(D + '/m0_monitor.out.json'))
if 'rows' not in body or 'columns' not in body or body.get('truncated'):
    print('NOT AN ANSWER', str(body)[:300]); sys.exit(5)
cols = body['columns']; rows = [dict(zip(cols, r)) for r in body['rows']]
def norm(s):
    c = ' '.join((s or '').lower().split()); t = c.rstrip('.,; ')
    return t or c
EXPECT = {'GBPPONPJXZ4423NDBT': (30953692, 'NHC'), 'GBPPONPXWV6492CGMN': (30913738, 'NWSSP'), 'GBPPONPVMQ1845NYPP': (30913798, 'WELLAND'),
          'GBCOHOC322760': (30913722, 'GIBBS'), 'GBPPONPGGP8622HHRX': (30930308, 'INVICTUS'), 'GBPPONPLMN9829JXLV': (30968739, 'GINGER'),
          'GBPPONPRYY6263NQHG': (30914820, 'RPJ3'), 'GBPPONPBWD3998QHRQ': (30916628, 'NHSLCH')}
PAT = A['agent_name_patterns']
# 1. resolution + head check
res = [r for r in rows if r['k'] == 'org']
org_agent = {}
heads = []
for ident, (want, ag) in EXPECT.items():
    got = [r for r in res if r['q'] == ident]
    ids = [r['org'] for r in got]
    head = got[0]['name'] if got else None
    ok_id = ids == [want]
    ok_head = bool(head) and re.search(PAT[ag], head.lower()) is not None
    note = ''
    if ag == 'RPJ3' and head and 'on behalf of' in head.lower(): note = 'known (524): head names a principal'
    heads.append(dict(identifier=ident, agent=ag, expected=want, resolved=ids, head=head, provisional=[r['n'] for r in got], id_ok=ok_id, head_ok=ok_head, note=note))
    for i in ids: org_agent[i] = ag
gj = [r for r in res if r['q'] == 'GBPPONPGJB3773RCMH']
# 2. sort every mention name
pk = {(p['org'], p['name_norm']): p for p in A['principal_keys']}
npn = {(p['org'], p['name_norm']): p for p in A['non_principal_names']}
SLIPS = {(30953692, 'riverside group limited'): 'Riverside 46798381 (Tenderer slip, transferred to 545, still bound)'}
m = [r for r in rows if r['k'] == 'm']
out = []
for r in m:
    o, nn = r['org'], norm(r['name']); ag = org_agent.get(o)
    if (o, nn) in pk: c = 'P@A'
    elif (o, nn) in SLIPS: c = 'P@A-slip'
    elif (o, nn) in npn: c = 'velindre' if npn[(o, nn)]['kind'].startswith('host trust') else 'non-principal'
    elif ag and re.search(PAT[ag], nn): c = 'own'
    else: c = 'UNREVIEWED'
    out.append(dict(org=o, agent=ag, name=r['name'], n=r['n'], lo=r['lo'], hi=r['hi'], cls=c, frozen_mentions=pk.get((o, nn), {}).get('mentions')))
def tot(f): return sum(x['n'] for x in out if f(x))
by = {}
for x in out: by.setdefault((x['agent'], x['cls']), 0); by[(x['agent'], x['cls'])] += x['n']
pa = tot(lambda x: x['cls'] in ('P@A', 'P@A-slip'))
pa_keys = len({(x['org'], norm(x['name'])) for x in out if x['cls'] in ('P@A', 'P@A-slip')})
nhc = [x for x in out if x['org'] == 30953692 and x['cls'] in ('P@A', 'P@A-slip')]
nhc_bodies = len({norm(x['name']) for x in nhc})
unrev = [x for x in out if x['cls'] == 'UNREVIEWED']
vel = tot(lambda x: x['cls'] == 'velindre')
# key-level growth against the frozen counts (list (a) keys only)
grown = [dict(org=x['org'], name=x['name'], now=x['n'], frozen=x['frozen_mentions']) for x in out if x['cls'] == 'P@A' and x['n'] != x['frozen_mentions']]
missing = [dict(org=k[0], name_norm=k[1], frozen=p['mentions']) for k, p in pk.items() if not any(x['org'] == k[0] and norm(x['name']) == k[1] for x in out)]
summary = dict(
    read='2026-10-24 u2/m0_monitor.sql (1 /v1/sql, rc=0, 159 rows, not truncated)',
    heads=heads, gibbs_ppon_arm=gj,
    total_mentions=tot(lambda x: True), per_org_total={str(o): tot(lambda x, o=o: x['org'] == o) for o in sorted(org_agent)},
    pa_on_agent_accounts=pa, pa_keys=pa_keys, pa_list_a_only=tot(lambda x: x['cls'] == 'P@A'),
    nhc_member_mentions=sum(x['n'] for x in nhc), nhc_member_bodies=nhc_bodies,
    velindre_on_nwssp=vel, unreviewed=unrev, unreviewed_mentions=sum(x['n'] for x in unrev),
    own=tot(lambda x: x['cls'] == 'own'), non_principal=tot(lambda x: x['cls'] == 'non-principal'),
    by_agent_class={'%s/%s' % k: v for k, v in sorted(by.items())},
    list_a_keys_changed=grown, list_a_keys_missing=missing, rows=out)
json.dump(summary, open(D + '/m0.json', 'w'), indent=1, ensure_ascii=False)
for h in heads: print('HEAD', h)
print('GJB arm', gj)
for k in ('total_mentions', 'per_org_total', 'pa_on_agent_accounts', 'pa_keys', 'pa_list_a_only', 'nhc_member_mentions', 'nhc_member_bodies', 'velindre_on_nwssp', 'unreviewed_mentions', 'own', 'non_principal', 'by_agent_class', 'list_a_keys_changed', 'list_a_keys_missing'):
    print(k, summary[k])
for u in unrev: print('UNREVIEWED', u)
