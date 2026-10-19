# Independent check: every UNION ALL arm's window constants vs an independently computed window.
import json, re, sys, datetime, calendar
D = '/home/user/tender-db/.scratch/tender-db/539-triage/'
F = {r[0]: dict(kind=r[1], period=r[2], fetched_at=r[4]) for r in json.load(open(D+'q01_fetches.out.json'))['rows']}
def ed(d): return (d - datetime.date(1970,1,1)).days
def win(f):
    k, p = F[f]['kind'], F[f]['period']
    if k == 'monthly':
        y, m = map(int, p.split('-')); w = datetime.date(y, m, 1)
        e = w + datetime.timedelta(days=calendar.monthrange(y, m)[1])
    else:
        w = datetime.date.fromisoformat(p); e = w + datetime.timedelta(days=1)
    return ed(w), ed(e)
bad = 0; arms = 0
import glob
for path in sorted(glob.glob(D+'q*.sql')):
    name = path.split('/')[-1]
    if name.startswith(('q01', 'q18', 'q19')): continue
    sql = open(path).read()
    for arm in sql.split('UNION ALL'):
        arm = arm.strip()
        if not arm: continue
        arms += 1
        m = re.match(r'SELECT (\d+) (AS )?f', arm); f = int(m.group(1))
        fid = re.findall(r'n.fetch_id ?= ?(\d+)', arm)
        assert all(int(x) == f for x in fid) and fid, (name, f, fid)
        w, e = win(f)
        # collect all comparisons with numeric literals of the epoch-day scale (5 digits)
        lits = set(int(x) for x in re.findall(r'(?<![\d.])(\d{5})(?![\d])', arm)) - {86400}
        exp = {e, w, w-1, w-30, w-180, w-365}
        if name.startswith(('q02', 'q03','q04','q05','q06','q07','q08','q09')):
            fday = (F[f]['fetched_at'] + 7200)//86400; exp |= {fday}
        extra = lits - exp
        # sup arms: no 'after'/'in' constant e, but < w
        if name.startswith(('q10','q11','q12','q13','q14','q15','q16','q17')):
            exp2 = {w, w-1, w-30, w-180, w-365}
            extra = lits - exp2
            W = re.findall(r'published_at<(\d{9,})', arm)
            if any(int(x) != w*86400 for x in W): print('BAD W', name, f, W); bad += 1
        if name.startswith(('q24','q25','q26','q27','q28','q29')):
            exp = {e, w, w-1, w-30}
            extra = lits - exp
        if name.startswith('q02'): extra -= {w-30}  # old bucket
        if extra:
            bad += 1; print('UNEXPECTED', name, f, F[f]['period'], sorted(extra), 'w', w, 'e', e)
        # check bucket structure: 'in' threshold is >= w, 'after' >= e
        for lab, val in [('after', e), ('in', w)]:
            mm = re.search(r">=(\d{5}) THEN '%s'" % lab, arm.replace(' ', '')) if False else None
print('arms', arms, 'bad', bad)
