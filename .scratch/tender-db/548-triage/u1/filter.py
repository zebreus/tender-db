# 548 U1: cut the held listing to the subset a release would ship (skeptic-corrected D2), then draw salted samples.
# Read-only; input = ../org-name-refresh-plan-2026-10-16.json.gz. Output: u1/shippable.json, u1/samples.json, stdout table.
import json, gzip, hashlib, datetime, collections, sys
D = '/home/user/tender-db/.scratch/tender-db/548-triage/'
o = json.load(gzip.open(D + 'org-name-refresh-plan-2026-10-16.json.gz'))
b = o['body']; b = json.loads(b) if isinstance(b, str) else b
as_of = b['as_of']
day = lambda s: None if s is None else int(datetime.date.fromisoformat(s).toordinal() - datetime.date(1970,1,1).toordinal())
cur_cut = as_of // 86400 - 1095   # head_current: head_newest >= as_of - 1095 d (day granularity; code is seconds)
stale_cut = as_of // 86400 - 365  # top_stale: top_newest < as_of - 365 d
BIND = ['family', 'gb_family', 'consortium_removed', 'nhs_check', 'placeholder_id']
SERVE = ['organ', 'code_added', 'address_added', 'seat_dropped', 'review_body', 'caps_variant', 'doubled', 'no_fit']
def cut(r):
    """None if a release of this hold would ship; else the later step that would re-hold / drop it."""
    if r['rule'] not in ('R0e', 'R0n', 'R1'): return 'rule_' + r['rule']
    if r['recent_released'] == 'unfit': return 'released_unfit'
    if r['rule'] != 'R1': return None
    if r['recent_released'] is None:
        hn = day(r['head_newest'])
        if hn is not None and hn >= cur_cut: return 'head_current'
    if r['recent_released'] is None:
        tn = day(r['top_newest'])
        if r['clusters'] > 1 + (1 if r['head'] > 0 else 0) and (tn is None or tn < stale_cut): return 'top_stale'
    return None
salt = '548-u1-'
key = lambda r: hashlib.sha256((salt + str(r['org_id'])).encode()).hexdigest()
out, samples = {}, {}
tab = []
for reason in BIND + SERVE:
    rows = [r for r in b['held'] if r['reason'] == reason]
    cuts = collections.Counter(cut(r) for r in rows)
    ship = sorted([r for r in rows if cut(r) is None], key=key)
    stray = [r for r in ship if r['head'] <= 1]
    out[reason] = ship
    samples[reason] = ship if len(ship) <= 10 else ship[:10]
    tab.append((reason, 'bind' if reason in BIND else 'serve', len(rows), len(ship), len(stray), dict(cuts)))
for t in tab: print(*t, sep='\t')
json.dump(out, open(D + 'u1/shippable.json', 'w'), ensure_ascii=False, indent=0)
json.dump(samples, open(D + 'u1/samples.json', 'w'), ensure_ascii=False, indent=1)
