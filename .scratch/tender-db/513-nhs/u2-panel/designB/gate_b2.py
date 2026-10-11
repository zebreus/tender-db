# 513 design B, refined gate: lexicographic (role agreement, distinctive-token Jaccard,
# descriptor-token Jaccard) against a frozen ODS name table; refuse when the code's own
# key is role-contradicted, has no distinctive overlap, or another code's key is strictly
# greater. Ties admit. python3 -I gate_b2.py <u1 dir>
import json, re, sys, collections, os, itertools
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gate_b as G
table, preds, allow, resolve_tail = G.table, G.preds, G.allow, G.resolve_tail
STOP = set('nhs the and of for in t as trading c o limited ltd plc at with'.split())
DESC = set('hospital university teaching healthcare health care community mental partnership service ambulance '
           'children women specialist acute general foundation trust integrated board commissioning support unit '
           'clinical group strategic local sub location solution provider collaborative'.split())
ABBR = [(r'\bicb\b', ' integrated care board '), (r'\bccg\b', ' clinical commissioning group '),
        (r'\bcsu\b', ' commissioning support unit '), (r'\bft\b', ' foundation trust '),
        (r'\bn h s\b', ' nhs '), (r'\bst\b', ' saint ')]
CODE_ROLE = {'RO197': 'trust', 'RO107': 'trust', 'RO261': 'icb', 'RO98': 'icb', 'RO213': 'csu', 'RO172': 'company', 'RO162': 'other'}
def words(n):
    s = (n or '').lower().replace('&', ' and ').replace('’', "'").replace("'", '')
    s = re.sub(r'[^a-z0-9]+', ' ', s)
    for a, b in ABBR: s = re.sub(a, b, s)
    out = []
    for w in s.split():
        if w in STOP: continue
        if len(w) > 3 and w.endswith('s') and not w.endswith('ss'): w = w[:-1]
        out.append(w)
    return out
def role_of_name(ws):
    S = set(ws)
    if {'commissioning', 'support'} <= S: return 'csu'
    if ({'integrated', 'board'} <= S) or ({'clinical', 'commissioning'} <= S): return 'icb'
    if 'trust' in S: return 'trust'
    return None
def split(n):
    ws = words(n)
    return set(w for w in ws if w not in DESC), set(w for w in ws if w in DESC), role_of_name(ws)
def j(A, B): return len(A & B) / len(A | B) if A and B else 0.0
_cache = {}
def code_names(code, use_preds=True):
    ns = [(table[code]['name'], table[code]['role'])]
    if use_preds: ns += [(table[p]['name'], table[p]['role']) for p in preds.get(code, []) if p in table]
    return ns
def key_for(mention, names):
    D, E, r = mention
    best = (0, 0.0, 0.0)
    for nm, role in names:
        d, e, _ = _cache.setdefault(nm, split(nm))
        crole = CODE_ROLE.get(role)
        compat = 1 if (r is None or crole == r) else 0
        best = max(best, (compat, j(D, d), j(E, e)))
    return best
def gate2(name, code, use_preds=True):
    m = split(name)
    own_set = {code} | (set(preds.get(code, [])) if use_preds else set())
    own = key_for(m, code_names(code, use_preds))
    if own[0] == 0: return False, own, None, None, 'role'
    if own[1] == 0: return False, own, None, None, 'zero'
    best, bc = max(((key_for(m, [(v['name'], v['role'])]), c) for c, v in table.items() if c not in own_set), default=((0, 0, 0), None))
    if best > own: return False, own, best, bc, 'better'
    return True, own, best, bc, ('tie' if best == own else 'ok')

if __name__ == '__main__':
    reg = G.reg
    res = collections.defaultdict(collections.Counter); det = []; ties = collections.Counter()
    for r in reg:
        if r['code'] not in table: continue
        tc, folded = resolve_tail(r['code'], True)
        if tc is None: ok, why = False, 'not-allowlisted'
        else: ok, own, best, bc, why = gate2(r['name'], tc)
        k = res[r['final']]; k['holders'] += 1; k['coded'] += r['coded']
        if ok and why == 'tie': ties[r['final']] += r['coded']; det.append(('TIE-ADMIT', r['final'], r['code'], r['name'][:50], bc, r['coded']))
        if not ok:
            k['rh'] += 1; k['rc'] += r['coded']
            if r['final'] == 'SAME': det.append(('REFUSED-SAME', r['code'], r['name'][:55], r['coded'], why, bc))
        elif r['final'] in ('DIFFERENT', 'NOT_ORG', 'HOSTED_OR_SITE'): det.append(('ADMITTED-' + r['final'], r['code'], r['name'][:60], r['coded'], why))
    print('== refined gate B2 (role, distinctive, descriptor), predecessor names, allowlist + O/0 fold')
    for cls in ('SAME', 'HOSTED_OR_SITE', 'DIFFERENT', 'NOT_ORG'):
        k = res[cls]; print(f"  {cls:15s} holders {k['holders']:3d} refused {k['rh']:3d} | coded {k['coded']:5d} refused {k['rc']:4d}")
    print('  admitted on an exact tie (coded):', dict(ties))
    for d in sorted(det, key=str): print('   ', d)
