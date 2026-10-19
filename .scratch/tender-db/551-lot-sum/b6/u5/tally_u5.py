#!/usr/bin/env python3
"""Issue 551 B6 U5 tally: round a (+ round b if any) and the B-band (decision points 7 and 8).

Offline. Reads b6/u5/order.json, band_set.json, excluded_sources.json, b6/u2/P.json, b6/u2/decisions.jsonl.gz,
the live heads under b6/u5/bodies/, api_requests.log, and the verdict list given by the orchestrator.
Writes b6/u5/u5-result.json only (the .md is written from its output).
"""
import collections
import datetime
import gzip
import hashlib
import json
import os
import re
import sys

B6 = '/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6'
U5 = os.path.join(B6, 'u5')
HERE = os.path.dirname(os.path.abspath(__file__))
SALT = '551-b6-fresh-'


def sha(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for c in iter(lambda: f.read(1 << 20), b''):
            h.update(c)
    return h.hexdigest()


def load(path):
    with open(path) as f:
        return json.load(f)


round_a = load(os.path.join(HERE, 'round_a_verdicts.json'))
round_b = []  # given empty by the orchestrator
band_reads = []  # given empty by the orchestrator

order = load(os.path.join(U5, 'order.json'))
band = load(os.path.join(U5, 'band_set.json'))
P = load(os.path.join(B6, 'u2', 'P.json'))['guard_sets']['v2+both']
ships = {s['id']: s for s in P['ships']}
entries = {e['id']: e for e in order['entries']}
excluded = set(order['excluded_from_P'])

decisions = {}
with gzip.open(os.path.join(B6, 'u2', 'decisions.jsonl.gz'), 'rt') as f:
    for line in f:
        r = json.loads(line)
        decisions[r['id']] = r

problems = []

# ---- order checks
ids_a = [v['id'] for v in round_a]
if len(ids_a) != 30:
    problems.append('round a has %d ids, not 30' % len(ids_a))
if ids_a != order['first_60'][:30]:
    problems.append('round a ids are not first_60[:30] in order')
for i, tid in enumerate(ids_a, 1):
    e = entries[tid]
    if e['rank'] != i:
        problems.append('%d rank %d != %d' % (tid, e['rank'], i))
    if hashlib.sha256((SALT + str(tid)).encode()).hexdigest() != e['order_key']:
        problems.append('%d order key mismatch' % tid)
    if tid in excluded:
        problems.append('%d is an already-read id' % tid)
    if tid not in ships:
        problems.append('%d not in P' % tid)
keys = [hashlib.sha256((SALT + str(t)).encode()).hexdigest() for t in order['first_60']]
if keys != sorted(keys):
    problems.append('first_60 not ascending by order key')

# ---- per id
VALID = {'right': {'ok'}, 'neutral': None, 'wrong': {'not_additive', 'double_count', 'farther', 'slip', 'other'}}
reader_of = {}
for k in range(1, 7):
    txt = open(os.path.join(U5, 'read_a_%d.md' % k)).read()
    for m in re.finditer(r'^\| (\d+) \| (\d+) \| ([A-Z]{3}) ([\d,]+\.\d\d) \| (\d+) \|.*\*\*(right|neutral|wrong) / (\w+)\*\*', txt, re.M):
        reader_of[int(m.group(2))] = {
            'file': 'b6/u5/read_a_%d.md' % k, 'rank': int(m.group(1)), 'currency': m.group(3),
            'S_cents': int(m.group(4).replace(',', '').replace('.', '')), 'n': int(m.group(5)),
            'verdict': m.group(6), 'cls': m.group(7)}

rows = []
for i, v in enumerate(round_a, 1):
    tid = v['id']
    e = entries[tid]
    s = ships[tid]
    d = decisions[tid]['decisions']['v2+both']
    for k in ('S', 'currency', 'n', 'vouched_by'):
        if not (e[k] == s[k] == d[k]):
            problems.append('%d %s differs among order/P/decisions' % (tid, k))
    if v['verdict'] not in VALID:
        problems.append('%d bad verdict %r' % (tid, v['verdict']))
    elif VALID[v['verdict']] is not None and v['cls'] not in VALID[v['verdict']]:
        problems.append('%d verdict %s with class %r' % (tid, v['verdict'], v['cls']))
    rd = reader_of.get(tid)
    if rd is None:
        problems.append('%d not found in a read_a table' % tid)
    else:
        if (rd['verdict'], rd['cls']) != (v['verdict'], v['cls']):
            problems.append('%d given verdict differs from %s' % (tid, rd['file']))
        if (rd['S_cents'], rd['currency'], rd['n'], rd['rank']) != (e['S'], e['currency'], e['n'], i):
            problems.append('%d reader table S/currency/n/rank differs from order' % tid)
    body = load(os.path.join(U5, 'bodies', 'tenders_%d.json' % tid))
    head = body.get('value')
    head_ratio = None
    if head is not None:
        if head['currency'] != e['currency']:
            problems.append('%d head currency %s != S currency' % (tid, head['currency']))
        head_ratio = round(100.0 * head['cents'] / e['S'], 1)
    rows.append({
        'rank': i, 'id': tid, 'verdict': v['verdict'], 'class': v['cls'],
        'S_cents': e['S'], 'currency': e['currency'], 'S_eur_cents': e['S_eur_cents'], 'n': e['n'],
        'vouched_by': e['vouched_by'], 'flags': e['flags'],
        'summed': decisions[tid]['summed'],
        'live_head': head, 'live_head_pct_of_S': head_ratio,
        'reader_file': rd['file'] if rd else None, 'why': v['why']})

counts_a = collections.Counter(r['verdict'] for r in rows)
wrong_a = counts_a.get('wrong', 0)
classes_a = collections.Counter('%s/%s' % (r['verdict'], r['class']) for r in rows)

if wrong_a == 0:
    decision, round_b_required = 'GO', False
elif wrong_a == 1:
    round_b_required = True
    wrong_b = sum(1 for r in round_b if r['verdict'] == 'wrong')
    decision = 'GO' if (len(round_b) == 30 and wrong_b == 0) else ('KEEP OFF' if round_b else 'PENDING ROUND B')
else:
    decision, round_b_required = 'KEEP OFF (re-decide)', False
if round_b and not round_b_required:
    problems.append('round b given but not required')

# ---- band (point 8)
recomputed_changed = []
for r in band['rows']:
    p, l = r['predicted_v2_both'], r['last_read']
    diff = [k for k in ('S', 'currency', 'n', 'vouched_by') if p[k] != l[k]]
    if diff:
        recomputed_changed.append({'id': r['id'], 'changed': diff})
    if r['id'] not in ships:
        problems.append('band head %d not in P' % r['id'])
band_req = sorted(set(band['band_reread_set']))
if recomputed_changed or band['changed_ids'] or band['l_changed']['band_rows_in_l_changed'] or band_req:
    band_required = True
else:
    band_required = False
band_wrong = sum(1 for r in band_reads if r['verdict'] == 'wrong')
if band_required and len(band_reads) < len(band_req):
    band_result = 'PENDING'
elif band_wrong == 0:
    band_result = 'PASS'
else:
    band_result = 'FAIL'
band_last = collections.Counter('%s/%s' % (r['last_read']['verdict'], r['last_read']['class']) for r in band['rows'])

# ---- request log audit
pat = re.compile(r'^(\S+) (\S+) (\S+) rc=(\S+) reader=(\S+)')
log = []
for line in open(os.path.join(U5, 'api_requests.log')).read().splitlines():
    m = pat.match(line)
    if not m:
        problems.append('unparsed log line: ' + line[:80])
        continue
    log.append((datetime.datetime.strptime(m.group(1), '%Y-%m-%dT%H:%M:%S.%fZ'), m.group(2), m.group(3),
                m.group(4), m.group(5)))
log.sort()
gaps = [(b[0] - a[0]).total_seconds() for a, b in zip(log, log[1:])]
heads_fetched = {int(x[2].rsplit('/', 1)[1]) for x in log if re.fullmatch(r'/v1/tenders/\d+', x[2])}
missing_heads = [t for t in ids_a if t not in heads_fetched]
if missing_heads:
    problems.append('no live head GET for %s' % missing_heads)
log_audit = {
    'requests': len(log), 'methods': dict(collections.Counter(x[1] for x in log)),
    'status': dict(collections.Counter(x[3] for x in log)),
    'by_reader': dict(sorted(collections.Counter(x[4] for x in log).items())),
    'first': log[0][0].isoformat() + 'Z', 'last': log[-1][0].isoformat() + 'Z',
    'min_gap_s': min(gaps), 'gaps_under_2s': sum(g < 2 for g in gaps),
    'http_408': sum(1 for x in log if x[3] == '408'),
    'repeated_paths': [p for p, c in collections.Counter(x[2] for x in log).items() if c > 1],
    'tender_heads_fetched': len(heads_fetched), 'round_a_heads_missing': missing_heads,
    'bodies_files': len(os.listdir(os.path.join(U5, 'bodies'))),
}
if log_audit['gaps_under_2s'] or set(log_audit['methods']) != {'GET'}:
    problems.append('log audit: pacing or method violation')

inputs = {}
for rel in ['u5/order.json', 'u5/band_set.json', 'u5/excluded_sources.json', 'u5/build_order.py', 'u2/P.json',
            'u2/decisions.jsonl.gz', 'u5/api_requests.log'] + ['u5/read_a_%d.md' % k for k in range(1, 7)]:
    inputs['b6/' + rel] = sha(os.path.join(B6, rel))

out = {
    'what': 'issue 551 B6 U5: the fresh pre-flip read (decision point 7) and the B-band re-read (point 8), tallied',
    'date': '2026-10-19',
    'P': order['P'],
    'order': {'salt': SALT, 'rule': order['order_rule'], 'eligible_n': order['eligible_n'],
              'excluded_from_P_n': order['excluded_from_P_n'], 'ordered_ids_sha256': order['ordered_ids_sha256']},
    'rule_point_7': '30 from P minus every read id: 0 wrong -> GO; exactly 1 wrong -> the next 30 in the same order '
                    'must have 0 wrong -> GO, else KEEP OFF; >= 2 wrong -> KEEP OFF and re-decide',
    'round_a': {'ids': ids_a, 'n': len(rows), 'right': counts_a.get('right', 0),
                'neutral': counts_a.get('neutral', 0), 'wrong': wrong_a,
                'by_class': dict(classes_a), 'verdicts': rows},
    'round_b': {'required': round_b_required, 'n': len(round_b), 'ids': [r['id'] for r in round_b],
                'note': 'not required: round a has 0 wrong' if not round_b_required else None},
    'decision_point_7': decision,
    'band_point_8': {
        'rule': band['rule'],
        'B_P_n': len(band['B_P']), 'B_P': band['B_P'],
        'B_P_last_read': dict(band_last),
        'changed_vs_last_read': band['changed_ids'], 'changed_recomputed': recomputed_changed,
        'band_rows_in_l_changed': band['l_changed']['band_rows_in_l_changed'],
        'P_in_l_changed': band['l_changed']['P_in_l_changed'],
        'delta': band['delta'],
        'reread_set': band_req, 'required': band_required,
        'reread_n': len(band_reads), 'wrong': band_wrong,
        'result': band_result,
        'result_note': 'no band head changed predicted S, currency, n or vouched_by against its last read, none is in '
                       'L_changed, and the delta does not exist pre-flip: the re-read set is empty, 0 re-reads, 0 wrong '
                       '(holds vacuously; the 29 heads keep their recorded verdicts)',
        'within_2pct_of_eur100m': band['within_2pct_of_eur100m'],
        'within_2pct_outside_B_P': band['within_2pct_outside_B_P'],
        'D4_note': '899272 (HUF, S_eur_cents %d) sits inside 2 %% of EUR 100 m but below it and outside B_P; under '
                   'point 8 D4 a drain-time difference from B_P on it is stop-and-explain, not a switch-off'
                   % ships[899272]['S_eur_cents'],
    },
    'go': decision == 'GO' and band_result == 'PASS',
    'request_log_audit': log_audit,
    'inputs_sha256': inputs,
    'checks_failed': problems,
}
with open(os.path.join(U5, 'u5-result.json'), 'w') as f:
    json.dump(out, f, indent=1, ensure_ascii=False)
    f.write('\n')
print(json.dumps({k: out[k] for k in ('decision_point_7', 'go', 'checks_failed')}))
print('round a', out['round_a']['right'], out['round_a']['neutral'], out['round_a']['wrong'], out['round_a']['by_class'])
print('band', out['band_point_8']['result'], out['band_point_8']['required'], out['band_point_8']['B_P_last_read'])
print('log', json.dumps(log_audit))
for r in rows:
    print(r['rank'], r['id'], r['currency'], r['S_cents'], r['n'], r['vouched_by']['field'], r['vouched_by']['notice_id'],
          r['live_head'], r['live_head_pct_of_S'], r['flags'], r['reader_file'])
