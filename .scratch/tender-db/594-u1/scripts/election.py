"""election.py -- an APPROXIMATE port of issue 524's vote for org 2660 (crosswalk::head_vote_key ≈ diacritics folded,
lower-cased, alphanumeric tokens, a run of single letters joined, `the` dropped; legal-form tokens are not modelled:
none of the top spellings carries one). Reads raw/sql/s3_names2660.out (all 9,103 mentions); prints V, T, H, the top
clusters and the R1/R2 bars of head_election.rs (R1: T*20 >= V*11, T >= 3, T >= 2H, unique; R2: H <= 1, T*4 >= V)."""
import json, sys, re, unicodedata, collections
sys.path.insert(0, '/home/user/tender-db/.scratch/tender-db/594-u1/scripts')
from classify import klass

def key(name):
    s = unicodedata.normalize('NFD', ' '.join((name or '').split()))
    s = ''.join(c for c in s if not unicodedata.combining(c)).lower()
    toks = [t for t in re.split(r'[^0-9a-z]+', s) if t and t != 'the']
    out, ini = [], ''
    for t in toks:
        if len(t) == 1 and t.isalpha():
            ini += t; continue
        if ini: out.append(ini); ini = ''
        out.append(t)
    if ini: out.append(ini)
    return ' '.join(out)

d = json.load(open('/home/user/tender-db/.scratch/tender-db/594-u1/raw/sql/s3_names2660.out'))
votes = collections.Counter()
for name, _rid, _sch, _cc, n, _f, _l in d['rows']:
    if (name or '').strip():
        votes[name] += n
clusters = collections.defaultdict(lambda: [0, collections.Counter()])
for name, n in votes.items():
    c = clusters[key(name)]; c[0] += n; c[1][name] += n
V = sum(votes.values())
head = 'Tribunal Administrativo de Recursos Contractuales de Andalucía'
H = clusters[key(head)][0]
ranked = sorted(clusters.items(), key=lambda kv: -kv[1][0])
T = ranked[0][1][0]
print(f'V={V} clusters={len(clusters)} T={T} ({T/V:.1%}) H={H} second={ranked[1][1][0]}')
print('R1 bar T*20>=V*11:', T * 20 >= V * 11, '| R2 needs H<=1:', H <= 1, '=> KEEP (bar)' if not (T * 20 >= V * 11) and H > 1 else '')
for k, (n, sp) in ranked[:12]:
    print(n, klass(sp.most_common(1)[0][0]), '|', k, '|', sp.most_common(3))
cls = collections.Counter()
for k, (n, sp) in clusters.items():
    cls[klass(sp.most_common(1)[0][0])] += 1
print('clusters by class', dict(cls))
