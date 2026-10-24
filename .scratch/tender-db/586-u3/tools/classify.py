"""classify.py: final.decision(tp, parent) (586-u1's oracle) over the aliased window rows; prints/returns tiers."""
import importlib.util, json, os, sys
U1 = '/home/user/tender-db/.scratch/tender-db/586-u1'
spec = importlib.util.spec_from_file_location('final586', os.path.join(U1, 'final.py'))
final = importlib.util.module_from_spec(spec)
sys.path.insert(0, U1)
spec.loader.exec_module(final)

def decision(tp, parent):
    return final.decision(tp or '', parent or '')

if __name__ == '__main__':
    import collections
    for n in sys.argv[1:]:
        d = json.load(open(f'/home/user/tender-db/.scratch/tender-db/586-u3/raw/sql/{n}.out'))
        rows = [dict(zip(d['columns'], r)) for r in d['rows']]
        c = collections.Counter()
        for r in rows:
            k = decision(r['tp_name'], r['parent_name'])
            c[k] += 1
            print(n, r['tender_id'], r['role'], r['org_id'], r['notice_id'], r['section_id'], r['tpo'], r['n_rows'], k, '|', (r['tp_name'] or '')[:60], '||', (r['parent_name'] or '')[:60])
        print('==', n, dict(c))
