#!/usr/bin/env python3
"""Issue 519 M1: a Python port of the unit 2 decision's `child_check` (unit2-decision.md §1.3),
extending 513's `check_sim.py` (../../513-nhs/u1-table/check_sim.py).

    python3 -I -B child_check_sim.py          # self-test: 513 tally + the decision's §1.3 table

What it ports, against crates/ingest/src/nhs_ods.rs at HEAD (2026-10-12):
- `check` (513): kept verbatim from check_sim.py, with two catch-ups to the Rust: the TSV now has
  nine fields (code, status, rc, role, allow, predecessors, successors, legal_end, name), and the
  NonNhs markers include `charity`, `charities`, `charitable` (nhs_ods.rs `non_nhs`).
- `child_check(parent, name)` (519 §1.3), two changes to `check`:
  1. rivals are every row except the parent's own names (parent + its predecessors) AND except the
     Active RO98 rows named "<current ICB> - <code>" (since 2026-04, RELABELS: 107 rows); the other
     Active RO98 rows (commissioning hubs, legacy CCGs, London reporting entities) and the Inactive
     RO98 rows stay rivals (narrowed after the U2 review, 2026-10-12);
  2. `nhse`: a name with the token `england` that declares no ICB, trust or CSU role is refused.
     Placed after NonNhs and before Role (the decision does not fix its position; the decision's
     own table pins 03Q → nhse, which this order gives).
  Only `Admitted` passes; `Tie` refuses.
"""
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
U519 = os.path.dirname(HERE)
SCRATCH = os.path.dirname(U519)
REPO = os.path.dirname(os.path.dirname(SCRATCH))
TSV = os.path.join(REPO, 'crates', 'ingest', 'sdk', 'nhs-ods.tsv')

table = {}
VERSION = None
for line in open(TSV, encoding='utf-8'):
    line = line.rstrip('\n')
    if line.startswith('# version: '):
        VERSION = line[len('# version: '):]
    if not line or line.startswith('#'):
        continue
    code, st, rc, role, allow, preds, succs, legal_end, name = line.split('\t')
    table[code] = dict(status=st, rc=rc, role=role, allow=allow == '1',
                       preds=[p for p in preds.split(',') if p], succs=[s for s in succs.split(',') if s],
                       legal_end=legal_end or None, name=name)
codes = sorted(table)
# 519 unit 2 §1.3 (narrowed after review, 2026-10-12): the rows `child_check` drops from the rivals are
# the Active RO98 rows named as their current ICB's duplicate, "<ICB> - <code>". The other 58 Active RO98
# rows (the NHS England commissioning hubs, the legacy CCGs 00W/12A/13P, London's "<borough> - SUB-ICB
# REPORTING ENTITY" rows) are bodies of their own and stay rivals.
RELABELS = {c for c, o in table.items()
            if o['role'] == 'RO98' and o['status'] == 'A' and o['name'].endswith(' - ' + c)}

STOP = set('nhs the and of for in t as trading c o limited ltd plc at with'.split())
DESC = set('hospital university teaching healthcare health care community mental partnership service ambulance '
           'children women specialist acute general foundation trust integrated board commissioning support unit '
           'clinical group strategic local sub location solution provider collaborative'.split())
ABBR = {'icb': 'integrated care board', 'ccg': 'clinical commissioning group',
        'csu': 'commissioning support unit', 'ft': 'foundation trust', 'st': 'saint'}
CLASS = {'RO197': 'trust', 'RO107': 'trust', 'RO261': 'icb', 'RO98': 'icb', 'RO213': 'csu'}
FORMS = set('limited ltd plc llp'.split())
CIVIC = set('council councils school schools charity charities charitable'.split())
ACADEMIC = set('university universities college colleges'.split())
NHS_ROLE = {'nhs', 'trust', 'trusts', 'commissioning'}
AGENT_MARKERS = ['c/o', 'care of', 'on behalf of', 'acting for', 'acting on behalf']


