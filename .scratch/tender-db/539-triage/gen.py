# usage: python3 -I gen.py agg <name> <fetch_id>...   — step-2 aggregation SQL, one UNION ALL arm per fetch,
#        each arm bounded by n.fetch_id = <id> (equality; GROUP BY +n.profile so notices_fetch_id drives).
import json, sys, datetime
D = '/home/user/tender-db/.scratch/tender-db/539-triage/'
EPOCH = datetime.date(1970, 1, 1)

def fetches():
    j = json.load(open(D + 'q01_fetches.out.json'))
    return {r[0]: dict(id=r[0], kind=r[1], period=r[2], bytes=r[3], fetched_at=r[4]) for r in j['rows']}

def window(f):
    """[W, E) as epoch-day numbers (civil days, Europe/Berlin as published)."""
    if f['kind'] == 'monthly':
        y, m = map(int, f['period'].split('-'))
        w = datetime.date(y, m, 1)
        e = datetime.date(y + (m == 12), 1 if m == 12 else m + 1, 1)
    else:
        w = datetime.date.fromisoformat(f['period'])
        e = w + datetime.timedelta(days=1)
    return (w - EPOCH).days, (e - EPOCH).days

PCD = "((n.published_at + COALESCE(n.published_offset, 0) * 60) / 86400)"
DCD = "((n.dispatched_at + COALESCE(n.dispatched_offset, 0) * 60) / 86400)"

def agg_arm(f):
    w, e = window(f)
    fday = (f['fetched_at'] + 7200) // 86400
    return f"""SELECT {f['id']} AS f, n.profile AS profile,
  CASE WHEN n.published_at IS NULL THEN 'null'
       WHEN {PCD} >= {e} THEN 'after'
       WHEN {PCD} >= {w} THEN 'in'
       WHEN {PCD} = {w - 1} THEN 'b001'
       WHEN {PCD} > {w - 30} THEN 'b002'
       WHEN {PCD} > {w - 180} THEN 'b030'
       WHEN {PCD} > {w - 365} THEN 'b180'
       ELSE 'b365' END AS pb,
  CASE WHEN n.dispatched_at IS NULL THEN 'none'
       WHEN {DCD} >= {e} THEN 'after'
       WHEN {DCD} >= {w} THEN 'in'
       ELSE 'before' END AS db,
  count(*) AS n,
  sum(CASE WHEN {DCD} > {PCD} THEN 1 ELSE 0 END) AS inv,
  sum(CASE WHEN {PCD} > {fday} THEN 1 ELSE 0 END) AS fut,
  sum(CASE WHEN n.published_offset IS NULL THEN 1 ELSE 0 END) AS nooff,
  sum(CASE WHEN n.published_has_time = 1 THEN 1 ELSE 0 END) AS timed,
  min({PCD}) AS min_pcd,
  min(n.id) AS min_id, max(n.id) AS max_id, min(n.ingested_at) AS min_ing, max(n.ingested_at) AS max_ing
  FROM notices n
 WHERE n.fetch_id = {f['id']}
 GROUP BY +n.profile, pb, db"""

if __name__ == '__main__':
    mode, name, *ids = sys.argv[1:]
    F = fetches()
    if mode == 'agg':
        sql = '\nUNION ALL\n'.join(agg_arm(F[int(i)]) for i in ids) + '\n'
        open(D + name + '.sql', 'w').write(sql)
        print(name, len(sql), 'bytes', [(i, F[int(i)]['period']) for i in ids])

P2 = "(n.published_at+ifnull(n.published_offset,0)*60)/86400"
Q2 = "(n.dispatched_at+ifnull(n.dispatched_offset,0)*60)/86400"

