"""Post-process the xb-country-review campaign (issue 355).

Reads the workflow journal (one result per agent, unlabelled) and maps every
result to its batch by ROOT SET: a review/challenge result covers exactly one
batch file's roots, a blind sample result exactly one sample file's roots.

Outputs (in the scratchpad):
  355-campaign.json     everything joined per case
  355-post-body.json    the POST /admin/country-verdicts body: HIGH moves the
                        challenger did not dispute, `from` re-validated against
                        the packet's live member country; medium/low and the
                        disputed ones are recorded too (never applied).
"""
import json
import os
import sys
from collections import Counter, defaultdict

S = os.environ.get("CAMPAIGN_DIR", os.getcwd())  # where the cases file lives and the outputs go
JOURNAL = sys.argv[1]
COHORT = sys.argv[2] if len(sys.argv) > 2 else "xb-country-2026-09-05"
BATCHES = sys.argv[3] if len(sys.argv) > 3 else f"{S}/355-batches-v2"

index = json.load(open(f"{BATCHES}/index.json"))
by_roots = {frozenset(b["roots"]): b for b in index}
cases = {c["root"]: c for c in json.load(open(f"{S}/xb-cases.json"))}
member_country = {m["org"]: m.get("country") for c in cases.values() for m in c["members"]}
member_case = {m["org"]: c["root"] for c in cases.values() for m in c["members"]}

