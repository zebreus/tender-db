# 548 U1b: votes per NOTICE instead of per mention, measured on the release sets (gb_family shippable 7,
# family shippable H<=1 22, consortium subset shippable 79) and on U1b's T1 sample 30.
# Inputs: sql/q03_pernotice_a.out.json, sql/q04_pernotice_b.out.json (org, notice, spelling, n from
# organization_mentions; bounded IN-list seeks on organization_mentions_org), the 2026-10-16 held listing.
# Clusters approximate crosswalk::head_vote_key with u1c's tokenizer minus family_token keys; an org counts as
# REPRODUCED when the per-mention clusters give the listing's top, head and voting exactly.
# Per-notice R1 bar: topN*20 >= votN*11 AND topN >= 3 AND topN >= 2*headN AND the elected cluster is the unique top.
import json, gzip, collections, sys
sys.path.insert(0, '/nonexistent')
T = '/home/user/tender-db/.scratch/tender-db/548-triage/'
src = open(T + 'u1c/subset.py').read().split('# --- U1')[0]
ns = {}; exec(src, ns)
tokens, FAM = ns['tokens'], set(ns['FAMILY_TOKEN_KEYS'])
def key(s):
    k = [t for t in tokens(s) if t not in FAM]
    return ' '.join(k) if k else ' '.join(tokens(s))
b = json.load(gzip.open(T + 'org-name-refresh-plan-2026-10-16.json.gz'))['body']; b = json.loads(b) if isinstance(b, str) else b
held = {r['org_id']: r for r in b['held']}
sh = json.load(open(T + 'u1/shippable.json')); sub = json.load(open(T + 'u1c/subset.json'))
shipc = {r['org_id'] for r in sh['consortium_removed']}
sets = {'gb_family_ship7': [r['org_id'] for r in sh['gb_family']],
        'family_H<=1_ship22': [r['org_id'] for r in sh['family'] if r['head'] <= 1],
        'consortium_subset_ship79': [r['org_id'] for r in sub['matched_full'] if r['org_id'] in shipc],
        't1_sample30': [r['org_id'] for r in json.load(open(T + 'u1b/sample.json'))]}
rows = collections.defaultdict(list)
for f in ('sql/q03_pernotice_a.out.json', 'sql/q04_pernotice_b.out.json'):
    j = json.load(open(T + 'u1b/' + f)); assert not j.get('truncated')
    for org, notice, sp, n in j['rows']: rows[org].append((notice, sp, n))
verd = {}
try:
    verd = {int(k): v['verdict'] for k, v in json.load(open(T + 'u1b/verdicts.json'))['verdicts'].items()}
except Exception: pass
u1 = json.load(open(T + 'u1/judgements.json')); u1c = json.load(open(T + 'u1c/verdicts.json'))
out = {}
for name, ids in sets.items():
    res = []
    for oid in ids:
        r = held[oid]; hk = key(r['old_name']); nk = key(r['new_name'])
        votes, notes = collections.Counter(), collections.defaultdict(set)
        for notice, sp, n in rows[oid]:
            k = key(sp); votes[k] += n; notes[k].add(notice)
        top_k = max(votes, key=lambda k: votes[k])
        rep = (votes[top_k] == r['top'] and votes.get(hk, 0) == r['head'] and sum(votes.values()) == r['voting'] and top_k == nk)
        allN = len({x for s in notes.values() for x in s})
        tN, hN = len(notes[nk]), len(notes.get(hk, ()))
        vN = sum(len(s) for s in notes.values())  # a notice votes once per cluster it prints
        uniq = all(len(s) < tN for k, s in notes.items() if k != nk)
        passes = tN * 20 >= vN * 11 and tN >= 3 and tN >= 2 * hN and uniq
        res.append(dict(org_id=oid, reproduced=rep, top=r['top'], head=r['head'], voting=r['voting'], topN=tN, headN=hN,
                        votN=vN, notices=allN, passes_per_notice=passes, old=r['old_name'][:90], new=r['new_name'][:90]))
    out[name] = res
    rp = [x for x in res if x['reproduced']]
    blk = [x for x in res if not x['passes_per_notice']]
    print(f"{name}: {len(res)} orgs, reproduced {len(rp)}; per-notice bar blocks {len(blk)} (of reproduced {sum(not x['passes_per_notice'] for x in rp)})")
    for x in blk:
        print(f"   {x['org_id']} rep={x['reproduced']} top {x['top']}/{x['voting']} H={x['head']} -> notices top {x['topN']}/{x['votN']} H={x['headN']} | {x['old'][:50]} -> {x['new'][:50]}")
# the U1 wrongs, for reference (q02: Barameda 20415415, CPR 18287356)
json.dump(out, open(T + 'u1b/pernotice.json', 'w'), ensure_ascii=False, indent=1)

# --- recorded verdicts (U1 judgements, U1c, U1b) for the rows the per-notice bar would block
rec = {}
for h in ('family', 'gb_family', 'consortium_removed'):
    for k, v in u1[h].get('verdicts', {}).items(): rec[int(k)] = v.split()[0].rstrip('?')
for k, v in u1c['verdicts'].items(): rec[int(k)] = v['v']
for k, v in json.load(open(T + 'u1b/verdicts.json'))['verdicts'].items(): rec[int(k)] = v['v']
summary = {}
for name, res in out.items():
    rp = [x for x in res if x['reproduced']]
    blk = [x for x in rp if not x['passes_per_notice']]
    summary[name] = dict(orgs=len(res), reproduced=len(rp), blocked=len(blk),
                         blocked_verdicts=dict(collections.Counter(rec.get(x['org_id'], 'unread') for x in blk)),
                         wrongs_in_set=[x['org_id'] for x in res if rec.get(x['org_id']) == 'W'],
                         wrongs_blocked=[x['org_id'] for x in blk if rec.get(x['org_id']) == 'W'])
    print(name, summary[name])
summary['u1_wrongs_q02'] = {'20415415 Barameda': 'top 7 mentions from 1 notice vs head 2 notices: blocked',
                            '18287356 CPR': 'top 3 mentions from 2 notices (< top_min 3): blocked'}
json.dump(dict(rows=out, summary=summary), open(T + 'u1b/pernotice.json', 'w'), ensure_ascii=False, indent=1)
