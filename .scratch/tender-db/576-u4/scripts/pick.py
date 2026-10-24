"""pick.py -> 576-u4/picks.json. Deterministic stratified pick of 30 from the window rows (s01..s07).
Rank of a candidate = sha256('576-u4-pick-' + 'tender|seq|role|org|notice|section') ascending.
Caps over the whole sample: <= 2 items per tender, <= 1 per organization_id, <= 1 per touchpoint section (notice, TPO id).
Order: (1) NL AddInfo/TenderReceipt (role Lot-AddInfo|Lot-TenderReceipt, org country NL) -> 6 by rank, skipping ids in
EXCLUDE_NL (buyer turned out not NL; filled by the next in rank); (2) no own BT-514 on the TPO (tp_bt514 NULL) -> 3 by rank;
(3) Lot-ReviewOrg -> 6 by rank; (4) the rest: round-robin over the remaining roles (alphabetical; Lot-ReviewOrg last),
each turn taking the best-ranked candidate whose country is least used so far (ties by rank), until 30."""
import hashlib, json, os, sys
SQL = '/home/user/tender-db/.scratch/tender-db/576-u4/raw/sql'
WIN = {'s01_g0': ('g', 0), 's02_g1': ('g', 1), 's03_g2': ('g', 2), 's04_g3': ('g', 3), 's05_g4': ('g', 4), 's06_g5': ('g', 5), 's07_n6': ('n', 6)}
EXCLUDE_NL = set(json.loads(os.environ.get('EXCLUDE_NL', '[]')))
cands = []
for name, (kind, i) in WIN.items():
    d = json.load(open(os.path.join(SQL, name + '.out')))
    cols = d['columns']
    for k, r in enumerate(d['rows']):
        row = dict(zip(cols, r))
        key = f"{row['tender_id']}|{row['seq']}|{row['role']}|{row['org_id']}|{row['notice_id']}|{row['section_id']}"
        row['key'] = key
        row['rank'] = hashlib.sha256(('576-u4-pick-' + key).encode()).hexdigest()
        row['window'] = {'statement': name, 'kind': kind, 'i': i, 'row_index': k}
        cands.append(row)
# dedupe identical keys (a row seen in two windows): keep the first window
seen, pool = set(), []
for c in cands:
    if c['key'] not in seen:
        seen.add(c['key']); pool.append(c)
pool.sort(key=lambda c: c['rank'])
picked, per_tender, per_org, per_country, per_section = [], {}, {}, {}, {}

def ok(c):
    return (c['key'] not in {p['key'] for p in picked} and per_tender.get(c['tender_id'], 0) < 2
            and per_org.get(c['org_id'], 0) < 1 and per_section.get((c['notice_id'], c['section_id']), 0) < 1)

def take(c, stratum):
    c = dict(c); c['stratum'] = stratum
    picked.append(c)
    per_tender[c['tender_id']] = per_tender.get(c['tender_id'], 0) + 1
    per_org[c['org_id']] = per_org.get(c['org_id'], 0) + 1
    per_country[c['org_country']] = per_country.get(c['org_country'], 0) + 1
    per_section[(c['notice_id'], c['section_id'])] = 1

def fill(pred, n, stratum):
    got = 0
    for c in pool:
        if got >= n:
            break
        if pred(c) and ok(c):
            take(c, stratum); got += 1
    return got

counts = {}
counts['nl_addinfo_tenderreceipt'] = fill(lambda c: c['role'] in ('Lot-AddInfo', 'Lot-TenderReceipt') and c['org_country'] == 'NL'
                                          and c['key'] not in EXCLUDE_NL, 6, 'nl_addinfo_tenderreceipt')
counts['country_fallback'] = fill(lambda c: c['tp_bt514'] is None, 3, 'country_fallback')
counts['review_org'] = fill(lambda c: c['role'] == 'Lot-ReviewOrg', 6, 'review_org')
roles = sorted({c['role'] for c in pool if c['role'] != 'Lot-ReviewOrg'}) + ['Lot-ReviewOrg']
rest = 0
while len(picked) < 30:
    progressed = False
    for role in roles:
        if len(picked) >= 30:
            break
        opts = [c for c in pool if c['role'] == role and ok(c) and c['key'] not in EXCLUDE_NL]
        if not opts:
            continue
        best = min(opts, key=lambda c: (per_country.get(c['org_country'], 0), c['rank']))
        take(best, 'rest'); rest += 1; progressed = True
    if not progressed:
        break
counts['rest'] = rest
for n, p in enumerate(picked, 1):
    p['item'] = n
out = {'pool_size': len(pool), 'rows_total': len(cands), 'counts': counts, 'exclude_nl': sorted(EXCLUDE_NL), 'picks': picked}
json.dump(out, open('/home/user/tender-db/.scratch/tender-db/576-u4/picks.json', 'w'), indent=1, ensure_ascii=False)
print(json.dumps(counts), 'pool', len(pool))
for p in picked:
    print(p['item'], p['stratum'], p['tender_id'], p['seq'], p['role'], p['org_id'], p['org_country'], p['tp_bt514'], p['notice_id'], p['section_id'], p['org_name'][:60])
