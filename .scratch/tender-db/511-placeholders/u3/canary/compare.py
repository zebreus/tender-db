#!/usr/bin/env python3
"""511 buyer canary: compare pre.json and post.json (snap.py output). usage: compare.py pre.json post.json"""
import json, sys
pre, post = (json.load(open(p)) for p in sys.argv[1:3])
buyer = set(pre["buyer_notices"])
tchg = [n for n in sorted(buyer) if pre["buyer_notices"][n]["tenders"] != post["buyer_notices"][n]["tenders"]]
kchg = [n for n in sorted(buyer) if n not in tchg and pre["buyer_notices"][n]["keys"] != post["buyer_notices"][n]["keys"]]
print(f"buyer notices {len(buyer)}: Tender changed {len(tchg)} ({100*len(tchg)/len(buyer):.1f}%), key only {len(kchg)}")
for n in tchg + kchg:
    print(" ", n, pre["buyer_notices"][n], "->", post["buyer_notices"][n])
def placement(fams):
    out = {}
    for k, f in fams.items():
        for t, v in f.items():
            for n in v["notices"]:
                out.setdefault(k, {})[str(n)] = (t, v["own"])
    return out
a, b = placement(pre["families"]), placement(post["families"])
beyond = []
for k in sorted(set(a) | set(b)):
    for n in sorted(set(a.get(k, {})) | set(b.get(k, {}))):
        if a.get(k, {}).get(n) != b.get(k, {}).get(n) and n not in buyer:
            beyond.append((k, n, a.get(k, {}).get(n), b.get(k, {}).get(n)))
print(f"family changes beyond the buyer notices: {len(beyond)}")
for x in beyond:
    print(" ", x)
