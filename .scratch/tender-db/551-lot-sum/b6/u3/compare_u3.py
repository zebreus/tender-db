"""Issue 551 B6 U3: compare the Rust twins (awards_path_check_guards / head_value_with_awards_path_guards under
R10Guards NONE / SINGLE / COVER / BOTH) with the port of record (b6/rule_v2_g.py decide under frozenset() /
{single} / {cover} / {single, cover}) per id and guard set, plus projection parity on the 3,502 (B3's
compare.projection, reused unmodified) and the port-side cross-checks.

    python3 -I -B compare_u3.py IN_DIR OUT_DIR

Reads IN_DIR/{pop}_port.jsonl (export_u3.py) and OUT_DIR/{pop}_rust.tsv (the store harness), OUT_DIR/f3502_proj_rust.jsonl
(the ingest harness). Writes OUT_DIR/u3-compare.json. Exit 0 iff 0 disagreements, every invariant holds and projection
parity is clean on the fields the guards read (or every projection difference is one B3 already recorded).

Per (id, guard set) a disagreement is any of: ship/refuse; the first refusing check (R10 for a guard's refusal); S,
currency, n; on a ship, Rust's vouched_by not a MEMBER of the port's kept vouch list (`vouches`); on a Rust ship, the
derived value not (S, currency, n); v1 fires; the head_value twin not equal to the check twin's derivation. A Rust
vouched_by that is a member but not the port's FIRST kept vouch is listed as name-level (canonical.rs's recorded
difference: chain order then read order vs the port's per-field grouping), not counted as a disagreement.
"""
import collections
import gzip
import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
B6 = os.path.dirname(HERE)
ROOT = os.path.dirname(B6)
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'
POPS = ('b3api', 'b3rec', 'c563', 's61', 'f3502')
GUARD = {'none': 'v2', 'single': 'v2+single', 'cover': 'v2+cover', 'both': 'v2+both'}
NAMES = ('v2', 'v2+single', 'v2+cover', 'v2+both')


def nz(v):
    return None if v == '-' else v


def rust_rows(p):
    out = collections.defaultdict(dict)
    for line in open(p):
        c = line.rstrip('\n').split('\t')
        assert len(c) == 27, (p, len(c), c[:3])
        tid, g = int(c[0]), GUARD[c[1]]
        vb = None if c[8] == '-' else [c[8], int(c[9]), int(c[10])]
        hvb = None if c[21] == '-' else [c[21], int(c[22]), int(c[23])]
        out[tid][g] = {
            'v1_fires': c[2] == '1', 'ship': c[3] == '1', 'first': nz(c[4]), 'S': None if c[5] == '-' else int(c[5]),
            'currency': nz(c[6]), 'n': int(c[7]), 'vouched_by': vb,
            'derived': None if c[11] == '-' else {'kind': c[11], 'cents': int(c[12]), 'currency': c[13], 'lots': int(c[14])},
            'today_eur': nz(c[15]), 'hv_eur': nz(c[16]), 'hv_derivation': nz(c[17]),
            'hv': None if c[18] == '-' else [int(c[18]), c[19], int(c[20])], 'hv_vouched_by': hvb,
            'hv_equals_check': c[24] == '1', 'production_eur': nz(c[25]), 'production_derivation': nz(c[26]),
        }
    return out


def port_rows(p):
    return {r['id']: r for r in (json.loads(l) for l in open(p))}


def vkey(vb):
    return None if vb is None else [vb['field'], vb['notice_id'], vb['cents']]


