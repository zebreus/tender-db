# usage: python3 -I prof.py [--only f1,f2] [--exclude f1,f2] <out.json>... — per-profile table:
# total, in, after, null, stale by gap bucket split by dispatch present; IssueDate position of stale rows.
import json, sys, collections
args = sys.argv[1:]
only = exclude = None
if args and args[0] == '--only': only = set(map(int, args[1].split(','))); args = args[2:]
if args and args[0] == '--exclude': exclude = set(map(int, args[1].split(','))); args = args[2:]
rows = []
for p in args:
    j = json.load(open(p)); assert 'rows' in j and not j.get('truncated'), p
    rows += [dict(zip(j['columns'], r)) for r in j['rows']]
rows = [r for r in rows if (only is None or r['f'] in only) and (exclude is None or r['f'] not in exclude)]
B = ['b001', 'b002', 'b030', 'b180', 'b365']
P = collections.defaultdict(collections.Counter)
for r in rows:
    c = P[r['profile']]; disp = r['db'] != 'none'
    c['n'] += r['n']; c['disp'] += r['n'] if disp else 0
    c[r['pb']] += r['n']
    c[(r['pb'], 'D' if disp else 'N')] += r['n']
    c[('db', r['pb'][:1] if r['pb'].startswith('b') else r['pb'], r['db'])] += r['n']
    c['inv'] += r['inv'] or 0; c['fut'] += r['fut'] or 0; c['nooff'] += r['nooff'] or 0; c['timed'] += r['timed'] or 0
    if r['pb'] in ('in', 'after'): c['inv_nonstale'] += r['inv'] or 0
for p, c in sorted(P.items(), key=lambda x: -x[1]['n']):
    stale = sum(c[b] for b in B)
    print(f"\n{p}: n={c['n']} disp={c['disp']} ({100*c['disp']/c['n']:.1f}%) in={c['in']} after={c['after']} null={c['null']} "
          f"stale={stale} ({100*stale/c['n']:.2f}%) fut={c['fut']} nooff={c['nooff']} timed={c['timed']} inv(dcd>pcd)={c['inv']} inv_nonstale={c['inv_nonstale']}")
    print('   gap    with-IssueDate  without   total')
    for b, lab in zip(B, ['1', '2-29', '30-179', '180-364', '365+']):
        print(f"   {lab:8s} {c[(b,'D')]:8d} {c[(b,'N')]:8d} {c[b]:8d}")
    s_d = sum(c[(b,'D')] for b in B); s_n = sum(c[(b,'N')] for b in B)
    print(f"   {'>=1':8s} {s_d:8d} {s_n:8d} {stale:8d}   (stale share of dispatch-bearing {100*s_d/max(c['disp'],1):.2f}%, of dispatch-less {100*s_n/max(c['n']-c['disp'],1):.2f}%)")
    print('   IssueDate position of stale rows: ' + ', '.join(f"{k[2]}={v}" for k, v in sorted((k, v) for k, v in c.items() if isinstance(k, tuple) and k[0]=='db' and k[1]=='b')))
    print('   IssueDate position of in-window rows: ' + ', '.join(f"{k[2]}={v}" for k, v in sorted((k, v) for k, v in c.items() if isinstance(k, tuple) and k[0]=='db' and k[1]=='in')))
