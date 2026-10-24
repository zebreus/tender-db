"""pick.py -> 586-u3/picks.json. Deterministic stratified picks of 30 minted (B) and 30 aliased (C) touchpoint parties.

B (salt 576-u5-, like 576 U4): pool = windows m1..m4 (m0, m5 empty), n6 (NL AddInfo/TenderReceipt), o8 (roles other than
ReviewOrg/ReviewInfo). Rank = sha256('576-u5-pick-' + 'tender|seq|role|org|notice|section').
  Caps: <= 2 per tender, <= 1 per organization_id, <= 1 per touchpoint section (notice, TPO).
  Strata in order: (1) NL AddInfo/TenderReceipt (org country NL) 6; (2) no own BT-514 on the TPO 3; (3) Lot-ReviewOrg 6;
  (4) rest: round-robin over the remaining roles (alphabetical, Lot-ReviewOrg last), each turn the least-used org country,
  ties by rank, until 30.
C (salt 586-u3-): pool = windows a0, a2, a3, a5, a6 (a1 empty) + the fixed SARGA read x540072, each row classified by
586-u1/final.py's decision(tp_name, parent_name); rows whose decision is 'norm' (pre-586 same-name alias) or not an alias
are excluded. Rank = sha256('586-u3-pick-' + 'tender|seq|role|org|notice|section|tpo').
  Caps: <= 2 per tender, <= 1 per organization_id, <= 1 per touchpoint section (notice, TPO).
  Strata: (0) SARGA 540072, 1 row by rank (the issue's exhibit); (1) tier A 6 by rank (roles round-robin);
  (2) tier C by sub-tier quota, each by rank with roles round-robin: C0 1, C3 1, C3r 1, C2 3, C4 4; (3) C1 fills to 30."""
import hashlib, json, os, sys, collections
HERE = '/home/user/tender-db/.scratch/tender-db/586-u3'
sys.path.insert(0, os.path.join(HERE, 'tools'))
import importlib.util
spec = importlib.util.spec_from_file_location('classify586', os.path.join(HERE, 'tools', 'classify.py'))
cl = importlib.util.module_from_spec(spec); spec.loader.exec_module(cl)

def rows(name):
    d = json.load(open(os.path.join(HERE, 'raw', 'sql', name + '.out')))
    return [dict(zip(d['columns'], r)) | {'window': {'statement': name, 'row_index': k}} for k, r in enumerate(d['rows'])]

class Picker:
    def __init__(self):
        self.picked, self.t, self.o, self.s, self.c = [], collections.Counter(), collections.Counter(), set(), collections.Counter()
    def ok(self, c):
        return (c['key'] not in {p['key'] for p in self.picked} and self.t[c['tender_id']] < 2 and self.o[c['org_id']] < 1
                and (c['notice_id'], c['tpo_section']) not in self.s)
    def take(self, c, stratum):
        c = dict(c); c['stratum'] = stratum; self.picked.append(c)
        self.t[c['tender_id']] += 1; self.o[c['org_id']] += 1; self.s.add((c['notice_id'], c['tpo_section']))
        self.c[c.get('org_country')] += 1
    def fill(self, pool, pred, n, stratum, rr_roles=False):
        got = 0
        if not rr_roles:
            for c in pool:
                if got >= n: break
                if pred(c) and self.ok(c): self.take(c, stratum); got += 1
            return got
        used = collections.Counter()
        while got < n:
            opts = [c for c in pool if pred(c) and self.ok(c)]
            if not opts: break
            best = min(opts, key=lambda c: (used[c['role']], c['rank']))
            self.take(best, stratum); used[best['role']] += 1; got += 1
        return got

