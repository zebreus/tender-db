# Classifies the unit-5 read (issue 562) from out/*.out. Prints the result.
import json, collections, os
D = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'out')
def load(name):
    rows = []
    for i in range(8):
        j = json.load(open(f'{D}/{name}_{i}.out'))
        assert 'rows' in j and 'columns' in j and not j.get('truncated'), (name, i)
        cols = j['columns']
        rows += [dict(zip(cols, r)) for r in j['rows']]
    return rows
seqless = load('seqless'); ted = load('ted')
print(f"seq-less tender 'changed' rows in range: {len(seqless)} (distinct tenders {len({r['tender_id'] for r in seqless})})")
print(f"TED-caused seq-carrying tender rows on Tenders with a DOE notice: {len(ted)} (distinct tenders {len({r['tender_id'] for r in ted})})")
for r in ted:
    r['pre'] = r['created_at'] < r['n_ingested_at']
    r['flip'] = bool(r['pre'] and r['doe_before'] and not r['ted_before'])
flips = [r for r in ted if r['flip']]
ft = collections.defaultdict(list)
for r in flips: ft[r['tender_id']].append(r)
print(f"\n(1) flip rows: {len(flips)}; distinct flipped tenders: {len(ft)}; prepended (seq=1): {sum(r['seq']==1 for r in flips)}")
print("    flip rows by op:", dict(collections.Counter(r['op'] for r in flips)))
print("    tenders with >1 flip row:", {k: [(x['seq'], x['nid']) for x in v] for k, v in ft.items() if len(v) > 1})
print("    flipped tenders' current source:", dict(collections.Counter(v[0]['source'] for v in ft.values())))
with_sl = [r for r in flips if r['seqless_cursor'] is not None]
print(f"\n(2) flip rows with a later same-changed_at seq-less tender 'changed' row: {len(with_sl)} of {len(flips)}; tenders {len({r['tender_id'] for r in with_sl})} of {len(ft)}")
missing = [r for r in flips if r['seqless_cursor'] is None]
for r in missing: print('    MISSING', r)
sl_by_t = {r['tender_id']: r for r in seqless}
assert len(sl_by_t) == len(seqless)
# (3) seq-less rows not explained by a flip
unexplained = [r for r in seqless if r['tender_id'] not in ft]
print(f"\n(3) seq-less tender rows whose Tender is not a flip: {len(unexplained)}")
tedjoin = collections.defaultdict(list)
for r in ted: tedjoin[r['tender_id']].append(r)
cls = collections.Counter()
for s in unexplained:
    rows = tedjoin.get(s['tender_id'], [])
    if not rows:
        cls['no TED join in this fold'] += 1; print('    NO-TED-JOIN', s); continue
    if any(x['ted_before'] for x in rows) and not any(x['doe_before'] and not x['ted_before'] for x in rows):
        cls['TED-first'] += 1; print('    TED-FIRST', s, rows); continue
    if all(not x['pre'] for x in rows):
        cls['co-minted'] += 1
        print('    CO-MINTED', {k: s[k] for k in ('tender_id','source','created_at','current_seq','paired_seq_rows','paired_min_seq')},
              [{k: x[k] for k in ('seq','op','nid','n_ingested_at','doe_before','ted_before','doe_min_nid')} for x in rows])
        continue
    cls['other'] += 1; print('    OTHER', s, rows)
print('    classes:', dict(cls))
# TED-joins in the fold that carry a seq-less row, by class (rows the 562 B path announced)
print("\nEvery Tender with a TED-caused row on a DOE chain, by class and seq-less row present:")
tc = collections.Counter()
for tid, rows in tedjoin.items():
    if tid in ft: c = 'flip'
    elif any(x['ted_before'] for x in rows): c = 'TED-first'
    elif all(not x['pre'] for x in rows): c = 'co-minted' if any(x['doe_before'] for x in rows) else 'co-minted (TED lowest id)'
    else: c = 'pre, no DOE of lower id'
    tc[(c, tid in sl_by_t)] += 1
for k, v in sorted(tc.items()): print('   ', k, v)
# lots
print("\nhead lots of the 102 relabelled tenders (current_seq):", sum(r['head_lots'] for r in seqless), "; seq-less lot 'changed' rows in range: see sum (156)")
print("seq-less rows with no paired seq-carrying row (unchanged-chain early return):", sum(r['paired_seq_rows'] == 0 for r in seqless))
print("seq-less tender rows by current source/kind:", dict(collections.Counter((r['source'], r['kind']) for r in seqless)))
# ordering: seq-less row after every paired seq row (pairing subquery counts those below its cursor)
