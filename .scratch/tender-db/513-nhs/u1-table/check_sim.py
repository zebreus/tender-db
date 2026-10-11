#!/usr/bin/env python3
"""Issue 513 U1: a Python reference of crates/ingest/src/nhs_ods.rs's `check`, over the
frozen TSV, for cross-checking the Rust port verdict by verdict.

    python3 -I check_sim.py [rows.json] [--verdicts out.tsv]

It is design B's B4 check (../u2-panel/designB/b4.py with gate_b/gate_b2/gate_b3) with
the judges' two grafts: a non-NHS institution name is refused (NonNhs), and an exact tie
with a rival is reported (Tie), not admitted. Rival order is code order and a tie between
rivals goes to the first code (b4.py's max() kept the last), which changes only the rival
named, never a verdict.

The tally is over every adjudication row whose code is in the table: SAME admitted vs
refused (Tie counts as not admitted), DIFFERENT / NOT_ORG / HOSTED admitted.
"""
import collections
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
NHS = os.path.dirname(HERE)
REPO = os.path.dirname(os.path.dirname(os.path.dirname(NHS)))
TSV = os.path.join(REPO, 'crates', 'ingest', 'sdk', 'nhs-ods.tsv')

table = {}
for line in open(TSV, encoding='utf-8'):
    line = line.rstrip('\n')
    if not line or line.startswith('#'):
        continue
    code, st, rc, role, allow, preds, name = line.split('\t')
    table[code] = dict(status=st, rc=rc, role=role, allow=allow == '1',
                       preds=[p for p in preds.split(',') if p], name=name)
codes = sorted(table)

STOP = set('nhs the and of for in t as trading c o limited ltd plc at with'.split())
DESC = set('hospital university teaching healthcare health care community mental partnership service ambulance '
           'children women specialist acute general foundation trust integrated board commissioning support unit '
           'clinical group strategic local sub location solution provider collaborative'.split())
ABBR = {'icb': 'integrated care board', 'ccg': 'clinical commissioning group',
        'csu': 'commissioning support unit', 'ft': 'foundation trust', 'st': 'saint'}
CLASS = {'RO197': 'trust', 'RO107': 'trust', 'RO261': 'icb', 'RO98': 'icb', 'RO213': 'csu'}
AGENT = re.compile(r'\bc/o\b|\bcare of\b|\bon behalf of\b|\bacting (?:for|on behalf)\b', re.I)
FORMS = set('limited ltd plc llp'.split())
CIVIC = set('council councils school schools'.split())
ACADEMIC = set('university universities college colleges'.split())
NHS_ROLE = {'nhs', 'trust', 'trusts', 'commissioning'}


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
    """A university, council, school or company name with no NHS role token. The
    university/college markers yield to 'hospital' (University Hospitals Birmingham,
    King's College Hospital), and a company code's own legal form is no marker."""
    t = set(tokens(name))
    if t & NHS_ROLE or ({'integrated', 'care'} <= t and t & {'board', 'boards'}):
        return False
    if t & CIVIC: return True
    if t & ACADEMIC and not t & {'hospital', 'hospitals'}: return True
    return bool(t & FORMS) and not company


def check(code, name):
    o = table.get(code)
    if o is None: return ('Refused', 'Unknown', None)
    if AGENT.search(name or ''): return ('Refused', 'Agent', None)
    if non_nhs(name, o['role'] == 'RO172'): return ('Refused', 'NonNhs', None)
    m = split(name)
    own_set = {code, *o['preds']}
    own_names = [(o['name'], o['role'])] + [(table[p]['name'], table[p]['role']) for p in o['preds']]
    own = key(m, own_names)
    if own[0] == 0: return ('Refused', 'Role', None)
    if own[1] == 0: return ('Refused', 'Zero', None)
    best, bc = None, None
    for c in codes:
        if c in own_set: continue
        k = key(m, [(table[c]['name'], table[c]['role'])])
        if best is None or k > best: best, bc = k, c
    if best is not None and best > own: return ('Refused', 'Better', bc)
    D = m[0]
    cov = max(sum(1 for a in D if any(tm(a, b) for b in split(nm)[0])) for nm, _ in own_names)
    if 2 * cov < len(D): return ('Refused', 'Coverage', None)
    if best is not None and best == own: return ('Tie', None, bc)
    return ('Admitted', None, None)


def main():
    args = [a for a in sys.argv[1:]]
    out = None
    if '--verdicts' in args:
        i = args.index('--verdicts'); out = args[i + 1]; del args[i:i + 2]
    rows_path = args[0] if args else os.path.join(NHS, 'u1', 'adjudication-rows.json')
    rows = json.load(open(rows_path, encoding='utf-8'))
    tally = collections.defaultdict(collections.Counter)
    det = []
    lines = []
    for r in rows:
        if r['code'] not in table:
            tally[(r['side'], r['final'])]['not-in-table'] += 1
            continue
        v, why, rival = check(r['code'], r['name'])
        lines.append('\t'.join([r['side'], r['code'], r['final'], v + (f':{why}' if why else '') + (f':{rival}' if rival else ''), r['name']]))
        k = tally[(r['side'], r['final'])]
        k['rows'] += 1
        k[v if v != 'Refused' else f'Refused:{why}'] += 1
        if r['final'] == 'SAME' and v != 'Admitted':
            det.append(('SAME-NOT-ADMITTED', r['side'], r['code'], r['name'][:70], v, why, rival))
        if r['final'] != 'SAME' and v == 'Admitted':
            det.append((f'ADMITTED-{r["final"]}', r['side'], r['code'], r['name'][:70]))
    for k in sorted(tally):
        print(k, dict(sorted(tally[k].items())))
    for d in sorted(det, key=str):
        print('  ', d)
    if out:
        with open(out, 'w', encoding='utf-8') as f:
            f.write('\n'.join(lines) + '\n')


if __name__ == '__main__':
    main()
