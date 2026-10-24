"""summarize.py -- 594 U1 tables from the saved answers (no box access): org 2660's mentions by name class (s3) and its
party rows by role family x name class (s4a/s4b/s4c, every party row on 2660, all versions; s4a also counts the rows on
the tender's current version). Writes summary.json beside raw/."""
import json, sys, collections
U1 = '/home/user/tender-db/.scratch/tender-db/594-u1'
sys.path.insert(0, U1 + '/scripts')
from classify import klass

def rows(n):
    d = json.load(open(f'{U1}/raw/sql/{n}.out'))
    assert 'rows' in d and 'columns' in d and not d.get('truncated'), n
    return d['rows']

FAM = {
    'buyer': {'Procedure-Buyer', 'buyer'},
    'review': {'Lot-ReviewOrg', 'Lot-ReviewInfo', 'Lot-Mediator', 'Part-ReviewOrg', 'Part-ReviewInfo', 'Part-Mediator'},
    'bidder': {'Tenderer', 'winner'},
}
def fam(role):
    for f, s in FAM.items():
        if role in s:
            return f
    return 'buyer-shaped other'  # AddInfo, DocProvider, TenderReceipt, TenderEval, Signatory, Paying/Financing, SProvider, *Legis …

mentions = collections.Counter(); names = collections.defaultdict(set)
for name, _rid, _s, _cc, n, _f, _l in rows('s3_names2660'):
    k = klass(name or ''); mentions[k] += n; names[k].add(name)
party = collections.defaultdict(collections.Counter); cur = collections.Counter(); byrole = collections.defaultdict(collections.Counter)
for r in rows('s4a_buyer_roles'):
    role, name, n, t, c = r; k = klass(name or '')
    party[k][fam(role)] += n; cur[(role, k)] += c; byrole[role][k] += n
for nm in ('s4b_review_roles', 's4c_other_roles'):
    for role, name, n, t in rows(nm):
        k = klass(name or ''); party[k][fam(role)] += n; byrole[role][k] += n
total_rows = sum(sum(v.values()) for v in party.values())
out = {'mentions_total': sum(mentions.values()), 'party_rows_total': total_rows,
       'by_class': {k: {'names': len(names[k]), 'mentions': mentions[k], 'party_rows': dict(party[k])} for k in mentions},
       'buyer_current_rows': {f'{r}/{k}': v for (r, k), v in sorted(cur.items())},
       'by_role': {r: dict(c) for r, c in sorted(byrole.items(), key=lambda kv: -sum(kv[1].values()))}}
json.dump(out, open(f'{U1}/summary.json', 'w'), indent=1, ensure_ascii=False)
print(json.dumps(out, indent=1, ensure_ascii=False))