def pick_b():
    pool, seen = [], set()
    for name in ['m1', 'm2', 'm3', 'm4', 'n6', 'o8']:
        for r in rows(name):
            key = f"{r['tender_id']}|{r['seq']}|{r['role']}|{r['org_id']}|{r['notice_id']}|{r['section_id']}"
            if key in seen: continue
            seen.add(key)
            r['key'] = key; r['tpo_section'] = r['section_id']
            r['rank'] = hashlib.sha256(('576-u5-pick-' + key).encode()).hexdigest()
            pool.append(r)
    pool.sort(key=lambda c: c['rank'])
    P = Picker(); counts = {}
    counts['nl_addinfo_tenderreceipt'] = P.fill(pool, lambda c: c['role'] in ('Lot-AddInfo', 'Lot-TenderReceipt') and c['org_country'] == 'NL', 6, 'nl_addinfo_tenderreceipt')
    counts['country_fallback'] = P.fill(pool, lambda c: c['tp_bt514'] is None, 3, 'country_fallback')
    counts['review_org'] = P.fill(pool, lambda c: c['role'] == 'Lot-ReviewOrg', 6, 'review_org')
    roles = sorted({c['role'] for c in pool if c['role'] != 'Lot-ReviewOrg'}) + ['Lot-ReviewOrg']
    rest = 0
    while len(P.picked) < 30:
        prog = False
        for role in roles:
            if len(P.picked) >= 30: break
            opts = [c for c in pool if c['role'] == role and P.ok(c)]
            if not opts: continue
            best = min(opts, key=lambda c: (P.c[c['org_country']], c['rank']))
            P.take(best, 'rest'); rest += 1; prog = True
        if not prog: break
    counts['rest'] = rest
    return pool, P.picked, counts

def pick_c():
    pool, seen = [], set()
    for name in ['a0', 'a2', 'a3', 'a5', 'a6', 'x540072']:
        for r in rows(name):
            key = f"{r['tender_id']}|{r['seq']}|{r['role']}|{r['org_id']}|{r['notice_id']}|{r['section_id']}|{r['tpo']}"
            if key in seen: continue
            seen.add(key)
            r['key'] = key; r['tpo_section'] = r['tpo']
            r['oracle'] = cl.decision(r['tp_name'], r['parent_name'])
            r['tier'] = 'A' if r['oracle'] == 'A' else ('C' if r['oracle'].startswith('C') else r['oracle'])
            r['rank'] = hashlib.sha256(('586-u3-pick-' + key).encode()).hexdigest()
            pool.append(r)
    pool.sort(key=lambda c: c['rank'])
    elig = [c for c in pool if c['tier'] in ('A', 'C')]
    P = Picker(); counts = {}
    counts['sarga_540072'] = P.fill(elig, lambda c: c['tender_id'] == 540072, 1, 'sarga_540072')
    counts['tier_a'] = P.fill(elig, lambda c: c['tier'] == 'A', 6, 'tier_a', rr_roles=True)
    for sub, n in [('C0', 1), ('C3', 1), ('C3r', 1), ('C2', 3), ('C4', 4)]:
        counts['tier_' + sub] = P.fill(elig, lambda c, sub=sub: c['oracle'] == sub, n, 'tier_' + sub, rr_roles=True)
    counts['tier_C1'] = P.fill(elig, lambda c: c['oracle'] == 'C1', 30 - len(P.picked), 'tier_C1', rr_roles=True)
    return pool, P.picked, counts

bpool, bp, bc = pick_b()
cpool, cp, cc = pick_c()
for n, p in enumerate(bp, 1): p['item'] = f'B{n:02d}'
for n, p in enumerate(cp, 1): p['item'] = f'C{n:02d}'
out = {'minted': {'pool_size': len(bpool), 'counts': bc, 'picks': bp},
       'aliased': {'pool_size': len(cpool), 'pool_by_oracle': dict(collections.Counter(c['oracle'] for c in cpool)), 'counts': cc, 'picks': cp}}
json.dump(out, open(os.path.join(HERE, 'picks.json'), 'w'), indent=1, ensure_ascii=False)
print('B', json.dumps(bc), 'pool', len(bpool))
for p in bp: print(p['item'], p['stratum'], p['tender_id'], p['seq'], p['role'], p['org_id'], p['org_country'], p['tp_bt514'], p['notice_id'], p['section_id'], p['org_name'][:60])
print('C', json.dumps(cc), 'pool', len(cpool), out['aliased']['pool_by_oracle'])
for p in cp: print(p['item'], p['stratum'], p['oracle'], p['tender_id'], p['seq'], p['role'], p['org_id'], p['notice_id'], p['section_id'], p['tpo'], '|', (p['tp_name'] or '')[:50], '||', (p['parent_name'] or '')[:50])
