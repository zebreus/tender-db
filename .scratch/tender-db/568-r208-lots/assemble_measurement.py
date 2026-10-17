"""Issue 568: assemble measurement.json from the six skeptic-final group files.

Usage: python3 -I assemble_measurement.py <out measurement.json> <groups.json> <sample.json> g1=<file> g2=<file> ...
Each group file is {"tenders": [...]} with per-tender blocks {result_key, lot_number, cls, matched_lot_key?, why}.
"""
import json
import sys
from collections import Counter

out_path, groups_path, sample_path, *specs = sys.argv[1:]
groups = json.load(open(groups_path, encoding="utf-8"))["groups"]
sample = json.load(open(sample_path, encoding="utf-8"))
member = {i["tender_id"]: (g["key"], i) for g in groups for i in g["items"]}
row_by_tender = {r["tender_id"]: r for r in sample["rows"]}

tenders, per_group = [], {}
for spec in specs:
    key, path = spec.split("=", 1)
    data = json.load(open(path, encoding="utf-8"))
    for t in data["tenders"]:
        g, item = member[t["tender_id"]]
        assert g == key, (t["tender_id"], g, key)
        assert item["can_notice_id"] == t["can_notice_id"], t["tender_id"]
        blocks = []
        for b in t["blocks"]:
            blocks.append({
                "result_key": b["result_key"],
                "lot_number": b.get("lot_number") or "",
                "cls": b["cls"],
                "matched_lot_key": b.get("matched_lot_key"),
                "why": b.get("why", ""),
            })
        blocks.sort(key=lambda b: int(b["result_key"].split("-")[1]))
        row = row_by_tender[t["tender_id"]]
        tenders.append({
            "group": key,
            "tender_id": t["tender_id"],
            "can_notice_id": t["can_notice_id"],
            "publication_id": item["publication_id"],
            "can_published": row["notice_published"],
            "tender_lots": item["tender_lots"],
            "n_award_blocks": row["n_res"],
            "n_award_lotnos": item["n_award_lotnos"],
            "cn_notice_ids": t.get("cn_notice_ids", []),
            "cn_lot_number_field": t.get("cn_lot_number_field", ""),
            "lot_keys_vs_printed": t.get("lot_keys_vs_printed", ""),
            "notes": t.get("notes", ""),
            "counts": dict(Counter(b["cls"] for b in blocks)),
            "blocks": blocks,
        })
        per_group.setdefault(key, Counter()).update(b["cls"] for b in blocks)

tenders.sort(key=lambda t: (t["group"], t["can_notice_id"]))
totals = Counter()
for c in per_group.values():
    totals.update(c)
zero_lot = [r for r in sample["rows"] if r["tender_lots"] < 2]
doc = {
    "issue": 568,
    "what": "Cross-notice match of R2.0.8 award blocks (LOT_NUMBER) to the lots the Tender's contract notice printed: "
            "per-block skeptic-final classification (wf 568 measurement, judged 2026-10-17).",
    "sample": {
        "source": sample["source"],
        "multi_lot_cans": sample["n"],
        "cans_probed": 470,
        "multi_lot_share_of_cans": round(sample["n"] / 470, 4),
        "on_tender_with_2plus_lots": len(tenders),
        "on_tender_with_0_or_1_lot": len(zero_lot),
        "award_blocks_all_100": sum(r["n_res"] for r in sample["rows"]),
        "award_blocks_measured": sum(len(t["blocks"]) for t in tenders),
    },
    "classes": {
        "unique_right": "exactly one Tender lot's CN section prints the block's number (after trim / leading zeros / a Lot-Los-Lote word) and the award's own evidence agrees",
        "unique_unverifiable": "one match, no text or figure to confirm or refute",
        "unique_wrong": "one lot matches by number but the award belongs to another lot",
        "ambiguous": "two or more lots print the number",
        "no_match": "no lot prints the number",
        "no_number": "the block has no LOT_NUMBER",
    },
    "totals": dict(totals),
    "per_group": {k: dict(v) for k, v in sorted(per_group.items())},
    "tenders": tenders,
}
json.dump(doc, open(out_path, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
print(json.dumps({"totals": doc["totals"], "per_group": doc["per_group"], "sample": doc["sample"]}, indent=1))
