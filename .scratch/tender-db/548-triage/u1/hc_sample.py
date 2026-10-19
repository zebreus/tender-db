# 548 U1: head_current dated 30-sample (salt '548-u1-hc-', sha256 ascending) + arm sizes.
import json, gzip, hashlib, datetime, collections
D = '/home/user/tender-db/.scratch/tender-db/548-triage/'
b = json.load(gzip.open(D + 'org-name-refresh-plan-2026-10-16.json.gz'))['body']; b = json.loads(b) if isinstance(b, str) else b
as_day = b['as_of'] // 86400
day = lambda s: None if s is None else datetime.date.fromisoformat(s).toordinal() - datetime.date(1970,1,1).toordinal()
hc = [r for r in b['held'] if r['reason'] == 'head_current']
def stale(r):
    tn = day(r['top_newest'])
    return r['clusters'] > 1 + (r['head'] > 0) and (tn is None or tn < as_day - 365)
c = collections.Counter(('top_stale_next' if stale(r) else 'ships_if_released', 'H<=1' if r['head'] <= 1 else 'H>=2', 'withheld:' + str(r['release_withheld'])) for r in hc)
for k, v in sorted(c.items()): print(k, v)
age = collections.Counter(('head<1y' if day(r['head_newest']) >= as_day-365 else 'head1-2y' if day(r['head_newest']) >= as_day-730 else 'head2-3y') for r in hc)
print(age)
key = lambda r: hashlib.sha256(('548-u1-hc-' + str(r['org_id'])).encode()).hexdigest()
samp = sorted(hc, key=key)[:30]
json.dump(samp, open(D + 'u1/hc_sample.json', 'w'), ensure_ascii=False, indent=1)
for r in samp:
    print(r['org_id'], r['identifier'], r['country'], 'H=%d top=%d V=%d cl=%d' % (r['head'], r['top'], r['voting'], r['clusters']), 'hn', r['head_newest'], 'tn', r['top_newest'], 'rn', r['rival_newest'], r['release_withheld'])
    print('   OLD:', r['old_name'][:200]); print('   NEW:', (r['new_name'] or 'None')[:200]); print('   SP:', [(a[:90], n) for a, n in r['spellings'][:5]])
