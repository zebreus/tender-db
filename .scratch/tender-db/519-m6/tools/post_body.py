#!/usr/bin/env python3
"""519 M6: review.json + live.json -> POST /admin/identifier-verdicts body (spec §4.4 with Check fixes 2-4).

usage: post_body.py review.json live.json <cohort> verdicts.json

(The spec names the review rows m6/verdicts.json and the body m6/post-body.json; here the body is
verdicts.json, as the task asked, and the review rows are review.json.)
"""
import collections, json, re, sys

RANK = {'low': 0, 'medium': 1, 'high': 2}


def cut(s, n):
    """Cut to at most n BYTES of UTF-8 (admin.rs checks rationale.len() in bytes; Check fix 3)."""
    return s.encode()[:n].decode('utf-8', 'ignore')


verdicts_p, live_p, cohort, out = sys.argv[1:5]
live = {r[0]: r[1] for r in json.load(open(live_p))['orgs']}          # id -> identifier, as R1 read it
body, why = [], collections.Counter()
for v in json.load(open(verdicts_p)):
    if v['verdict'] not in ('wrong', 'right'):
        why['not posted: ' + v['verdict']] += 1
        continue
    lit = live.get(v['org'])
    if lit != v['identifier']:
        why['gone or re-keyed since R1'] += 1
        continue
    if len(lit) > 64:                                                   # admin.rs:474-479 (D8)
        why['identifier over the 64-char POST cap'] += 1
        continue
    c = v.get('correct_identifier') or None                             # Check fix 3: never ""
    if c:
        n = re.sub(r'[^A-Z0-9]', '', c.upper())
        if 'PPON' in n[:6]:
            c = 'GBPPON' + n.split('PPON', 1)[1]                        # crosswalk.rs:392 needs the prefix
        elif re.fullmatch(r'(GBCOH)?\d{6,8}', n):
            if v['route'] not in ('coh-value', 'company-coh'):          # Check fix 4: pad only a register-checked number
                sys.exit(f"org {v['org']}: an all-digit correction {c!r} on route {v['route']}: settle it by hand")
            c = n.removeprefix('GBCOH').zfill(8)                        # E1 only (D4)
        else:
            c = n.removeprefix('GBCOH').removeprefix('GBNHS')           # 2+6 register form, or an ODS code
        c = c or None
    if c and len(c) > 64:
        sys.exit(f"org {v['org']}: correct_identifier over 64 chars")
    conf = v['confidence']
    if not v.get('challenger_agrees'):                                  # Check fix 4 (minor): never upgrade low
        conf = conf if RANK[conf] <= RANK['medium'] else 'medium'
    if (v.get('expected') or {}).get('shape') == 'hold':
        conf = 'medium' if RANK[conf] >= RANK['medium'] else conf      # D3
    rationale = cut(f"[519 M6 {v['route']}/{v['confidence']}] {v['rationale']} | challenger: {v.get('challenge') or ''}", 4000)
    body.append({'org_id': v['org'], 'identifier': lit, 'verdict': v['verdict'],
                 'correct_identifier': c, 'rationale': rationale, 'confidence': conf})
    why[f"{v['verdict']}/{conf}/{'corrected' if c else 'bare'}"] += 1

# Check fix 2: the issue's Verify row must go up at high, or the org must be gone.
q = [b for b in body if b['org_id'] == 31601033]
if 31601033 in live and not (q and q[0]['correct_identifier'] == 'GBPPONPWQY9673JBYN' and q[0]['confidence'] == 'high'):
    sys.exit('QHL15E (31601033) is not in the body as GBPPONPWQY9673JBYN at high')
for b in body:                                                          # the handler's own refusals, checked here
    assert 0 < len(b['identifier']) <= 64 and len(b['rationale'].encode()) <= 4000
    assert b['verdict'] in ('wrong', 'related', 'right') and b['confidence'] in ('high', 'medium', 'low')
    assert b['correct_identifier'] is None or 0 < len(b['correct_identifier']) <= 64
assert 0 < len(body) <= 5000
print(dict(why), '| posted', len(body))
json.dump({'cohort': cohort, 'verdicts': body}, open(out, 'w'), indent=1, ensure_ascii=False)
