#!/usr/bin/env python3
"""519 U5 S3 read of a child-cohort wet run (decision §5 S3): per folded body,
- each child loser answers 308;
- `?identifier=GBNHS<C>` (every child code a loser carried, and each zero-only tail) answers exactly the survivor,
  and `?identifier=GBNHS<P>` still does;
- the survivor's mentions now >= its own before + every loser's before;
- non-members (the rehome targets among them) keep their mention counts and stay holders;
- the fresh census lists no loser, the ledger grew by exactly the losers folded, and S4 stays at 0.

usage: s3check_children.py PRE_CENSUS WET_REPORT POST_CENSUS VERDICTS OUT
(census/report files are the job report bodies, as saved from /admin/reports/nhs-body-plan)."""
import json, subprocess, sys


def body(path):
    d = json.load(open(path))
    b = d.get('body', d) if isinstance(d, dict) else d
    return json.loads(b) if isinstance(b, str) else b


pre, wet, post = body(sys.argv[1]), body(sys.argv[2]), body(sys.argv[3])
verd = {v['key']: v for v in json.load(open(sys.argv[4]))['verdicts']}
out_path = sys.argv[5]
pre_c = {c['code']: c for c in pre['codes']}
post_c = {c['code']: c for c in post['codes']}


def box(cmd):
    return subprocess.run(['ssh', 'root@zebreus.click', cmd], capture_output=True, text=True).stdout


def sql(q):
    r = subprocess.run(['ssh', 'root@zebreus.click', '/root/sq.sh'], input=q.encode(), capture_output=True, check=True)
    return json.loads(r.stdout)['rows']


def lookup(identifier):
    j = box(f"curl -s 'http://127.0.0.1:8080/v1/organizations?identifier={identifier}&limit=10'")
    try:
        jj = json.loads(j)
        items = jj.get('data') or jj.get('organizations') or jj.get('items') or (jj if isinstance(jj, list) else [])
        return sorted(it.get('id') for it in items)
    except Exception:
        return ['unparsed:' + j[:120]]


fails, out = [], []
folded_losers = 0
for f in wet['folded']:
    code, keep, losers = f['code'], f['keep'], f['losers']
    folded_losers += len(losers)
    pc = pre_c[code]
    pre_h = {h['org_id']: h for h in pc['holders']}
    v = verd.get(code)
    if v is None:
        fails.append(f'{code}: folded without a verdict in this cohort')
        continue
    members = set(v['members'])
    if set([keep] + losers) != members:
        fails.append(f'{code}: folded {sorted([keep] + losers)} != verdict members {sorted(members)}')
    nonmembers = set(pre_h) - members
    http = {l: box(f"curl -s -o /dev/null -w '%{{http_code}}' http://127.0.0.1:8080/v1/organizations/{l}") for l in losers}
    bad = {l: c for l, c in http.items() if c != '308'}
    if bad:
        fails.append(f'{code}: losers not 308 {bad}')
    child_codes = sorted({c for l in losers for c in (pre_h.get(l, {}).get('child_of') or [])})
    ids = {c: lookup(f'GBNHS{c}') for c in child_codes + [code]}
    for c, got in ids.items():
        if got != [keep]:
            fails.append(f'{code}: ?identifier=GBNHS{c} answers {got}, survivor {keep}')
    ph = {h['org_id']: h for h in post_c.get(code, {}).get('holders', [])}
    if set(losers) & set(ph):
        fails.append(f'{code}: losers still holders {sorted(set(losers) & set(ph))}')
    for n in nonmembers:
        if n not in ph:
            fails.append(f'{code}: non-member {n} left the holders')
        elif ph[n]['mentions'] != pre_h[n]['mentions']:
            fails.append(f'{code}: non-member {n} mentions {pre_h[n]["mentions"]} -> {ph[n]["mentions"]}')
    now = sql(f'SELECT count(*) FROM organization_mentions WHERE organization_id = {keep}')[0][0]
    before = sum(pre_h[m]['mentions'] for m in members if m in pre_h)
    if now < before:
        fails.append(f'{code}: survivor {keep} has {now} mentions < members sum {before}')
    out.append({'code': code, 'keep': keep, 'target': f['target'], 'losers': losers, 'child_codes': child_codes,
                'nonmembers': sorted(nonmembers), 'mentions_before_sum': before, 'mentions_now': now,
                'id_lookup': ids, 'losers_http': http})
if post.get('ledger_rows') is not None and pre.get('ledger_rows') is not None:
    if post['ledger_rows'] != pre['ledger_rows'] + folded_losers:
        fails.append(f"ledger rows {pre['ledger_rows']} -> {post['ledger_rows']}, losers folded {folded_losers}")
if post.get('nhs_body_regrown'):
    halting = [r for r in post['nhs_body_regrown'] if r.get('kind') in ('name', 'ppon', 'identity')]
    if halting:
        fails.append(f'S4: {len(halting)} halting regrown entries')
json.dump({'checks': out, 'fails': fails}, open(out_path, 'w'), indent=1)
for o in out:
    print(o['code'], o['target'], 'keep', o['keep'], 'losers', o['losers'], 'codes', o['child_codes'],
          'mentions', o['mentions_before_sum'], '->', o['mentions_now'])
print('FAILS', len(fails))
for x in fails:
    print(' ', x)
