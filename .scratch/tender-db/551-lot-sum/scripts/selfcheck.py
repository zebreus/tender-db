import sys, os
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE); sys.path.insert(0, os.path.dirname(HERE))
import load, elect as E
trunc = {"b_04","b_06","b_08","b_09","b_14","b_18"}
names = [f"r_{i:02d}" for i in range(1,6)] + [f"b_{i:02d}" for i in range(1,26) if f"b_{i:02d}" not in trunc]
D = load.parse(names)
same = diff = err = fid = nofid = 0
for tid, t in D.items():
    if t["T"][0] is None: continue
    try:
        h, c, k = load.inputs(t, tid)
        e1, _, _ = E.elect(h["amounts"], h["lots"], h["results"], chain=c, lot_keys=k)
        h2, c2, k2 = load.inputs(t, tid, mode="compact")
        e2, _, _ = E.elect(h2["amounts"], h2["lots"], h2["results"], chain=c2, lot_keys=k2)
    except Exception as ex:
        err += 1; print("ERR", tid, repr(ex)[:200]); continue
    if e1 == e2: same += 1
    else: diff += 1; print("DIFF", tid, e1, e2)
    if e1 == t["T"][1]: fid += 1
    else: nofid += 1
print(f"tenders {len(D)} full==compact {same} differ {diff} errors {err}; full==stored {fid} != {nofid}")
