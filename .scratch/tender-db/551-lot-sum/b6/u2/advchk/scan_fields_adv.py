"""Which of the 3,502 name a notice carrying any of the given field ids (drain predicates of 541 / 571)?
python3 -I -B scan_fields_adv.py FIELD [FIELD ...]"""
import gzip, json, os, sys
sys.path[:0] = []
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'
ROOT = '/home/user/tender-db/.scratch/tender-db/551-lot-sum'
LAYERS = [(SP + '/b6/notices', '{}.json.gz'), (SP + '/b3/raw/notices', '{}.json'), (SP + '/m551/cache/notices', '{}.json')]
def load(n):
    for b, p in LAYERS:
        f = os.path.join(b, p.format(n))
        if os.path.exists(f):
            return json.load(gzip.open(f, 'rt', encoding='utf-8') if f.endswith('.gz') else open(f, encoding='utf-8'))
    raise SystemExit(f'miss {n}')
want = set(sys.argv[1:])
ids = sorted(int(r[0]) for r in json.load(open(ROOT + '/b5/investigate/served_lsa_rows.json')) if int(r[0]) != 179732)
hits = {}
for t in ids:
    x = json.load(gzip.open(f'{SP}/b6/tenders/{t}.json.gz', 'rt', encoding='utf-8'))
    ns = {v['caused_by_notice_id'] for v in x.get('versions', []) if v.get('caused_by_notice_id')}
    for k in ('lot_results', 'bids', 'contracts'):
        ns |= {r['notice_id'] for r in x.get(k) or [] if r.get('notice_id')}
    for n in sorted(ns):
        c = load(n)
        f = {v['field_id'] for s in c.get('sections', []) for v in s.get('values', [])}
        if f & want:
            hits.setdefault(t, []).append((n, sorted(f & want)))
print(len(hits), hits)
