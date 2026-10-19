"""Issue 551 B6 U3: write u3-result.json from outputs/u3-compare.json, the export meta, the run log and the
crates-restored proof (outputs/crates_restored_*.txt).

    python3 -I -B make_result.py
"""
import hashlib
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, 'outputs')
IN = os.path.join(HERE, 'inputs')
NAMES = ('v2', 'v2+single', 'v2+cover', 'v2+both')
LABEL = {'b3api': "B3's 562 API ids (B3's reads, B3's api_in.tsv verbatim)",
         'b3rec': "B3's 4,745 unit-1 records (B3's rec_in.tsv verbatim)",
         'c563': "the 563 cohort (B5's re-reads, path W on, served-aware v1)",
         's61': "the 61 (B5 investigation served snapshots, path W on, served-aware v1)",
         'f3502': "the 3,502 (fresh B6 layer, path W off)",
         'f3502[61]': "the 61 inside the fresh layer (subset of f3502)"}


def sha(p):
    with open(p, 'rb') as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def main():
    C = json.load(open(os.path.join(OUT, 'u3-compare.json')))
    meta = json.load(open(os.path.join(IN, 'export_meta.json')))
    gate, harness = {}, []
    for n in sorted(os.listdir(OUT)):
        if n.startswith(('run1', 'run2')) and n.endswith('.log'):
            t = open(os.path.join(OUT, n)).read()
            gate[n] = re.findall(r'GATE-EXIT=(\d+)', t)
            harness += re.findall(r'U3 (?:PARITY|PROJECTION) HARNESS: [^\n]*', t)
    pops = {}
    for pop, P in C['populations'].items():
        pops[pop] = {
            'label': LABEL.get(pop, pop),
            'ids_compared': P['stats'].get('ids_compared', 0),
            'not_exported': P['stats'].get('not_exported', 0),
            'missing_in_rust': P['stats'].get('missing_in_rust', 0),
            'per_guard_set': {g: {'compared': P['per_guard'][g].get('compared', 0), 'agree': P['per_guard'][g].get('agree', 0),
                                  'disagree': P['per_guard'][g].get('disagree', 0),
                                  'ships_port': P['per_guard'][g].get('port_ship', 0), 'ships_rust': P['per_guard'][g].get('rust_ship', 0),
                                  'vouched_by_first_equal': P['per_guard'][g].get('vouched_by_first_equal', 0),
                                  'vouched_by_name_level': P['per_guard'][g].get('vouched_by_name_level', 0)} for g in NAMES},
            'R_single': P['R_single'], 'R_cover': P['R_cover'], 'R_excess': P['R_excess'],
            'refusal_sets_equal': P['refusal_sets_equal'],
            'vouched_by_moved_under_guard': P['vouched_by_moved_under_guard'],
            'served_aware_ids': len(P['served_aware_ids']),
            'disagreements': P['disagreements'], 'name_level': P['name_level'],
            'invariant_failures': P['invariant_failures'],
        }
    proof = {}
    for n in ('crates_restored_1.txt', 'crates_restored_2.txt'):
        p = os.path.join(OUT, n)
        if os.path.exists(p):
            proof[n] = open(p).read()
    res = {
        'what': 'issue 551 B6 U3: Rust v2+G (awards_path_check_guards / head_value_with_awards_path_guards under '
                'R10Guards NONE/SINGLE/COVER/BOTH) vs the port of record b6/rule_v2_g.py decide(id, guards) under '
                'frozenset()/{single}/{cover}/{single,cover}, per id and guard set; projection parity on the 3,502',
        'verdict': 'PASS' if C['totals'].get('disagreements', 1) == 0 and C['totals'].get('invariant_failures', 1) == 0 else 'FAIL',
        'totals': C['totals'],
        'populations': pops,
        'projection_parity': {pop: {'stats': P['stats'], 'by_what': P['by_what'],
                                    'only_LotState_kind_differs': set(P['by_what']) <= {'LotState.kind'}}
                              for pop, P in C['projection'].items()},
        'projection_disagreements': {pop: P['disagreements'] for pop, P in C['projection'].items()},
        'populations_record_only': {k: {'disagreements': v['disagreements'], 'per_guard': v['per_guard']}
                                    for k, v in C['populations_record_only'].items()},
        'cross_checks': C['cross_checks'],
        'cargo': {'gate_exit': gate, 'harness_lines': harness,
                  'command': 'CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model '
                             '-p store -p ingest -p tender-db --features tender-db/server u3_harness -- --ignored --nocapture'},
        'export_meta': meta,
        'files': {n: sha(os.path.join(d, n)) for d in (IN, OUT) for n in sorted(os.listdir(d))
                  if n.endswith(('.tsv', '.jsonl', '.json')) and n != 'u3-result.json'},
        'harness_sha256': {n: sha(os.path.join(HERE, 'harness', n)) for n in sorted(os.listdir(os.path.join(HERE, 'harness')))},
        'scripts_sha256': {n: sha(os.path.join(HERE, n)) for n in ('export_u3.py', 'compare_u3.py', 'make_result.py', 'run.sh')},
        'port_sha256': sha(os.path.join(os.path.dirname(HERE), 'rule_v2_g.py')),
        'crates_restored': proof,
    }
    json.dump(res, open(os.path.join(HERE, 'u3-result.json'), 'w'), indent=1, default=str, sort_keys=True)
    print(res['verdict'], json.dumps(res['totals']))


if __name__ == '__main__':
    main()