def agent(name):
    """nhs_ods.rs `agent`: a marker at word boundaries (alphanumeric or `_` is a word char)."""
    lower = (name or '').lower()
    word = lambda ch: ch.isalnum() or ch == '_'
    for m in AGENT_MARKERS:
        start = 0
        while True:
            at = lower.find(m, start)
            if at < 0:
                break
            before = lower[at - 1] if at > 0 else ''
            after = lower[at + len(m)] if at + len(m) < len(lower) else ''
            if not (before and word(before)) and not (after and word(after)):
                return True
            start = at + 1
    return False


def tokens(n):
    s = (n or '').lower().replace('&', ' and ').replace('’', "'").replace("'", '')
    raw = re.sub(r'[^a-z0-9]+', ' ', s).split()
    out, i = [], 0
    while i < len(raw):
        if raw[i:i + 3] == ['n', 'h', 's']:
            out.append('nhs'); i += 3; continue
        out += ABBR.get(raw[i], raw[i]).split(); i += 1
    return out


def words(n):
    out = []
    for w in tokens(n):
        if w in STOP:
            continue
        if len(w) > 3 and w.endswith('s') and not w.endswith('ss'):
            w = w[:-1]
        out.append(w)
    return out


def role_of(ws):
    S = set(ws)
    if {'commissioning', 'support'} <= S: return 'csu'
    if {'integrated', 'board'} <= S or {'clinical', 'commissioning'} <= S: return 'icb'
    if 'trust' in S: return 'trust'
    return None


_split = {}


def split(n):
    if n not in _split:
        ws = words(n)
        _split[n] = ({w for w in ws if w not in DESC}, {w for w in ws if w in DESC}, role_of(ws))
    return _split[n]


def tm(a, b):
    return a == b or (min(len(a), len(b)) >= 6 and (a.startswith(b) or b.startswith(a)))


def jf(A, B):
    if not A or not B: return 0.0
    m = sum(1 for a in A if any(tm(a, b) for b in B))
    return m / (len(A) + len(B) - m)


def j(A, B):
    return len(A & B) / len(A | B) if A and B else 0.0


def key(m, names):
    D, E, r = m
    best = (0, 0.0, 0.0)
    for nm, role in names:
        d, e, _ = split(nm)
        compat = 1 if (r is None or CLASS.get(role) == r) else 0
        best = max(best, (compat, jf(D, d), j(E, e)))
    return best


def non_nhs(name, company):
    t = set(tokens(name))
    if t & NHS_ROLE or ({'integrated', 'care'} <= t and t & {'board', 'boards'}):
        return False
    if t & CIVIC: return True
    if t & ACADEMIC and not t & {'hospital', 'hospitals'}: return True
    return bool(t & FORMS) and not company


def _gate(code, name, child):
    o = table.get(code)
    if o is None: return ('Refused', 'Unknown', None)
    if agent(name): return ('Refused', 'Agent', None)
    if non_nhs(name, o['role'] == 'RO172'): return ('Refused', 'NonNhs', None)
    m = split(name)
    if child and 'england' in tokens(name) and m[2] is None:
        return ('Refused', 'Nhse', None)
    own_set = {code, *[p for p in o['preds'] if p in table]}
    own_names = [(o['name'], o['role'])] + [(table[p]['name'], table[p]['role']) for p in o['preds'] if p in table]
    own = key(m, own_names)
    if own[0] == 0: return ('Refused', 'Role', None)
    if own[1] == 0: return ('Refused', 'Zero', None)
    best, bc = None, None
    for c in codes:
        if c in own_set: continue
        if child and c in RELABELS:
            continue
        k = key(m, [(table[c]['name'], table[c]['role'])])
        if best is None or k > best: best, bc = k, c
    if best is not None and best > own: return ('Refused', 'Better', bc)
    D = m[0]
    cov = max(sum(1 for a in D if any(tm(a, b) for b in split(nm)[0])) for nm, _ in own_names)
    if 2 * cov < len(D): return ('Refused', 'Coverage', None)
    if best is not None and best == own: return ('Tie', None, bc)
    return ('Admitted', None, None)