def compare_pop(pop, port, rust):
    stats = collections.Counter()
    dis, name_level, inv = [], [], []
    per_guard = {g: collections.Counter() for g in NAMES}
    moved = {g: {'port': [], 'rust': []} for g in NAMES}
    refused_by = {g: {'port': [], 'rust': []} for g in NAMES}
    for tid, p in sorted(port.items()):
        if not p.get('exported', True):
            stats['not_exported'] += 1
            dis.append({'id': tid, 'guard': None, 'field': 'not exported to Rust', 'port': p.get('notes'), 'rust': None})
            continue
        r = rust.get(tid)
        if r is None or set(r) != set(NAMES):
            stats['missing_in_rust'] += 1
            dis.append({'id': tid, 'guard': None, 'field': 'missing in Rust output', 'port': None, 'rust': sorted(r or {})})
            continue
        stats['ids_compared'] += 1
        for g in NAMES:
            d, q = p['decisions'][g], r[g]
            c = per_guard[g]
            c['compared'] += 1
            diffs = []
            if d['ship'] != q['ship']:
                diffs.append(('ship', d['ship'], q['ship']))
            if (d['first'] or None) != (q['first'] or None):
                diffs.append(('first', d['first'], q['first']))
            if d['S'] != q['S'] or (d['currency'] or None) != (q['currency'] or None) or (d['n'] or 0) != q['n']:
                diffs.append(('S/currency/n', [d['S'], d['currency'], d['n']], [q['S'], q['currency'], q['n']]))
            if d['ship'] and q['ship']:
                pv, rv = vkey(d['vouched_by']), q['vouched_by']
                kept = [list(v) for v in d['vouches']]
                if rv not in kept:
                    diffs.append(('vouched_by (not a member of the port\'s kept vouches)', kept, rv))
                elif rv != pv:
                    name_level.append({'id': tid, 'guard': g, 'port_first': pv, 'rust': rv, 'port_kept': kept})
                    c['vouched_by_name_level'] += 1
                else:
                    c['vouched_by_first_equal'] += 1
            if q['ship'] and (q['derived'] is None or [q['derived']['cents'], q['derived']['currency'], q['derived']['lots']] != [d['S'], d['currency'], d['n']]):
                diffs.append(('derived', [d['S'], d['currency'], d['n']], q['derived']))
            if p['v1_fires'] != q['v1_fires']:
                diffs.append(('v1_fires', p['v1_fires'], q['v1_fires']))
            hv_ok = q['hv_equals_check'] and ((q['hv_derivation'] == 'awards') == q['ship']) and \
                (not q['ship'] or (q['hv_vouched_by'] == q['vouched_by'] and q['hv'] == [q['derived']['cents'], q['derived']['currency'], q['derived']['lots']]))
            if not hv_ok:
                diffs.append(('head_value twin != check twin', None, {k: q[k] for k in ('hv_derivation', 'hv', 'hv_vouched_by', 'hv_equals_check')}))
            c['port_ship'] += d['ship']
            c['rust_ship'] += q['ship']
            if diffs:
                c['disagree'] += 1
                for f_, a, b in diffs:
                    dis.append({'id': tid, 'guard': g, 'field': f_, 'port': a, 'rust': b,
                                'port_row': {k: d[k] for k in ('ship', 'first', 'S', 'currency', 'n', 'vouched_by', 'vouches', 'refusals', 'g_removed')},
                                'port_v1_why': p['v1_why'], 'rust_row': q})
            else:
                c['agree'] += 1
        # invariants, both sides: guarded ships within v2's; a guard's refusal is R10; S/currency/n the same everywhere
        for side, get in (('port', lambda g: p['decisions'][g]), ('rust', lambda g: r[g])):
            v2 = get('v2')
            for g in NAMES[1:]:
                x = get(g)
                if x['ship'] and not v2['ship']:
                    inv.append({'id': tid, 'side': side, 'guard': g, 'what': 'guarded ship that v2 refuses'})
                if v2['ship'] and not x['ship']:
                    refused_by[g][side].append(tid)
                    if x['first'] != 'R10':
                        inv.append({'id': tid, 'side': side, 'guard': g, 'what': f'guard refusal named {x["first"]}, not R10'})
                if not v2['ship'] and (x['first'] or None) != (v2['first'] or None):
                    inv.append({'id': tid, 'side': side, 'guard': g, 'what': 'a v2 refusal changed its first check under a guard'})
                if [x['S'], x['currency'], x['n']] != [v2['S'], v2['currency'], v2['n']]:
                    inv.append({'id': tid, 'side': side, 'guard': g, 'what': 'S/currency/n moved under a guard'})
                if x['ship'] and v2['ship']:
                    a = vkey(x['vouched_by']) if side == 'port' else x['vouched_by']
                    b = vkey(v2['vouched_by']) if side == 'port' else v2['vouched_by']
                    if a != b:
                        moved[g][side].append(tid)
            # every v2 ship is a v1 fire
            if v2['ship'] and not (p['v1_fires'] if side == 'port' else v2['v1_fires']):
                inv.append({'id': tid, 'side': side, 'guard': 'v2', 'what': 'v2 ship not a v1 fire'})
    ids_dis = {e['id'] for e in dis}
    stats['ids_agree_all_guards'] = stats['ids_compared'] - len(ids_dis & set(rust))
    sets = {}
    for g in NAMES[1:]:
        sets[g] = {side: sorted(refused_by[g][side]) for side in ('port', 'rust')}
    r_single = {side: set(sets['v2+single'][side]) for side in ('port', 'rust')}
    r_cover = {side: set(sets['v2+cover'][side]) for side in ('port', 'rust')}
    return {
        'stats': dict(stats), 'per_guard': {g: dict(c) for g, c in per_guard.items()},
        'refused_by_guard': sets,
        'R_single': {s: sorted(v) for s, v in r_single.items()},
        'R_cover': {s: sorted(v) for s, v in r_cover.items()},
        'R_excess': {s: sorted(r_cover[s] - r_single[s]) for s in ('port', 'rust')},
        'refusal_sets_equal': all(sets[g]['port'] == sets[g]['rust'] for g in sets),
        'vouched_by_moved_under_guard': {g: moved[g] for g in NAMES[1:]},
        'served_aware_ids': sorted(tid for tid, p in port.items() if p.get('v1_served_aware')),
        'disagreements': dis, 'name_level': name_level, 'invariant_failures': inv,
    }


