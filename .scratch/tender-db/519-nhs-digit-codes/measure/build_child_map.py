#!/usr/bin/env python3
"""519 M2: the dated child map from cached ORD records (offline; run fetch_ro98.py first).

    python3 -I build_child_map.py

RO98 children: every RO98 row of crates/ingest/sdk/nhs-ods.tsv (cache measure/ods-ro98/), keeping
Rels.Rel with id RE5 whose target's primary role is RO261, Operational dates.
RO198 children: the measured site codes (gbnhs-orgs-classified.json primary RO198; cache ../ods/),
keeping RE6 -> RO197.
Adjacent intervals (end + 1 day == next start) with the same parent are merged. Writes
measure/child-map.json and prints the M2 report. nhs-ods.tsv is read, never written.
Exit 1 on: a record status other than 200/404, a parent that is not a row of nhs-ods.tsv, a site
whose RE6 target is not RO197, overlapping intervals for one child (the report is still written).
"""
import collections, datetime, hashlib, json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
U1 = os.path.dirname(HERE)
REPO = os.path.abspath(os.path.join(U1, '..', '..', '..'))
TSV = os.path.join(REPO, 'crates', 'ingest', 'sdk', 'nhs-ods.tsv')


def d(s):
    return datetime.date.fromisoformat(s) if s else None


def iso(x):
    return x.isoformat() if x else None


def dates(rel, typ):
    for dd in rel.get('Date', []):
        if dd.get('Type') == typ:
            return dd.get('Start'), dd.get('End')
    return None, None


