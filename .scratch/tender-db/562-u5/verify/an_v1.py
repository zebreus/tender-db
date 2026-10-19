import json, collections, sys
D = '/home/user/tender-db/.scratch/tender-db/562-u5'
sl = []
for i in range(8):
    j = json.load(open(f'{D}/out/seqless_{i}.out')); sl += [dict(zip(j['columns'], r)) for r in j['rows']]
by_anchor = {r['cursor']: r for r in sl}
j = json.load(open(f'{D}/verify/out/v1.out')); assert 'rows' in j and not j['truncated']
rows = [dict(zip(j['columns'], r)) for r in j['rows']]
g = collections.defaultdict(list)
for r in rows: g[r['anchor']].append(r)
ok = bad = 0; lots_owned = 0
for a, s in by_anchor.items():
    rs = sorted(g[a], key=lambda r: r['cursor'])
    k = s['head_lots']
    head = rs[0]
    cond = head['entity_kind'] == 'tender' and head['entity_id'] == s['tender_id'] and head['version_seq'] is None and head['op'] == 'changed'
    lots = rs[1:1 + k]
    cond &= len(lots) == k and all(l['entity_kind'] == 'lot' and l['version_seq'] is None and l['op'] == 'changed' and l['lot_tender'] == s['tender_id'] and l['changed_at'] == 1792395355 for l in lots)
    nxt = rs[1 + k] if len(rs) > 1 + k else None
    # the row after the set must not be another seq-less lot of the same tender
    cond &= not (nxt and nxt['entity_kind'] == 'lot' and nxt['version_seq'] is None and nxt['lot_tender'] == s['tender_id'])
    lots_owned += len(lots)
    if cond: ok += 1
    else:
        bad += 1; print('BAD', s['tender_id'], k, rs)
print(f"correction sets checked: {len(by_anchor)}; contiguous rule-T + exactly head_lots seq-less lot rows owned by the Tender: {ok}; bad: {bad}; lot rows owned: {lots_owned}")
print("rows after each set:", collections.Counter((r['entity_kind'], r['version_seq'] is None, r['op']) for a in by_anchor for r in sorted(g[a], key=lambda r: r['cursor'])[1 + by_anchor[a]['head_lots']:]))
