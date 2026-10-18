"""Issue 551 B3: compare the Rust harness outputs with the port (rule_v2_b3.py) — decision parity on the 562 API ids
and the 4,745 records, the invariant "every v2 fire is a v1 fire", and projection parity (B1's real fold vs the
port's facts) on the 562 API ids.

    python3 -I compare.py RUN_DIR OUT_JSON
"""
import collections, json, os, sys

def rust_rows(p):
    out = {}
    for line in open(p):
        c = line.rstrip('\n').split('\t')
        nz = lambda v: None if v == '-' else v
        out[int(c[0])] = {'v1_fires': c[1] == '1', 'ship': c[2] == '1', 'first': nz(c[3]), 'S': None if c[4] == '-' else int(c[4]),
                          'cur': nz(c[5]), 'n': int(c[6]), 'vouched_by': None if c[7] == '-' else {'field': c[7], 'notice_id': int(c[8]), 'cents': int(c[9])},
                          'derived': None if c[10] == '-' else {'kind': c[10], 'cents': int(c[11]), 'currency': c[12], 'lots': int(c[13])},
                          'today_eur': nz(c[14]), 'prod_eur': nz(c[15]), 'prod_derivation': nz(c[16])}
    return out

def port_first(p):
    f = p['first']
    return f

def decide(port, rust, population_of):
    rows, dis = [], []
    stats = collections.Counter()
    for tid, p in sorted(port.items()):
        pop = population_of(p)
        r = rust.get(tid)
        if r is None:
            dis.append({'id': tid, 'population': pop, 'field': 'missing', 'port': p.get('first'), 'rust': None})
            stats[(pop, 'missing_in_rust')] += 1
            continue
        stats[(pop, 'compared')] += 1
        diffs = []
        if p['ship'] != r['ship']:
            diffs.append(('ship', p['ship'], r['ship']))
        if (p['first'] or None) != (r['first'] or None):
            diffs.append(('first', p['first'], r['first']))
        if p['S'] != r['S'] or (p['cur'] or None) != (r['cur'] or None) or (p['n'] or 0) != r['n']:
            diffs.append(('S/cur/n', [p['S'], p['cur'], p['n']], [r['S'], r['cur'], r['n']]))
        if p['ship'] and r['ship']:
            pv, rv = p.get('vouched_by'), r['vouched_by']
            if (pv or {}).get('field') != (rv or {}).get('field') or (pv or {}).get('notice_id') != (rv or {}).get('notice_id') or (pv or {}).get('cents') != (rv or {}).get('cents'):
                in_port_list = rv is not None and any(v[0] == rv['field'] and v[1] == rv['notice_id'] and v[2] == rv['cents'] for v in (p.get('vouches') or []))
                diffs.append(('vouched_by' + (' (name-level: Rust names a total the port also counts)' if in_port_list else ''), pv, rv))
            if r['derived'] is None or r['derived']['cents'] != p['S'] or r['derived']['currency'] != p['cur'] or r['derived']['lots'] != p['n']:
                diffs.append(('derived', [p['S'], p['cur'], p['n']], r['derived']))
        if p['v1_fires'] != r['v1_fires']:
            diffs.append(('v1_fires', p['v1_fires'], r['v1_fires']))
        # invariant: every v2 fire is a v1 fire
        if r['ship'] and not p['v1_fires']:
            diffs.append(('INVARIANT rust v2 ship not a port v1 fire', p['v1_why'], r['first']))
        if p['ship'] and not p['v1_fires']:
            diffs.append(('INVARIANT port v2 ship not a port v1 fire', p['v1_why'], None))
        if r['ship'] and not r['v1_fires']:
            diffs.append(('INVARIANT rust v2 ship not a rust v1 fire', None, r['first']))
        stats[(pop, 'port_ship')] += p['ship']
        stats[(pop, 'rust_ship')] += r['ship']
        stats[(pop, 'port_v1_fires')] += p['v1_fires']
        stats[(pop, 'rust_v1_fires')] += r['v1_fires']
        old = p.get('old')
        if old is not None:
            stats[(pop, 'm1era_v1_fires')] += bool(old.get('v1_fires'))
            if r['ship'] and not old.get('v1_fires'):
                diffs.append(('INVARIANT rust v2 ship not an M1-era (pre-drain) v1 fire', old, r['first']))
        for d in diffs:
            dis.append({'id': tid, 'population': pop, 'field': d[0], 'port': d[1], 'rust': d[2],
                        'port_row': {k: p.get(k) for k in ('ship', 'first', 'S', 'cur', 'n', 'v1_why', 'refusals', 'vouched_by', 'flags', 'remap', 'served_matches_election')},
                        'rust_row': r})
        stats[(pop, 'agree' if not diffs else 'disagree')] += 1
        rows.append({'id': tid, 'population': pop, 'port': [p['ship'], p['first'], p['S'], p['cur'], p['n']],
                     'rust': [r['ship'], r['first'], r['S'], r['cur'], r['n']], 'agree': not diffs})
    return rows, dis, stats

