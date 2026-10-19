# Issue 545 unit 2a-tail + 2b — proposed verdict per candidate, with u2a/verdicts.py's guards applied as code.
#
# usage: python3 -I verdicts2b.py      (writes verdicts2b.json + verdicts2b.txt; prints refcount needs)
#
# Reuses u2a/verdicts.py unchanged as a module (its id_kind, distinct_entities, near_identified, destination and
# its G2 hand list), loading its 2a inputs as it always does, and ADDS this unit's reads to its tables:
#   r10_sec.out.json   every mention of each F (org, name) key, ALL notices (not only first/last: a HELD-3..6 key
#                      has up to 6), with the org's party (j) and bid-party (k) rows bound to that (notice, section)
#   r11_dest.out.json  destination lookups (name_norm range seeks)
#   r12_refs.out.json  mention counts of tied destination rows (most-referenced tiebreak), when present
# One change from verdicts.judge: G1/side/unbound are evaluated over all of a key's notices (r10), so a
# collision on a middle notice of a HELD-k key is not missed. Everything else is verdicts.judge's logic.
import json, os, sys
from collections import defaultdict
D = os.path.dirname(os.path.abspath(__file__)) + '/'
sys.path.insert(0, D + '../u2a')
import verdicts as V  # noqa: E402  (module-level code loads the 2a reads into V.mentions/roles/dest_rows/refcount)

# ---- r10: all notices, sections, roles -------------------------------------------------------------
keys = {i: (o, n) for i, o, n in json.load(open(D + 'r10_keys.json'))}
sec = defaultdict(set)      # (org, name) -> {(notice, section, country)}
roles = defaultdict(set)    # (org, notice, section) -> {role}
tenders = defaultdict(set)  # (org, notice, section) -> {tender}
_, rows = V.load_rows(D + 'r10_sec.out.json')
for k, a, b, c, d, e, f, g, h in rows:
    t, org, i = k.split('|'); org = int(org); i = int(i)
    assert keys[i][0] == org and c == keys[i][1], (k, c)
    sec[keys[i]].add((a, b, d))
    if h is not None:
        roles[(org, a, b)].add('Tenderer' if h == 'tenderer' else (('bid:' + h) if t == 'k' else h))
        tenders[(org, a, b)].add(e)

# ---- r11 (+ r12): destinations ----------------------------------------------------------------------
_, rows = V.load_rows(D + 'r11_dest.out.json')
for q, i, country, ident, prov, name, nn in rows:
    V.dest_rows[i] = (country, ident, prov, name)
if os.path.exists(D + 'r12_refs.out.json'):
    _, rows = V.load_rows(D + 'r12_refs.out.json')
    for r in rows:
        assert r[0].startswith('rc|'), r
        V.refcount[int(r[0][3:])] = r[1]

# G2 additions (buyer-side F on a public buyer's own identifier), by (org, name); read from r10's roles
SHARED_PROCUREMENT_2B = {
 (30913851, 'London Borough of Hounslow'),        # on London Borough of Wandsworth, Procedure-Buyer x3
 (30917637, 'Create Partnership Trust '),         # on The Howard Partnership Trust (MAT on MAT), buyer
 (31560408, 'Wyre Forest District Council'),      # on Shropshire Fire and Rescue Service, buyer x3
 (31577532, 'The Bolton Impact Trust'),           # on Embark Multi Academy Trust, buyer x2
 (31577532, 'University of Essex'),               # on Embark Multi Academy Trust, buyer x2
}

# G3 refusals by hand: the destination rule picked a row although an identified row of the same body exists
# under ANOTHER head name, which near_identified cannot see (the skeptic's structural limit, 2a check).
REFUSE_DEST = {
 (30916932, 'PwC LLP'): 'provisional "PWC LLP" chosen, but identified "PricewaterhouseCoopers LLP" rows exist '
                        '(8881710 OC303525, 31609131 GBCOHOC309347, 16151538 OC3030525, 12448138 GB243823957)',
 (30975996, 'Kingston Upon Hull City Council'): 'vote-key match 31541038 is keyed GBCOH168524345 (a VAT-shaped '
                        'number under GBCOH); the council\'s PPON row is 30914698 "Hull City Council" GBPPONPRNP1874YZZR',
}

