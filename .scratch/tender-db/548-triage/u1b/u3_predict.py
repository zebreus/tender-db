# 548 U2: U3's predicted by_hold moves for the adopted releases, from the 2026-10-16 held listing (v12).
# A release takes every matching row out of its reason; each row then lands where U1's cut() says
# (ship = an R1 change, head_current, top_stale, or an R2 change that wet runs exclude). Serve holds run after the
# bind guards and are not modelled, so the ship counts are upper bounds. Read-only.
import json, gzip, collections
T = '/home/user/tender-db/.scratch/tender-db/548-triage/'
ns = {}; exec(open(T + 'u1c/subset.py').read().split('# --- U1')[0], ns); subset = ns['subset']
src = open(T + 'u1c/subset.py').read()
b = json.load(gzip.open(T + 'org-name-refresh-plan-2026-10-16.json.gz'))['body']; b = json.loads(b) if isinstance(b, str) else b
import datetime
as_of = b['as_of']
day = lambda s: None if s is None else datetime.date.fromisoformat(s).toordinal() - datetime.date(1970, 1, 1).toordinal()
cur_cut, stale_cut = as_of // 86400 - 1095, as_of // 86400 - 365
def cut(r):  # u1/filter.py cut(), verbatim logic
    if r['rule'] not in ('R0e', 'R0n', 'R1'): return 'rule_' + r['rule']
    if r['recent_released'] == 'unfit': return 'released_unfit'
    if r['rule'] != 'R1': return None
    if r['recent_released'] is None:
        hn = day(r['head_newest'])
        if hn is not None and hn >= cur_cut: return 'head_current'
        tn = day(r['top_newest'])
        if r['clusters'] > 1 + (1 if r['head'] > 0 else 0) and (tn is None or tn < stale_cut): return 'top_stale'
    return None
sh = json.load(open(T + 'u1/shippable.json'))
H = b['held']
def dest(r):
    c = cut(r)
    return 'ship' if c is None else ('R2_change' if c == 'rule_R2' else c)
sets = {
 'gb_family_whole': [r for r in H if r['reason'] == 'gb_family'],
 'gb_family_H<=1': [r for r in H if r['reason'] == 'gb_family' and r['head'] <= 1],
 'family_H<=1': [r for r in H if r['reason'] == 'family' and r['head'] <= 1],
 'consortium_subset': [r for r in H if r['reason'] == 'consortium_removed' and subset(r['old_name'], r['new_name'])],
}
out = {}
for k, rows in sets.items():
    d = collections.Counter(dest(r) for r in rows)
    out[k] = dict(drop_orgs=-len(rows), drop_mentions=-sum(r['n'] for r in rows), rules=dict(collections.Counter(r['rule'] for r in rows)),
                  lands=dict(d), ship_ids=sorted(r['org_id'] for r in rows if dest(r) == 'ship'))
    print(k, {x: y for x, y in out[k].items() if x != 'ship_ids'}, 'ship', len(out[k]['ship_ids']))
for name, keys in (('adopted (gb H<=1)', ['gb_family_H<=1', 'family_H<=1', 'consortium_subset']),
                   ('alt (gb whole)', ['gb_family_whole', 'family_H<=1', 'consortium_subset'])):
    tot = collections.Counter()
    for k in keys:
        for x, y in out[k]['lands'].items(): tot[x] += y
    print(name, 'lands', dict(tot))
    out['combined_' + name] = dict(tot)
for k, rows in (('gb_family_whole', 'gb_family'), ('family_H<=1', 'family'), ('consortium_subset', 'consortium_removed')):
    assert set(out[k]['ship_ids']) <= {r['org_id'] for r in sh[rows]}
json.dump(out, open(T + 'u1b/u3_predict.json', 'w'), ensure_ascii=False, indent=1)
