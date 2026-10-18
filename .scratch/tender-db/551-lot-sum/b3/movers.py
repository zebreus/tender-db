"""Issue 551 B3: what moved, and why. For the 562 API ids, the port's outcome three ways:
  (a) M1 / M2-era (pre-drain reads; R10 at every size applied to the recorded outcome as gen_fixtures does: a vouched
      ship stays a ship, an unvouched ship becomes R10, a refusal keeps its first check);
  (b) post-drain reads, the port WITHOUT 570's remap (rule_v2_b3 with the inverse map forced to identity);
  (c) post-drain reads, the port WITH the remap (rule_v2_b3 as is; what the Rust was compared against).
(a)->(b) is the drain (567 refold, 568 drain + unit 8 guards, 570 remap, read through served lot keys); (b)->(c) is
the port edit.

    python3 -I movers.py RUN_DIR OUT_JSON
"""
import importlib.util, json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('rule_v2_b3', os.path.join(HERE, 'rule_v2_b3.py'))
R = importlib.util.module_from_spec(spec)
spec.loader.exec_module(R)


def outcome(f):
    ship, first, S, cur, n = R.v2_decision(f)
    return {'ship': ship, 'first': first, 'S': S, 'cur': cur, 'n': n, 'v1': R.v1_fires(f),
            'vouched_by': (f.get('vouched_by') or {}).get('field')}


def main():
    run, out = sys.argv[1], sys.argv[2]
    rows = [json.loads(l) for l in open(os.path.join(run, 'api_port.jsonl'))]
    real = R.remap_inverse

    def identity(x, contents):
        inv, how = real(x, contents)
        return {nid: {own: own for own in m.values()} for nid, m in inv.items()}, how
    res = []
    for r in rows:
        tid = r['id']
        c = outcome(R.facts(tid))
        R.remap_inverse = identity
        b = outcome(R.facts(tid))
        R.remap_inverse = real
        o = r['old'] or {}
        a = {'ship': bool(o.get('ship_m1') and o.get('vouched')),
             'first': (o.get('first_m1') if not o.get('ship_m1') else (None if o.get('vouched') else 'R10')),
             'S': o.get('S'), 'cur': o.get('cur'), 'n': o.get('n'), 'v1': o.get('v1_fires')}
        key = lambda d: (d['ship'], d['first'], d['S'], d['cur'], d['n'])
        res.append({'id': tid, 'population': r['population'], 'a_m1_era': a, 'b_postdrain_noremap': b, 'c_postdrain_remap': c,
                    'drain_moved': key(a) != key(b), 'remap_moved': key(b) != key(c),
                    'drain_moved_decision': (a['ship'], a['first']) != (b['ship'], b['first']),
                    'remap_moved_decision': (b['ship'], b['first']) != (c['ship'], c['first'])})
    json.dump(res, open(out, 'w'), indent=1, default=str)
    for k in ('drain_moved', 'drain_moved_decision', 'remap_moved', 'remap_moved_decision'):
        print(k, sum(x[k] for x in res))
    for x in res:
        if x['drain_moved_decision'] or x['remap_moved_decision'] or x['remap_moved']:
            a, b, c = x['a_m1_era'], x['b_postdrain_noremap'], x['c_postdrain_remap']
            print(x['id'], x['population'], 'M1era', (a['ship'], a['first'], a['S'], a['n']), '| noremap', (b['ship'], b['first'], b['S'], b['n']),
                  '| remap', (c['ship'], c['first'], c['S'], c['n']))


if __name__ == '__main__':
    main()
