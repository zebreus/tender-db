# usage: python3 -I summ_post.py — issue 539 post-drain audit: per-fetch pre vs post table from the q03..q09 agg/day reads.
# Same per-fetch logic as ../summ.py (n summed, pb buckets, stale = pb 'b*'), plus inv (IssueDate civil day > published civil day)
# and the per-profile split as ../prof.py. Pre = ../qNN.out.json (2026-10-19), post = ./qNN.out.json (2026-10-24).
# Also compares the post buckets with option (a)'s pre-drain prediction (../q20..q23 opta reads, `ra` = bucket of max(P, IssueDate day)).
# Writes table.txt and table.json next to this script.
import json, collections
S = '/home/user/tender-db/.scratch/tender-db/539-triage/'
P = S + 'post-drain/'
READS = ['q03_agg', 'q04_agg', 'q05_agg', 'q06_agg', 'q07_day', 'q08_day', 'q09_day']
F = {r[0]: r for r in json.load(open(S + 'q01_fetches.out.json'))['rows']}
B = ['b001', 'b002', 'b030', 'b180', 'b365']
COLS = ['n', 'inv', 'in', 'after', 'null'] + B

def load(d, names):
    rows = []
    for n in names:
        j = json.load(open(d + n + '.out.json'))
        assert 'rows' in j and 'columns' in j and not j.get('truncated'), d + n
        rows += [dict(zip(j['columns'], r)) for r in j['rows']]
    return rows

def per(rows, key):
    T = collections.defaultdict(collections.Counter)
    for r in rows:
        c = T[key(r)]
        c['n'] += r['n']; c[r['pb']] += r['n']; c['inv'] += r['inv'] or 0; c['fut'] += r['fut'] or 0
        c['nooff'] += r['nooff'] or 0; c['timed'] += r['timed'] or 0
        c['disp'] += r['n'] if r['db'] != 'none' else 0
        c['db_after'] += r['n'] if r['db'] == 'after' else 0
        c['min_id'] = min(c.get('min_id', 1 << 62), r['min_id']); c['max_id'] = max(c.get('max_id', 0), r['max_id'])
        c['min_ing'] = min(c.get('min_ing', 1 << 62), r['min_ing']); c['max_ing'] = max(c.get('max_ing', 0), r['max_ing'])
    return T

def stale(c): return sum(c[b] for b in B)
def era(f): return '2022-12' if f == 444 else ('monthly' if F[f][1] == 'monthly' else 'daily')

pre_rows, post_rows = load(S, READS), load(P, READS)
pre_f, post_f = per(pre_rows, lambda r: r['f']), per(post_rows, lambda r: r['f'])
pre_p, post_p = per(pre_rows, lambda r: r['profile']), per(post_rows, lambda r: r['profile'])
pre_fp, post_fp = per(pre_rows, lambda r: (r['f'], r['profile'])), per(post_rows, lambda r: (r['f'], r['profile']))
pre_e, post_e = per(pre_rows, lambda r: era(r['f'])), per(post_rows, lambda r: era(r['f']))

# option (a) prediction from the pre-drain opta reads, at (f, profile, bucket, IssueDate present)
pred = collections.Counter()
for r in load(S, ['q20_opta', 'q21_opta', 'q22_opta', 'q23_opta']):
    ra = r['ra'] if r['ra'] in ('null', 'after', 'in') else 'b' + r['ra'][1:]
    pred[(r['f'], r['profile'], ra, int(bool(r['d'])))] += r['n']
got = collections.Counter()
for r in post_rows:
    got[(r['f'], r['profile'], r['pb'], int(r['db'] != 'none'))] += r['n']
pre_key = collections.Counter()
for r in pre_rows:
    pre_key[(r['f'], r['profile'], r['pb'], int(r['db'] != 'none'))] += r['n']

out = []
w = out.append
w('# issue 539 post-drain audit: per-fetch pre (2026-10-19, ../q03..q09) vs post (2026-10-24, ./q03..q09). Each cell pre>post, or one value when unchanged.')
w('# buckets: published civil day vs the delivering export window [W,E): in, after (>= E), null, gap1 (W-1), gap2-29, gap30-179, gap180-364, gap365+.')
w('# inv = notices whose IssueDate civil day > published civil day. opta = post buckets equal option (a)\'s pre-drain prediction (q20..q23) at (profile, bucket, IssueDate present).')
hdr = ['fetch', 'kind', 'period', 'notices', 'inv', 'in', 'after', 'null', 'gap1', 'gap2-29', 'gap30-179', 'gap180-364', 'gap365+', 'stale%', 'opta']
lines = [hdr]
def cell(a, b): return str(a) if a == b else f'{a}>{b}'
fetches = sorted(F, key=lambda f: (F[f][1] != 'monthly', F[f][2]))
fetches = sorted(F, key=lambda f: [l.split()[0] for l in open(S + 'fetch_table.txt').read().splitlines()[1:]].index(str(f)))
opta_bad = []
for f in fetches:
    a, b = pre_f.get(f, collections.Counter()), post_f.get(f, collections.Counter())
    sa = 100 * stale(a) / a['n'] if a['n'] else 0; sb = 100 * stale(b) / b['n'] if b['n'] else 0
    keys = {k for k in set(pred) | set(got) if k[0] == f}
    ok = all(pred[k] == got[k] for k in keys)
    if not ok: opta_bad.append(f)
    lines.append([str(f), F[f][1], F[f][2]] + [cell(a[k], b[k]) for k in COLS] + [cell('%.2f' % sa, '%.2f' % sb), 'yes' if ok else 'NO'])
