# usage: python3 -I sample.py — builds R1's sample from q8_class.json, before any GET (decision.md "Read bars", R1):
# (a) the 10 exemplar tenders whose class party is current + the 3 v1-only exemplars (expected N);
# (b) 30 current class rows, excluding the 5 transferred mentions and the 13 exemplar tenders, one row per tender,
#     strata P@A 22 / A@P 8, each ordered by sha256('572-u2-' + tender_id + '/' + notice_id + '/' + section_id).
# Writes r1_sample.json (the picks plus the ordered replacement queue of each stratum).
import json, hashlib, os
from collections import defaultdict
D = os.path.dirname(os.path.abspath(__file__))
cls = json.load(open(D + '/q8_class.json'))
EX_CUR = [8641682, 8636304, 8677647, 8698064, 8655231, 8677040, 8639202, 8580124, 8596944, 8687157]
EX_V1 = [8654651, 8621643, 8613154]
EX_ALL = set(EX_CUR) | set(EX_V1)
def h(r): return hashlib.sha256(('572-u2-%d/%d/%s' % (r['tender_id'], r['notice_id'], r['section_id'])).encode()).hexdigest()
ex = {}
for t in EX_CUR + EX_V1:
    rs = [r for r in cls if r['tender_id'] == t]
    ex[t] = dict(kind='current' if t in EX_CUR else 'v1-only (expected N)', class_rows=rs)
elig = [dict(r, salt=h(r)) for r in cls if r['cur'] == 1 and not r['transferred'] and r['tender_id'] not in EX_ALL]
by = defaultdict(list)
for r in elig: by[r['direction']].append(r)
for k in by: by[k].sort(key=lambda r: r['salt'])
tenders_both = {r['tender_id'] for r in by['A@P']} & {r['tender_id'] for r in by['P@A']}
def dedupe(lst):
    seen = set(); out = []
    for r in lst:
        if r['tender_id'] in seen: continue
        seen.add(r['tender_id']); out.append(r)
    return out
ap = dedupe(by['A@P']); pa = dedupe(by['P@A'])
n_ap = min(8, len(ap)); n_pa = 30 - n_ap
pick_ap = ap[:n_ap]
taken = {r['tender_id'] for r in pick_ap}
pa = [r for r in pa if r['tender_id'] not in taken]
pick_pa = pa[:n_pa]
out = dict(salt='572-u2-', eligible_rows=len(elig), eligible_by_stratum={k: len(v) for k, v in by.items()},
           eligible_tenders_by_stratum={'A@P': len(ap), 'P@A': len(pa)}, tenders_in_both_strata=sorted(tenders_both),
           exemplars={str(k): v for k, v in ex.items()},
           pick={'A@P': pick_ap, 'P@A': pick_pa},
           replacement_queue={'A@P': ap[n_ap:], 'P@A': pa[n_pa:]})
json.dump(out, open(D + '/r1_sample.json', 'w'), indent=1, ensure_ascii=False)
print('eligible rows', len(elig), out['eligible_by_stratum'], 'tenders', out['eligible_tenders_by_stratum'], 'both', sorted(tenders_both))
for t, v in ex.items():
    print('EX', t, v['kind'], [(r['cur'], r['org'], r['role'], r['notice_id'], r['name']) for r in v['class_rows']])
for s in ('A@P', 'P@A'):
    print('==', s, len(out['pick'][s]), 'queue', len(out['replacement_queue'][s]))
    for r in out['pick'][s]: print('  ', r['salt'][:10], r['tender_id'], r['org'], r['role'], r['notice_id'], r['section_id'], repr(r['name']), '' if r['in_q4'] else 'NEW')
