"""sq.py NAME SQLFILE -- one bounded /v1/sql statement for 584 U2 (preparer).
Plan-probes locally first with turso plan-probe on a copy of 577-triage/plan-copy.db WITH prod's
deferred indexes (576-u4's plan_prod.db); refuses any plan step that starts with SCAN.
Workflow budget 10 statements (this agent self-caps at 6); >= 3.5 s apart; stop flag after any
non-answer (408 etc.), never retried. SQL goes on ssh STDIN only; the token is never read here."""
import json, os, subprocess, sys, time, fcntl, hashlib
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/u584'
PROBE = os.path.join(SP, 'plan-probe')
DBS = [('plan-copy+prod-indexes', os.path.join(SP, 'plan_prod.db'))]
U2 = '/home/user/tender-db/.scratch/tender-db/584-u2'
OUT = os.path.join(U2, 'raw', 'sql')
STATE = os.path.join(OUT, 'state.json')
LOG = os.path.join(U2, 'requests.log')
SSH = ['ssh', '-o', 'ConnectTimeout=15', '-o', 'BatchMode=yes', 'root@zebreus.click', '/root/sq.sh']
SELF_CAP = 6
GAP = 3.5

def main():
    name, path = sys.argv[1], sys.argv[2]
    sql = open(path).read().strip()
    lock = open(os.path.join(SP, 'sq.lock'), 'a'); fcntl.flock(lock, fcntl.LOCK_EX)
    st = json.load(open(STATE)) if os.path.exists(STATE) else {'used': 0, 'stop': None, 'last': 0, 'log': []}
    if st['stop']:
        sys.exit(f'REFUSED: stop flag set: {st["stop"]}')
    if st['used'] >= SELF_CAP:
        sys.exit(f'REFUSED: self cap of {SELF_CAP} statements used')
    plans = {}
    for label, db in DBS:
        sf = os.path.join(SP, f'.{name}.stmts')
        open(sf, 'w').write(f'-- name: {name}\n{sql}\n')
        p = subprocess.run([PROBE, 'plan', db, sf], capture_output=True, text=True, timeout=120)
        steps = [ln.split('\t', 2)[2] for ln in p.stdout.splitlines() if ln.startswith('PLAN\t')]
        if p.returncode != 0 or not steps:
            sys.exit(f'REFUSED: plan failed on {label}: rc={p.returncode} {p.stderr[:300]} {p.stdout[:300]}')
        bad = [s for s in steps if s.strip().startswith('SCAN') and not s.strip().startswith('SCAN (subquery-')]
        if bad:
            sys.exit(f'REFUSED: plan on {label} SCANs {bad}')
        plans[label] = steps
    if '--plan-only' in sys.argv:
        for label, steps in plans.items():
            print('#', label); print('\n'.join(steps))
        return
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
    ok = False
    try:
        d = json.loads(out)
        ok = isinstance(d, dict) and 'rows' in d and 'columns' in d
    except ValueError:
        d = None
    entry = {'name': name, 'sha256': hashlib.sha256(sql.encode()).hexdigest()[:16],
             'sent_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(t0)),
             'secs': round(t1 - t0, 2), 'ok': ok, 'rows': (d.get('row_count', len(d['rows'])) if ok else None),
             'truncated': (d.get('truncated') if ok else None)}
    if not ok:
        st['stop'] = f'{name}: not an answer: {out[:200]!r} {err[:200]!r}'
        entry['body_head'] = out[:300]
    st['log'].append(entry)
    json.dump(st, open(STATE, 'w'), indent=1)
    with open(LOG, 'a') as fh:
        fh.write(f"{entry['sent_at']} SQL #{st['used']} {name} sha256:{entry['sha256']} secs={entry['secs']} ok={ok} rows={entry['rows']} truncated={entry['truncated']} sql=raw/sql/{name}.sql\n")
    print(json.dumps(entry))

main()
