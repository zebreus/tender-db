"""Adversarial recount of U2's parity and guard sets from decisions.jsonl.gz + served_lsa_rows.json (own code)."""
import collections, gzip, hashlib, json
U2 = '/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/u2'
ROOT = '/home/user/tender-db/.scratch/tender-db/551-lot-sum'
rows = {int(r[0]): r for r in json.load(open(ROOT + '/b5/investigate/served_lsa_rows.json'))}
ids = sorted(set(rows) - {179732})
recs = {}
with gzip.open(U2 + '/decisions.jsonl.gz', 'rt') as fh:
    for line in fh:
        r = json.loads(line); recs[r['id']] = r
print('recs', len(recs), 'missing', len(set(ids) - set(recs)), 'extra', len(set(recs) - set(ids)))
dis = collections.Counter(); bad = {}
first_eq = 0; member = 0
for t in ids:
    r = rows[t]; d = json.loads(r[7]); v2 = recs[t]['decisions']['v2']
    k = []
    if r[3] != 'lot-sum-awards': k.append('served-not-lsa')
    if not v2['ship']: k.append('ship')
    if v2['S'] != r[4] or type(v2['S']) is not int: k.append('S')
    if v2['currency'] != r[5]: k.append('cur')
    if v2['n'] != d.get('lots'): k.append('n')
    vb = d['vouched_by']; key = [vb['field'], vb['notice_id'], vb['cents']]
    if key in v2['vouches']: member += 1
    else: k.append('vb')
    if v2['vouched_by'] == vb: first_eq += 1
    # also: S equals sum of summed cents? and summed count == n?
    if k: bad[t] = k; dis.update(k)
print('parity: compared', len(ids), 'disagree', len(bad), dict(dis), 'vb member', member, 'vb == first', first_eq)
# sanity: summed cents sum vs S (in its currency), n vs len(summed)
mism = [t for t in ids if recs[t]['decisions']['v2']['n'] != len(recs[t]['summed'])]
print('n != len(summed):', len(mism), mism[:5])
mism2 = [t for t in ids if sum(c for _k, _n, c in recs[t]['summed']) != recs[t]['decisions']['v2']['S']]
print('S != sum(summed cents):', len(mism2), mism2[:5])
# value_eur column vs port S_eur
de = [(t, rows[t][6], recs[t]['S_eur']) for t in ids if rows[t][6] != recs[t]['S_eur']]
print('served value_eur != port S_eur:', len(de), de[:5])
# v1
print('v1_why counter', collections.Counter(recs[t]['v1_why'] for t in ids).most_common(5))
# guard sets
NAMES = ('v2', 'v2+single', 'v2+cover', 'v2+both')
ships = {nm: {t for t in ids if recs[t]['decisions'][nm]['ship']} for nm in NAMES}
for nm in NAMES: print(nm, len(ships[nm]), 'subset of v2', ships[nm] <= ships['v2'])
Rs = ships['v2'] - ships['v2+single']; Rc = ships['v2'] - ships['v2+cover']; Rb = ships['v2'] - ships['v2+both']
print('R_single', len(Rs), 'R_cover', len(Rc), 'R_both', len(Rb), 'Rc&Rs', len(Rc & Rs), 'Rc-Rs', sorted(Rc - Rs), 'Rb-Rs', sorted(Rb - Rs),
      'Rb == Rs|Rc', Rb == Rs | Rc, 'Rs-Rc', len(Rs - Rc))
print('both ships == single & cover ships', ships['v2+both'] == ships['v2+single'] & ships['v2+cover'])
# refused ids: first must be R10 and the only refusal R10
for nm in NAMES[1:]:
    ref = ships['v2'] - ships[nm]
    fr = collections.Counter((recs[t]['decisions'][nm]['first'], tuple(recs[t]['decisions'][nm]['refusals'])) for t in ref)
    print(nm, 'refused first/refusals', dict(fr))
# ships under guards keep S/cur/n of v2
for nm in NAMES[1:]:
    ch = [t for t in ships[nm] if any(recs[t]['decisions'][nm][k] != recs[t]['decisions']['v2'][k] for k in ('S', 'currency', 'n'))]
    mv = [t for t in ships[nm] if recs[t]['decisions'][nm]['vouched_by'] != recs[t]['decisions']['v2']['vouched_by']]
    # kept vouches subset of v2 vouches
    sub = [t for t in ships[nm] if not all(v in recs[t]['decisions']['v2']['vouches'] for v in recs[t]['decisions'][nm]['vouches'])]
    print(nm, 'S/cur/n changed', len(ch), 'vouched_by moved', len(mv), 'kept not subset', len(sub))
# compare with U2's files
Rsf = json.load(open(U2 + '/R_single.json')); Rxf = json.load(open(U2 + '/R_excess.json')); Pf = json.load(open(U2 + '/P.json'))
print('R_single file eq', sorted(Rs) == Rsf['ids'], 'R_excess file eq', sorted(Rc - Rs) == Rxf['ids'])
for nm in NAMES:
    print('P file', nm, Pf['guard_sets'][nm]['n'], sorted(e['id'] for e in Pf['guard_sets'][nm]['ships']) == sorted(ships[nm]))
print('6465510 in Rs', 6465510 in Rs, '6595330 in Rs', 6595330 in Rs, '6595330 in Rc', 6595330 in Rc, '5816379 ships both', 5816379 in ships['v2+both'])
# R_single removed-candidate shape: every removed vouch round_results == 1 ; kinds
kinds = collections.Counter()
for t in sorted(Rs):
    d = recs[t]['decisions']['v2+single']
    cands = {(c['field'], c['notice_id'], c['cents']): c for c in recs[t]['vouch_candidates']}
    for k in d['g_removed']:
        c = cands[tuple(k)]; kinds[(c['kind'], c['round_results'], c['field'])] += 1
    # all v2 vouches removed?
    assert not d['vouches'], t
print('R_single removed kinds', dict(kinds))
# R_cover candidates
for t in sorted(Rc):
    d = recs[t]['decisions']['v2+cover']; cands = {(c['field'], c['notice_id'], c['cents']): c for c in recs[t]['vouch_candidates']}
    print('Rc', t, 'inRs', t in Rs, [(cands[tuple(k)]['field'], cands[tuple(k)]['kind'], cands[tuple(k)]['round_results'], cands[tuple(k)]['names_summed_keys']) for k in d['g_removed']])
# candidate flags sanity: g_single == (round_results == 1), g_cover == kind in (award, range) and not names
badf = 0
for t in ids:
    for c in recs[t]['vouch_candidates'] or []:
        if c['g_single'] != (c['round_results'] == 1): badf += 1
        if c['g_cover'] != (c['kind'] in ('award', 'range') and not c['names_summed_keys']): badf += 1
        if c['field'] in ('TED-LOW', 'TED-HIGH') and c['kind'] != 'range': badf += 1
print('flag inconsistencies', badf)
print('kind counter of all candidates', collections.Counter(c['kind'] for t in ids for c in recs[t]['vouch_candidates'] or []))
print('v2 vouched_by kind', collections.Counter(next(c['kind'] for c in recs[t]['vouch_candidates'] if [c['field'], c['notice_id'], c['cents']] == [recs[t]['decisions']['v2']['vouched_by'][k] for k in ('field','notice_id','cents')]) for t in ids))