# ------------------------------------------------------------------------------------------------ cross-checks
def b3_api_check(port):
    """Port v2 (rule_v2_g on B3's reads) vs B3's recorded port outcomes (api_port.jsonl): the port inputs are B3's."""
    out = collections.Counter()
    diffs = []
    for l in open(os.path.join(SP, 'b3', 'run', 'api_port.jsonl')):
        b = json.loads(l)
        p = port.get(b['id'])
        d = p['decisions']['v2']
        same = (d['ship'], d['first'], d['S'], d['currency'], d['n'], d['vouched_by']) == \
            (b['ship'], b['first'], b['S'], b['cur'], b['n'], b['vouched_by'] if b['ship'] or b['vouched_by'] else None) \
            or ((d['ship'], d['first'], d['S'], d['currency'], d['n']) == (b['ship'], b['first'], b['S'], b['cur'], b['n']) and not d['ship'])
        out['same' if same else 'differs'] += 1
        if not same:
            diffs.append({'id': b['id'], 'u3': d, 'b3': {k: b[k] for k in ('ship', 'first', 'S', 'cur', 'n', 'vouched_by')}})
    return {'counts': dict(out), 'diffs': diffs}


def b3_rec_check(port):
    out = collections.Counter()
    diffs = []
    for l in open(os.path.join(SP, 'b3', 'run', 'rec_port.jsonl')):
        b = json.loads(l)
        d = port[b['id']]['decisions']['v2']
        same = (d['ship'], d['first'], d['S'], d['currency'], d['n']) == (b['ship'], b['first'], b['S'], b['cur'], b['n']) \
            and (not d['ship'] or d['vouched_by'] == b['vouched_by'])
        out['same' if same else 'differs'] += 1
        if not same:
            diffs.append({'id': b['id'], 'u3': d, 'b3': {k: b[k] for k in ('ship', 'first', 'S', 'cur', 'n', 'vouched_by')}})
    return {'counts': dict(out), 'diffs': diffs}


