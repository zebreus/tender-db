# Classifies the unit-5 second-half read (issue 562, 10-20 fold = project 2669) from out/*.out.
# Same classification as ../analyze.py, plus the D3/B split the 10-20 chain needs: rederive-eur-recent
# 2668 stamped 913 Tenders epoch-stale (each has a seq-less tender row in its D5 sub-range, d5_rows > 0),
# and the fold refolded them under compare On (16 verified, 897 corrected) -> ADR-0017 D3's acting set,
# which is counted under 'corrected', never under 'relabelled'. B (relabelled) is the seq-less rows on
# Tenders the fold did NOT refold stale (d5_rows == 0).
import json, collections, os, sys
D = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'out')
N = 11
fails = 0
def load(name):
    global fails
    rows = []
    for i in range(N):
        j = json.load(open(f'{D}/{name}_{i}.out'))
        if not ('rows' in j and 'columns' in j and 'row_count' in j) or j.get('truncated'):
            fails += 1; print('NOT AN ANSWER', name, i); continue
        lim = 1000 if name == 'seqless' else 2000
        assert j['row_count'] < lim, (name, i, 'hit LIMIT')
        rows += [dict(zip(j['columns'], r)) for r in j['rows']]
    return rows
seqless = load('seqless'); ted = load('ted')
print(f"responses that were not answers: {fails}")
sums = []
for i in range(12):
    j = json.load(open(f'{D}/sum_{i}.out'))
    assert 'rows' in j and 'columns' in j and not j.get('truncated')
    sums += [dict(zip(j['columns'], r)) for r in j['rows']]
PLO, PHI, FOLD_AT = 958853608, 958904203, 1792482087
proj = [r for r in sums if r['lo'] > PLO]
assert all(r['at'] == FOLD_AT for r in proj) and all(r['hi'] <= PLO for r in sums if r['at'] != FOLD_AT)
print(f"range rows {sum(r['n'] for r in sums)}; D5 sub-range rows {sum(r['n'] for r in sums if r['at'] != FOLD_AT)}; project sub-range rows {sum(r['n'] for r in proj)}")
sl_t = sum(r['n'] for r in proj if r['kind'] == 'tender' and r['seqless'] == 1 and r['op'] == 'changed')
sl_l = sum(r['n'] for r in proj if r['kind'] == 'lot' and r['seqless'] == 1 and r['op'] == 'changed')
print(f"project sub-range: seq-less tender changed {sl_t}, seq-less lot changed {sl_l}")
print(f"seq-less tender 'changed' rows read: {len(seqless)} (distinct tenders {len({r['tender_id'] for r in seqless})}); all changed_at {FOLD_AT}: {all(r['changed_at']==FOLD_AT for r in seqless)}")
assert len(seqless) == sl_t
print(f"TED-caused seq-carrying tender rows on Tenders with a DOE notice: {len(ted)} (distinct tenders {len({r['tender_id'] for r in ted})})")

for r in ted:
    r['pre'] = r['created_at'] < r['n_ingested_at']
    r['flip'] = bool(r['pre'] and r['doe_before'] and not r['ted_before'])
flips = [r for r in ted if r['flip']]
ft = collections.defaultdict(list)
for r in flips: ft[r['tender_id']].append(r)
print(f"\n(1) flip rows: {len(flips)}; distinct flipped tenders: {len(ft)}; prepended (seq=1): {sum(r['seq']==1 for r in flips)}")
print("    flip rows by op:", dict(collections.Counter(r['op'] for r in flips)))
print("    flip rows seq == current_seq:", sum(r['seq'] == r['current_seq'] for r in flips))
print("    tenders with >1 flip row:", {k: [(x['seq'], x['nid']) for x in v] for k, v in ft.items() if len(v) > 1})
print("    flipped tenders' current source:", dict(collections.Counter(v[0]['source'] for v in ft.values())))
print("    flipped tenders stale-refolded (D5-stamped, d5_rows>0):", sum(1 for v in ft.values() if v[0]['d5_rows'] > 0))
print("    TED nid range:", min(r['nid'] for r in flips) if flips else None, max(r['nid'] for r in flips) if flips else None,
      "; DOE first nid range:", min(r['doe_min_nid'] for r in flips) if flips else None, max(r['doe_min_nid'] for r in flips) if flips else None)
print("    created_at range:", min(r['created_at'] for r in flips) if flips else None, max(r['created_at'] for r in flips) if flips else None,
      "; TED ingested_at range:", min(r['n_ingested_at'] for r in flips) if flips else None, max(r['n_ingested_at'] for r in flips) if flips else None)

