import json, re, glob, datetime, calendar
D = '/home/user/tender-db/.scratch/tender-db/539-triage/'
F = {r[0]: dict(kind=r[1], period=r[2]) for r in json.load(open(D+'q01_fetches.out.json'))['rows']}
def ed(d): return (d - datetime.date(1970,1,1)).days
def win(f):
    k, p = F[f]['kind'], F[f]['period']
    if k == 'monthly':
        y, m = map(int, p.split('-')); w = datetime.date(y, m, 1); e = w + datetime.timedelta(days=calendar.monthrange(y, m)[1])
    else:
        w = datetime.date.fromisoformat(p); e = w + datetime.timedelta(days=1)
    return ed(w), ed(e)
bad=0; arms=0
for path in sorted(glob.glob(D+'q1[0-7]*.sql')):
    for arm in open(path).read().split('UNION ALL'):
        arm=arm.strip()
        if not arm: continue
        arms+=1
        f=int(re.match(r'SELECT (\d+)',arm).group(1)); w,_=win(f)
        m=re.search(r"m\.source='doe' AND (.*?)\)\) r_doe", arm)
        if not m:
            assert '0 r_doe' in arm; pred='0'
        else: pred=m.group(1)
        for g in F:
            s=pred.replace('+m.fetch_id', str(g)).replace('<>','!=').replace(' AND ',' and ').replace(' OR ',' or ')
            s=re.sub(r'(?<![<>!=])=(?!=)','==',s)
            got=bool(eval(s)); want = win(g)[1] <= w
            if got!=want: bad+=1; print('MISMATCH', path.split('/')[-1], f, g, got, want)
print('arms', arms, 'bad', bad)