def b3_rust_drift(pop, rust, b3_tsv):
    """Rust v2 (guards NONE) today vs B3's recorded Rust output on the SAME input file (code drift since B3)."""
    old = {}
    for line in open(b3_tsv):
        c = line.rstrip('\n').split('\t')
        old[int(c[0])] = {'ship': c[2] == '1', 'first': nz(c[3]), 'S': nz(c[4]), 'cur': nz(c[5]), 'n': int(c[6]),
                          'vouched': None if c[7] == '-' else [c[7], int(c[8]), int(c[9])]}
    out = collections.Counter()
    diffs = []
    for tid, o in sorted(old.items()):
        q = rust[tid]['v2']
        now = {'ship': q['ship'], 'first': q['first'], 'S': None if q['S'] is None else str(q['S']), 'cur': q['currency'],
               'n': q['n'], 'vouched': q['vouched_by']}
        if now == o:
            out['same'] += 1
        else:
            out['differs'] += 1
            diffs.append({'id': tid, 'b3_rust': o, 'u3_rust_none': now})
    return {'population': pop, 'counts': dict(out), 'diffs': diffs}


def served_check(port, served_rows):
    """Port v2 vs the served lot-sum-awards rows (B5's snapshot, served_lsa_rows.json): ship, S, currency, n, and the
    served vouched_by a member of the port's kept vouches (U1's T4 / U2's v2 parity, here on this population's reads)."""
    out = collections.Counter()
    diffs = []
    for tid, p in sorted(port.items()):
        s = served_rows.get(tid)
        if s is None:
            out['not_served_lsa'] += 1
            continue
        d = p['decisions']['v2']
        vb = s.get('vouched_by')
        kv = None if vb is None else [vb.get('field'), vb.get('notice_id'), vb.get('cents')]
        same = d['ship'] and (d['S'], d['currency'], d['n']) == (s['cents'], s['currency'], s['lots']) and kv in [list(v) for v in d['vouches']]
        out['same' if same else 'differs'] += 1
        out['first_vouch_equal'] += bool(same and kv == vkey(d['vouched_by']))
        if not same:
            diffs.append({'id': tid, 'port_v2': d, 'served': s})
    return {'counts': dict(out), 'diffs': diffs}


def load_served_rows():
    """served_lsa_rows.json: [id, source, published, derivation, cents, currency, eur_cents, detail JSON] (B5's PK
    snapshot of the 3,503 corrected ids before the switch-off; selftest_g.py's `served_rows`)."""
    out = {}
    for r in json.load(open(os.path.join(ROOT, 'b5', 'investigate', 'served_lsa_rows.json'))):
        if r[3] != 'lot-sum-awards':
            continue
        detail = json.loads(r[7]) if r[7] else {}
        out[int(r[0])] = {'cents': r[4], 'currency': r[5], 'lots': detail.get('lots'), 'vouched_by': detail.get('vouched_by')}
    return out


def c563_b5_check(port):
    """Port v2 ship (served-aware) vs B5's cohort parity result 2 (rows [id, served_ship, port_ship, agree]; 563/563
    agree, 73 ships each side)."""
    res = json.load(open(os.path.join(ROOT, 'b5', 'cohort_parity_result2.json')))
    assert res['rows_legend'] == ['id', 'served_ship', 'port_ship', 'agree'], res['rows_legend']
    out = collections.Counter()
    diffs = []
    for tid, served_ship, port_ship, agree in res['rows']:
        d = port[int(tid)]['decisions']['v2']
        same = bool(d['ship']) == bool(port_ship) == bool(served_ship)
        out['same' if same else 'differs'] += 1
        out['ships'] += bool(d['ship'])
        if not same:
            diffs.append({'id': tid, 'u3_port_v2': d, 'b5': [served_ship, port_ship, agree]})
    return {'counts': dict(out), 'diffs': diffs, 'b5_counts': res['counts']}


