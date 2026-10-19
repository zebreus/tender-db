import json, re, glob, datetime, calendar
D = '/home/user/tender-db/.scratch/tender-db/539-triage/'
F = {r[0]: dict(kind=r[1], period=r[2], fetched_at=r[4]) for r in json.load(open(D+'q01_fetches.out.json'))['rows']}
def ed(d): return (d - datetime.date(1970,1,1)).days
def win(f):
    k, p = F[f]['kind'], F[f]['period']
    if k == 'monthly':
        y, m = map(int, p.split('-')); w = datetime.date(y, m, 1); e = w + datetime.timedelta(days=calendar.monthrange(y, m)[1])
    else:
        w = datetime.date.fromisoformat(p); e = w + datetime.timedelta(days=1)
    return ed(w), ed(e)
PCD = [r'\(\(n\.published_at \+ COALESCE\(n\.published_offset, 0\) \* 60\) / 86400\)', r'\(n\.published_at\+ifnull\(n\.published_offset,0\)\*60\)/86400', r'p']
bad=0; n=0
for path in sorted(glob.glob(D+'q0[3-9]*.sql'))+sorted(glob.glob(D+'q2[0-3]*.sql')):
    for arm in open(path).read().split('UNION ALL'):
        arm=' '.join(arm.split())
        if not arm: continue
        f=int(re.match(r'SELECT (\d+)',arm).group(1)); w,e=win(f); n+=1
        ok=False
        for P in PCD:
            pat=(rf"WHEN {P} ?>= ?{e} THEN 'after' WHEN {P} ?>= ?{w} THEN 'in' WHEN {P} ?= ?{w-1} THEN 'b001' WHEN {P} ?> ?{w-30} THEN 'b002' WHEN {P} ?> ?{w-180} THEN 'b030' WHEN {P} ?> ?{w-365} THEN 'b180' ELSE 'b365' END")
            if re.search(pat, arm): ok=True
        if not ok: bad+=1; print('STRUCT?', path.split('/')[-1], f)
        # dispatched bucket in agg
        if 'q0' in path:
            Q=[r'\(\(n\.dispatched_at \+ COALESCE\(n\.dispatched_offset, 0\) \* 60\) / 86400\)', r'\(n\.dispatched_at\+ifnull\(n\.dispatched_offset,0\)\*60\)/86400']
            if not any(re.search(rf"WHEN n\.dispatched_at IS NULL THEN 'none' WHEN {q} ?>= ?{e} THEN 'after' WHEN {q} ?>= ?{w} THEN 'in' ELSE 'before' END", arm) for q in Q):
                bad+=1; print('DSTRUCT?', path, f)
print('arms', n, 'bad', bad)
