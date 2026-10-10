#!/usr/bin/env python3
"""Review cases for an incremental census (issue 466 unit 4), in 452's case shape.

usage: build_cases.py OUT.json PAGES.json VERDICTS.json RUN_DIR [RUN_DIR ...] [--residue]

From each census RUN_DIR (census.py output):
  live-mismatch   every live-name-disjoint candidate;
  never-issued    an absent number whose register page is a 404;
  absent-mismatch an absent number whose page names none of the org's head (448's matcher
                  and census.related_spelling);
  page-unread     an absent number whose page did not answer (403 etc.), kept as a case
                  with no register entry so a reviewer can fetch it.
An absent number whose page matches the head is right and makes no case. A (org, number)
that already carries a verdict in VERDICTS.json (452's 720) makes no case.

--residue adds 452's unposted verdicts (unclear, disputed:*) as cases again, minus 8805068
(a head-election question, issue 456), each carrying the earlier rationale and challenge.

Mention names come from `organization_mentions` by bounded per-org reads through
/root/sq.sh (run on the box).
"""
import json, os, subprocess, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "448-campaign"))
from altid_cases import matches  # noqa: E402
from census import related_spelling  # noqa: E402


def sql(q):
    p = subprocess.run(["/root/sq.sh"], input=q.encode(), capture_output=True)
    d = json.loads(p.stdout)
    if "error" in d:
        raise SystemExit(f"query failed: {d['error']}")
    return d["rows"]


def mention_names(orgs):
    out = {}
    orgs = sorted(set(orgs))
    for i in range(0, len(orgs), 100):
        chunk = orgs[i:i + 100]
        rows = sql(
            "SELECT organization_id, name, count(*) FROM organization_mentions WHERE organization_id IN ("
            + ",".join(str(o) for o in chunk) + ") GROUP BY organization_id, name LIMIT 20000"
        )
        for org, name, n in rows:
            out.setdefault(org, []).append({"name": name, "mentions": n})
    for v in out.values():
        v.sort(key=lambda m: -m["mentions"])
    return out


def main(argv):
    out_path, pages_path, verdicts_path = argv[1:4]
    run_dirs = [a for a in argv[4:] if not a.startswith("--")]
    residue = "--residue" in argv
    pages = json.load(open(pages_path))
    done = {(v["org"], v["number"]) for v in json.load(open(verdicts_path))}
    cases, seen = [], set()

    def add(c):
        key = (c["org"], c["number"])
        if key in done or key in seen:
            return
        seen.add(key)
        c["case"] = f"{c['org']}:{c['number']}"
        cases.append(c)

    for d in run_dirs:
        for r in json.load(open(os.path.join(d, "live-name-disjoint-candidates.json"))):
            reg = dict(r["register"], source="live snapshot")
            add({"kind": "live-mismatch", "org": r["org"], "number": r["number"], "head": r["head"], "register": reg})
        for r in json.load(open(os.path.join(d, "absent.json"))):
            page = pages.get(r["number"], {})
            http = page.get("http")
            if http == "404":
                add({"kind": "never-issued", "org": r["org"], "number": r["number"], "head": r["head"],
                     "register": {"source": "register page", "http": "404"}})
            elif http == "200":
                names = [n for n in [page.get("name")] + page.get("previous", []) if n]
                if matches(r["head"], names) or related_spelling(r["head"], names):
                    continue
                add({"kind": "absent-mismatch", "org": r["org"], "number": r["number"], "head": r["head"],
                     "register": dict(page, source="register page")})
            else:
                add({"kind": "page-unread", "org": r["org"], "number": r["number"], "head": r["head"],
                     "register": {"source": "register page", "http": http}})
    if residue:
        for v in json.load(open(verdicts_path)):
            verdict = v.get("verdict", "")
            if not (verdict == "unclear" or verdict.startswith("disputed")) or v["org"] == 8805068:
                continue
            c = {"kind": "residue-" + v.get("kind", ""), "org": v["org"], "number": v["number"], "head": v["head"],
                 "earlier": {k: v.get(k) for k in ("verdict", "confidence", "correct_number", "rationale", "challenge")}}
            c["case"] = f"{c['org']}:{c['number']}"
            cases.append(c)
    names = mention_names([c["org"] for c in cases])
    for c in cases:
        c["mention_names"] = names.get(c["org"], [])[:15]
    json.dump(cases, open(out_path, "w"), indent=0, ensure_ascii=False)
    kinds = {}
    for c in cases:
        kinds[c["kind"]] = kinds.get(c["kind"], 0) + 1
    print(len(cases), kinds)


if __name__ == "__main__":
    main(sys.argv)
