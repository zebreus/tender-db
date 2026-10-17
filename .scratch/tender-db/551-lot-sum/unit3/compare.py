"""Port check: the Rust election (rust_out.tsv) against the Python merge (py_out.json) on the 4,745 records."""
import json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
py = json.load(open(os.path.join(HERE, "py_out.json")))
LABEL = {0: "awards", 3: "BT-27-Lot", 4: "BT-271-Lot"}
dis = []
n = 0
stats = {"today_eq": 0, "eur_eq": 0, "derived_both": 0, "derived_none_both": 0}
for line in open(os.path.join(HERE, "rust_out.tsv")):
    p = line.rstrip("\n").split("\t")
    tid, today, eur = p[0], p[1], p[2]
    o = py[tid]
    n += 1
    rt = None if today == "-" else int(today)
    re_ = None if eur == "-" else int(eur)
    if rt != o["today"]:
        dis.append((tid, "today", rt, o["today"]))
    else:
        stats["today_eq"] += 1
    if re_ != o["eur"]:
        dis.append((tid, "eur", re_, o["eur"]))
    else:
        stats["eur_eq"] += 1
    d = o["derived"]
    if p[3] == "-":
        if d is not None:
            dis.append((tid, "derived", None, d))
        else:
            stats["derived_none_both"] += 1
        continue
    method, rule, k, field, fcents, fcur, kind, cents, cur, lots = p[3:13]
    if d is None:
        dis.append((tid, "derived", p[3:], None)); continue
    want = (d["path"], d.get("rule") or "-", str(d.get("k")) if d.get("k") is not None else "-", d.get("f_field") or "-",
            str(d.get("f_cents")) if d.get("f_cents") is not None else "-", d.get("f_currency") or "-",
            LABEL[d["kind"]], str(d["cents"]), d["currency"], str(d["lots"]))
    got = (method, rule, k, field, fcents, fcur, kind, cents, cur, lots)
    if got != want:
        dis.append((tid, "derived", got, want))
    else:
        stats["derived_both"] += 1
print("records", n, stats)
print("disagreements", len(dis))
for x in dis[:50]:
    print(" ", x)
