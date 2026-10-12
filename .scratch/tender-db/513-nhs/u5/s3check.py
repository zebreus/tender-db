#!/usr/bin/env python3
"""513 U5 S3 read of the capped wet run: per folded body, the survivor, losers' 308, the
identifier lookup, the fresh census's holders, and mention conservation."""
import json, subprocess, sys
pre = json.loads(json.load(open('plan-2274-dry.json'))['body'])
wet = json.loads(json.load(open('plan-2275-wet.json'))['body'])
post = json.loads(json.load(open('plan-2276-dry-after20.json'))['body'])
verd = {v['key']: v for v in json.load(open('../u3/verdicts-nhs-body-2026-10-11.json'))['verdicts']} if True else {}
pre_c = {c['code']: c for c in pre['codes']}
post_c = {c['code']: c for c in post['codes']}
def box(cmd):
    return subprocess.run(['ssh', 'root@zebreus.click', cmd], capture_output=True, text=True).stdout
def sql(q):
    r = subprocess.run(['ssh', 'root@zebreus.click', '/root/sq.sh'], input=q.encode(), capture_output=True, check=True)
    return json.loads(r.stdout)['rows']
fails = []
out = []
for f in wet['folded']:
    code, keep, losers = f['code'], f['keep'], f['losers']
    pc = pre_c[code]
    members = set([keep] + losers)
    pre_h = {h['org_id']: h for h in pc['holders']}
    nonmembers = set(pre_h) - members
    v = verd.get(code, {})
    reviewed = set(v.get('reviewed_holders') or [])
    if reviewed and set(pre_h) & members != members:
        fails.append(f'{code}: member not a pre holder')
    if reviewed and members - reviewed:
        fails.append(f'{code}: folded members {sorted(members - reviewed)} not in reviewed_holders {sorted(reviewed)}')
    if f['target'] == 'ppon' and not pre_h.get(keep, {}).get('ppons'):
        fails.append(f'{code}: target ppon but survivor {keep} carries no PPON')
    if f['target'] != 'ppon' and any(pre_h[m].get('ppons') for m in members if m in pre_h):
        fails.append(f'{code}: a member carries a PPON but the target is {f["target"]}')
    # losers answer 308
    codes308 = {l: box(f"curl -s -o /dev/null -w '%{{http_code}}' http://127.0.0.1:8080/v1/organizations/{l}") for l in losers}
    bad = {l: c for l, c in codes308.items() if c != '308'}
    if bad:
        fails.append(f'{code}: losers not 308 {bad}')
    # identifier lookup
    j = box(f"curl -s 'http://127.0.0.1:8080/v1/organizations?identifier=GBNHS{code}'")
    try:
        jj = json.loads(j)
        items = jj.get('organizations') or jj.get('items') or jj.get('results') or (jj if isinstance(jj, list) else [])
        ids = [it.get('id') for it in items]
    except Exception:
        ids = ['unparsed:' + j[:120]]
    if ids != [keep]:
        fails.append(f'{code}: ?identifier=GBNHS{code} answers {ids}, survivor {keep}')
    # fresh census holders: survivor + non-members only
    ph = {h['org_id']: h for h in post_c.get(code, {}).get('holders', [])}
    if set(losers) & set(ph):
        fails.append(f'{code}: losers still holders {sorted(set(losers) & set(ph))}')
    if keep not in ph and pc.get('holders'):
        fails.append(f'{code}: survivor {keep} not a holder in the fresh census')
    for n in nonmembers:
        if n in ph and ph[n]['mentions'] != pre_h[n]['mentions']:
            fails.append(f'{code}: non-member {n} mentions {pre_h[n]["mentions"]} -> {ph[n]["mentions"]}')
        if n not in ph:
            fails.append(f'{code}: non-member {n} left the holders')
    # conservation: survivor mentions now vs members' sum before
    now = sql(f'SELECT count(*) FROM organization_mentions WHERE organization_id = {keep}')[0][0]
    before = sum(pre_h[m]['mentions'] for m in members if m in pre_h)
    if now < before:
        fails.append(f'{code}: survivor {keep} has {now} mentions < members sum {before}')
    out.append({'code': code, 'keep': keep, 'target': f['target'], 'losers': losers, 'nonmembers': sorted(nonmembers),
                'mentions_before_sum': before, 'mentions_now': now, 'id_lookup': ids, 'losers_http': codes308})
json.dump({'checks': out, 'fails': fails}, open('s3-read.json', 'w'), indent=1)
for o in out:
    print(o['code'], o['target'], 'keep', o['keep'], 'losers', len(o['losers']), 'nonmembers', len(o['nonmembers']), 'mentions', o['mentions_before_sum'], '->', o['mentions_now'])
print('FAILS', len(fails))
for x in fails:
    print(' ', x)
