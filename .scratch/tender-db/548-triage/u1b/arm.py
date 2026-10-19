# 548 U1b: top_stale's T1 release arm (hold only when rival_newest > top_newest; release otherwise) on the
# 2026-10-16 v12 held listing, sized against the 2,425 top_stale holds, then a salted 30 (salt '548-u1b-',
# sha256 ascending) excluding gate (d)'s 20 top_stale ids, 524-v11/skip-ids.tsv and the ids in 524-v12/rejudge-2569.
# Read-only. Output: u1b/arm.json (the arm), u1b/sample.json (the 30), stdout tables.
import json, gzip, hashlib, collections, re, glob, datetime
D = '/home/user/tender-db/.scratch/tender-db/'
T = D + '548-triage/'
b = json.load(gzip.open(T + 'org-name-refresh-plan-2026-10-16.json.gz'))['body']; b = json.loads(b) if isinstance(b, str) else b
as_day = b['as_of'] // 86400
day = lambda s: None if s is None else datetime.date.fromisoformat(s).toordinal() - datetime.date(1970, 1, 1).toordinal()
ts = [r for r in b['held'] if r['reason'] == 'top_stale']
assert len(ts) == 2425 == b['by_hold']['top_stale']['orgs'] and sum(r['n'] for r in ts) == b['by_hold']['top_stale']['mentions']
assert all(r['rule'] == 'R1' and r['recent_released'] is None and r['release_withheld'] is None for r in ts)
fresher = lambda r: r['rival_newest'] is not None and r['top_newest'] is not None and r['rival_newest'] > r['top_newest']
arm = [r for r in ts if not fresher(r)]
held = [r for r in ts if fresher(r)]
print('top_stale', len(ts), 'T1 release arm', len(arm), 'T1 still held', len(held))
print('arm mentions', sum(r['n'] for r in arm), 'H<=1', sum(r['head'] <= 1 for r in arm),
      'rival==top date', sum(r['rival_newest'] == r['top_newest'] for r in arm))
age = lambda r: as_day - day(r['top_newest'])
print('arm top age (y):', collections.Counter(min(age(r) // 365, 5) for r in arm))
print('arm head age (y):', collections.Counter(min((as_day - day(r['head_newest'])) // 365, 9) for r in arm))
print('arm newest_is_head:', collections.Counter(r['newest_is_head'] for r in arm))
gate = json.load(open(D + '524-v11/gates-2562/v11_gate_groups.json'))
gate_ids = {i['org_id'] for gr in gate['groups'] if gr['key'].startswith('hold:top_stale') for i in gr['items']}
assert len(gate_ids) == 20
skip = {int(l.split('\t')[0]) for l in open(D + '524-v11/skip-ids.tsv').read().splitlines()[1:] if l.strip()}
v12 = set()
for f in glob.glob(D + '524-v12/rejudge-2569/*.json'):
    v12 |= {int(m) for m in re.findall(r'"org_id": ?(\d+)', open(f).read())}
excl = gate_ids | skip | v12
print('excluded in arm: gate', sum(r['org_id'] in gate_ids for r in arm), 'skip', sum(r['org_id'] in skip for r in arm),
      'v12', sum(r['org_id'] in v12 for r in arm))
pool = [r for r in arm if r['org_id'] not in excl]
key = lambda r: hashlib.sha256(('548-u1b-' + str(r['org_id'])).encode()).hexdigest()
samp = sorted(pool, key=key)[:30]
print('pool', len(pool))
json.dump(arm, open(T + 'u1b/arm.json', 'w'), ensure_ascii=False, indent=0)
json.dump(samp, open(T + 'u1b/sample.json', 'w'), ensure_ascii=False, indent=1)
for r in samp:
    print(r['org_id'], r['identifier'], r['country'], 'H=%d top=%d V=%d cl=%d n=%d' % (r['head'], r['top'], r['voting'], r['clusters'], r['n']),
          'hn', r['head_newest'], 'tn', r['top_newest'], 'rn', r['rival_newest'])
    print('   OLD:', r['old_name'][:200]); print('   NEW:', (r['new_name'] or 'None')[:200])
    print('   SP:', [(a[:80], n) for a, n in r['spellings'][:6]])
