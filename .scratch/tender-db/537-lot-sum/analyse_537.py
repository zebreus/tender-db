"""Issue 537 unit 1: over every >= EUR 1 bn head, test whether the head is 10^k x a sum of its own lot figures
(same currency), k in 1..6, to the rounding a dropped decimal separator leaves.

Partners (each summed over distinct lots, same currency as the head):
  awards  — lot_results[].awarded (latest per lot, by notice id)
  est     — amounts estimated_value with a lot
  fwmax   — amounts result_framework_maximum (BT-709) with a lot
  lotval  — every other lot-scoped amount field, per field
Match: |H - 10^k * S| <= 10^k * n_lots (one cent per lot after scaling), S > 0.
Also records ratio-1 matches (|H - S| <= n_lots cents): the head IS its lot sum (a sanity population).
python3 -I analyse_537.py <details.jsonl> <out.json>
"""
import json, sys
from collections import Counter, defaultdict

rows = [json.loads(l) for l in open(sys.argv[1])]
out = {"n": len(rows), "missing": 0, "matches": [], "ratio1": Counter(), "by_k": Counter(), "partners_present": Counter(),
       "heads_by_source": Counter(), "no_lot_figure": 0}
for t in rows:
    if "status" in t:
        out["missing"] += 1; continue
    v = t.get("value") or {}
    H, cur = v.get("cents"), v.get("currency")
    if not H: out["missing"] += 1; continue
    src = t.get("source") or "?"
    out["heads_by_source"][src] += 1
    partners = {}
    # awards: latest per lot (by notice id), same currency
    per_lot = {}
    for r in t.get("lot_results") or []:
        a = r.get("awarded")
        if not a or a.get("currency") != cur or not r.get("lot"): continue
        prev = per_lot.get(r["lot"])
        if prev is None or (r.get("notice_id") or 0) >= prev[0]:
            per_lot[r["lot"]] = (r.get("notice_id") or 0, a["cents"])
    if per_lot:
        partners["awards"] = (sum(c for _, c in per_lot.values()), len(per_lot))
    byfield = defaultdict(dict)
    for a in t.get("amounts") or []:
        val = a.get("value") or {}
        if not a.get("lot") or val.get("currency") != cur or val.get("cents") is None: continue
        byfield[a["field"]][a["lot"]] = val["cents"]
    for field, lots in byfield.items():
        name = {"estimated_value": "est", "result_framework_maximum": "fwmax"}.get(field, "lot:" + field)
        partners[name] = (sum(lots.values()), len(lots))
    if not partners:
        out["no_lot_figure"] += 1; continue
    for p in partners: out["partners_present"][p] += 1
    for p, (S, n) in partners.items():
        if S <= 0: continue
        if abs(H - S) <= max(n, 1):
            out["ratio1"][p] += 1
        for k in range(1, 7):
            m = 10 ** k
            if abs(H - m * S) <= m * max(n, 1):
                out["matches"].append({"id": t["id"], "k": k, "partner": p, "head": H, "currency": cur, "lot_sum": S,
                                        "lots": n, "residual": H - m * S, "source": src,
                                        "publication_id": t.get("publication_id"), "versions": len(t.get("versions") or []),
                                        "head_eur_cents": t["head_eur_cents"], "title": t.get("title"),
                                        "head_field": next((a["field"] for a in t.get("amounts") or [] if not a.get("lot")
                                                            and (a.get("value") or {}).get("cents") == H), None),
                                        "other_procedure_figures": sorted({(a["field"], (a.get("value") or {}).get("cents"))
                                                            for a in t.get("amounts") or [] if not a.get("lot")
                                                            and (a.get("value") or {}).get("cents") != H}, key=str)})
                out["by_k"][(k, p)] += 1
ids = {m["id"] for m in out["matches"]}
out["matched_tenders"] = len(ids)
out["by_k"] = {f"k{k}:{p}": c for (k, p), c in sorted(out["by_k"].items())}
out["matches_by_source"] = dict(Counter(m["source"] for m in {m["id"]: m for m in out["matches"]}.values()))
json.dump(out, open(sys.argv[2], "w"), ensure_ascii=False, indent=1, default=list)
print(json.dumps({k: out[k] for k in ("n", "missing", "no_lot_figure", "matched_tenders", "by_k", "matches_by_source")}, default=dict))
print("ratio1", dict(out["ratio1"]), "partners", dict(out["partners_present"]), "sources", dict(out["heads_by_source"]))
