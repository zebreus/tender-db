# 513 design B: ODS-allowlist mint + best-match ODS-name gate, simulated over the
# listed registry holders. python3 -I gate_b.py <u1 dir>
import json, re, sys, collections, os, itertools
U1 = sys.argv[1]; ODS = f'{U1}/ods-lists'
rows = json.load(open(f'{U1}/adjudication-rows.json'))
mc = collections.defaultdict(dict)
for line in open(f'{U1}/mention-counts-by-org.jsonl'):
    for org, raw, n in json.loads(line)['rows']:
        mc[org][raw] = mc[org].get(raw, 0) + n

# ---- the frozen ODS table ------------------------------------------------
table = {}   # code -> dict(name, status, role)
for f in ['list_RO197.json', 'list_RO261.json', 'list_RO213.json', 'list_RO98.json']:
    for o in json.load(open(f'{ODS}/{f}'))['Organisations']:
        if len(o['OrgId']) == 3:
            table[o['OrgId']] = dict(name=o['Name'], status=o['Status'], rc=o['OrgRecordClass'], role=o['PrimaryRoleId'])
preds = collections.defaultdict(list)
for fn in os.listdir(f'{U1}/ods'):
    if not fn.endswith('.json') or fn.startswith('_'): continue
    d = json.load(open(f'{U1}/ods/{fn}')); b = d.get('body') or {}
    o = b.get('Organisation')
    if not o: continue
    code = fn[:-5]
    if len(code) != 3: continue
    prim = [r['id'] for r in o['Roles']['Role'] if r.get('primaryRole')][0]
    table.setdefault(code, dict(name=o['Name'], status=o['Status'], rc=o.get('orgRecordClass'), role=prim))
    for s in o.get('Succs', {}).get('Succ', []):
        if s.get('Type') == 'Predecessor':
            preds[code].append(s['Target']['OrgId']['extension'])
ALLOW_ROLES = {'RO197', 'RO261', 'RO213', 'RO107', 'RO172'}
allow = {c for c, v in table.items() if v['status'] == 'Active' and v['rc'] == 'RC1' and v['role'] in ALLOW_ROLES
         and not any(ch.isdigit() and ch != '0' for ch in c)}   # only the digit-rule family needs the arm
allow_all3 = {c for c, v in table.items() if v['status'] == 'Active' and v['rc'] == 'RC1' and v['role'] in ALLOW_ROLES}

def fold_variants(t):
    pos = [i for i, ch in enumerate(t) if ch in 'O0']
    out = set()
    for k in range(1, len(pos) + 1):
        for sub in itertools.combinations(pos, k):
            s = list(t)
            for i in sub: s[i] = '0' if s[i] == 'O' else 'O'
            out.add(''.join(s))
    return out
collide = [(a, b) for a in allow_all3 for b in fold_variants(a) if b in allow_all3 and a < b]

def resolve_tail(t, fold=True):
    if t in allow: return t, False
    if not fold: return None, False
    hits = [v for v in fold_variants(t) if v in allow]
    return (hits[0], True) if len(hits) == 1 else (None, False)

# ---- name tokens -----------------------------------------------------------
GENERIC = set('nhs the and of trust foundation integrated care board ft icb limited ltd university universities hospital hospitals teaching healthcare health services service partnership community mental group commissioning support unit csu ccg for in t as trading c o sub location strategic local'.split())
SYN_BASE = [(r'\bicb\b', 'integrated care board'), (r'\bft\b', 'foundation trust'), (r"'s\b", 's'), (r'\bst\b', 'saint')]
SYN_UHB = [(r'\buhb\b', 'university hospitals birmingham')]
def toks(n, syn):
    s = (n or '').lower().replace('&', ' and ').replace('’', "'")
    for a, b in syn: s = re.sub(a, b, s)
    s = s.replace("'", '')
    return {t for t in re.findall(r'[a-z0-9]+', s) if t not in GENERIC}
def jacc(A, B):
    return len(A & B) / len(A | B) if A and B else 0.0
def contain(A, B):   # share of the mention's tokens the ODS name carries, jaccard tiebreak
    return (len(A & B) / len(A) + 1e-3 * jacc(A, B)) if A and B else 0.0

def names_of(code, use_preds):
    ns = [table[code]['name']]
    if use_preds: ns += [table[p]['name'] for p in preds.get(code, []) if p in table]
    return ns

def gate(name, code, cfg):
    syn = SYN_BASE + (SYN_UHB if cfg.get('uhb', True) else [])
    sim = contain if cfg.get('sim') == 'contain' else jacc
    A = toks(name, syn)
    own_set = {code} | (set(preds.get(code, [])) if cfg['preds'] else set())
    own = max(sim(A, toks(n, syn)) for n in names_of(code, cfg['preds']))
    other, oc = max(((sim(A, toks(v['name'], syn)), c) for c, v in table.items() if c not in own_set), default=(0, None))
    if cfg['zero'] and own == 0: return False, own, other, oc, 'zero'
    if other > own: return False, own, other, oc, 'better'
    return True, own, other, oc, 'ok'

CFGS = {
  'G0 original (jaccard, current name)': dict(preds=False, zero=False, allow=False, fold=False),
  'G1 +zero-score refusal': dict(preds=False, zero=True, allow=False, fold=False),
  'G2 +predecessor names': dict(preds=True, zero=True, allow=False, fold=False),
  'G3 +allowlist +O/0 fold (DESIGN B)': dict(preds=True, zero=True, allow=True, fold=True),
  'G3 without the uhb synonym': dict(preds=True, zero=True, allow=True, fold=True, uhb=False),
  'G3 containment similarity': dict(preds=True, zero=True, allow=True, fold=True, sim='contain'),
}
reg = [r for r in rows if r['side'] == 'registry']
def norm(raw): return ''.join(c for c in (raw or '') if c.isascii() and c.isalnum()).upper()

if __name__ == '__main__':
    print('table codes', len(table), 'allowlist (digit-rule family)', len(allow), 'all active allowed-role 3-char', len(allow_all3))
    print('allow by role', collections.Counter(table[c]['role'] for c in allow))
    print('O/0 twin collisions inside the allowlist:', collide)
    for label, cfg in CFGS.items():
        res = collections.defaultdict(collections.Counter); det = []
        for r in reg:
            code = r['code']
            if code not in table: continue
            if cfg['allow']:
                tc, folded = resolve_tail(code, cfg['fold'])
                if tc is None:
                    ok, why = False, 'not-allowlisted'
                else:
                    ok, own, other, oc, why = gate(r['name'], tc, cfg)
            else:
                ok, own, other, oc, why = gate(r['name'], code, cfg)
            k = res[r['final']]
            k['holders'] += 1; k['coded'] += r['coded']
            if not ok:
                k['ref_holders'] += 1; k['ref_coded'] += r['coded']
                if r['final'] == 'SAME': det.append((code, r['name'][:60], r['coded'], why))
            elif r['final'] != 'SAME':
                det.append(('ADMITTED-' + r['final'], code, r['name'][:60], r['coded']))
        print('\n==', label)
        for cls in ('SAME', 'HOSTED_OR_SITE', 'DIFFERENT', 'NOT_ORG'):
            k = res[cls]; print(f"  {cls:15s} holders {k['holders']:3d} refused {k['ref_holders']:3d} | coded {k['coded']:5d} refused {k['ref_coded']:4d}")
        for d in sorted(det, key=str): print('   ', d)
