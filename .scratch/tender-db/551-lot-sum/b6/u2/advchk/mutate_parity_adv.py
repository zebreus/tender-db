"""Fault injection on run_u2.parity (scratch copy): every mutation of the port's v2 (ship, S, currency, n, vouches) on every
one of the 3,502 must be flagged; the unmutated decision must pass.

python3 -I -B mutate_parity_adv.py RUN_U2_COPY DECISIONS.jsonl.gz
"""
import collections
import copy
import gzip
import importlib.util
import json
import sys


def main(run_u2, dec):
    spec = importlib.util.spec_from_file_location('run_u2_adv', run_u2)
    U = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(U)
    ids, served = U.the_ids()
    recs = {}
    for line in gzip.open(dec, 'rt', encoding='utf-8'):
        r = json.loads(line)
        recs[r['id']] = r
    caught = collections.Counter()
    missed = collections.defaultdict(list)
    base_ok = 0
    for t in ids:
        v2 = recs[t]['decisions']['v2']
        base_ok += U.parity(served[t], v2)['ok']
        muts = {}
        m = copy.deepcopy(v2); m['ship'] = False; muts['ship'] = m
        m = copy.deepcopy(v2); m['S'] = v2['S'] + 1; muts['S+1'] = m
        m = copy.deepcopy(v2); m['currency'] = 'XXX'; muts['currency'] = m
        m = copy.deepcopy(v2); m['n'] = v2['n'] + 1; muts['n+1'] = m
        det = json.loads(served[t][7])
        vb = det['vouched_by']
        k = [vb['field'], vb['notice_id'], vb['cents']]
        m = copy.deepcopy(v2); m['vouches'] = [v for v in v2['vouches'] if v != k]; muts['drop_served_vouch'] = m
        m = copy.deepcopy(v2); m['vouches'] = [[v[0], v[1], v[2] + 1] for v in v2['vouches']]; muts['vouch_cents+1'] = m
        m = copy.deepcopy(v2); m['vouches'] = [[v[0], str(v[1]), v[2]] for v in v2['vouches']]; muts['vouch_nid_str'] = m
        for name, mm in muts.items():
            if U.parity(served[t], mm)['ok']:
                missed[name].append(t)
            else:
                caught[name] += 1
    print('baseline ok', base_ok, 'of', len(ids))
    print('caught', dict(caught))
    print('missed', {k: (len(v), v[:5]) for k, v in missed.items()})


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