with_sl = [r for r in flips if r['seqless_cursor'] is not None]
print(f"\n(2) flip rows with a later same-changed_at seq-less tender 'changed' row: {len(with_sl)} of {len(flips)}; tenders {len({r['tender_id'] for r in with_sl})} of {len(ft)}")
if with_sl:
    gaps = [r['seqless_cursor'] - r['cursor'] for r in with_sl]
    print(f"    cursor gap {min(gaps)}..{max(gaps)}")
for r in flips:
    if r['seqless_cursor'] is None: print('    MISSING', r)

sl_by_t = {r['tender_id']: r for r in seqless}
assert len(sl_by_t) == len(seqless), 'a Tender with two seq-less rows'
d3 = [r for r in seqless if r['d5_rows'] > 0]
b = [r for r in seqless if r['d5_rows'] == 0]
print(f"\nseq-less tender rows on D5-stamped (stale, compare On -> D3) Tenders: {len(d3)}; on the others (B): {len(b)}")
print(f"    D3 rows with a paired seq row (versions appended past the prefix): {sum(r['paired_seq_rows']>0 for r in d3)}; without: {sum(r['paired_seq_rows']==0 for r in d3)}")
print(f"    D3 rows on flipped Tenders: {sum(r['tender_id'] in ft for r in d3)}")

tedjoin = collections.defaultdict(list)
for r in ted: tedjoin[r['tender_id']].append(r)
def cls_of(tid):
    rows = tedjoin.get(tid, [])
    if tid in ft: return 'flip'
    if not rows: return 'no TED join in this fold'
    if any(x['ted_before'] for x in rows): return 'TED-first'
    if all(not x['pre'] for x in rows): return 'co-minted' if any(x['doe_before'] for x in rows) else 'co-minted (TED lowest id)'
    return 'pre, no DOE of lower id'

print(f"\n(3) seq-less tender rows whose Tender is not a flip, by writer and class:")
c3 = collections.Counter((('D3' if s['d5_rows'] > 0 else 'B'), cls_of(s['tender_id'])) for s in seqless if s['tender_id'] not in ft)
for k, v in sorted(c3.items()): print('   ', k, v)
for s in seqless:
    if s['tender_id'] in ft or s['d5_rows'] > 0: continue
    print('    B NOT-FLIP', cls_of(s['tender_id']), s, tedjoin.get(s['tender_id']))
print("    D3 not-flip rows that are TED-first or co-minted (listed):")
for s in seqless:
    if s['tender_id'] in ft or s['d5_rows'] == 0: continue
    c = cls_of(s['tender_id'])
    if c in ('TED-first', 'co-minted', 'co-minted (TED lowest id)', 'pre, no DOE of lower id'):
        print('     ', c, {k: s[k] for k in ('tender_id','source','created_at','current_seq','paired_seq_rows','paired_min_seq','d5_rows')},
              [{k: x[k] for k in ('seq','op','nid','n_ingested_at','doe_before','ted_before','doe_min_nid','seqless_cursor')} for x in tedjoin[s['tender_id']]])

print("\nEvery Tender with a TED-caused row on a DOE chain, by class, D5-stamped, seq-less row present:")
tc = collections.Counter()
for tid, rows in tedjoin.items():
    tc[(cls_of(tid), rows[0]['d5_rows'] > 0, tid in sl_by_t)] += 1
for k, v in sorted(tc.items()): print('   ', k, v)
print("co-minted Tenders with a lower-id DOE notice (all, listed):")
for tid, rows in tedjoin.items():
    if cls_of(tid) == 'co-minted':
        print('   ', tid, 'seqless_row' if tid in sl_by_t else 'no seqless row', [{k: x[k] for k in ('seq','op','nid','n_ingested_at','created_at','doe_min_nid','d5_rows','source')} for x in rows])

print("\n(4) log cross-check")
print(f"    B seq-less tender rows (not D5-stamped): {len(b)}  vs relabelled 57")
print(f"    D3 seq-less tender rows (D5-stamped): {len(d3)}  vs compare corrected 897")
hl_b = sum(r['head_lots'] for r in b); hl_d3 = 4609 - 897
print(f"    B head lots at current_seq: {hl_b}; D3 lot rows from the job line (4609 - 897): {hl_d3}; sum {hl_b + hl_d3} vs seq-less lot changed rows {sl_l}")
print("    B rows with no paired seq row (unchanged-chain early return):", sum(r['paired_seq_rows'] == 0 for r in b))
print("    B rows: flips", sum(r['tender_id'] in ft for r in b), "; paired min seq:", dict(collections.Counter(r['paired_min_seq'] for r in b)))
print("    seq-less tender rows by current source/kind (B):", dict(collections.Counter((r['source'], r['kind']) for r in b)))
print("    seq-less tender rows by current source/kind (D3):", dict(collections.Counter((r['source'], r['kind']) for r in d3)))
print("    flipped Tenders' kind:", dict(collections.Counter(sl_by_t[t]['kind'] if t in sl_by_t else '?' for t in ft)))
