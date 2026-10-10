#!/usr/bin/env python3
"""A register digest of a rekey plan, for its adversarial review (issues 453, 466).

usage: rekey_digest.py PLAN.json SNAPSHOT.zip OUT.json VERDICTS.json [VERDICTS.json ...]

PLAN.json is `/admin/reports/rekey-plan` as fetched. VERDICTS.json are census verdict
files (452/466 shape: org, number, verdict, confidence, correct_number, rationale,
challenge). For every planned row it writes the wrong and right numbers' register
entries from the snapshot, the verdict that drove it, and both orgs' mention names WITH
the raw identifier each mention carried (`organization_mentions`, bounded per-org reads
through /root/sq.sh; run on the box). The raw identifiers are the issue-466 review's
lesson: a related org can hold a mention of the related company under that company's
own number, and a merge carries it along (Milestone 16782120 held M Group (Services)'s
award), which a head-to-head name pair does not show.
"""
import json, os, subprocess, sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "452-census"))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "448-campaign"))
from census import company_number, load_register  # noqa: E402


def sql(q):
    p = subprocess.run(["/root/sq.sh"], input=q.encode(), capture_output=True)
    d = json.loads(p.stdout)
    if "error" in d:
        raise SystemExit(f"query failed: {d['error']}")
    return d["rows"]


def mentions(orgs):
    out = {}
    orgs = sorted(set(orgs))
    for i in range(0, len(orgs), 50):
        chunk = orgs[i:i + 50]
        rows = sql(
            "SELECT organization_id, name, raw_identifier, count(*) FROM organization_mentions "
            "WHERE organization_id IN (" + ",".join(str(o) for o in chunk) + ") "
            "GROUP BY organization_id, name, raw_identifier LIMIT 20000"
        )
        for org, name, raw, n in rows:
            out.setdefault(org, []).append({"name": name, "raw_identifier": raw, "mentions": n})
    for v in out.values():
        v.sort(key=lambda m: -m["mentions"])
    return out


def register_number(literal):
    return company_number(literal, pad_short=True) or company_number("GBCOH" + literal, pad_short=True)


def main(argv):
    plan_path, zip_path, out_path = argv[1:4]
    p = json.load(open(plan_path))
    body = p.get("body", p)
    if isinstance(body, str):
        body = json.loads(body)
    verdicts = {}
    for path in argv[4:]:
        for v in json.load(open(path)):
            verdicts[(v["org"], v["number"])] = v
    rows = body["plan"]
    numbers = set()
    for r in rows:
        for lit in (r["wrong"], r["right"]):
            n = register_number(lit)
            if n:
                numbers.add(n)
    register = load_register(zip_path, numbers)
    orgs = [r["org"] for r in rows] + [r["target"]["org"] for r in rows if r.get("target")]
    named = mentions(orgs)
    out = []
    for r in rows:
        wn, rn = register_number(r["wrong"]), register_number(r["right"])
        v = next((x for (o, num), x in verdicts.items() if o == r["org"] and register_number(num) == wn), None)
        target = r.get("target")
        out.append({
            "key": r["key"],
            "shape": r["shape"],
            "org": r["org"],
            "head": r["name"],
            "wrong": r["wrong"],
            "right": r["right"],
            "org_mentions": named.get(r["org"], [])[:25],
            "target": dict(target, mentions=named.get(target["org"], [])[:25]) if target else None,
            "register_wrong": register.get(wn, "not in the 2026-10-01 live snapshot"),
            "register_right": register.get(rn, "not in the 2026-10-01 live snapshot"),
            "verdict": {k: v.get(k) for k in ("verdict", "confidence", "correct_number", "rationale", "challenge")} if v else None,
        })
    json.dump(out, open(out_path, "w"), indent=0, ensure_ascii=False)
    print(len(out), "rows;", sum(1 for x in out if x["verdict"] and x["verdict"]["verdict"] == "related-company"), "related")


if __name__ == "__main__":
    main(sys.argv)
