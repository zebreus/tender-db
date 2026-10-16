import json, gzip, datetime, collections
S='/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/'
def labels():
    return json.load(open(S+'labels524.json'))
_plan=None
def plan():
    global _plan
    if _plan is None:
        p=json.load(gzip.open(S+'plan2543.json.gz'))
        b=p['body']
        if isinstance(b,str): b=json.loads(b)
        _plan=b
    return _plan
ASOF={'2523':'2026-10-14','2526':'2026-10-15','2527':'2026-10-16','2543':'2026-10-16','2555':'2026-10-16'}
def d(s): return datetime.date.fromisoformat(s)
PLAN_ASOF=datetime.datetime.utcfromtimestamp(1792132805).date()
def age(date_s, asof):
    if date_s is None: return None
    return (asof - d(date_s)).days
