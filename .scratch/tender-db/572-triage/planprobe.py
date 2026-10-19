# usage: python3 -I planprobe.py <name> — EXPLAIN QUERY PLAN of <name>.sql on the local plan.db copy
# (545 u2b's plan.db: the prod schema + the two prod org indexes organizations_identifier_id and
# organizations_name_norm_id), written to <name>.plan.txt. Exit 2 if any step is a SCAN.
import sqlite3, sys
d = '/home/user/tender-db/.scratch/tender-db/572-triage/'
n = sys.argv[1]
sql = open(d + n + '.sql').read()
c = sqlite3.connect('file:' + d + 'plan.db?mode=ro', uri=True)
rows = list(c.execute('EXPLAIN QUERY PLAN ' + sql))
with open(d + n + '.plan.txt', 'w') as f:
    f.write('-- EXPLAIN QUERY PLAN, python sqlite3 %s on plan.db (545 u2b copy: prod schema + prod org indexes)\n' % sqlite3.sqlite_version)
    for r in rows: f.write(repr(r) + '\n')
scans = [r for r in rows if 'SCAN' in r[3]]
print(n, len(rows), 'plan steps;', 'SCANS: %r' % scans if scans else 'no SCAN')
for r in rows[:8]: print('  ', r)
sys.exit(2 if scans else 0)
