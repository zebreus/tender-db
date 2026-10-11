"""Issue 469 U2: join the register-sample read (reviewer + challenger) with the org facts and apply
stop rules S2 and S3 (unit2-decision.md). Usage: python3 u2_synth.py <workflow journal.jsonl>
Writes u2-read.json (one record per pair) and prints the tallies."""
import collections
import json
import sys

HERE = sys.path[0]
sample = json.load(open(f"{HERE}/registry-sample-2163.json"))
facts = json.load(open(f"{HERE}/org-facts.json"))

# The journal: `started` carries the label, `result` the agent's structured output, joined by key.
labels, results = {}, {}
for line in open(sys.argv[1]):
    e = json.loads(line)
    if e.get("type") == "started":
        labels[e["key"]] = e.get("label")
    elif e.get("type") == "result":
        results[e["key"]] = e.get("result")
review, challenge = {}, {}
for key, label in labels.items():
    r = results.get(key)
    if not label or r is None:
        continue
    if label.startswith("review:"):
        for v in r.get("verdicts", []):
            review[v["case"]] = v
    elif label.startswith("challenge:"):
        for v in r.get("reviews", []):
            challenge[v["case"]] = v

ppon_values = collections.defaultdict(set)
for x in sample:
    reg, ppon = x["key"].rsplit("~", 1)
    ppon_values[ppon].add(reg)


def fact(oid):
    f = facts.get(str(oid)) or {}
    o = f.get("org") or {}
    return {
        "mentions": o.get("mentions"),
        "identifier": o.get("identifier"),
        "merged": [m["identifier"] for m in o.get("merged_identifiers") or []],
        "buyer": f.get("buyer"),
        "winner": f.get("winner"),
        "bidder": f.get("bidder"),
    }


def role(f):
    b, w = (f["buyer"] or 0), (f["winner"] or 0) + (f["bidder"] or 0)
    return "buyer" if b and not w else "supplier" if w and not b else "both" if b and w else "none"


records = []
for x in sample:
    key = x["key"]
    reg_m, ppon_m = x["members"][0], x["members"][1]
    fr, fp = fact(reg_m["org_id"]), fact(ppon_m["org_id"])
    rv, ch = review.get(key), challenge.get(key)
    ppon = key.rsplit("~", 1)[1]
    survivor = (fp["identifier"] or "").startswith("GBCOH") or any(m.startswith("GBCOH") for m in fp["merged"])
    # The structural gates U4 would apply, as far as one sample can show them.
    if rv and ch and "mislabelled" in (rv["register_verdict"], ch["register_verdict"]):
        structural = "registry-shape"
    elif survivor:
        structural = "triple"
    elif len(ppon_values[ppon]) > 1:
        structural = "cross-register"
    elif reg_m["identifier"] in (None, "None"):
        structural = "no-identity"
    else:
        structural = None
    agreed = bool(rv and ch and ch["agree"])
    if not rv or not ch:
        final = "missing"
    elif agreed:
        final = rv["verdict"]
    else:
        # A disputed pair takes the safer side: keep beats needs-more-evidence beats merge.
        vs = {rv["verdict"], ch["verdict"]}
        final = "keep" if "keep" in vs else "needs-more-evidence" if "needs-more-evidence" in vs else "merge"
    high = agreed and rv["confidence"] == "high" and ch["confidence"] == "high"
    confirmed = bool(rv and ch and rv["register_verdict"] == ch["register_verdict"] == "confirmed")
    cls = rv["class"] if agreed else (ch["class"] if ch else None)
    records.append({
        "key": key, "scheme": x["scheme"],
        "register_org": reg_m["org_id"], "ppon_org": ppon_m["org_id"],
        "register_head": reg_m["name"], "ppon_head": ppon_m["name"],
        "register_mentions": fr["mentions"], "ppon_mentions": fp["mentions"],
        "register_role": role(fr), "ppon_role": role(fp),
        "ppon_holder_448_survivor": survivor,
        "register_verdict": rv and rv["register_verdict"], "register_verdict_challenger": ch and ch["register_verdict"],
        "register_name": rv and rv.get("register_name"),
        "verdict": final, "high": high, "agreed": agreed, "class": cls,
        "reviewer": rv, "challenger": ch,
        "register_confirmed": confirmed, "structural_deny": structural,
    })

json.dump(records, open(f"{HERE}/u2-read.json", "w"), indent=1, ensure_ascii=False)

print(f"{len(records)} pairs; reviewed {len(review)}, challenged {len(challenge)}; "
      f"missing {sum(r['verdict'] == 'missing' for r in records)}")
for scheme in sorted({r["scheme"] for r in records}):
    rs = [r for r in records if r["scheme"] == scheme]
    print(f"\n== {scheme} ({len(rs)})")
    print("  verdict:", dict(collections.Counter(r["verdict"] for r in rs)))
    print("  agreed:", sum(r["agreed"] for r in rs), " high:", sum(r["high"] for r in rs))
    print("  register:", dict(collections.Counter(r["register_verdict"] for r in rs)))
    print("  class:", dict(collections.Counter(r["class"] for r in rs)))
    print("  structural:", dict(collections.Counter(r["structural_deny"] for r in rs)))
    print("  roles reg/ppon:", dict(collections.Counter((r["register_role"], r["ppon_role"]) for r in rs)))

non_nhs = [r for r in records if r["scheme"] != "GB-NHS"]
print("\n== S2 (non-NHS): wrong classes among keep / disputed pairs")
wrong = collections.Counter(r["class"] for r in non_nhs if r["verdict"] != "merge")
print("  ", dict(wrong))
for cls, n in wrong.items():
    if n >= 3 and cls not in ("none", "renamed-or-merged"):
        caught = collections.Counter(r["structural_deny"] for r in non_nhs if r["verdict"] != "merge" and r["class"] == cls)
        print(f"   S2 class {cls}: {n} — caught by structural gates: {dict(caught)}")

print("\n== S3 (non-NHS)")
plannable = [r for r in non_nhs if r["structural_deny"] is None]
unconf = [r for r in plannable if not r["register_confirmed"]]
keeps = [r for r in unconf if r["verdict"] == "keep"]
print(f"  plannable after structural gates: {len(plannable)} of {len(non_nhs)}")
print(f"  register-confirmed: {sum(r['register_confirmed'] for r in plannable)} of {len(plannable)} "
      f"({100 * sum(r['register_confirmed'] for r in plannable) / max(1, len(plannable)):.0f}%; S3 needs >= 50%)")
print(f"  keep rate among unconfirmed plannable: {len(keeps)} of {len(unconf)} "
      f"({100 * len(keeps) / max(1, len(unconf)):.1f}%; 448's bar 8.5%)")
all_keeps = [r for r in plannable if r["verdict"] == "keep"]
print(f"  keep rate among all plannable: {len(all_keeps)} of {len(plannable)}")
for r in all_keeps + [r for r in plannable if r["verdict"] == "needs-more-evidence"]:
    print(f"    {r['verdict']}: {r['key']} | {r['register_head'][:40]} || {r['ppon_head'][:40]} | {r['class']}")