def main():
    tsv_bytes = open(TSV, 'rb').read()
    rows = {}
    for l in tsv_bytes.decode('utf-8').split('\n'):
        if l.startswith('#') or not l.strip():
            continue
        f = l.split('\t')
        rows[f[0]] = dict(status=f[1], rc=f[2], role=f[3], allow=f[4] == '1', legal_end=f[7], name=f[8])
    errors, notes = [], []
    cls = json.load(open(os.path.join(U1, 'gbnhs-orgs-classified.json')))
    holders = collections.defaultdict(list)
    for o in cls:
        holders[o['code']].append({'org': o['id'], 'mentions': o['mentions'], 'name': o['name']})
    children = []   # (code, role, record path)
    for c in sorted(c for c, r in rows.items() if r['role'] == 'RO98'):
        children.append((c, 'RO98', os.path.join(HERE, 'ods-ro98', c + '.json')))
    sites = sorted(o['code'] for o in cls if o['ods'].get('primary') == 'RO198')
    for c in sites:
        children.append((c, 'RO198', os.path.join(U1, 'ods', c + '.json')))

    out, no_parent, rec_mismatch, dropped = [], [], [], collections.Counter()
    dropped_by_child = {}
    http = collections.Counter()
    for code, role, path in children:
        w = json.load(open(path, encoding='utf-8'))
        http[w.get('http')] += 1
        if w.get('http') == '404':
            no_parent.append({'code': code, 'role': role, 'why': 'ORD 404'})
            out.append(dict(code=code, child_role=role, rel='-', parent=None, **{'from': None, 'to': None}))
            continue
        if w.get('http') != '200':
            errors.append(f'{code}: fetch status {w.get("http")}')
            continue
        o = w['body']['Organisation']
        prim = next((r['id'] for r in o.get('Roles', {}).get('Role', []) if r.get('primaryRole')), None)
        if prim != role:
            errors.append(f'{code}: record primary role {prim}, expected {role}')
        if role == 'RO98':
            t = rows[code]
            got = (o['Name'].strip(), o['Status'][0], o.get('orgRecordClass'))
            if got != (t['name'], t['status'], t['rc']):
                rec_mismatch.append({'code': code, 'record': got, 'tsv': (t['name'], t['status'], t['rc'])})
        want_rel, want_role = ('RE5', 'RO261') if role == 'RO98' else ('RE6', 'RO197')
        ivs = collections.defaultdict(list)
        other = collections.Counter()
        for r in o.get('Rels', {}).get('Rel', []):
            tr = r['Target']['PrimaryRoleId']['id']
            tc = r['Target']['OrgId']['extension']
            if r['id'] != want_rel or tr != want_role:
                if role == 'RO198' and r['id'] == 'RE6':
                    errors.append(f'{code}: RE6 target {tc} is {tr}, not RO197')
                other[f'{r["id"]}->{tr}'] += 1
                dropped[f'{r["id"]}->{tr}'] += 1
                continue
            s, e = dates(r, 'Operational')
            ls, le = dates(r, 'Legal')
            if not s:
                notes.append(f'{code}: {want_rel}->{tc} has no Operational start; Legal {ls}..{le} used')
                s, e = ls, le
            if tc not in rows:
                errors.append(f'{code}: parent {tc} is not a row of nhs-ods.tsv')
            elif rows[tc]['role'] != want_role:
                errors.append(f'{code}: parent {tc} is {rows[tc]["role"]} in nhs-ods.tsv, not {want_role}')
            ivs[tc].append(dict(start=d(s), end=d(e), legal_start=ls, legal_end=le, rel_status=r.get('Status')))
        if other:
            dropped_by_child[code] = dict(other)
        if not ivs:
            no_parent.append({'code': code, 'role': role, 'why': 'no ' + want_rel + '->' + want_role,
                              'dropped': dict(other), 'status': o['Status'], 'name': o['Name']})
            out.append(dict(code=code, child_role=role, child_status=o['Status'], child_name=o['Name'],
                            rel='-', parent=None, **{'from': None, 'to': None}))
            continue
        for p, lst in ivs.items():
            lst.sort(key=lambda x: x['start'])
            merged = [dict(lst[0])]
            for x in lst[1:]:
                last = merged[-1]
                if last['end'] is not None and x['start'] == last['end'] + datetime.timedelta(days=1):
                    notes.append(f'{code}: {p} {iso(last["start"])}..{iso(last["end"])} + {iso(x["start"])}..{iso(x["end"])} merged (adjacent)')
                    last['end'] = x['end']
                    last['legal_end'] = x['legal_end']
                    last['rel_status'] = x['rel_status']
                    last['merged'] = last.get('merged', 1) + 1
                else:
                    merged.append(dict(x))
            for x in merged:
                pr = rows.get(p, {})
                out.append(dict(code=code, child_role=role, child_status=o['Status'], child_name=o['Name'],
                                rel=want_rel, parent=p, parent_role=pr.get('role'), parent_status=pr.get('status'),
                                parent_allow=pr.get('allow'), parent_name=pr.get('name'),
                                **{'from': iso(x['start']), 'to': iso(x['end'])},
                                legal_from=x['legal_start'], legal_to=x['legal_end'], rel_status=x['rel_status'],
                                intervals_merged=x.get('merged', 1)))

    # overlaps per child (any two intervals, same or different parent)
    overlaps = []
    by_child = collections.defaultdict(list)
    for r in out:
        if r['parent']:
            by_child[r['code']].append(r)
    far = datetime.date(9999, 12, 31)
    for c, lst in by_child.items():
        for i in range(len(lst)):
            for j in range(i + 1, len(lst)):
                a, b = lst[i], lst[j]
                a0, a1 = d(a['from']), d(a['to']) or far
                b0, b1 = d(b['from']), d(b['to']) or far
                if a0 <= b1 and b0 <= a1:
                    overlaps.append({'code': c, 'a': [a['parent'], a['from'], a['to']], 'b': [b['parent'], b['from'], b['to']],
                                     'days': ((min(a1, b1) - max(a0, b0)).days + 1) if min(a1, b1) != far else None})
    if overlaps:
        errors.append(f'{len(overlaps)} overlapping interval pairs')

    # children per allowlisted parent
    per_parent = collections.defaultdict(lambda: {'children': set(), 'current': set(), 'held': set(), 'held_mentions': 0})
    today = datetime.date(2026, 10, 12)
    for r in out:
        p = r['parent']
        if not p:
            continue
        e = per_parent[p]
        e['children'].add(r['code'])
        if d(r['to']) is None or d(r['to']) >= today:
            e['current'].add(r['code'])
        if holders.get(r['code']):
            e['held'].add(r['code'])
    parents = []
    for p, e in sorted(per_parent.items()):
        pr = rows.get(p, {})
        parents.append(dict(parent=p, role=pr.get('role'), allow=pr.get('allow'), status=pr.get('status'),
                            name=pr.get('name'), children=len(e['children']), current=len(e['current']),
                            held=sorted(e['held']),
                            held_mentions=sum(h['mentions'] for c in e['held'] for h in holders[c]),
                            child_codes=sorted(e['children'])))
    tsv_sha = hashlib.sha256(tsv_bytes).hexdigest()
    res = dict(
        generated='2026-10-12', tsv_sha256=tsv_sha,
        source=dict(ro98='measure/ods-ro98/ (ORD, fetched 2026-10-12; 03K refetched)', ro198='../ods/ (ORD, unit 1, 2026-10-12)'),
        columns='code, parent, rel, from, to (Operational, ISO dates; to null = open) + child/parent context',
        http=dict(http), errors=errors, notes=notes, record_vs_tsv_mismatch=rec_mismatch,
        dropped_relations=dict(dropped), overlaps=overlaps, no_parent=no_parent,
        parents=parents, rows=out, holders={c: holders[c] for c, _, _ in children if holders.get(c)})
    with open(os.path.join(HERE, 'child-map.json'), 'w', encoding='utf-8') as f:
        json.dump(res, f, indent=1, ensure_ascii=False)
    print('http', dict(http), 'rows', len(out), 'errors', len(errors))
    for x in errors:
        print('ERROR', x)
    sys.exit(1 if errors else 0)


if __name__ == '__main__':
    main()