def aggc_arm(f):
    """The agg arm, compact (same columns and buckets) — to fit ~30 daily arms in the 64 KB body cap."""
    w, e = window(f)
    fday = (f['fetched_at'] + 7200) // 86400
    P, Q = P2, Q2
    return (f"SELECT {f['id']} f,n.profile profile,CASE WHEN n.published_at IS NULL THEN 'null' "
            f"WHEN {P}>={e} THEN 'after' WHEN {P}>={w} THEN 'in' WHEN {P}={w-1} THEN 'b001' "
            f"WHEN {P}>{w-30} THEN 'b002' WHEN {P}>{w-180} THEN 'b030' WHEN {P}>{w-365} THEN 'b180' ELSE 'b365' END pb,"
            f"CASE WHEN n.dispatched_at IS NULL THEN 'none' WHEN {Q}>={e} THEN 'after' WHEN {Q}>={w} THEN 'in' ELSE 'before' END db,"
            f"count(*) n,sum({Q}>{P}) inv,sum({P}>{fday}) fut,sum(n.published_offset IS NULL) nooff,"
            f"sum(n.published_has_time=1) timed,min({P}) min_pcd,min(n.id) min_id,max(n.id) max_id,"
            f"min(n.ingested_at) min_ing,max(n.ingested_at) max_ing FROM notices n WHERE n.fetch_id={f['id']} GROUP BY +n.profile,pb,db")

if __name__ == '__main__' and sys.argv[1] == 'aggc':
    mode, name, *ids = sys.argv[1:]
    F = fetches()
    sql = '\nUNION ALL\n'.join(aggc_arm(F[int(i)]) for i in ids) + '\n'
    open(D + name + '.sql', 'w').write(sql)
    print(name, len(sql), 'bytes', len(ids), 'arms', F[int(ids[0])]['period'], '..', F[int(ids[-1])]['period'])

def earlier(f, col='+m.fetch_id'):
    """SQL predicate: DÖE fetch `col` delivered an export whose window ENDS at or before f's window START.
    Fetch ids: 4 = 2026-06, 402 = 2026-07, 403..444 = 2026-05 descending to 2022-12, dailies 3, 5, 445+ ascending
    by day (all >= 2026-07-17). Checked against window() for every fetch pair in check_earlier()."""
    i = f['id']
    if f['kind'] == 'monthly':
        if 403 <= i <= 444: return f"{col}>{i} AND {col}<=444" if i < 444 else "0"
        if i == 4: return f"{col}>=403 AND {col}<=444"
        if i == 402: return f"({col}=4 OR ({col}>=403 AND {col}<=444))"
    else:
        w, e = window(f)
        aug1 = (datetime.date(2026, 8, 1) - EPOCH).days
        return f"{col}<{i}" + ("" if w >= aug1 else f" AND {col}<>402")
    raise ValueError(i)

def check_earlier():
    F = fetches()
    def ev(pred, j):
        import re
        s = pred.replace('+m.fetch_id', str(j)).replace('<>', '!=').replace('AND', 'and').replace('OR', 'or')
        s = re.sub(r'(?<![<>!=])=(?!=)', '==', s)
        return bool(eval(s))
    bad = 0
    for f in F.values():
        p = earlier(f); w, _ = window(f)
        for g in F.values():
            want = window(g)[1] <= w
            if ev(p, g['id']) != want: bad += 1; print('MISMATCH', f['id'], f['period'], g['id'], g['period'], want)
    print('earlier() mismatches:', bad, 'of', len(F) ** 2)

def sup_arm(f):
    w, e = window(f)
    P = P2
    W = w * 86400
    return (f"SELECT {f['id']} f,n.profile profile,CASE WHEN {P}={w-1} THEN 'b001' WHEN {P}>{w-30} THEN 'b002' "
            f"WHEN {P}>{w-180} THEN 'b030' WHEN {P}>{w-365} THEN 'b180' ELSE 'b365' END pb,n.dispatched_at IS NOT NULL d,"
            f"count(DISTINCT n.id) stale,count(DISTINCT CASE WHEN v.tender_id IS NOT NULL THEN n.id END) with_v,count(v.tender_id) vers,"
            f"sum(EXISTS(SELECT 1 FROM tender_versions x WHERE x.tender_id=v.tender_id AND x.seq<>v.seq)) multi,"
            + (f"sum(EXISTS(SELECT 1 FROM tender_versions x JOIN notices m ON m.id=x.caused_by_notice_id WHERE x.tender_id=v.tender_id AND x.seq>v.seq AND m.source='doe' AND {earlier(f)})) r_doe,"
               if earlier(f) != "0" else "0 r_doe,") +  # 444 (2022-12) has no earlier export; a constant-false EXISTS planned as a SCAN
            f"sum(EXISTS(SELECT 1 FROM tender_versions x JOIN notices m ON m.id=x.caused_by_notice_id WHERE x.tender_id=v.tender_id AND x.seq>v.seq AND m.source<>'doe' AND x.published_at<{W})) r_ted,"
            f"sum(EXISTS(SELECT 1 FROM tender_versions x WHERE x.tender_id=v.tender_id AND x.seq>v.seq AND x.published_at<{W})) r_pub "
            f"FROM notices n LEFT JOIN tender_versions v ON v.caused_by_notice_id=n.id "
            f"WHERE n.fetch_id={f['id']} AND n.published_at IS NOT NULL AND {P}<{w} GROUP BY +n.profile,pb,d")

