import json, os, sys
sys.dont_write_bytecode = True
U8 = '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8'
sys.path.insert(0, U8)
import combined_port as C
R = C.R
cache, tid = sys.argv[1], int(sys.argv[2])
t = json.load(open(os.path.join(cache, f'tenders_{tid}.json')))
ch = C.chain_of(t)
for v in sorted(t['versions'], key=lambda v: v['seq']):
    print('v', v['seq'], v['caused_by_notice_id'], {k: v.get(k) for k in v if k not in ('seq', 'caused_by_notice_id')})
for n in ch:
    c = json.load(open(os.path.join(cache, f'notices_{n}_content.json')))
    secs = R.lot_sections(c)
    print('==', n, 'legacy', C.is_legacy(c), 'secs', len(secs), 'pub', c.get('published_at') or c.get('publication_date'), [k for k in c if k != 'sections'])
    for key, kind, num, item, title in secs:
        print('   ', key, kind, repr(num), repr(item), (title or '')[:70])
    sm = R.same_notice_map(c)
    if sm:
        print('   results', sm)