reviews, challenges, samples = {}, {}, {}
unmapped = 0
for line in open(JOURNAL):
    d = json.loads(line)
    if d.get("type") != "result" or not isinstance(d.get("result"), dict):
        continue
    r = d["result"]
    if "verdicts" in r:
        roots = frozenset(v["root"] for v in r["verdicts"])
        b = by_roots.get(roots)
        if b is None:  # a reviewer that dropped or invented a root: match by best overlap
            b = max(index, key=lambda x: len(roots & set(x["roots"])))
            if len(roots & set(b["roots"])) < max(1, len(b["roots"]) // 2):
                unmapped += 1
                continue
        (samples if b["stratum"] == "SAMPLE" else reviews)[b["id"]] = r["verdicts"]
    elif "reviews" in r:
        roots = frozenset(v["root"] for v in r["reviews"])
        b = by_roots.get(roots) or max(index, key=lambda x: len(roots & set(x["roots"])))
        challenges[b["id"]] = {v["root"]: v for v in r["reviews"]}

nrev = len([b for b in index if b["stratum"] != "SAMPLE"]); nsam = len(index) - nrev
print(f"review batches: {len(reviews)}/{nrev} | challenges: {len(challenges)} | sample batches: {len(samples)}/{nsam} | unmapped results: {unmapped}")

# ---- join per case
joined = {}
for bid, verdicts in reviews.items():
    stratum = next(b["stratum"] for b in index if b["id"] == bid)
    ch = challenges.get(bid, {})
    for v in verdicts:
        c = ch.get(v["root"])
        joined[v["root"]] = {"batch": bid, "stratum": stratum, "review": v, "challenge": c}
missing = [r for r in cases if r not in joined]
print(f"cases with a verdict: {len(joined)}/487 | missing: {len(missing)}")

# ---- stats
vd = Counter((j["stratum"], j["review"]["verdict"]) for j in joined.values())
print("verdicts by stratum:")
for k in sorted(vd):
    print(f"  {k[0]:28s} {k[1]:30s} {vd[k]}")
moves = [(j, m) for j in joined.values() for m in j["review"]["country_moves"]]
print("moves:", len(moves), "| by confidence:", dict(Counter(m["confidence"] for _, m in moves)),
      "| by evidence:", dict(Counter(m["evidence"] for _, m in moves)))
disputed = set()
missed = 0
no_challenge = 0
for j in joined.values():
    c = j["challenge"]
    if c is None:
        no_challenge += 1
        continue
    for dm in c.get("disputed_moves", []):
        disputed.add((j["review"]["root"], dm["org"]))
    if c.get("missed_move"):
        missed += 1
agree = sum(1 for j in joined.values() if j["challenge"] and j["challenge"]["agree"])
print(f"challenger: agree {agree}, disputed moves {len(disputed)}, missed-move flags {missed}, cases without a challenge {no_challenge}")

# ---- blind sample agreement
sample_v = {v["root"]: v for vs in samples.values() for v in vs}
same_verdict = same_moves = n = 0
diffs = []
for root, sv in sample_v.items():
    j = joined.get(root)
    if not j:
        continue
    n += 1
    rv = j["review"]
    if sv["verdict"] == rv["verdict"]:
        same_verdict += 1
    a = {(m["org"], m["to"]) for m in sv["country_moves"] if m["confidence"] == "high"}
    b = {(m["org"], m["to"]) for m in rv["country_moves"] if m["confidence"] == "high"}
    if a == b:
        same_moves += 1
    else:
        diffs.append({"root": root, "sample": sorted(a), "review": sorted(b), "sample_verdict": sv["verdict"], "review_verdict": rv["verdict"]})
print(f"blind sample: {n} overlapping cases, same verdict {same_verdict}, same HIGH move set {same_moves}")
for d in diffs[:12]:
    print("  diff:", d)

# ---- the POST body
SHARED_CHECKSUM = {frozenset({"CZ", "SK"}), frozenset({"CZ", "SI"}), frozenset({"SK", "SI"})}
SHARED_REGISTER = {frozenset({"FI", "AX"}), frozenset({"DK", "FO"}), frozenset({"DK", "GL"}), frozenset({"NO", "SJ"})} | {frozenset({"FR", x}) for x in ("RE", "GP", "MQ", "GF", "YT", "NC", "PF", "PM", "BL", "MF", "WF")} | {frozenset({"NL", x}) for x in ("AW", "CW", "SX", "BQ")}
verdict_rows = []
skipped_from = 0
floored = []
for j in joined.values():
    root = j["review"]["root"]
    for m in j["review"]["country_moves"]:
        org = m["org"]
        if member_case.get(org) != root:
            skipped_from += 1
            continue
        live = member_country.get(org)
        if m.get("from") != live:
            skipped_from += 1
            continue
        conf = m["confidence"]
        if (root, org) in disputed and conf == "high":
            conf = "medium"  # a disputed high is recorded, never applied
        # The rubric's hard rules as a deterministic floor (the deny direction):
        # an agreeing row never moves; weight alone never moves; a row with
        # standing (>= 30 mentions) moves only on arithmetic; the destination
        # must be a sibling's country unless arithmetic/format names it.
        mem = next(x for x in cases[root]["members"] if x["org"] == org)
        sibling_cc = {x["country"] for x in cases[root]["members"] if x["org"] != org}
        floor = None
        if mem.get("country_agrees"):
            floor = "agreeing-row"
        elif m["evidence"] == "identical-identifier-weight" and not any(
            a.split(":")[0] == m["to"] for x in cases[root]["members"] for a in x.get("anchors", [])
        ):
            floor = "weight-without-any-anchor"
        elif mem["mentions"] >= 30 and m["evidence"] != "arithmetic":
            floor = "row-has-standing"
        elif m["evidence"] == "arithmetic" and not any(a.split(":")[0] == m["to"] for a in mem.get("anchors", [])):
            floor = "arithmetic-claimed-but-no-anchor-for-to"
        elif {m["from"], m["to"]} in SHARED_REGISTER:
            # One register serves both codes (Åland is in the Finnish trade
            # register; Réunion, Guadeloupe, Martinique, Guiana, Mayotte carry
            # SIREN; the Faroes and Greenland sit under the Danish CVR): a
            # validating number cannot separate the tags, and which code such
            # a row should carry is a policy question, not a contamination.
            floor = "shared-register"
        elif m["evidence"] == "arithmetic" and not mem.get("country_probed") and {m["from"], m["to"]} in SHARED_CHECKSUM:
            # CZ and SK IČO (and SI davčna) share one mod-11 rule and the
            # vocabulary carries no SK scheme: a CZ anchor on an unprobed SK
            # row is silence plus a coincidence-proof hit, not a discriminator
            # (issue 314's own note; the challengers re-found it).
            floor = "shared-checksum-family"
        elif m["to"] not in sibling_cc and m["evidence"] not in ("arithmetic", "national-format"):
            floor = "third-country-on-soft-evidence"
        if floor and conf == "high":
            conf = "medium"
            floored.append((root, org, floor))
        verdict_rows.append({
            "org_id": org, "action": "move", "from_country": live, "to_country": m["to"],
            "rationale": f"[{m['evidence']}; case {root}; batch {j['batch']}] " + m["rationale"][:900],
            "confidence": conf,
        })
seen = set()
dedup = []
for r in verdict_rows:
    if r["org_id"] in seen:
        continue
    seen.add(r["org_id"])
    dedup.append(r)
body = {"cohort": COHORT, "verdicts": dedup}
json.dump(body, open(f"{S}/355-post-body.json", "w"), ensure_ascii=False)
json.dump({"joined": joined, "samples": sample_v, "missing": missing}, open(f"{S}/355-campaign.json", "w"), ensure_ascii=False)
print("floored highs:", len(floored), dict(Counter(f for _, _, f in floored)))
print(f"POST body: {len(dedup)} verdict rows ({dict(Counter(r['confidence'] for r in dedup))}); moves whose org/from did not match the packet: {skipped_from}")
hi = [r for r in dedup if r["confidence"] == "high"]
print("HIGH moves by (from -> to):", Counter(f"{r['from_country']}->{r['to_country']}" for r in hi).most_common(15))