if __name__ == '__main__' and sys.argv[1] == 'check':
    check_earlier()
if __name__ == '__main__' and sys.argv[1] == 'sup':
    mode, name, *ids = sys.argv[1:]
    F = fetches()
    sql = '\nUNION ALL\n'.join(sup_arm(F[int(i)]) for i in ids) + '\n'
    open(D + name + '.sql', 'w').write(sql)
    print(name, len(sql), 'bytes', len(ids), 'arms', F[int(ids[0])]['period'], '..', F[int(ids[-1])]['period'])

def opta_arm(f):
    """Option (a) residual: A = max(requested civil day, IssueDate civil day) (A = requested when no IssueDate);
    pb = the served gap bucket, ra = the gap bucket A would leave, against the delivering export's window."""
    w, e = window(f)
    P, Q = P2, Q2
    A = f"(CASE WHEN n.dispatched_at IS NULL OR {Q}<{P} THEN {P} ELSE {Q} END)"
    return (f"SELECT {f['id']} f,n.profile profile,CASE WHEN n.published_at IS NULL THEN 'null' "
            f"WHEN {P}>={e} THEN 'after' WHEN {P}>={w} THEN 'in' WHEN {P}={w-1} THEN 'b001' "
            f"WHEN {P}>{w-30} THEN 'b002' WHEN {P}>{w-180} THEN 'b030' WHEN {P}>{w-365} THEN 'b180' ELSE 'b365' END pb,"
            f"CASE WHEN n.published_at IS NULL THEN 'null' WHEN {A}>={e} THEN 'after' WHEN {A}>={w} THEN 'in' WHEN {A}={w-1} THEN 'a001' "
            f"WHEN {A}>{w-30} THEN 'a002' WHEN {A}>{w-180} THEN 'a030' WHEN {A}>{w-365} THEN 'a180' ELSE 'a365' END ra,"
            f"n.dispatched_at IS NOT NULL d,count(*) n FROM notices n WHERE n.fetch_id={f['id']} GROUP BY +n.profile,pb,ra,d")

if __name__ == '__main__' and sys.argv[1] == 'opta':
    mode, name, *ids = sys.argv[1:]
    F = fetches()
    sql = '\nUNION ALL\n'.join(opta_arm(F[int(i)]) for i in ids) + '\n'
    open(D + name + '.sql', 'w').write(sql)
    print(name, len(sql), 'bytes', len(ids), 'arms', F[int(ids[0])]['period'], '..', F[int(ids[-1])]['period'])

def opta2_arm(f):
    """opta_arm with the civil days computed once in a derived table (same buckets, shorter SQL)."""
    w, e = window(f)
    A = "max(p,ifnull(q,p))"
    def bk(x, pre):
        return (f"CASE WHEN p IS NULL THEN 'null' WHEN {x}>={e} THEN 'after' WHEN {x}>={w} THEN 'in' WHEN {x}={w-1} THEN '{pre}001' "
                f"WHEN {x}>{w-30} THEN '{pre}002' WHEN {x}>{w-180} THEN '{pre}030' WHEN {x}>{w-365} THEN '{pre}180' ELSE '{pre}365' END")
    return (f"SELECT {f['id']} f,profile,{bk('p','b')} pb,{bk(A,'a')} ra,q IS NOT NULL d,count(*) n FROM "
            f"(SELECT n.profile profile,{P2} p,{Q2} q FROM notices n WHERE n.fetch_id={f['id']}) GROUP BY profile,pb,ra,d")

if __name__ == '__main__' and sys.argv[1] == 'opta2':
    mode, name, *ids = sys.argv[1:]
    F = fetches()
    sql = '\nUNION ALL\n'.join(opta2_arm(F[int(i)]) for i in ids) + '\n'
    open(D + name + '.sql', 'w').write(sql)
    print(name, len(sql), 'bytes', len(ids), 'arms', F[int(ids[0])]['period'], '..', F[int(ids[-1])]['period'])