wd = [max(len(l[i]) for l in lines) for i in range(len(hdr))]
for l in lines: w(' '.join(x.ljust(wd[i]) if i < 3 else x.rjust(wd[i]) for i, x in enumerate(l)))

def tot(T):
    c = collections.Counter()
    for v in T.values():
        for k in COLS + ['fut', 'nooff', 'timed', 'disp', 'db_after']: c[k] += v[k]
    return c
w('')
w('# totals, by era, by profile (pre>post)')
hdr2 = ['group', 'notices', 'inv', 'in', 'after', 'null', 'gap1', 'gap2-29', 'gap30-179', 'gap180-364', 'gap365+', 'stale', 'stale%', 'stale>=2', 'stale>=30', 'fut', 'nooff', 'timed']
rows2 = [hdr2]
def row2(name, a, b):
    def s2(c): return sum(c[x] for x in B[1:])
    def s30(c): return sum(c[x] for x in B[2:])
    pa = 100 * stale(a) / a['n'] if a['n'] else 0; pb = 100 * stale(b) / b['n'] if b['n'] else 0
    rows2.append([name] + [cell(a[k], b[k]) for k in COLS] + [cell(stale(a), stale(b)), cell('%.2f' % pa, '%.2f' % pb),
                 cell(s2(a), s2(b)), cell(s30(a), s30(b)), cell(a['fut'], b['fut']), cell(a['nooff'], b['nooff']), cell(a['timed'], b['timed'])])
row2('ALL', tot(pre_f), tot(post_f))
reg_a = tot({f: v for f, v in pre_f.items() if f != 444}); reg_b = tot({f: v for f, v in post_f.items() if f != 444})
row2('regular (excl. 444)', reg_a, reg_b)
for e in ['2022-12', 'monthly', 'daily']: row2('era ' + e, pre_e[e], post_e[e])
for p in sorted(set(pre_p) | set(post_p), key=lambda p: -pre_p[p]['n']): row2(p.replace('eforms:eforms-', ''), pre_p[p], post_p[p])
wd2 = [max(len(l[i]) for l in rows2) for i in range(len(hdr2))]
for l in rows2: w(' '.join(x.ljust(wd2[i]) if i == 0 else x.rjust(wd2[i]) for i, x in enumerate(l)))

w('')
w('# checks')
inv_post = {f: c['inv'] for f, c in post_f.items() if c['inv']}
w(f'fetches covered: {len(set(pre_f) | set(post_f))} with notices (+3 empty: 3, 5, 445) of {len(F)}; fetches with inv>0 pre: {sum(1 for c in pre_f.values() if c["inv"])}, post: {len(inv_post)}')
w(f'inv total pre {sum(c["inv"] for c in pre_f.values())} post {sum(c["inv"] for c in post_f.values())}')
for f, v in sorted(inv_post.items()):
    w(f'  inv>0 post: fetch {f} ({F[f][2]}): {v} = ' + ', '.join(f'{p.replace("eforms:eforms-", "")} {c["inv"]}' for (g, p), c in sorted(post_fp.items()) if g == f and c['inv']))
nchg = [f for f in F if pre_f.get(f, {}).get('n', 0) != post_f.get(f, {}).get('n', 0)]
w(f'fetches whose notice count changed: {len(nchg)} ' + ' '.join(f'{f}:{pre_f[f]["n"]}>{post_f[f]["n"]}' for f in nchg))
pchg = [k for k in set(pre_fp) | set(post_fp) if pre_fp.get(k, {}).get('n', 0) != post_fp.get(k, {}).get('n', 0)]
w(f'(fetch, profile) notice counts changed: {len(pchg)}')
idchg = [f for f in pre_f if (pre_f[f]['min_id'], pre_f[f]['max_id'], pre_f[f]['min_ing'], pre_f[f]['max_ing']) != (post_f[f]['min_id'], post_f[f]['max_id'], post_f[f]['min_ing'], post_f[f]['max_ing'])]
w(f'fetches whose id range or ingested_at range changed: {len(idchg)} {idchg}')
dchg = [f for f in pre_f if (pre_f[f]['disp'], pre_f[f]['db_after']) != (post_f[f]['disp'], post_f[f]['db_after'])]
w(f'fetches whose IssueDate-present count or IssueDate-after-window count changed: {len(dchg)} {dchg}')
moved = sum(abs(pre_key[k] - got[k]) for k in set(pre_key) | set(got)) // 2
w(f'(fetch, profile, bucket, IssueDate) cells: notices that changed cell pre->post (lower bound on re-dated rows that crossed a bucket): {moved}')
pd = sum(abs(pred[k] - got[k]) for k in set(pred) | set(got)) // 2
w(f'option (a) prediction (q20..q23, pre-drain) vs post buckets: {"IDENTICAL" if not opta_bad else "DIFF"}; fetches differing: {len(opta_bad)} {opta_bad}; notices off-prediction: {pd}')
for k in sorted(k for k in set(pred) | set(got) if pred[k] != got[k]):
    w(f'  off-prediction cell {k}: predicted {pred[k]} got {got[k]} (pre {pre_key[k]})')
print('\n'.join(out))
open(P + 'table.txt', 'w').write('\n'.join(out) + '\n')
json.dump({'pre_fetch': {str(f): dict(c) for f, c in pre_f.items()}, 'post_fetch': {str(f): dict(c) for f, c in post_f.items()},
           'pre_profile': {p: dict(c) for p, c in pre_p.items()}, 'post_profile': {p: dict(c) for p, c in post_p.items()}},
          open(P + 'table.json', 'w'), indent=0)
