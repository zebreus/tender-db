"""Issue 575 triage: scan every locally cached notice content body (the /v1/notices/{id}/content
shape: {notice_id, sections[{kind, section_id, parent_section_id, values[...]}]}) under
.scratch/tender-db for ORG sections, classify each BT-501 raw value's shape, and tabulate the
8-4-4-16 (35-char) population by name, role (the copy's own OPT-300/OPT-301 refs) and recurrence.

Read-only; run with python3 -I <this> <root> [out.json]."""
import json, os, re, sys
from collections import Counter, defaultdict

ROOT = sys.argv[1]
OUT = sys.argv[2] if len(sys.argv) > 2 else None
CF = re.compile(r'^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{16}$')
RFC = re.compile(r'^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}$')
PLAIN = re.compile(r'^[0-9A-Fa-f]{32}$')


def shape(v):
    v = v.strip()
    if CF.match(v):
        return 'cf35'
    if RFC.match(v):
        return 'rfc36'
    if PLAIN.match(v):
        return 'hex32'
    return 'other'


def v4(v):
    h = v.replace('-', '')
    if len(h) != 32 or not all(c in '0123456789abcdefABCDEF' for c in h):
        return False
    return h[12] == '4' and h[16].lower() in '89ab'


notices = {}
for dp, dn, fn in os.walk(ROOT):
    if '575-triage' in dp:
        continue
    for f in fn:
        if not f.endswith('.json'):
            continue
        p = os.path.join(dp, f)
        try:
            with open(p) as fh:
                d = json.load(fh)
        except Exception:
            continue
        if not isinstance(d, dict) or 'sections' not in d or 'notice_id' not in d:
            continue
        notices.setdefault(d['notice_id'], (p, d))

rows = []
for nid, (p, d) in notices.items():
    secs = d['sections']
    org = {}
    parent = {}
    for s in secs:
        parent[s['section_id']] = s.get('parent_section_id')
    def owner(sid):
        seen = 0
        while sid is not None and seen < 20:
            if sid.startswith('ORG-'):
                return sid
            sid = parent.get(sid)
            seen += 1
        return None
    roles = defaultdict(set)
    for s in secs:
        for v in s.get('values', []):
            fid = v.get('field_id', '')
            if fid.startswith('OPT-300') or fid.startswith('OPT-301') or fid.startswith('OPT-030'):
                if v.get('type') == 'id' and isinstance(v.get('value'), str):
                    roles[v['value']].add(fid.replace('OPT-300-', '').replace('OPT-301-', ''))
        o = owner(s['section_id'])
        if o is None:
            continue
        rec = org.setdefault(o, {'name': None, 'bt501': None, 'scheme': None, 'country': None, 'city': None})
        for v in s.get('values', []):
            fid = v.get('field_id', '')
            if fid == 'BT-500-Organization-Company' and rec['name'] is None:
                rec['name'] = v.get('value')
            elif fid == 'BT-501-Organization-Company' and rec['bt501'] is None:
                rec['bt501'] = v.get('value'); rec['scheme'] = v.get('scheme')
            elif fid == 'BT-514-Organization-Company' and rec['country'] is None:
                rec['country'] = v.get('code') or v.get('value')
            elif fid == 'BT-513-Organization-Company' and rec['city'] is None:
                rec['city'] = v.get('value')
    for sid, rec in org.items():
        if rec['bt501'] is None:
            continue
        rows.append({'notice_id': nid, 'section': sid, 'name': rec['name'], 'country': rec['country'],
                     'city': rec['city'], 'raw': rec['bt501'], 'scheme': rec['scheme'],
                     'shape': shape(rec['bt501']), 'v4': v4(rec['bt501']),
                     'roles': sorted(roles.get(sid, []))})

by_shape = Counter(r['shape'] for r in rows)
cf = [r for r in rows if r['shape'] == 'cf35']
cf_notices = sorted({r['notice_id'] for r in cf})
names = Counter(r['name'] for r in cf)
countries = Counter(r['country'] for r in cf)
role_sets = Counter('+'.join(r['roles']) or '(none)' for r in cf)
buyer = [r for r in cf if any('Buyer' in x for x in r['roles'])]
values = Counter(r['raw'].upper() for r in cf)
recur = {k: c for k, c in values.items() if c > 1}
v4n = sum(r['v4'] for r in cf)
# within-notice twins: same notice, same folded name, >= 2 distinct cf35 values
tw = defaultdict(set)
for r in cf:
    tw[(r['notice_id'], (r['name'] or '').casefold())].add(r['raw'].upper())
twins = {f'{k[0]}|{k[1]}': len(v) for k, v in tw.items() if len(v) >= 2}
# per notice: is every ORG section of a cf35 notice cf35, or do buyers carry other shapes?
mix = Counter()
for n in cf_notices:
    rs = [r for r in rows if r['notice_id'] == n]
    mix[tuple(sorted(Counter(r['shape'] for r in rs).items()))] += 1
buyer_shapes_in_cf_notices = Counter()
for n in cf_notices:
    for r in rows:
        if r['notice_id'] == n and any('Buyer' in x for x in r['roles']):
            buyer_shapes_in_cf_notices[r['shape'] + ':' + (r['scheme'] or '')] += 1
# sequential prefix within a notice
seq = {}
for n in cf_notices:
    vs = sorted(r['raw'].upper() for r in cf if r['notice_id'] == n)
    seq[n] = len({v[:4] for v in vs}) == 1 if len(vs) > 1 else None

rep = {
    'notices_scanned': len(notices), 'org_rows_with_bt501': len(rows), 'by_shape': dict(by_shape),
    'cf35_rows': len(cf), 'cf35_notices': len(cf_notices), 'cf35_v4_passes': v4n,
    'cf35_countries': dict(countries), 'cf35_names': names.most_common(), 'cf35_role_sets': role_sets.most_common(),
    'cf35_buyer_rows': [{k: r[k] for k in ('notice_id', 'section', 'name', 'raw', 'roles')} for r in buyer],
    'cf35_values_recurring': recur, 'cf35_within_notice_twins': twins,
    'cf35_notice_shape_mix': {str(k): v for k, v in mix.items()},
    'buyer_shapes_in_cf35_notices': dict(buyer_shapes_in_cf_notices),
    'cf35_same_4hex_prefix_per_notice': seq,
    'cf35_rows_detail': cf,
}
print(json.dumps({k: v for k, v in rep.items() if k != 'cf35_rows_detail'}, ensure_ascii=False, indent=1))
if OUT:
    with open(OUT, 'w') as fh:
        json.dump(rep, fh, ensure_ascii=False, indent=1)
