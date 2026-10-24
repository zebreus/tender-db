#!/usr/bin/env python3
"""519 M6: one bounded /v1/sql statement through `ssh root@zebreus.click /root/sq.sh`.

usage: sq.py NAME SQLFILE [--send]

- Always plans the statement locally first with turso's `plan-probe` (EXPLAIN QUERY PLAN) on
  (1) plan/plan-copy.db  -- byte copy of 577-triage/plan-copy.db (sha256 80391c50...ce51)
  (2) plan/plan-idx.db   -- the same copy plus prod's deferred indexes (organizations_identity,
      organizations_name_norm_id, organization_mentions_notice, ...), from 576 U4's plan_prod.db.
- --send refuses unless every step on plan-idx (the copy with prod's indexes) is a SEARCH (or a
  sorter/temp-b-tree helper); plan-copy's plan is recorded beside it.
- Budget: 23 statements in total for 519 M6 across both stages (the state file carries the count),
  >= 3.5 s between sends, a stop flag after any non-answer (408 etc.), and a name is never resent.
- The SQL goes on ssh STDIN (subprocess input=), never as an argument.
"""
import fcntl, hashlib, json, os, re, subprocess, sys, time

HERE = '/home/user/tender-db/.scratch/tender-db/519-m6'
PROBE = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/u4plan/plan-probe'
PROBE_SHA = '50a8094ed7de7ed93ba1c5ba76dd4787b760859c3b50968c96921409d9b8b712'
DBS = [('plan-copy', os.path.join(HERE, 'plan', 'plan-copy.db')),
       ('plan-idx', os.path.join(HERE, 'plan', 'plan-idx.db'))]
OUT = os.path.join(HERE, 'raw', 'sql')
STATE = os.path.join(HERE, 'raw', 'state', 'sql_state.json')
SSH = ['ssh', '-o', 'ConnectTimeout=15', '-o', 'BatchMode=yes', 'root@zebreus.click', '/root/sq.sh']
BUDGET = 23
GAP = 3.5
OK_STEP = re.compile(r'^(SEARCH |USE TEMP B-TREE|USE SORTER|SCAN CONSTANT ROW|COMPOUND QUERY|UNION ALL|LEFT-MOST SUBQUERY)')


def plan(name, sql):
    plans = {}
    sf = os.path.join(HERE, 'raw', 'state', f'.{name}.stmts')
    open(sf, 'w').write(f'-- name: {name}\n{sql}\n')
    for label, db in DBS:
        p = subprocess.run([PROBE, 'plan', db, sf], capture_output=True, text=True, timeout=120)
        steps = [ln.split('\t', 2)[2] for ln in p.stdout.splitlines() if ln.startswith('PLAN\t')]
        plans[label] = steps if (p.returncode == 0 and steps) else [
            f'PLAN-ERR rc={p.returncode} {p.stderr[:300]} {p.stdout[:300]}']
    os.remove(sf)
    return plans


def state():
    return json.load(open(STATE)) if os.path.exists(STATE) else {'used': 0, 'stop': None, 'last': 0, 'log': []}


def main():
    name, path = sys.argv[1], sys.argv[2]
    send = '--send' in sys.argv
    sql = ' '.join(open(path).read().split())
    os.makedirs(OUT, exist_ok=True)
    plans = plan(name, sql)
    print('SQL:', sql[:400] + ('...' if len(sql) > 400 else ''))
    for label, steps in plans.items():
        print(f'PLAN[{label}]:')
        for s in steps:
            print('   ', s)
    # A SEARCH that pins only the leading columns of a composite index walks the slice
    # (issue 329: an IN on a later column is not split into seeks). Refuse those too.
    WALK = re.compile(r'USING (COVERING )?INDEX \S+ \((country=\?|country=\? AND identifier_kind=\?)\)$')
    bad = [s for s in plans['plan-idx'] if not OK_STEP.match(s.strip()) or WALK.search(s.strip())]
    if not send:
        print('NOT-SEND-OK' if bad else 'SEND-OK', bad)
        return
    if hashlib.sha256(open(PROBE, 'rb').read()).hexdigest() != PROBE_SHA:
        sys.exit('REFUSED: plan-probe sha mismatch')
    if bad:
        sys.exit(f'REFUSED: non-SEARCH plan steps on plan-idx {bad}')
    lock = open(os.path.join(HERE, 'raw', 'state', 'sq.lock'), 'a')
    fcntl.flock(lock, fcntl.LOCK_EX)
    st = state()
    if st['stop']:
        sys.exit(f'REFUSED: stop flag set: {st["stop"]}')
    if st['used'] >= BUDGET:
        sys.exit(f'REFUSED: budget of {BUDGET} statements used')
    if any(e['name'] == name for e in st['log']):
        sys.exit(f'REFUSED: {name} already sent (never retried)')
    open(os.path.join(OUT, f'{name}.sql'), 'w').write(sql + '\n')
    with open(os.path.join(OUT, f'{name}.plan'), 'w') as fh:
        for label, steps in plans.items():
            fh.write(f'# {label}\n' + '\n'.join(steps) + '\n')
    wait = GAP - (time.time() - st['last'])
    if wait > 0:
        time.sleep(wait)
    t0 = time.time()
    try:
        p = subprocess.run(SSH, input=sql, capture_output=True, text=True, timeout=120)
        out, err = p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        out, err = '', 'local timeout 120 s'
    t1 = time.time()
    st['used'] += 1
    st['last'] = t1
    open(os.path.join(OUT, f'{name}.out'), 'w').write(out)
    if err.strip():
        open(os.path.join(OUT, f'{name}.err'), 'w').write(err)
    ok, d = False, None
    try:
        d = json.loads(out)
        ok = isinstance(d, dict) and 'rows' in d and 'columns' in d
    except ValueError:
        pass
    entry = {'n': st['used'], 'name': name, 'sha256': hashlib.sha256(sql.encode()).hexdigest(),
             'sent_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(t0)), 'secs': round(t1 - t0, 2), 'ok': ok,
             'rows': (d.get('row_count', len(d['rows'])) if ok else None),
             'truncated': (d.get('truncated') if ok else None)}
    if not ok or d.get('truncated'):
        st['stop'] = f'{name}: not a whole answer: {out[:200]!r} {err[:200]!r}'
        entry['body_head'] = out[:300]
        entry['err_head'] = err[:300]
    st['log'].append(entry)
    json.dump(st, open(STATE, 'w'), indent=1)
    print(json.dumps(entry))
    if not ok:
        sys.exit(3)


main()
