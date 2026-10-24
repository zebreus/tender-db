"""Refutation pass: one bounded /v1/sql statement per call, plan-probed first on a byte copy of
577-triage/plan-copy.db (refute/plan/plan-copy.db, sha256 80391c50...). Budget 6, >= 4 s apart, never retried,
halts after any non-answer (a 408 included). usage: python3 -I rbox.py <name> <sqlfile> [--send]"""
import json, os, re, subprocess, sys, time, datetime, hashlib
HERE = os.path.dirname(os.path.abspath(__file__))
PROBE = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b6u6kit/plan-probe'
PROBE_SHA = '50a8094ed7de7ed93ba1c5ba76dd4787b760859c3b50968c96921409d9b8b712'
DB = os.path.join(HERE, 'plan', 'plan-copy.db')
STATE = os.path.join(HERE, 'state.json'); LOG = os.path.join(HERE, 'queries.log')
SSH = ['ssh', '-o', 'ConnectTimeout=15', '-o', 'BatchMode=yes', 'root@zebreus.click', '/root/sq.sh']
BUDGET, GAP = 6, 4.0
SCAN_OK = re.compile(r'^SCAN (probe|ids)( AS \w+)?$')
def plan(sql, name):
    tmp = os.path.join(HERE, f'.plan_{os.getpid()}.sql')
    open(tmp, 'w').write(f'-- name: {name}\n{sql}\n')
    try:
        p = subprocess.run([PROBE, 'plan', DB, tmp], capture_output=True, text=True, timeout=120)
    finally:
        os.remove(tmp)
    steps = [ln.split('\t', 2)[2] for ln in p.stdout.splitlines() if ln.startswith('PLAN\t')]
    return steps if (p.returncode == 0 and steps) else [f'PLAN-ERR rc={p.returncode} {p.stderr[:400]} {p.stdout[:400]}']
def main():
    name, path = sys.argv[1], sys.argv[2]; send = '--send' in sys.argv
    sql = ' '.join(open(path).read().split())
    steps = plan(sql, name)
    print('PLAN:'); [print('   ', s) for s in steps]
    if not send: return
    if hashlib.sha256(open(PROBE, 'rb').read()).hexdigest() != PROBE_SHA: sys.exit('probe sha mismatch')
    bad = [s for s in steps if s.startswith('PLAN-ERR') or (s.strip().startswith('SCAN') and not SCAN_OK.match(s.strip()))]
    if bad: sys.exit(f'REFUSED: {bad}')
    try: st = json.load(open(STATE))
    except FileNotFoundError: st = {'used': 0, 'last': 0.0, 'halt': None}
    if st['halt']: sys.exit(f'REFUSED: halted after {st["halt"]}')
    if st['used'] >= BUDGET: sys.exit('REFUSED: budget spent')
    w = st['last'] + GAP - time.time()
    if w > 0: time.sleep(w)
    n = st['used'] + 1; t0 = time.time()
    try:
        p = subprocess.run(SSH, input=sql, capture_output=True, text=True, timeout=60); out, err = p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        out, err = '', 'local timeout 60 s'
    t1 = time.time(); st['used'] = n; st['last'] = t1
    raw = os.path.join(HERE, 'raw', f'{n:02d}-{name}.json'); open(raw, 'w').write(out)
    try:
        d = json.loads(out); ok = isinstance(d, dict) and 'rows' in d and 'columns' in d
    except ValueError:
        d, ok = None, False
    if not ok: st['halt'] = f'{n}-{name}: {out[:200]!r} {err[:200]!r}'
    json.dump(st, open(STATE, 'w'))
    open(LOG, 'a').write(json.dumps({'n': n, 'name': name, 'at': datetime.datetime.utcfromtimestamp(t0).isoformat() + 'Z',
        'secs': round(t1 - t0, 2), 'ok': ok, 'sql': sql, 'plan': steps,
        'row_count': d.get('row_count') if ok else None, 'truncated': d.get('truncated') if ok else None,
        'err': None if ok else (out[:300] + ' | ' + err[:300])}) + '\n')
    print(f'SENT #{n} {name} in {t1 - t0:.2f}s ok={ok}')
    if ok: print('columns:', d['columns'], 'rows:', d.get('row_count'), 'truncated:', d.get('truncated'))
    else: print('NOT AN ANSWER:', out[:300], err[:300])
main()
