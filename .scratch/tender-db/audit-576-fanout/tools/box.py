"""Audit 576 fan-out: one bounded /v1/sql statement per call, plan-probed locally first.

usage: python3 -I box.py <name> <sqlfile> [--send]
  Always: EXPLAIN QUERY PLAN through turso (plan-probe) on plan/plan-copy.db (byte copy of 577-triage/plan-copy.db)
  and on plan/plan-idx.db (the same copy + prod's deferred organization indexes, canonical.rs ~17706-17726).
  --send: refuses if the idx plan has a SCAN step other than the statement's own literal CTE (probe/ids) or if the
  budget (25) is spent or a 408/unknown body was seen; waits until >= 4 s since the last send; one ssh call, never
  retried. Raw body saved to box/raw/<n>-<name>.json; every send logged to box/queries.log.
"""
import json, os, re, subprocess, sys, time, hashlib, datetime
HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROBE = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b6u6kit/plan-probe'
PROBE_SHA = '50a8094ed7de7ed93ba1c5ba76dd4787b760859c3b50968c96921409d9b8b712'
DBS = {'plan-copy': os.path.join(HERE, 'plan', 'plan-copy.db'), 'plan-idx': os.path.join(HERE, 'plan', 'plan-idx.db')}
STATE = os.path.join(HERE, 'box', 'state.json')
LOG = os.path.join(HERE, 'box', 'queries.log')
SSH = ['ssh', '-o', 'ConnectTimeout=15', '-o', 'BatchMode=yes', 'root@zebreus.click', '/root/sq.sh']
BUDGET = 25
GAP = 4.0
SCAN_OK = re.compile(r'^SCAN (probe|ids|ns)( AS \w+)?$')

def plan(sql, name):
    tmp = os.path.join(HERE, 'box', f'.plan_{os.getpid()}.sql')
    with open(tmp, 'w') as fh:
        fh.write(f'-- name: {name}\n{sql}\n')
    out = {}
    try:
        for k, db in DBS.items():
            p = subprocess.run([PROBE, 'plan', db, tmp], capture_output=True, text=True, timeout=120)
            steps = [ln.split('\t', 2)[2] for ln in p.stdout.splitlines() if ln.startswith('PLAN\t')]
            out[k] = steps if (p.returncode == 0 and steps) else [f'PLAN-ERR rc={p.returncode} {p.stderr[:300]} {p.stdout[:300]}']
    finally:
        os.remove(tmp)
    return out

def load():
    try:
        return json.load(open(STATE))
    except FileNotFoundError:
        return {'used': 0, 'last': 0.0, 'halt': None}

def main():
    name, path = sys.argv[1], sys.argv[2]
    send = '--send' in sys.argv
    sql = ' '.join(open(path).read().split())
    plans = plan(sql, name)
    print('SQL:', sql)
    for k, steps in plans.items():
        print(f'PLAN[{k}]:')
        for s in steps:
            print('   ', s)
    if not send:
        return
    h = hashlib.sha256(open(PROBE, 'rb').read()).hexdigest()
    if h != PROBE_SHA:
        sys.exit('plan-probe sha mismatch')
    bad = [s for s in plans['plan-idx'] if s.strip().startswith('SCAN') and not SCAN_OK.match(s.strip())] \
        + [s for s in plans['plan-idx'] if s.startswith('PLAN-ERR')]
    if bad:
        sys.exit(f'REFUSED: plan has {bad}')
    st = load()
    if st['halt']:
        sys.exit(f'REFUSED: halted after {st["halt"]}')
    if st['used'] >= BUDGET:
        sys.exit('REFUSED: budget spent')
    wait = st['last'] + GAP - time.time()
    if wait > 0:
        time.sleep(wait)
    n = st['used'] + 1
    t0 = time.time()
    try:
        p = subprocess.run(SSH, input=sql, capture_output=True, text=True, timeout=120)
        out, err = p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        out, err = '', 'local timeout 120 s'
    t1 = time.time()
    st['used'] = n
    st['last'] = t1
    raw = os.path.join(HERE, 'box', 'raw', f'{n:02d}-{name}.json')
    with open(raw, 'w') as fh:
        fh.write(out)
    ok = False
    try:
        d = json.loads(out)
        ok = isinstance(d, dict) and 'rows' in d and 'columns' in d
    except ValueError:
        d = None
    if not ok:
        st['halt'] = f'{n}-{name}: {out[:200]!r} {err[:200]!r}'
    json.dump(st, open(STATE, 'w'))
    with open(LOG, 'a') as fh:
        fh.write(json.dumps({'n': n, 'name': name, 'at': datetime.datetime.utcfromtimestamp(t0).isoformat() + 'Z',
                             'secs': round(t1 - t0, 2), 'ok': ok, 'sql': sql, 'plan_copy': plans['plan-copy'],
                             'plan_idx': plans['plan-idx'],
                             'row_count': d.get('row_count') if ok else None,
                             'truncated': d.get('truncated') if ok else None,
                             'err': None if ok else (out[:300] + ' | ' + err[:300])}) + '\n')
    print(f'SENT #{n} {name} in {t1 - t0:.2f}s ok={ok}')
    if ok:
        print('columns:', d['columns'], 'row_count:', d.get('row_count'), 'truncated:', d.get('truncated'))
        rows = d['rows']
        for r in rows[:60]:
            print('  ', r)
        if len(rows) > 60:
            print(f'   ... {len(rows) - 60} more rows in {raw}')
    else:
        print('NOT AN ANSWER:', out[:500], err[:500])

main()
