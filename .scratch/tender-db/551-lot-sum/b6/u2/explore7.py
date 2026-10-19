import json, gzip, importlib.util, sys, copy, random
spec=importlib.util.spec_from_file_location('run_u2','run_u2.py'); U=importlib.util.module_from_spec(spec); spec.loader.exec_module(U)
ids, served = U.the_ids()
recs={}
for line in gzip.open('decisions.jsonl.gz','rt'):
    d=json.loads(line); recs[d['id']]=d
ok=sum(U.parity(served[t], recs[t]['decisions']['v2'])['ok'] for t in ids); print('ok', ok)
random.seed(1); s=random.sample(ids,5)
for t in s:
    v2=recs[t]['decisions']['v2']; print(t, served[t][3:7], json.loads(served[t][7]), '| port', v2['S'], v2['currency'], v2['n'], v2['vouched_by'])
# fault injection: each perturbation must be caught
t=s[0]; base=recs[t]['decisions']['v2']
for name,mut in [('S',lambda d: d.__setitem__('S', d['S']+1)), ('cur',lambda d: d.__setitem__('currency','XXX')),
                 ('n',lambda d: d.__setitem__('n', d['n']+1)), ('ship',lambda d: d.__setitem__('ship', False)),
                 ('vouch',lambda d: d.__setitem__('vouches', [[v[0], v[1]+1, v[2]] for v in d['vouches']]))]:
    d=copy.deepcopy(base); mut(d); print(name, U.parity(served[t], d)['kinds'])
