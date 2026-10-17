import json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
import merge_rule as M
for tid, path in M.API_DUMPS.items():
    d = json.load(open(path))
    lots = [{"lot_id": i, "kind": l["kind"], "lot_key": l["lot_key"]} for i, l in enumerate(d["lot_details"])]
    key_id = {l["lot_key"]: l["lot_id"] for l in lots}
    am, res = [], []
    for a in d["amounts"]:
        if a["value"] is None: continue
        am.append({"lot_id": None if a["lot"] is None else key_id[a["lot"]], "field": a["field"], "cents": a["value"]["cents"],
                   "currency": a["value"]["currency"], "tax_basis": None, "eur_cents": a["value"]["cents"] if a["value"]["currency"] == "EUR" else None, "quality": None})
    for i, r in enumerate(d["lot_results"]):
        aw = r.get("awarded") or {}
        res.append({"lot_result_id": i, "lot_id": None if r["lot"] is None else key_id.get(r["lot"]), "awarded_cents": aw.get("cents"),
                    "awarded_currency": aw.get("currency"), "notice_id": r["notice_id"], "result_key": r["key"]})
    chain = []
    if tid == 928196:
        chain = [{"amounts": [{"lot_id": None, "field": "estimated_value", "cents": 4679746035, "currency": "EUR", "tax_basis": None, "eur_cents": 4679746035, "quality": None}], "lots": [], "results": []}]
    entry = {"head": {"amounts": am, "lots": [{"lot_id": l["lot_id"], "kind": l["kind"]} for l in lots], "results": res}, "chain": chain,
             "keys": {l["lot_id"]: l["lot_key"] for l in lots}}
    cur = {a["currency"] for a in am} | {r["awarded_currency"] for r in res if r["awarded_currency"]}
    rates = {c: 1.0 for c in cur if c != "EUR"}   # unit rate for the API approximations
    o = M.serve(entry, rates)
    print(tid, o["why"], o["today"], "->", o["eur"], o["derived"], "value", d["value"])
