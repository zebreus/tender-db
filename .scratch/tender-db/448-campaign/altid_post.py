"""Issue 448 unit 4: turn the review campaign's results into a POST /admin/merge-verdicts body.

usage: altid_post.py <workflow-result.json> <cases.json> <cohort> <out-body.json>

Rules (the safe direction is NOT merging; a keep on a planned pair only holds it out):
  plan pair, reviewer keep|needs-more-evidence:
      challenger agrees           -> keep, reviewer's confidence (needs-more-evidence -> low)
      challenger says merge       -> keep low  (held out; re-post the same cohort as merge to release it)
  plan pair, reviewer merge       -> nothing (the plan merges it)
  denied/conflict pair, reviewer merge/high AND challenger agrees -> merge high (admitted)
  anything else on a denied pair  -> nothing (the gate keeps denying)
"""
import json, sys, collections

res_path, cases_path, cohort, out = sys.argv[1:5]
results = json.load(open(res_path))
cases = {c["case"]: c for c in json.load(open(cases_path))}
verdicts, stats = [], collections.Counter()
seen = set()
for r in results:
    if not r:
        continue
    review = {v["case"]: v for v in (r.get("review") or {}).get("verdicts", [])}
    challenge = {v["case"]: v for v in ((r.get("challenge") or {}).get("reviews") or [])}
    for key, v in review.items():
        c = cases.get(key)
        if c is None or key in seen:
            stats["unknown-or-dup"] += 1
            continue
        seen.add(key)
        ch = challenge.get(key)
        members = sorted(m["org"] for m in c["members"])
        tag = f"[{v['verdict']}/{v['confidence']}"
        if c["kind"] == "plan":
            if v["verdict"] == "merge":
                stats["plan:merge"] += 1
                continue
            conf = "low" if v["verdict"] == "needs-more-evidence" else v["confidence"]
            if ch is None:
                stats["plan:hold-unchallenged"] += 1
                conf = "low"
                tag += "; unchallenged"
            elif not ch["agree"]:
                stats["plan:hold-disputed"] += 1
                conf = "low"
                tag += f"; challenger: {ch['verdict']}/{ch['confidence']}"
            else:
                stats[f"plan:hold-{conf}"] += 1
            verdicts.append({"country": "GB", "scheme": "GB:altid", "key": key, "members": members,
                             "action": "keep", "confidence": conf,
                             "rationale": f"{tag}] {v['rationale']}" + (f" | challenger: {ch['rationale']}" if ch else "")})
        else:
            if v["verdict"] == "merge" and v["confidence"] == "high" and ch and ch["agree"]:
                stats[f"{c['kind']}:admit"] += 1
                verdicts.append({"country": "GB", "scheme": "GB:altid", "key": key, "members": members,
                                 "action": "merge", "confidence": "high",
                                 "rationale": f"{tag}; challenger agreed] {v['rationale']} | challenger: {ch['rationale']}"})
            else:
                stats[f"{c['kind']}:{v['verdict']}/{v['confidence']}" + ("" if not ch else ("/agreed" if ch["agree"] else "/disputed"))] += 1
missing = [k for k in cases if k not in seen]
print(dict(stats), "| verdicts", len(verdicts), "| cases without a review", len(missing))
json.dump({"cohort": cohort, "verdicts": verdicts}, open(out, "w"), indent=1)
json.dump(missing, open(out + ".missing.json", "w"))
