import json, os, sys, collections
sys.dont_write_bytecode = True
RB = '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8'
sys.path.insert(0, RB)
import combined_port as C
S = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/fresh568'
cache = C.Cache([os.path.join(S, 'bar_cache')])
draw = json.load(open(os.path.join(S, 'draw.json')))
for x in draw['draw']:
    tid = x['id']
    t = cache.tender(tid)
    c = cache.contents(t)
    pre = C.expected(t, c, True, False)
    cnt = collections.Counter(r['reason'].split(' (')[0].split(' LOT')[0].split(' ->')[0] for r in pre.values())
    # notices with no lot sections but results
    nolot = []
    for n, cc in c.items():
        secs = C.P.sections_of(cc)
        lsecs = [s for s in secs.values() if s.get('kind') in C.P.LOT_KINDS]
        refs = C.P.award_refs(secs)
        if refs and not lsecs:
            nolot.append((n, len(refs)))
    print(tid, 'g2' if x['g2'] else '  ', dict(cnt), 'no-lot-section award notices:', nolot)
