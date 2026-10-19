# Issue 545 unit 2a — candidate listing over GB-PPON-keyed orgs.
#
# usage: python3 -I candidates.py <out_prefix> <slice.out.json>...
#
# Input: /v1/sql slice outputs, one row per (org, mention name, scheme):
#   id, identifier, name(head), country, mname, scheme, n, first_notice[, last_notice]
# (unit 1's q3_slice has no last_notice; then last_notice = first_notice when n == 1, else None).
#
# Two detectors, both scripted (the skeptic's point: the twice-held check was done by eye in unit 1):
#   STRAY  — the census rule (issue 524 unit 1, org-head-drift): an alt key held exactly once,
#            under a top key held >= 2 times, that agrees with no other key of the org.
#   HELD-k — the twice-held foreign-key check: an alt key held 2..K times (default K=2) that is
#            NOT the org's top key (its count is strictly below the top key's count) and agrees
#            with no other key. The stray rule misses these by construction (CEBR x2 on IFS).
# Then the PREFILTER (prefilter.py's rule, applied to both detectors): keep a candidate only when
# its distinctive tokens share nothing with the org's head and the names of every MORE-established
# key (count strictly greater than the candidate's own). Using only more-established keys keeps
# two foreign spellings held once each from knocking each other out.
#
# Outputs <out_prefix>.json (all detector hits, with keep flag) and <out_prefix>.txt (kept ones).
import json, os, re, sys
from collections import defaultdict
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from keys import alt, vote_key  # port of crosswalk::altid_name_key (r524judge/keys.py, copied here)

K = int(os.environ.get('HELD_K', '2'))
GENERIC = set('nhs trust foundation university council borough county city district authority services '
              'service limited ltd group uk the and of for institute company healthcare health care community '
              'national royal college school academy partnership centre center association development '
              'solutions systems management consulting consultants construction international europe holdings '
              'north south east west northern southern london wales cymru scotland england'.split())

def toks(s):
    return {t for t in re.findall(r'[a-z0-9]+', vote_key(s).lower()) if len(t) >= 3 and t not in GENERIC}

def agree(a, b):
    # approximation of altid_keys_agree: equal once legal-form (§) tokens are dropped
    va = ' '.join(t for t in a.split() if not t.startswith('§'))
    vb = ' '.join(t for t in b.split() if not t.startswith('§'))
    return va == vb and va != ''

def load(paths):
    orgs = {}
    for p in paths:
        j = json.load(open(p))
        assert 'rows' in j and 'columns' in j, p
        assert not j.get('truncated'), p + ' truncated'
        cols = j['columns']
        for row in j['rows']:
            r = dict(zip(cols, row))
            o = orgs.setdefault(r['id'], {'ident': r['identifier'], 'head': r['name'], 'country': r['country'],
                                          'spell': {}})
            if r['mname'] is None or not r['mname'].strip():
                o.setdefault('blank', 0); o['blank'] += r['n']; continue
            last = r.get('last_notice', r['first_notice'] if r['n'] == 1 else None)
            s = o['spell'].setdefault((r['mname'], r['scheme']), {'n': 0, 'first': None, 'last': None})
            s['n'] += r['n']
            s['first'] = r['first_notice'] if s['first'] is None else min(s['first'], r['first_notice'])
            s['last'] = last if s['last'] is None or last is None else max(s['last'], last)
    return orgs

def detect(orgs):
    out = []
    stats = defaultdict(int)
    for oid, o in orgs.items():
        stats['orgs'] += 1
        n = sum(s['n'] for s in o['spell'].values())
        if n < 2:
            continue
        stats['orgs_ge2'] += 1
        altc = defaultdict(int); spellings = defaultdict(list)
        for (sp, sch), s in o['spell'].items():
            k = alt(sp); altc[k] += s['n']; spellings[k].append((sp, sch, s))
        top = max(altc.values())
        if top < 2:
            continue
        for k, c in altc.items():
            if any(other != k and agree(k, other) for other in altc):
                continue
            if c == 1:
                det = 'STRAY'
            elif 2 <= c <= K and c < top:
                det = 'HELD-%d' % c
            else:
                continue
            pool = toks(o['head'])
            for k2, c2 in altc.items():
                if k2 != k and c2 > c:
                    for sp2, _, _ in spellings[k2]:
                        pool |= toks(sp2)
            for sp, sch, s in spellings[k]:
                st = toks(sp)
                keep = not (st & pool)  # an empty token set is kept (recall first)
                stats[det + (':kept' if keep else ':dropped')] += 1
                others = sorted(((s2['n'], sp2) for (sp2, _), s2 in o['spell'].items() if sp2 != sp), reverse=True)
                out.append({'org': oid, 'ident': o['ident'], 'head': o['head'], 'country': o['country'],
                            'org_mentions': n, 'detector': det, 'key_count': c, 'name': sp, 'scheme': sch,
                            'n': s['n'], 'first_notice': s['first'], 'last_notice': s['last'], 'keep': keep,
                            'tokens': sorted(st), 'others': others[:5]})
    return out, stats

if __name__ == '__main__':
    prefix, paths = sys.argv[1], sys.argv[2:]
    orgs = load(paths)
    rows, stats = detect(orgs)
    rows.sort(key=lambda r: (r['ident'] or '', r['org'], r['name']))
    json.dump(rows, open(prefix + '.json', 'w'), indent=1, ensure_ascii=False)
    print('K', K, dict(sorted(stats.items())))
    with open(prefix + '.txt', 'w') as f:
        for i, r in enumerate(x for x in rows if x['keep']):
            f.write(f"{i:3d} {r['detector']:7s} {r['org']} [{r['ident']}] head={r['head']!r} n={r['org_mentions']} | "
                    f"{r['name']!r} ({r['scheme']}, x{r['n']}, notices {r['first_notice']}..{r['last_notice']}) | "
                    f"others {r['others']}\n")
