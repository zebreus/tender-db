"""Fresh reads for the fresh570 verifier: Tender + chain notice contents + profiles, then a few change pages.
One GET per >= 1.6 s (check_live.Fetcher), a 408 retried once."""
import json, os, sys, time, subprocess
sys.dont_write_bytecode = True
U8 = '/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8'
sys.path.insert(0, U8)
import combined_port as C
import check_live as CL
OUT = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/verify-fresh570/cache'
f = CL.Fetcher(OUT, 1.6, False, [])
ids = [int(x) for x in sys.argv[1].split(',')]
for tid in ids:
    t = f.tender(tid)
    if t is None:
        print(tid, 'tender read failed', f.last_code, f.errors[-1:], flush=True); continue
    ch = C.chain_of(t)
    miss = []
    for n in ch:
        if f.notice(n) is None:
            miss.append((n, f.last_code))
    p = f.profiles(tid)
    print(time.strftime('%H:%M:%SZ', time.gmtime()), tid, 'chain', len(ch), 'missing', miss, 'profiles', None if p is None else len(p), 'gets', f.gets, flush=True)
print('errors', f.errors)
