"""Issue 452: turn the census verdicts into a POST /admin/identifier-verdicts body.

usage: verdict_post.py <verdicts.json> <org-rows.json> <cohort> <out-body.json>

<org-rows.json> is {org_id: identifier} as the live rows carry it (a bounded
/v1/sql id lookup); the POST names the literal exactly, so a verdict is live only
while the org still carries that number.

Posted: wrong-number -> wrong, related-company -> related, right-number -> right
(all challenger-agreed; `right` changes nothing but records the check). Not
posted: unclear and every disputed verdict.
"""
import json, sys, collections

verdicts_path, rows_path, cohort, out = sys.argv[1:5]
rows = {int(k): v for k, v in json.load(open(rows_path)).items()}
MAP = {"wrong-number": "wrong", "related-company": "related", "right-number": "right"}
body, stats = [], collections.Counter()
for v in json.load(open(verdicts_path)):
    verdict = MAP.get(v["verdict"])
    if verdict is None:
        stats["skipped:" + v["verdict"]] += 1
        continue
    literal = rows[v["org"]]
    correct = v.get("correct_number")
    if not correct or correct == v["number"] or verdict == "right":
        correct = None
    rationale = f"[{v['verdict']}/{v['confidence']}; {v['kind']}] {v['rationale']} | challenger: {v.get('challenge') or ''}"
    body.append({
        "org_id": v["org"],
        "identifier": literal,
        "verdict": verdict,
        "correct_identifier": correct,
        "rationale": rationale[:4000],
        "confidence": v["confidence"],
    })
    stats[verdict] += 1
print(dict(stats), "| verdicts", len(body))
json.dump({"cohort": cohort, "verdicts": body}, open(out, "w"), indent=1)
