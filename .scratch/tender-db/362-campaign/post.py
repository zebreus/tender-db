"""Post-process the issue-362 review (R2 name-gate denials).

    CAMPAIGN_DIR=<dir> python3 post.py <journal.jsonl> <cohort> <batches-dir>

Joins reviews, challenges and blind samples by case-key set, prints agreement
statistics, and writes the POST /admin/merge-verdicts body:
  merge/high undisputed  -> merge high      (executed by the next wet R2 run)
  merge otherwise        -> merge medium    (recorded, never executed)
  keep high|medium, undisputed -> keep      (denies the fold for good)
  keep disputed          -> keep medium... recorded as keep only if the reviewer said high AND
                            the challenger agreed; else nothing (the gate keeps denying anyway)
  needs-more-evidence    -> nothing recorded
"""
import json
import os
import sys
from collections import Counter

S = os.environ.get("CAMPAIGN_DIR", os.getcwd())
JOURNAL, COHORT, BATCHES = sys.argv[1], sys.argv[2], sys.argv[3]
index = json.load(open(f"{BATCHES}/index.json"))
by_keys = {frozenset(b["keys"]): b for b in index}
cases = {c["case"]: c for c in json.load(open(f"{BATCHES}/cases.json"))}

reviews, challenges, samples = {}, {}, {}
unmapped = 0
for line in open(JOURNAL):
    d = json.loads(line)
    if d.get("type") != "result" or not isinstance(d.get("result"), dict):
        continue
    r = d["result"]
    if "verdicts" in r:
        keys = frozenset(v["case"] for v in r["verdicts"])
        b = by_keys.get(keys) or max(index, key=lambda x: len(keys & set(x["keys"])))
        if len(keys & set(b["keys"])) < max(1, len(b["keys"]) // 2):
            unmapped += 1
            continue
        (samples if b["stratum"] == "SAMPLE" else reviews)[b["id"]] = r["verdicts"]
    elif "reviews" in r:
        keys = frozenset(v["case"] for v in r["reviews"])
        b = by_keys.get(keys) or max(index, key=lambda x: len(keys & set(x["keys"])))
        challenges[b["id"]] = {v["case"]: v for v in r["reviews"]}
nrev = len([b for b in index if b["stratum"] != "SAMPLE"])
print(f"review batches: {len(reviews)}/{nrev} | challenges: {len(challenges)} | sample batches: {len(samples)}/{len(index) - nrev} | unmapped: {unmapped}")

joined = {}
for bid, verdicts in reviews.items():
    ch = challenges.get(bid, {})
    for v in verdicts:
        if v["case"] in cases:
            joined[v["case"]] = {"batch": bid, "review": v, "challenge": ch.get(v["case"])}
print(f"cases with a verdict: {len(joined)}/{len(cases)}")
print("verdicts:", dict(Counter((j["review"]["verdict"], j["review"]["confidence"]) for j in joined.values())))
agree = sum(1 for j in joined.values() if j["challenge"] and j["challenge"]["agree"])
print(f"challenger agreed on {agree}; disagreed on {sum(1 for j in joined.values() if j['challenge'] and not j['challenge']['agree'])}")
sample_v = {v["case"]: v for vs in samples.values() for v in vs}
n = same = 0
for k, sv in sample_v.items():
    if k in joined:
        n += 1
        same += sv["verdict"] == joined[k]["review"]["verdict"]
print(f"blind sample: {n} overlapping, same verdict {same}")

rows = []
for k, j in joined.items():
    rv, ch = j["review"], j["challenge"]
    disputed = ch is not None and not ch["agree"]
    c = cases[k]
    base = {"country": c["country"], "scheme": c["scheme"], "key": c["key"], "members": c["member_ids"],
            "rationale": f"[{rv['verdict']}/{rv['confidence']}{'; disputed: ' + ch['reason'][:200] if disputed else ''}; batch {j['batch']}] " + rv["rationale"][:1200]}
    if rv["verdict"] == "merge":
        conf = "high" if (rv["confidence"] == "high" and not disputed) else "medium"
        rows.append({**base, "action": "merge", "confidence": conf})
    elif rv["verdict"] == "keep" and rv["confidence"] in ("high", "medium") and not disputed:
        rows.append({**base, "action": "keep", "confidence": rv["confidence"]})
json.dump({"cohort": COHORT, "verdicts": rows}, open(f"{S}/362-post-body.json", "w"), ensure_ascii=False)
json.dump({"joined": joined, "samples": sample_v}, open(f"{S}/362-campaign.json", "w"), ensure_ascii=False)
print("POST body:", len(rows), dict(Counter((r["action"], r["confidence"]) for r in rows)))