def load_mod(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def main():
    ind, outd = sys.argv[1], sys.argv[2]
    res = {'populations': {}, 'cross_checks': {}}
    port_all, rust_all = {}, {}
    for pop in POPS:
        port = port_rows(os.path.join(ind, f'{pop}_port.jsonl'))
        rust = rust_rows(os.path.join(outd, f'{pop}_rust.tsv'))
        port_all[pop], rust_all[pop] = port, rust
        res['populations'][pop] = compare_pop(pop, port, rust)
        extra = set(rust) - set(port)
        res['populations'][pop]['rust_only_ids'] = sorted(extra)
    # the 61 inside the fresh layer (a sub-population of f3502, same reads as U1's T1-T7)
    ids61 = set(port_all['s61'])
    sub = compare_pop('f3502[61]', {k: v for k, v in port_all['f3502'].items() if k in ids61}, rust_all['f3502'])
    res['populations']['f3502[61]'] = sub
    # cross-checks of the port side
    res['cross_checks']['b3api_port_v2_vs_B3_api_port'] = b3_api_check(port_all['b3api'])
    res['cross_checks']['b3rec_port_v2_vs_B3_rec_port'] = b3_rec_check(port_all['b3rec'])
    # B3's own records encoding of the port side (before U3's withheld-encoding fix), for the record: not in the totals
    b3enc = port_rows(os.path.join(ind, 'b3rec_port_b3enc.jsonl'))
    res['populations_record_only'] = {'b3rec[B3 port encoding]': compare_pop('b3rec[B3 port encoding]', b3enc, rust_all['b3rec'])}
    res['cross_checks']['b3rec_b3enc_port_v2_vs_B3_rec_port'] = b3_rec_check(b3enc)
    moved = [t for t in port_all['b3rec'] if port_all['b3rec'][t]['decisions'] != b3enc[t]['decisions']]
    res['cross_checks']['b3rec_port_moved_by_the_withheld_encoding'] = {
        'counts': {'ids': len(moved)}, 'ids': sorted(moved),
        'detail': {t: {'b3enc': {g: [b3enc[t]['decisions'][g][k] for k in ('ship', 'first', 'S', 'currency', 'n')] for g in NAMES},
                       'api': {g: [port_all['b3rec'][t]['decisions'][g][k] for k in ('ship', 'first', 'S', 'currency', 'n')] for g in NAMES}}
                   for t in sorted(moved)}}
    res['cross_checks']['b3api_rust_none_vs_B3_api_rust'] = b3_rust_drift('b3api', rust_all['b3api'], os.path.join(SP, 'b3', 'run', 'api_rust.tsv'))
    res['cross_checks']['b3rec_rust_none_vs_B3_rec_rust'] = b3_rust_drift('b3rec', rust_all['b3rec'], os.path.join(SP, 'b3', 'run', 'rec_rust.tsv'))
    served = load_served_rows()
    res['cross_checks']['served_lsa_rows_loaded'] = len(served)
    res['cross_checks']['s61_port_v2_vs_served_lsa_rows'] = served_check(port_all['s61'], served)
    res['cross_checks']['f3502_port_v2_vs_served_lsa_rows'] = served_check(port_all['f3502'], served)
    res['cross_checks']['f3502_port_equals_U2'] = collections.Counter(str(r.get('u2_same')) for r in port_all['f3502'].values())
    res['cross_checks']['c563_port_v2_vs_B5_result2'] = c563_b5_check(port_all['c563'])
    # projection parity (B3's compare.projection, unmodified) on the 3,502 and on the 562 / 61 / 563. B3's function
    # reads the served chain from the key order of port_lots_all_versions; export_u3 writes rows with sort_keys, so the
    # rows are re-keyed in `served_chain` order first (U3 harness fix: without it 832 of the 3,502 report a 'version
    # chain' difference that is only the sorted key order).
    C = load_mod('b3_compare_u3', os.path.join(ROOT, 'b3', 'compare.py'))

    def chain_ordered(rows):
        out = {}
        for t, r in rows.items():
            r = dict(r)
            r['port_lots_all_versions'] = {str(n): r['port_lots_all_versions'][str(n)] for n in r['served_chain']}
            assert len(r['port_lots_all_versions']) == len(rows[t]['port_lots_all_versions']), t
            out[t] = r
        return out
    res['projection'] = {}
    for pop, rows in (('f3502', port_all['f3502']),
                      ('b3api', port_rows(os.path.join(ind, 'b3api_projport.jsonl'))),
                      ('s61', port_rows(os.path.join(ind, 's61_projport.jsonl'))),
                      ('c563', port_rows(os.path.join(ind, 'c563_projport.jsonl')))):
        proj = {json.loads(l)['id']: json.loads(l) for l in open(os.path.join(outd, f'{pop}_proj_rust.jsonl'))}
        T, dis_p = C.projection(chain_ordered(rows), proj)
        res['projection'][pop] = {'stats': dict(T), 'by_what': dict(collections.Counter(d['what'] for d in dis_p)),
                                  'disagreements': dis_p}
    # totals
    tot = collections.Counter()
    for pop in POPS:
        P = res['populations'][pop]
        tot['disagreements'] += len(P['disagreements'])
        tot['invariant_failures'] += len(P['invariant_failures'])
        tot['name_level'] += len(P['name_level'])
        for g in NAMES:
            tot['compared'] += P['per_guard'][g].get('compared', 0)
            tot['agree'] += P['per_guard'][g].get('agree', 0)
    res['totals'] = dict(tot)
    json.dump(res, open(os.path.join(outd, 'u3-compare.json'), 'w'), indent=1, default=str, sort_keys=True)
    # print the summary
    for pop in list(POPS) + ['f3502[61]']:
        P = res['populations'][pop]
        print(pop, json.dumps(P['stats']), 'served-aware', len(P['served_aware_ids']))
        for g in NAMES:
            print('   ', g, json.dumps(P['per_guard'][g], sort_keys=True))
        print('    refusal sets equal', P['refusal_sets_equal'], 'R_single', len(P['R_single']['port']), '/', len(P['R_single']['rust']),
              'R_cover', len(P['R_cover']['port']), '/', len(P['R_cover']['rust']), 'R_excess', P['R_excess']['port'], '/', P['R_excess']['rust'])
        print('    disagreements', len(P['disagreements']), 'invariant failures', len(P['invariant_failures']), 'name-level', len(P['name_level']))
        for d in P['disagreements'][:20]:
            print('     ', d['id'], d['guard'], d['field'], 'port=', d['port'], 'rust=', d['rust'])
    for k, v in res['cross_checks'].items():
        print('cross-check', k, json.dumps(v['counts'] if isinstance(v, dict) and 'counts' in v else v, default=str)[:400])
    for pop, P in res['projection'].items():
        print('projection', pop, json.dumps(P['stats'], sort_keys=True), 'by what', P['by_what'])
    for name, P in res['populations_record_only'].items():
        print('record only:', name, 'disagreements', len(P['disagreements']),
              sorted({(d['id'], d['field'], str(d['port']), str(d['rust'])) for d in P['disagreements']}))
    print('TOTALS', json.dumps(res['totals']))
    ok = tot['disagreements'] == 0 and tot['invariant_failures'] == 0
    sys.exit(0 if ok else 1)


if __name__ == '__main__':
    main()
