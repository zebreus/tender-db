#!/usr/bin/env python3
"""Issue 519 unit 2: predict what the U2 dry census lists for child codes on prod, offline.

    python3 -I -B predict_child_census.py

Inputs (all offline, read-only): ../measure/m1.json (the 85 child holders, every name scored by
the Python port of child_check under every dated parent), crates/ingest/sdk/nhs-ods-children.tsv
(the frozen dated map U1 built), ../../513-nhs/u5/plan-u6-dry-after.json (the closing 513 census:
which allowlisted codes the census lists, i.e. `by_code`), and ../measure/child_check_sim.py (for
the head-name and successor-name checks m1 did not store per parent).

Mirrors `plan_nhs_bodies_on`'s child pass and `nhs_child_gates` (canonical.rs): a holder is listed
under every dated parent that is allowlisted AND listed by the census; E3/E4 are taken from M3 (0
impure, 0 unread, 0 PPON, 0 withheld over the 77 sub-ICB and site holders); E5 gates every mention
name (an identity holder's head gates only as one of them; a zero-only cluster's head always).
Writes predicted-child-census.json beside this script and prints the summary.
"""
import collections, json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
U519 = os.path.dirname(HERE)
SCRATCH = os.path.dirname(U519)
REPO = os.path.dirname(os.path.dirname(SCRATCH))
sys.path.insert(0, os.path.join(U519, 'measure'))
import child_check_sim as sim  # noqa: E402

m1 = json.load(open(os.path.join(U519, 'measure', 'm1.json')))
plan = json.load(open(os.path.join(SCRATCH, '513-nhs', 'u5', 'plan-u6-dry-after.json')))['body']
plan = json.loads(plan) if isinstance(plan, str) else plan
listed = {c['code']: c for c in plan['codes']}

children = collections.defaultdict(list)   # child -> [(parent, rel, start, end)]
child_rows = {}
for line in open(os.path.join(REPO, 'crates', 'ingest', 'sdk', 'nhs-ods-children.tsv'), encoding='utf-8'):
    if line.startswith('#') or not line.strip():
        continue
    f = line.rstrip('\n').split('\t')
    child_rows[f[0]] = f
    if f[4] != '-':
        children[f[0]].append((f[5], f[4], f[7], f[8] or None))


def mints_nothing(code):
    return all(not ch.isdigit() or ch == '0' for ch in code)


def survivor_kind(parent_code):
    """The applied body's survivor kind from m1's per-parent body (ppon / anchor / None)."""
    for h in m1['holders']:
        for p in h['parents']:
            if p['parent'] == parent_code and p.get('body') and p['body'].get('survivor'):
                return p['body']['survivor'].get('kind'), p['body'].get('status')
    return None, None


out = {'listed': [], 'unanchored': [], 'no_parent': [], 'invisible': []}
for h in m1['holders']:
    code = h['child']
    shape = 'identity' if h.get('identifier') == f'GBNHS{code}' else (
        'zero-only' if not h.get('identifier') and h.get('provisional') and mints_nothing(code) else None)
    parents = children.get(code, [])
    allow_listed = [p for p in parents if sim.table.get(p[0], {}).get('allow') and p[0] in listed]
    if not parents:
        out['no_parent'].append({'code': code, 'org': h['org_id'], 'mentions': h['mentions']})
        continue
    if not allow_listed:
        target = 'unanchored' if len(code) == 3 else 'invisible'
        out[target].append({'code': code, 'org': h['org_id'], 'mentions': h['mentions'],
                            'parents': [p[0] for p in parents], 'kind': h['kind']})
        continue
    for parent in sorted({p[0] for p in allow_listed}):
        names = h.get('name_detail') or []
        checks = [(n['name'], n['n'], (n['check'].get(parent) or {}).get('verdict')) for n in names]
        head = sim.label(sim.child_check(parent, h['head_name']))
        gating = [c[2] == 'admitted' for c in checks]
        if shape == 'zero-only' or not gating:
            gating.append(head[0] == 'admitted')
        denied = []
        succ = [s for s in sim.table[parent]['succs']
                if s in sim.table and (sim.label(sim.check(s, h['head_name']))[0] == 'admitted')]
        if succ:
            denied.append('successor-name')
        if shape is None:
            denied.append('child-triple-shape')
        if not all(gating):
            denied.append('child-name-refused' if not any(gating) else 'child-name-mixed')
        kind, status = survivor_kind(parent)
        refused = [(n, k, v) for n, k, v in checks if v != 'admitted']
        out['listed'].append({
            'parent': parent, 'parent_survivor': kind, 'parent_body': status, 'code': code, 'org': h['org_id'],
            'kind': h['kind'], 'shape': shape, 'mentions': h['mentions'], 'head': h['head_name'], 'head_check': head,
            'denied': denied, 'refused_names': refused,
            'periods_here': [p[2:] for p in parents if p[0] == parent],
            'other_parents': sorted({p[0] for p in parents if p[0] != parent}),
        })
json.dump(out, open(os.path.join(HERE, 'predicted-child-census.json'), 'w'), indent=1, ensure_ascii=False)

L = out['listed']
print(f"listed child-holder entries {len(L)} over {len({e['org'] for e in L})} orgs, "
      f"{sum(e['mentions'] for e in {e['org']: e for e in L}.values())} mentions, under {len({e['parent'] for e in L})} codes; "
      f"(code, child) pairs {len({(e['parent'], e['code']) for e in L})}")
den = collections.Counter(d for e in L for d in e['denied'])
print('child_denied:', dict(den), '; eligible entries', sum(1 for e in L if not e['denied']))
by_survivor = collections.Counter((e['parent_survivor'], 'eligible' if not e['denied'] else 'denied') for e in L)
print('by parent survivor:', dict(by_survivor))
for e in sorted(L, key=lambda e: (e['parent_survivor'] != 'ppon', e['parent'], e['code'], e['org'])):
    print(f"  {e['parent']:5s} {e['parent_survivor'] or '-':6s} {e['code']:5s} {e['org']:>9} {e['kind']:9s} "
          f"{e['shape'] or '-':9s} m={e['mentions']:<4} {','.join(e['denied']) or 'eligible'}"
          + (f"  other_parents={e['other_parents']}" if e['other_parents'] else ''))
print('child_unanchored (3-char, every parent off the allowlist):',
      sorted({(e['code'], tuple(e['parents'])) for e in out['unanchored']}))
print('child_no_parent:', sorted({e['code'] for e in out['no_parent']}))
print('invisible (5-char under off-allowlist parents only):', sorted({(e['code'], tuple(e['parents'])) for e in out['invisible']}))