def check(code, name):
    """513's `check` (nhs_ods.rs)."""
    return _gate(code, name, child=False)


def child_check(parent, name):
    """519's `child_check(P, name)` (decision §1.3)."""
    return _gate(parent, name, child=True)


def label(v):
    """The census label (`census_check` spelling): admitted | tie | agent | non-nhs | nhse | role | zero |
    better | coverage | unknown, plus the rival when there is one."""
    verdict, why, rival = v
    if verdict == 'Admitted': return 'admitted', None
    if verdict == 'Tie': return 'tie', rival
    return {'Agent': 'agent', 'NonNhs': 'non-nhs', 'Nhse': 'nhse', 'Role': 'role', 'Zero': 'zero',
            'Better': 'better', 'Coverage': 'coverage', 'Unknown': 'unknown'}[why], rival


def _selftest():
    import collections
    ok = True
    # 1. The 513 tally the Rust test pins (233/235 SAME registry, 116/116 SAME PPON, no DIFFERENT /
    #    NOT_ORG admitted; HOSTED_OR_SITE is not pinned) — proves the port still matches the Rust `check`.
    rows = json.load(open(os.path.join(SCRATCH, '513-nhs', 'u1', 'adjudication-rows.json'), encoding='utf-8'))
    t = collections.Counter()
    for r in rows:
        if r['code'] not in table:
            continue
        adm = check(r['code'], r['name'])[0] == 'Admitted'
        if r['final'] in ('SAME', 'DIFFERENT', 'NOT_ORG'):
            t[(r['side'], r['final'] == 'SAME', adm)] += 1
    same_reg = (t[('registry', True, True)], t[('registry', True, True)] + t[('registry', True, False)])
    same_ppon = (t[('ppon', True, True)], t[('ppon', True, True)] + t[('ppon', True, False)])
    bad = t[('registry', False, True)] + t[('ppon', False, True)]
    print(f'513 tally: SAME registry {same_reg[0]}/{same_reg[1]}, SAME ppon {same_ppon[0]}/{same_ppon[1]}, '
          f'DIFFERENT/NOT_ORG admitted {bad}')
    ok &= same_reg == (233, 235) and same_ppon == (116, 116) and bad == 0
    # 2. The decision's §1.3 and §6 expected answers.
    cases = [
        ('QMM', 'NHS Norfolk and Suffolk ICB', ('better', 'T6Y0W')),
        ('QHM', 'NORTH OF ENGLAND COMMISSIONING SUPPORT', ('role', None)),
        ('QRV', 'NHS North West London Integrated Care Board', None),  # W2U3Z's name: printed, see below
        ('QWU', 'NHS COVENTRY AND WARWICKSHIRE INTEGRATED CARE BOARD', ('admitted', None)),
        ('QWU', 'NHS Coventry and Warwickshire ICB - B2M3M', ('admitted', None)),
        # The decision's §6 says `zero`; the Rust `non_nhs` refuses it first (`college`, no `hospital`).
        ('QHL', 'College Road Surgery', ('non-nhs', None)),
        ('QWO', 'NHS West Yorkshire Integrated Care Board (Leeds Place)', ('admitted', None)),
        ('QMM', 'James Paget University Hospitals NHS Foundation Trust', 'refused'),
    ]
    for p, n, want in cases:
        got = label(child_check(p, n))
        if want is None:
            print(f'  {p} {n!r}: {got} (informational)')
            continue
        good = (got[0] != 'admitted') if want == 'refused' else got == want
        ok &= good
        print(f'  {"ok " if good else "BAD"} {p} {n!r}: {got} want {want}')
    print('SELFTEST', 'PASS' if ok else 'FAIL')
    return ok


if __name__ == '__main__':
    sys.exit(0 if _selftest() else 1)