def src_arm(f):
    """Which field the stale published_at came from, for every profile but sdk-0.1 (which carries only the requested date):
    hp = a stamped publication date exists (OPP-012-notice / DE1-Publication-PublicationDate, ranked above the requested date),
    rb = where the requested date (BT-738-notice / DE1-RequestedPublicationDate) falls against the window."""
    w, e = window(f)
    def bk(x, pre):
        return (f"CASE WHEN {x} IS NULL THEN 'none' WHEN {x}>={e} THEN 'after' WHEN {x}>={w} THEN 'in' WHEN {x}={w-1} THEN '{pre}001' "
                f"WHEN {x}>{w-30} THEN '{pre}002' ELSE '{pre}030' END")
    civ = "(d.utc_seconds+d.offset_minutes*60)/86400"
    return (f"SELECT {f['id']} f,profile,{bk('p','b')} pb,hp,{bk('rq','r')} rb,count(*) n FROM "
            f"(SELECT n.profile profile,{P2} p,"
            f"EXISTS(SELECT 1 FROM notice_dates d WHERE d.notice_id=n.id AND d.field_id IN ('OPP-012-notice','DE1-Publication-PublicationDate')) hp,"
            f"(SELECT min({civ}) FROM notice_dates d WHERE d.notice_id=n.id AND d.field_id IN ('BT-738-notice','DE1-RequestedPublicationDate')) rq "
            f"FROM notices n WHERE n.fetch_id={f['id']} AND +n.profile<>'eforms:eforms-sdk-0.1' AND n.published_at IS NOT NULL AND {P2}<{w}) "
            f"GROUP BY profile,pb,hp,rb")

if __name__ == '__main__' and sys.argv[1] == 'src':
    mode, name, *ids = sys.argv[1:]
    F = fetches()
    sql = '\nUNION ALL\n'.join(src_arm(F[int(i)]) for i in ids) + '\n'
    open(D + name + '.sql', 'w').write(sql)
    print(name, len(sql), 'bytes', len(ids), 'arms', F[int(ids[0])]['period'], '..', F[int(ids[-1])]['period'])

def rel_arm(f):
    """All non-sdk-0.1 notices of a fetch: does a stamped publication date exist, and how does it relate to the
    requested date (same civil day / earlier / later), split by whether the served date is stale."""
    w, e = window(f)
    civ = "(d.utc_seconds+d.offset_minutes*60)/86400"
    return (f"SELECT {f['id']} f,profile,CASE WHEN p IS NULL THEN 'null' WHEN p>={e} THEN 'after' WHEN p>={w} THEN 'in' WHEN p={w-1} THEN 'b001' "
            f"WHEN p>{w-30} THEN 'b002' ELSE 'b030' END pb,"
            f"CASE WHEN pd IS NULL THEN 'nopub' WHEN rq IS NULL THEN 'noreq' WHEN pd=rq THEN 'eq' WHEN pd<rq-29 THEN 'pub<req-29' WHEN pd<rq THEN 'pub<req' ELSE 'pub>req' END rel,"
            f"count(*) n FROM (SELECT n.profile profile,{P2} p,"
            f"(SELECT min({civ}) FROM notice_dates d WHERE d.notice_id=n.id AND d.field_id IN ('OPP-012-notice','DE1-Publication-PublicationDate')) pd,"
            f"(SELECT min({civ}) FROM notice_dates d WHERE d.notice_id=n.id AND d.field_id IN ('BT-738-notice','DE1-RequestedPublicationDate')) rq "
            f"FROM notices n WHERE n.fetch_id={f['id']} AND +n.profile<>'eforms:eforms-sdk-0.1') GROUP BY profile,pb,rel")

if __name__ == '__main__' and sys.argv[1] == 'rel':
    mode, name, *ids = sys.argv[1:]
    F = fetches()
    sql = '\nUNION ALL\n'.join(rel_arm(F[int(i)]) for i in ids) + '\n'
    open(D + name + '.sql', 'w').write(sql)
    print(name, len(sql), 'bytes', len(ids), 'arms', [F[int(i)]['period'] for i in ids])
