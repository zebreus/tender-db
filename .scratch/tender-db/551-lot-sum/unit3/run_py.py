"""Run merge_rule.serve on every ctx.pkl record; write py_out.json and print the effect summary."""
import json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import merge_rule as M
ctx = M.load_ctx()
recs = {}
for l in open(os.path.join(HERE, "..", "results-slim.jsonl")):
    r = json.loads(l); recs[r["id"]] = r
out = {}
for tid, e in ctx.items():
    rates = M.rates_of(e)
    o = M.serve(e, rates)
    o["pop"] = recs[tid]["pop"]
    o["stored"] = recs[tid]["elected_eur_cents"]
    out[tid] = o
json.dump({str(k): v for k, v in out.items()}, open(os.path.join(HERE, "py_out.json"), "w"))
ch = {t: o for t, o in out.items() if o["eur"] != o["today"]}
print("records", len(out), "changes", len(ch), "B", sum(o["pop"] == "B" for o in ch.values()), "R", sum(o["pop"] == "R" for o in ch.values()))
# today vs stored (port fidelity of the rate table)
bad = [t for t, o in out.items() if (o["today"] is None) != (o["stored"] is None) or (o["today"] is not None and abs(o["today"] - o["stored"]) > 2)]
print("today != stored (beyond 2 cents):", len(bad), bad[:20])
for t, o in sorted(ch.items(), key=lambda x: x[1]["why"]):
    d = o["derived"]
    print(f"  {t:8d} {o['pop']} {o['why']} today={o['today']} -> {o['eur']} {d['currency']} S={d['cents']} n={d['lots']} kind={d['kind']} rule={d.get('rule')} k={d.get('k')}")
from collections import Counter
print(Counter(o["why"] for o in out.values()).most_common())
