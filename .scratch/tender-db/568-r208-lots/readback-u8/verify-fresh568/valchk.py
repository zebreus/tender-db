import json, os, sys, re, unicodedata
sys.dont_write_bytecode = True
RB = '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8'
sys.path.insert(0, RB)
import combined_port as C
P = C.P
S = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/fresh568'
cache = C.Cache([os.path.join(S, 'bar_cache')])
tid = int(sys.argv[1])
t = cache.tender(tid); c = cache.contents(t)
chain = C.chain_of(t)
maps = C.remap_chain(chain, c, True)
pre = C.expected(t, c, True, False)
def vals(sec, f=None, ty=None):
    return [v for v in sec.get('values', []) if (f is None or v['field_id'] in f) and (ty is None or v.get('type') in ty)]
def norm(s):
    s = unicodedata.normalize('NFKD', s or '')
    s = ''.join(ch for ch in s if not unicodedata.combining(ch)).lower()
    return re.sub(r'\W+', ' ', s).strip()
# lot facts from CNs (by mapped key)
lots = {}
for n in chain:
    cc = c.get(n)
    if not cc: continue
    secs = P.sections_of(cc)
    target, how = maps[n]
    for s in secs.values():
        if s.get('kind') not in P.LOT_KINDS: continue
        k = target.get(s['section_id'], s['section_id'])
        d = lots.setdefault(k, {'texts': [], 'amts': set(), 'no': P.published_number(s)})
        d['texts'] += [v.get('value') for v in vals(s, ty=('text',))]
        d['amts'] |= {v.get('cents') for v in vals(s, ty=('amount',)) if v.get('cents')}
for n in chain:
    cc = c.get(n)
    if not cc: continue
    secs = P.sections_of(cc)
    refs = P.award_refs(secs)
    for res in sorted(refs, key=P.res_order):
        row = pre.get((n, res), {})
        if not row.get('reason', '').startswith('568'): continue
        s = secs[res]
        tl = row.get('expected_lot')
        titles = [v.get('value') for v in vals(s, ('TED-CONTRACT_TITLE', 'TED-TITLE_CONTRACT'), ('text',))]
        est = [v.get('cents') for v in vals(s, ty=('amount',)) if 'INITIAL' in v['field_id']]
        tn = norm(' '.join(titles))
        # strip pakiet prefix
        core = re.sub(r'^(pakiet|lot|czesc|zadanie)( nr)? ?\d+ ?', '', tn).strip()
        val_hits = sorted(k for k, d in lots.items() if est and est[0] in d['amts'])
        txt_hits = []
        for k, d in lots.items():
            lt = norm(' '.join(x for x in d['texts'] if x))
            if core and len(core) > 3 and core[:25] in lt:
                txt_hits.append(k)
        flag = ''
        if val_hits and tl not in val_hits: flag += ' VALUE-POINTS-ELSEWHERE'
        if txt_hits and tl not in txt_hits: flag += ' TEXT-POINTS-ELSEWHERE'
        if not val_hits: flag += ' no-value-hit'
        if not txt_hits: flag += ' no-text-hit'
        lt = lots.get(tl, {})
        print(f"{n}/{res} p={row.get('printed')} lot={tl} est={est} val_hits={val_hits} txt_hits={sorted(txt_hits)[:4]}{flag}")
        print(f"     block: {' | '.join(titles)[:150]}")
        print(f"     lot  : {' | '.join(x for x in lt.get('texts', []) if x)[:150]}  amts={sorted(lt.get('amts', []))}")
