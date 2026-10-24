#!/usr/bin/env python3
"""519 M6: live.json + the offline prior -> packet.json (one case per cohort org).

usage: build_packet.py   (paths fixed under 519-m6/)

- live.json = {"orgs": R1 rows, "mentions": R2 rows, "targets": R3 + R4 rows}, assembled here from raw/sql/*.out.
- A prior org missing from R1 -> gone. A new R1 org -> classified by spec §3.1 with ODS answers in ods/ (WebFetch).
- Candidates: the prior's target(s) re-checked against R3 (dropped with a note when the literal changed),
  the code_survivors row of the code the value seems to spell, and the R4 owners of PPON values.
"""
import collections, glob, json, os, re

D = '/home/user/tender-db/.scratch/tender-db/519-m6'
PRIOR = '/home/user/tender-db/.scratch/tender-db/build-specs/519-m6-prior.json'
CLASSIFIED = '/home/user/tender-db/.scratch/tender-db/519-nhs-digit-codes/gbnhs-orgs-classified.json'


def rows(pattern):
    out = []
    for f in sorted(glob.glob(os.path.join(D, 'raw', 'sql', pattern))):
        out += json.load(open(f))['rows']
    return out


orgs = rows('r1_*.out')
ments = rows('r2_*.out')
r3 = rows('r3_*.out')        # id, country, identifier_kind, identifier, name, provisional
r4 = rows('r4_*.out')        # id, identifier, name, provisional
targets = [[r[0], r[3], r[4], r[5]] for r in r3] + [list(r) for r in r4]
json.dump({'orgs': orgs, 'mentions': ments, 'targets': targets,
           'read': '2026-10-24T18:15:58Z..18:18:41Z (raw/sql/*.out; 12 statements)'},
          open(os.path.join(D, 'live.json'), 'w'))

prior = json.load(open(PRIOR))
cls = {x['id']: x for x in json.load(open(CLASSIFIED))}
live = {r[0]: r for r in orgs}
tgt = {r[0]: r for r in targets}
by_org = collections.defaultdict(list)
for m in ments:
    by_org[m[0]].append(m)

cases = []
for p in prior['rows']:
    cases.append(dict(p, new=False))
for oid, r in live.items():
    if oid in cls:
        continue
    code = r[1][5:]
    ods = json.load(open(os.path.join(D, 'ods', code + '.json'))) if os.path.exists(os.path.join(D, 'ods', code + '.json')) else None
    shaped = bool(re.fullmatch(r'[A-Z0-9]{3}|[A-Z0-9]{5}|[A-Z0-9]{6}', code))
    if shaped and ods and ods.get('found'):
        print(f'new {oid} {r[1]}: ODS found ({ods["name"]}, {ods["primary_role"]}) -> out of M6')
        continue
    cases.append({'org': oid, 'identifier': r[1], 'code': code, 'name': r[2], 'mentions': len(by_org[oid]),
                  'ods': '404' if shaped else 'not-ods-shaped', 'shape': 'other', 'new': True,
                  'prior': {'route': None, 'correct': None, 'target': None, 'note': 'new since 2026-10-12'}})

packet = []
for c in cases:
    oid = c['org']
    lv = live.get(oid)
    ms = by_org.get(oid, [])
    names = collections.Counter(m[3] for m in ms)
    raws = collections.Counter(m[4] for m in ms)
    cands, notes = [], []
    t = c['prior'].get('target')
    if t:
        for x in (t if isinstance(t[0], list) else [t]):
            now = tgt.get(x[0])
            if now and now[1] == x[1]:
                cands.append({'org': x[0], 'identifier': x[1], 'name': now[2], 'provisional': now[3], 'via': 'prior-target'})
            else:
                notes.append(f'candidate {x[0]} {x[1]} dropped: live {now}')
    packet.append({
        'org': oid, 'identifier': c['identifier'], 'code': c['code'],
        'standing': bool(lv and lv[1] == c['identifier']),
        'head': lv[2] if lv else c['name'], 'provisional': lv[4] if lv else None,
        'mentions': len(ms), 'mentions_prior': c['mentions'],
        'mention_names': dict(names.most_common()), 'raw_identifiers': dict(raws.most_common()),
        'notices': sorted({m[1] for m in ms}),
        'sections': sorted({m[2] for m in ms}),
        'ods': c['ods'], 'shape': c['shape'], 'route': c['prior']['route'], 'prior': c['prior'], 'new': c['new'],
        'candidates': cands, 'notes': notes,
    })
json.dump({'generated': '2026-10-24 from live.json + build-specs/519-m6-prior.json',
           'code_survivors': prior['code_survivors'],
           'r4_owners': [list(r) for r in r4],
           'cases': packet}, open(os.path.join(D, 'packet.json'), 'w'), indent=1)
print('cases', len(packet), 'standing', sum(p['standing'] for p in packet),
      'mentions', sum(p['mentions'] for p in packet))
