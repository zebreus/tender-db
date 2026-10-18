"""Band stage step 4 (offline): for every corrected band / path-V Tender, the combined port's per-block expectation
and cause vs the served lot now; which notices 570 remaps; and whether any lot-dependent quantity feeds the value."""
import collections, gzip, json, os, sys
sys.dont_write_bytecode = True
sys.path.insert(0, '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8')
import combined_port as C  # noqa: E402

RAW = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
plan = json.load(open(os.path.join(RAW, 'plan.json')))
diff = json.load(open(os.path.join(RAW, 'diff.json')))
base = json.load(open('/home/user/tender-db/.scratch/tender-db/568-r208-lots/drain/baseline-pre568u8-570.json'))
exp_all = json.load(open('/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8/expected.json'))
bundle = {}
with gzip.open('/home/user/tender-db/.scratch/tender-db/568-r208-lots/u7/u7-cache.jsonl.gz', 'rt') as fh:
    for line in fh:
        r = json.loads(line)
        if r['kind'] != 'tender':
            bundle[r['id']] = r['data']

out, tot = [], collections.Counter()
for sid, nmap in sorted(plan.items(), key=lambda kv: int(kv[0])):
    i = int(sid)
    p = os.path.join(RAW, 'pathv', f't{i}.json')
    t = json.load(open(p if os.path.exists(p) else os.path.join(RAW, 'tenders', f't{i}.json')))
    contents = {}
    for n, src in nmap.items():
        n = int(n)
        if src == 'bundle':
            contents[n] = bundle[n]
        elif src == 'fetched':
            contents[n] = json.load(open(os.path.join(RAW, 'notices', f'notices_{n}_content.json')))
        else:
            contents[n] = json.load(open(src))
    rows = C.attribute(t, contents)
    served = {(r['notice_id'], r['key']): r.get('lot') for r in t.get('lot_results') or []}
    breaks, causes, remaps = [], collections.Counter(), {}
    for k, row in rows.items():
        causes[row['cause']] += 1
        if k in served and served[k] != row['expected_lot'] and C.is_legacy(contents[k[0]]):
            breaks.append({'notice_id': k[0], 'key': k[1], 'served': served[k], 'expected': row['expected_lot'],
                           'reason': row['reason'], 'cause': row['cause']})
        if row['notice_remap'].startswith('remapped'):
            remaps[k[0]] = row['notice_remap']
    maps = C.remap_chain(C.chain_of(t), contents, True)
    for n, (target, how) in maps.items():
        moved = {a: b for a, b in target.items() if a != b}
        if moved:
            remaps[n] = 'remapped ' + ','.join(f'{a}->{b}' for a, b in sorted(moved.items()))
    not_computed = sorted(set(served) - set(rows))
    computed_not_served = sorted(set(rows) - set(served))
    moved_blocks = [{'notice_id': k[0], 'key': k[1], 'base': r['base_lot'], 'now': r['expected_lot'], 'served': served.get(k, '<absent>'),
                     'cause': r['cause'], 'reason': r['reason']} for k, r in sorted(rows.items()) if r['cause'] != 'unchanged']
    legacy = {n: C.is_legacy(c) for n, c in contents.items()}
    prior = exp_all.get(sid)
    rec = {'id': i, 'version': t['version'], 'value': t.get('value'), 'derivation': t.get('value_derivation'),
           'in_band': sid in base['band'], 'path_v': sid in base['path_v'],
           'chain': C.chain_of(t), 'legacy': legacy, 'blocks_served': len(served), 'blocks_computed': len(rows),
           'parity_breaks': breaks, 'causes': dict(causes), 'moved_blocks': moved_blocks, 'notice_remaps': remaps,
           'served_outside_chain': [list(k) for k in not_computed], 'computed_not_served': [list(k) for k in computed_not_served],
           'lot_keys': [l.get('lot_key') for l in t.get('lot_details') or []],
           'in_expected_json': prior is not None}
    out.append(rec)
    tot['tenders'] += 1
    tot['blocks'] += len(rows)
    tot['breaks'] += len(breaks)
    for c, v in causes.items():
        tot['cause_' + c] += v
    tot['with_moved_block'] += bool(moved_blocks)
    tot['with_570_remap'] += bool(remaps)
    tot['nonlegacy_chain'] += (not all(legacy.values()))
json.dump({'totals': dict(tot), 'tenders': out}, open(os.path.join(RAW, 'attrib.json'), 'w'), indent=1)
print(json.dumps(dict(tot), indent=1))
for r in out:
    print(r['id'], 'v', r['version'], 'band' if r['in_band'] else 'pathV', 'blocks', r['blocks_served'], '/', r['blocks_computed'],
          'causes', r['causes'], 'remaps', len(r['notice_remaps']), 'breaks', len(r['parity_breaks']),
          'outside', len(r['served_outside_chain']), 'cns', len(r['computed_not_served']),
          'legacy', sum(r['legacy'].values()), '/', len(r['legacy']))