def judge(c):
    v = {k: c[k] for k in ('org', 'ident', 'head', 'name', 'scheme', 'n', 'detector', 'class')}
    v['source'] = c['source']
    v['key_count'] = c['key_count']
    cls = c['class']
    if cls in ('V', 'T', 'C'):
        v['verdict'] = 'NOT-545'; return v
    if cls in ('U', 'A', 'F?'):
        v['verdict'] = {'U': 'PARK-sub-unit/group/sister', 'A': 'PARK-agent-principal',
                        'F?': 'PARK-relation-unknown'}[cls]
        return v
    assert cls == 'F'
    ms = sorted(sec.get((c['org'], c['name']), set()))
    if len(ms) != c['n']:
        v['verdict'] = 'PARK-evidence-missing'; v['evidence_rows'] = len(ms); return v
    ev = []; collision = False; unbound = False; country = None
    for notice, section, mc in ms:
        country = country or mc
        rs = sorted(roles.get((c['org'], notice, section), set()))
        ev.append({'notice': notice, 'section': section, 'roles': rs,
                   'tenders': sorted(tenders.get((c['org'], notice, section), set()))})
        if len(rs) > 1: collision = True
        if not rs: unbound = True
    v['evidence'] = ev
    v['side'] = 'buyer' if any('Procedure-Buyer' in e['roles'] for e in ev) else (
        'supplier' if any('Tenderer' in e['roles'] for e in ev) else 'unbound')
    if collision:
        v['verdict'] = 'PARK-shared-section'
    elif (c['org'], c['name']) in SHARED_PROCUREMENT_2B:
        v['verdict'] = 'PARK-shared-procurement'
    d, why, pool, need = V.destination(c['name'], country or 'GB')
    # G4 (new): a destination-pool row carrying the HOLDER's own company number means the name is the holder's
    # (a former or trading name), not another organization: park, do not rehome.
    hk = V.id_kind(c['ident'])
    same = [i for i in pool if V.id_kind(V.dest_rows[i][1]) == hk and hk[1]]
    if same and 'verdict' not in v:
        v['verdict'] = 'PARK-holder-number-on-destination'; v['note'] = f'pool row(s) {same} carry the holder\'s number {c["ident"]}'
    if (c['org'], c['name']) in REFUSE_DEST and 'verdict' not in v:
        v['verdict'] = 'PARK-destination-ambiguous (identified row under another head name)'
        v['note'] = REFUSE_DEST[(c['org'], c['name'])]
    v['destination_rule'] = why
    v['destination_pool'] = [(i,) + tuple(V.dest_rows[i]) + ((V.refcount[i],) if i in V.refcount else ()) for i in pool]
    if need: v['needs_refcount'] = need
    hint = V.near_identified(c['name'], country or 'GB')
    if hint and d is None: v['near_identified_hint'] = [(i,) + tuple(V.dest_rows[i]) for i in hint]
    if 'verdict' in v:
        if d: v['would_be_destination'] = d
        return v
    if d is None:
        v['verdict'] = why; return v
    v['verdict'] = 'REHOME-HIGH'; v['destination'] = d
    v['destination_row'] = V.dest_rows[d]
    if unbound: v['note'] = 'no party row binds this mention: the move changes the org\'s names only'
    return v

if __name__ == '__main__':
    from collections import Counter
    sys.path.insert(0, D)
    from keys import alt
    cands = [dict(r, source='2a-tail') for r in json.load(open(D + 'tail_judged.json'))] + \
            [dict(r, source='2b') for r in json.load(open(D + 'cand2b_judged.json'))]
    out = [judge(c) for c in cands]
    json.dump(out, open(D + 'verdicts2b.json', 'w'), indent=1, ensure_ascii=False, default=list)
    need = sorted({i for v in out for i in v.get('needs_refcount', [])})
    print('NEEDS REFCOUNT', len(need), need)
    for src in ('2a-tail', '2b'):
        vs = [v for v in out if v['source'] == src]
        rows_c = Counter(v['verdict'] for v in vs)
        seen = {}
        for v in vs: seen.setdefault((v['org'], alt(v['name'])), v)
        keys_c = Counter(v['verdict'] for v in seen.values())
        men = Counter()
        for v in vs: men[v['verdict']] += v['n']
        print(f'{src}: {len(vs)} rows, {len(seen)} keys')
        for k in sorted(rows_c): print(f'   {k:58s} rows {rows_c[k]:4d}  keys {keys_c[k]:4d}  mentions {men[k]:4d}')
    with open(D + 'verdicts2b.txt', 'w') as f:
        for v in out:
            if v['verdict'] == 'NOT-545': continue
            f.write(f"{v['verdict']:28s} [{v['class']:2s}] {v['source']:7s} {v['org']} {v['head'][:38]!r} <- {v['name']!r} x{v['n']}"
                    f" ({v['detector']}) side={v.get('side', '-')} dest={v.get('destination', v.get('would_be_destination', '-'))}"
                    f" rule={v.get('destination_rule', '-')}\n")
            for e in v.get('evidence', []):
                f.write(f"      evidence {e}\n")
            for p in v.get('destination_pool', []):
                f.write(f"      pool {p}\n")
            for p in v.get('near_identified_hint', []):
                f.write(f"      hint {p}\n")
            if v.get('note'): f.write(f"      note {v['note']}\n")
