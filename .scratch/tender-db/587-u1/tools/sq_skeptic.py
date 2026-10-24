"""sq_skeptic.py (587 U1 skeptic copy of sq.py: plan DB copies and .stmts under raw/plan_skeptic/, same budget state and log) NAME SQLFILE [--send] -- one bounded /v1/sql statement for 587 U1, logged to requests.log.
Always: EXPLAIN QUERY PLAN through turso (plan-probe, prebuilt, sha 50a8094e...) on two local schema copies
(plan.db: the canonical schema; plan_prod.db: + prod's deferred indexes, 584's copies).
--send refuses unless every plan step on BOTH copies is a SEARCH (or a temp-b-tree / compound / coroutine helper);
budget 12 statements in total, >= 3.5 s between sends, stop flag after any non-answer (408 etc.), never retried.
SQL goes on ssh STDIN (subprocess input=), never as an argument; the token is never read or printed here."""
import json, os, subprocess, sys, time, fcntl, hashlib, re
HERE = '/home/user/tender-db/.scratch/tender-db/587-u1'
PDIR = '/home/user/tender-db/.scratch/tender-db/587-u1/raw/plan_skeptic'
PROBE_DIR = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/u587'
PROBE = os.path.join(PROBE_DIR, 'plan-probe')
PROBE_SHA = '50a8094ed7de7ed93ba1c5ba76dd4787b760859c3b50968c96921409d9b8b712'
DBS = [('plan', os.path.join(PDIR, 'plan.db')), ('plan_prod', os.path.join(PDIR, 'plan_prod.db'))]
OUT = os.path.join(HERE, 'raw', 'sql')
STATE = os.path.join(HERE, 'raw', 'sql_state.json')
LOG = os.path.join(HERE, 'requests.log')
SSH = ['ssh', '-o', 'ConnectTimeout=15', '-o', 'BatchMode=yes', 'root@zebreus.click', '/root/sq.sh']
BUDGET, GAP = 12, 3.5
OK_STEP = re.compile(r'^(SEARCH |USE TEMP B-TREE|USE SORTER|USE HASH TABLE|SCAN CONSTANT ROW|COMPOUND|UNION ALL|LEFT-MOST SUBQUERY|MERGE|CO-ROUTINE|SCAN \w+$|MATERIALIZE)')

def plan(name, sql):
    plans = {}
    sf = os.path.join(PDIR, f'.{name}.stmts')
    open(sf, 'w').write(f'-- name: {name}\n{sql}\n')
    for label, db in DBS:
        p = subprocess.run([PROBE, 'plan', db, sf], capture_output=True, text=True, timeout=120)
        steps = [ln.split('\t', 2)[2] for ln in p.stdout.splitlines() if ln.startswith('PLAN\t')]
        plans[label] = steps if (p.returncode == 0 and steps) else [f'PLAN-ERR rc={p.returncode} {p.stderr[:300]} {p.stdout[:300]}']
    return plans

def main():
    name, path = sys.argv[1], sys.argv[2]
    send = '--send' in sys.argv
    sql = ' '.join(open(path).read().split())
    os.makedirs(OUT, exist_ok=True)
    plans = plan(name, sql)
    print('SQL bytes:', len(sql))
    for label, steps in plans.items():
        print(f'PLAN[{label}]: {len(steps)} steps')
        for s in steps[:14]:
            print('   ', s)
        if len(steps) > 14:
            print('    ...')
    bad = [(l, s) for l, steps in plans.items() for s in steps if not OK_STEP.match(s.strip())]
    # A bare `SCAN <alias>` is allowed only for a subquery/coroutine alias, never a base table.
    base_scans = [(l, s) for l, steps in plans.items() for s in steps
                  if re.match(r'^SCAN (p|t|b|o|m|tender_version_parties|tenders|organizations|organization_mentions)\b', s.strip())]
    bad += base_scans
    if not send:
        print('NOT-SEND-OK' if bad else 'SEND-OK', bad[:6])
        return
    if hashlib.sha256(open(PROBE, 'rb').read()).hexdigest() != PROBE_SHA:
        sys.exit('REFUSED: plan-probe sha mismatch')
    if bad:
        sys.exit(f'REFUSED: non-SEARCH plan steps {bad[:6]}')
    lock = open(os.path.join(HERE, 'raw', 'sq.lock'), 'a'); fcntl.flock(lock, fcntl.LOCK_EX)
    st = json.load(open(STATE)) if os.path.exists(STATE) else {'used': 0, 'stop': None, 'last': 0, 'log': []}
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
             'rows': (d.get('row_count', len(d['rows'])) if ok else None), 'truncated': (d.get('truncated') if ok else None)}
    if not ok:
        st['stop'] = f'{name}: not an answer: {out[:200]!r} {err[:200]!r}'
        entry['body_head'] = out[:300]
        entry['err_head'] = err[:300]
    st['log'].append(entry)
    json.dump(st, open(STATE, 'w'), indent=1)
    open(LOG, 'a').write(f"{entry['sent_at']} SQL /v1/sql name={name} sha256={entry['sha256'][:16]} ok={ok} rows={entry['rows']} "
                         f"truncated={entry['truncated']} secs={entry['secs']} n={st['used']}/{BUDGET}\n")
    print(json.dumps(entry))

main()