def projection(port, proj):
    T = collections.Counter()
    dis = []
    for tid, p in sorted(port.items()):
        pf = p.get('port_facts')
        q = proj.get(tid)
        if pf is None or q is None:
            T['missing'] += 1
            dis.append({'id': tid, 'what': 'missing', 'port': pf is not None, 'rust': q is not None})
            continue
        T['tenders'] += 1
        # versions: the fold's chain = the served versions' notices
        served_chain = [int(k) for k in p['port_lots_all_versions']]
        if q['versions'] != served_chain:
            dis.append({'id': tid, 'what': 'version chain', 'port': served_chain, 'rust': q['versions']})
        # LotState.published_lot_no (and the keys, kinds) at every version
        tid_ok = True
        for nid, plots in p['port_lots_all_versions'].items():
            rl = {k: [kind, no] for k, kind, no in q['lots_by_version'].get(nid, [])}
            T['lot_versions'] += 1
            keys_p, keys_r = set(plots), set(rl)
            if keys_p != keys_r:
                T['lot_versions_keyset_diff'] += 1
                tid_ok = False
                dis.append({'id': tid, 'what': 'LotState keys', 'version': nid, 'port_only': sorted(keys_p - keys_r), 'rust_only': sorted(keys_r - keys_p)})
            for k in sorted(keys_p & keys_r):
                T['lot_rows'] += 1
                if plots[k][1] != rl[k][1]:
                    T['lot_no_diff'] += 1
                    tid_ok = False
                    dis.append({'id': tid, 'what': 'LotState.published_lot_no', 'version': nid, 'key': k, 'port': plots[k][1], 'rust': rl[k][1]})
                if plots[k][0] != rl[k][0]:
                    T['lot_kind_diff'] += 1
                    dis.append({'id': tid, 'what': 'LotState.kind', 'version': nid, 'key': k, 'port': plots[k][0], 'rust': rl[k][0]})
        # head lot keys vs served lot_details
        head = str(served_chain[-1]) if served_chain else None
        rk = sorted(k for k, _, _ in q['lots_by_version'].get(head, [])) if head else []
        if rk != pf['head_lot_keys']:
            T['head_keys_diff'] += 1
            dis.append({'id': tid, 'what': 'head lot keys vs served lot_details', 'port': pf['head_lot_keys'], 'rust': rk})
        # LotResultState: lot_key, published_lot_no, group per (round notice, result key)
        pr = {(n, k): (lot, no, g) for n, k, lot, no, g in pf['results']}
        rr = {(x[0], x[1]): (x[2], x[3], x[4]) for x in q['results']}
        if set(pr) != set(rr):
            T['result_keyset_diff'] += 1
            dis.append({'id': tid, 'what': 'LotResult rows', 'port_only': sorted(map(list, set(pr) - set(rr))), 'rust_only': sorted(map(list, set(rr) - set(pr)))})
        for key in sorted(set(pr) & set(rr)):
            T['result_rows'] += 1
            for i, name in enumerate(('LotResultState.lot_key (served lot)', 'LotResultState.published_lot_no', 'LotResultState.group')):
                if pr[key][i] != rr[key][i]:
                    T[name + ' diff'] += 1
                    dis.append({'id': tid, 'what': name, 'result': list(key), 'port': pr[key][i], 'rust': rr[key][i]})
        # stated totals (multiset)
        canon = lambda t: (t[0], t[1], t[2], t[3], t[4], t[5], tuple(sorted(t[6] or [])))
        pt = collections.Counter(canon(t) for t in pf['totals'])
        rt = collections.Counter(canon(t) for t in q['totals'])
        T['total_rows_port'] += sum(pt.values())
        T['total_rows_rust'] += sum(rt.values())
        if pt != rt:
            T['totals_diff_tenders'] += 1
            # split: same total, different lots_awarded vs missing/extra
            strip = lambda c: collections.Counter({k[:6]: v for k, v in c.items()})
            dis.append({'id': tid, 'what': 'TenderVersion.stated_totals' + (' (lots_awarded only)' if strip(pt) == strip(rt) else ''),
                        'port_only': [list(k) for k in (pt - rt)], 'rust_only': [list(k) for k in (rt - pt)]})
    return T, dis

def main():
    run, out = sys.argv[1], sys.argv[2]
    api_port = {json.loads(l)['id']: json.loads(l) for l in open(os.path.join(run, 'api_port.jsonl'))}
    rec_port = {json.loads(l)['id']: json.loads(l) for l in open(os.path.join(run, 'rec_port.jsonl'))}
    api_rust = rust_rows(os.path.join(run, 'api_rust.tsv'))
    rec_rust = rust_rows(os.path.join(run, 'rec_rust.tsv'))
    def api_pop(p):
        pop = p['population']
        return 'labelled-279' if pop.startswith('labelled') else ('oos-6' if pop == 'oos' else 'walked-277')
    rows_a, dis_a, st_a = decide({k: v for k, v in api_port.items() if v['exported']}, api_rust, api_pop)
    rows_r, dis_r, st_r = decide(rec_port, rec_rust, lambda p: 'records-4745')
    proj = {json.loads(l)['id']: json.loads(l) for l in open(os.path.join(run, 'proj_rust.jsonl'))}
    T, dis_p = projection({k: v for k, v in api_port.items()}, proj)
    res = {'decision_stats': {f'{a}|{b}': v for (a, b), v in sorted({**st_a, **st_r}.items())},
           'decision_disagreements': dis_a + dis_r,
           'not_exported': [{'id': k, 'notes': v['notes']} for k, v in api_port.items() if not v['exported']],
           'projection_stats': dict(T), 'projection_disagreements': dis_p,
           'decision_rows': rows_a + rows_r}
    json.dump(res, open(out, 'w'), indent=1, default=str)
    print(json.dumps(res['decision_stats'], indent=1))
    print('decision disagreements', len(res['decision_disagreements']))
    for d in res['decision_disagreements'][:80]:
        print(' ', d['id'], d['population'], d['field'], 'port=', d['port'], 'rust=', d['rust'])
    print('not exported', res['not_exported'])
    print(json.dumps(res['projection_stats'], indent=1))
    print('projection disagreements', len(dis_p))
    c = collections.Counter(d['what'] for d in dis_p)
    print(c.most_common())

if __name__ == '__main__':
    main()
