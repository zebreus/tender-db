import json, os, sys, collections
sys.dont_write_bytecode = True
RB = '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8'
sys.path.insert(0, RB)
import combined_port as C
S = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/fresh568'
cache = C.Cache([os.path.join(S, 'bar_cache')])
draw = json.load(open(os.path.join(S, 'draw.json')))
tot = ok = 0; bad = []; extra = []; reasons = collections.Counter()
for x in draw['draw']:
    t = cache.tender(x['id']); c = cache.contents(t)
    exp = C.expected(t, c, True, True)
    served = {(r['notice_id'], r['key']): r.get('lot') for r in t.get('lot_results', [])}
    for k, r in exp.items():
        tot += 1
        reasons[r['reason'].split(' (')[0].split(' LOT')[0].split(' ->')[0]] += 1
        if k not in served: bad.append((x['id'], k, 'not served', r['expected_lot'])); continue
        if served[k] != r['expected_lot']: bad.append((x['id'], k, served[k], r['expected_lot'], r['reason']))
        else: ok += 1
    for k in served:
        if k not in exp: extra.append((x['id'], k, served[k]))
print('blocks', tot, 'ok', ok, 'bad', len(bad), 'served-not-computed', len(extra))
print(reasons)
for b in bad[:20]: print(b)
for e in extra[:20]: print(e)
