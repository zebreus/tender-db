"""csq.py NAME SQLFILE [--plan-only] -- challenger's bounded /v1/sql statement for 584 U2.
Plan-probes locally (preparer's plan-probe binary + plan_prod.db, read-only) and refuses any SCAN of a
base table or any temp b-tree over a base table walk. Global numbering continues from the preparer's 6;
self cap 8 of the workflow's 10. SQL on ssh STDIN only; the token is never read. Never retried."""
import json, os, subprocess, sys, time, hashlib
SP0 = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/u584'
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/u584c'
PROBE = os.path.join(SP0, 'plan-probe'); DB = os.path.join(SP0, 'plan_prod.db')
U2 = '/home/user/tender-db/.scratch/tender-db/584-u2'
OUT = os.path.join(U2, 'challenge-raw', 'sql'); STATE = os.path.join(SP, 'sq_state.json'); LOG = os.path.join(U2, 'requests.log')
SSH = ['ssh', '-o', 'ConnectTimeout=15', '-o', 'BatchMode=yes', 'root@zebreus.click', '/root/sq.sh']
CAP = 8
name, path = sys.argv[1], sys.argv[2]
sql = open(path).read().strip()
st = json.load(open(STATE)) if os.path.exists(STATE) else {'used_global': 6, 'stop': None}
if st['stop']: sys.exit('REFUSED: stop flag ' + st['stop'])
if st['used_global'] >= CAP: sys.exit('REFUSED: cap')
sf = os.path.join(SP, f'.{name}.stmts'); open(sf, 'w').write(f'-- name: {name}\n{sql}\n')
p = subprocess.run([PROBE, 'plan', DB, sf], capture_output=True, text=True, timeout=120)
steps = [ln.split('\t', 2)[2] for ln in p.stdout.splitlines() if ln.startswith('PLAN\t')]
if p.returncode != 0 or not steps: sys.exit(f'REFUSED: plan failed rc={p.returncode} {p.stderr[:400]} {p.stdout[:400]}')
bad = [s for s in steps if s.strip().startswith('SCAN') and not s.strip().startswith('SCAN (subquery-')]
if bad: sys.exit(f'REFUSED: SCAN {bad}')
if '--plan-only' in sys.argv:
    print('\n'.join(steps)); sys.exit(0)
open(os.path.join(OUT, f'{name}.sql'), 'w').write(sql + '\n')
open(os.path.join(OUT, f'{name}.plan'), 'w').write('\n'.join(steps) + '\n')
t0 = time.time()
try:
    r = subprocess.run(SSH, input=sql, capture_output=True, text=True, timeout=120); out, err = r.stdout, r.stderr
except subprocess.TimeoutExpired:
    out, err = '', 'local timeout'
t1 = time.time(); st['used_global'] += 1
open(os.path.join(OUT, f'{name}.out'), 'w').write(out)
ok = False
try:
    d = json.loads(out); ok = isinstance(d, dict) and 'rows' in d
except ValueError:
    d = None
if not ok: st['stop'] = f'{name}: {out[:200]!r} {err[:200]!r}'
json.dump(st, open(STATE, 'w'))
rows = (d.get('row_count', len(d['rows'])) if ok else None); trunc = d.get('truncated') if ok else None
with open(LOG, 'a') as fh:
    fh.write(f"{time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(t0))} SQL #{st['used_global']} (challenger) {name} sha256:{hashlib.sha256(sql.encode()).hexdigest()[:16]} secs={t1-t0:.2f} ok={ok} rows={rows} truncated={trunc} sql=challenge-raw/sql/{name}.sql\n")
print(json.dumps({'ok': ok, 'rows': rows, 'truncated': trunc, 'secs': round(t1 - t0, 2), 'err': err[:300]}))
