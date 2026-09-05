"""Post-process the cluster-country campaign (issue 357). Cases are keyed by
IDENTIFIER; results map to batches by identifier set. Same floors as 355 plus
the spray shape's own checks. Outputs 357-campaign.json and 357-post-body.json.
"""
import json
import os
import re
import sys
from collections import Counter

S = os.environ.get("CAMPAIGN_DIR", os.getcwd())  # where the cases file lives and the outputs go
JOURNAL = sys.argv[1]
COHORT = sys.argv[2] if len(sys.argv) > 2 else "cluster-country-2026-09-05"
BATCHES = sys.argv[3] if len(sys.argv) > 3 else f"{S}/357-batches"

index = json.load(open(f"{BATCHES}/index.json"))
by_keys = {frozenset(b["keys"]): b for b in index}
CASES = f"{BATCHES}/cases.json" if os.path.exists(f"{BATCHES}/cases.json") else f"{S}/357-cases.json"
cases = {c["identifier"]: c for c in json.load(open(CASES))}
member_country = {m["org"]: m.get("country") for c in cases.values() for m in c["members"]}
member_case = {m["org"]: c["identifier"] for c in cases.values() for m in c["members"]}

reviews, challenges, samples = {}, {}, {}
unmapped = 0
for line in open(JOURNAL):
    d = json.loads(line)
    if d.get("type") != "result" or not isinstance(d.get("result"), dict):
        continue
    r = d["result"]
    if "verdicts" in r:
        keys = frozenset(str(v["identifier"]) for v in r["verdicts"])
        b = by_keys.get(keys)
        if b is None:
            b = max(index, key=lambda x: len(keys & set(x["keys"])))
            if len(keys & set(b["keys"])) < max(1, len(b["keys"]) // 2):
                unmapped += 1
                continue
        (samples if b["stratum"] == "SAMPLE" else reviews)[b["id"]] = r["verdicts"]
    elif "reviews" in r:
        keys = frozenset(str(v["identifier"]) for v in r["reviews"])
        b = by_keys.get(keys) or max(index, key=lambda x: len(keys & set(x["keys"])))
        challenges[b["id"]] = {str(v["identifier"]): v for v in r["reviews"]}

nrev = len([b for b in index if b["stratum"] != "SAMPLE"])
nsam = len(index) - nrev
print(f"review batches: {len(reviews)}/{nrev} | challenges: {len(challenges)} | sample batches: {len(samples)}/{nsam} | unmapped: {unmapped}")

joined = {}
for bid, verdicts in reviews.items():
    stratum = next(b["stratum"] for b in index if b["id"] == bid)
    ch = challenges.get(bid, {})
    for v in verdicts:
        k = str(v["identifier"])
        joined[k] = {"batch": bid, "stratum": stratum, "review": v, "challenge": ch.get(k)}
missing = [k for k in cases if k not in joined]
print(f"cases with a verdict: {len(joined)}/{len(cases)} | missing: {len(missing)}")

vd = Counter((j["stratum"], j["review"]["verdict"]) for j in joined.values())
print("verdicts by stratum:")
for k in sorted(vd):
    print(f"  {k[0]:34s} {k[1]:30s} {vd[k]}")
moves = [(j, m) for j in joined.values() for m in j["review"]["country_moves"]]
print("moves:", len(moves), "| by confidence:", dict(Counter(m["confidence"] for _, m in moves)),
      "| by evidence:", dict(Counter(m["evidence"] for _, m in moves)))
disputed = set()
missed = no_challenge = 0
for j in joined.values():
    c = j["challenge"]
    if c is None:
        no_challenge += 1
        continue
    for dm in c.get("disputed_moves", []):
        disputed.add((str(j["review"]["identifier"]), dm["org"]))
    if c.get("missed_move"):
        missed += 1
agree = sum(1 for j in joined.values() if j["challenge"] and j["challenge"]["agree"])
print(f"challenger: agree {agree}, disputed moves {len(disputed)}, missed-move flags {missed}, cases without a challenge {no_challenge}")

sample_v = {str(v["identifier"]): v for vs in samples.values() for v in vs}
same_verdict = same_moves = n = 0
diffs = []
for k, sv in sample_v.items():
    j = joined.get(k)
    if not j:
        continue
    n += 1
    rv = j["review"]
    same_verdict += sv["verdict"] == rv["verdict"]
    a = {(m["org"], m["to"]) for m in sv["country_moves"] if m["confidence"] == "high"}
    b = {(m["org"], m["to"]) for m in rv["country_moves"] if m["confidence"] == "high"}
    if a == b:
        same_moves += 1
    else:
        diffs.append({"identifier": k, "sample": sorted(a), "review": sorted(b), "sample_verdict": sv["verdict"], "review_verdict": rv["verdict"]})
print(f"blind sample: {n} overlapping cases, same verdict {same_verdict}, same HIGH move set {same_moves}")
for d in diffs[:12]:
    print("  diff:", d)

SHARED_CHECKSUM = {frozenset({"CZ", "SK"}), frozenset({"CZ", "SI"}), frozenset({"SK", "SI"})}
SHARED_REGISTER = {frozenset({"FI", "AX"}), frozenset({"DK", "FO"}), frozenset({"DK", "GL"}), frozenset({"NO", "SJ"})} | {frozenset({"FR", x}) for x in ("RE", "GP", "MQ", "GF", "YT", "NC", "PF", "PM", "BL", "MF", "WF")} | {frozenset({"NL", x}) for x in ("AW", "CW", "SX", "BQ")}
BRANCH_WORDS = re.compile(r"клон|filial|sivukonttori|succursale|sucursal|niederlassung|zweigniederlassung|branch|sede secondaria|sede secundaria|oddzia|pobo[cč]ka|filiaal|filiale|rappresentanza|representa", re.I)
COUNTRY_WORDS = {"FI": "finland|suomi|suomessa", "SE": "sweden|sverige|schweden", "NO": "norway|norge|norwegen", "DK": "denmark|danmark|dänemark", "DE": "germany|deutschland|deutsche", "IT": "italy|italia|italien", "ES": "spain|españa|espana|spanien", "PT": "portugal", "FR": "france|frankreich", "PL": "poland|polska|polen", "NL": "netherlands|nederland|niederlande", "BE": "belgium|belgique|belgië|belgie", "AT": "austria|österreich", "CZ": "czech|česk", "SK": "slovak|slovensk", "HU": "hungary|magyar", "RO": "romania|românia", "BG": "bulgaria|българия", "GB": "united kingdom|uk\\b|britain", "IE": "ireland|éire", "LT": "lithuania|lietuv", "LV": "latvia|latvij", "EE": "estonia|eesti", "GR": "greece|ελλά", "HR": "croatia|hrvatsk", "SI": "slovenia|slovenij", "CH": "switzerland|schweiz|suisse", "LU": "luxembourg", "MD": "moldova"}
# A legal form or script that names ONE country (the ambiguous ones — S.A., AB, Ltd,
# s.r.o., S.R.L. — are left out on purpose): a heavy mover whose own name carries the
# DESTINATION's form is that country's company mis-tagged by foreign buyers, not a
# parent carrying a branch's number.
NAME_NAMES = {"LT": r"\bUAB\b|Uždaroji", "LV": r"\bSIA\b", "FI": r"\bOyj?\b", "DK": r"\bA/S\b|\bApS\b", "NO": r"\bASA\b",
              "PL": r"Sp\.? ?z ?o\.? ?o", "BG": r"\b(ЕАД|АД|ЕООД|ООД)\b|[\u0400-\u04FF]{4,}", "DE": r"\bGmbH\b|\bAG\b", "AT": r"\bGmbH\b",
              "NL": r"\bB\.?V\.?\b|\bN\.?V\.?\b", "IT": r"\bS\.?r\.?l\.?\b|\bS\.?p\.?A\.?\b", "FR": r"\bSAS\b|\bSARL\b",
              "PT": r"\bLda\b", "HU": r"\bKft\b|\bZrt\b", "EE": r"\bO[ÜüU]\b|osaühing", "CZ": r"\ba\.s\.\b", "RO": r"\bS\.?C\.?\s", "SE": r"\bAktiebolag\b", "ES": r"\bS\.?L\.?\b|[áéíóúñ]", "GR": r"[\u0370-\u03FF]{4,}"}
def NAME_NAMES_TO(mem, to, frm):
    name = " ".join([mem.get("name") or ""] + [v.get("name", "") for v in mem.get("variants", [])])
    names_to = bool(re.search(NAME_NAMES.get(to, "$^"), name))
    names_from = bool(re.search(NAME_NAMES.get(frm, "$^"), name))
    return names_to and not names_from
def NAME_NAMES_FROM_ONLY(mem, to, frm):
    name = " ".join([mem.get("name") or ""] + [v.get("name", "") for v in mem.get("variants", [])])
    return bool(re.search(NAME_NAMES.get(frm, "$^"), name)) and not re.search(NAME_NAMES.get(to, "$^"), name)
def BRANCH_OF_TO(mem, to):
    name = " ".join([mem.get("name") or ""] + [v.get("name", "") for v in mem.get("variants", [])])
    return bool(BRANCH_WORDS.search(name)) and bool(re.search(COUNTRY_WORDS.get(to, "$^"), name, re.I))
FOOTPRINT = re.compile(r"embassy|botschaft|ambassade|ambasada|consulate|konsulat|niederlassung|sivukonttori|branch|filial|sucursal|succursale|representative office|delegation", re.I)
# Hand adjudication (2026-09-05, slice 2): heavy movers read one by one — these five are the
# destination's own companies mis-tagged by foreign buyers and go back to HIGH.
HAND_HIGH = {22392645, 13525110, 23120710, 14659804, 5259766}
# Hand adjudication (slice 4): read against the blind reader and parked.
HAND_MEDIUM = {19099862}
verdict_rows, floored = [], []
skipped = 0
ok_code = lambda c: c is not None and re.fullmatch(r"[A-Z]{2}", c) is not None
for j in joined.values():
    k = str(j["review"]["identifier"])
    case = cases[k]
    heavy = max(case["members"], key=lambda m: m["mentions"])
    for m in j["review"]["country_moves"]:
        org = m["org"]
        if member_case.get(org) != k:
            skipped += 1
            continue
        live = member_country.get(org)
        if m.get("from") != live:
            skipped += 1
            continue
        conf = m["confidence"]
        if (k, org) in disputed and conf == "high":
            conf = "medium"
        mem = next(x for x in case["members"] if x["org"] == org)
        any_anchor_to = any(a.split(":")[0] == m["to"] for x in case["members"] for a in x.get("anchors", []))
        floor = None
        if org in HAND_MEDIUM:
            floor = "hand-read-parked"
        elif m["to"] == "ES" and re.match(r"^N\d{7}[A-Z0-9]$", (mem.get("identifier") or "").upper()):
            # A Spanish CIF starting with N is issued to a NON-RESIDENT foreign
            # entity: the row carrying it is a foreign company's own Spanish
            # tax number, not a Spanish company mis-tagged (slice 4, blind
            # reader vs reviewer on STEMCELL Technologies FR/ES).
            floor = "non-resident-cif"
        elif mem.get("country_agrees"):
            floor = "agreeing-row"
        elif FOOTPRINT.search(mem["name"] or "") and not BRANCH_OF_TO(mem, m["to"]):
            floor = "name-says-foreign-filing"
        elif {m["from"], m["to"]} in SHARED_REGISTER:
            floor = "shared-register"
        elif m["evidence"] == "arithmetic" and not any(a.split(":")[0] == m["to"] for a in mem.get("anchors", [])):
            floor = "arithmetic-claimed-but-no-anchor-for-to"
        elif m["evidence"] == "arithmetic" and not mem.get("country_probed") and {m["from"], m["to"]} in SHARED_CHECKSUM:
            floor = "shared-checksum-family"
        elif m["evidence"] == "identical-identifier-weight":
            to_mem = [x for x in case["members"] if x["country"] == m["to"]]
            to_mentions = max((x["mentions"] for x in to_mem), default=0)
            named_to = m["to"] in case.get("named", []) and any(not s.startswith("SE:") for s in case.get("named_schemes", []) if s.split(":")[0] == m["to"])
            if not to_mem:
                floor = "weight-to-a-code-with-no-row"
            elif mem["mentions"] > 2:
                floor = "weight-but-mover-not-a-stray"
            elif not (named_to or to_mentions >= 10 * max(1, mem["mentions"])):
                floor = "weight-without-named-or-10x"
        elif NAME_NAMES_FROM_ONLY(mem, m["to"], m["from"]) and not BRANCH_OF_TO(mem, m["to"]) and org not in HAND_HIGH:
            # The mover's own name carries ITS country's unambiguous legal form
            # and not the destination's (a French SAS, a German GmbH, a Belgian
            # NV): a registration in its own right carrying a foreign number —
            # the parent-with-branch-number shape at any weight.
            floor = "legal-form-names-own-country"
        elif mem["mentions"] >= 10 and not BRANCH_OF_TO(mem, m["to"]) and not NAME_NAMES_TO(mem, m["to"], m["from"]) and org not in HAND_HIGH:
            # A mover with real standing is a registration in its own right:
            # a parent carrying its branch's or subsidiary's number (Roland
            # Rechtsschutz DE with its Italian VAT id, Agfa Graphics NV with
            # its Polish NIP) is the rubric's same-entity-two-registrations
            # or a wrong IDENTIFIER — never a country move, which would fuse
            # the parent's mentions into the branch. Only a name that itself
            # says "branch in <to>" ("filial i Finland") overrides it.
            floor = "mover-has-standing"
        if floor and conf == "high":
            conf = "medium"
            floored.append((k, org, floor))
        if not ok_code(live) or not ok_code(m["to"]):
            # a junk pre-image is recordable since 356; a junk DESTINATION is not a move
            if not ok_code(m["to"]):
                skipped += 1
                continue
        verdict_rows.append({
            "org_id": org, "action": "move", "from_country": live, "to_country": m["to"],
            "rationale": f"[{m['evidence']}; cluster {k}; batch {j['batch']}] " + m["rationale"][:900],
            "confidence": conf,
        })
# EVERY reviewed cluster gets a `keep` on its heaviest member (unless that member
# is itself a mover — the move row, appended first, wins the dedup below), so the
# packet's already-reviewed skip sees it and the next slice carries new work. It
# used to be only clusters without a move: after the wet apply R2 folds the moved
# rows into the survivor, the move rows then point at merged-away orgs, the skip's
# JOIN organizations finds nothing, and the cluster came back in the next packet
# (17 of slice 6's 600; backfilled 2026-09-05 20:1x).
for j in joined.values():
    k = str(j["review"]["identifier"])
    heavy = max(cases[k]["members"], key=lambda m: m["mentions"])
    nmoves = len(j["review"]["country_moves"])
    tag = f"survivor; {nmoves} move(s) reviewed; " if nmoves else ""
    verdict_rows.append({
        "org_id": heavy["org"], "action": "keep", "from_country": heavy["country"], "to_country": None,
        "rationale": f"[{j['review']['verdict']}; {tag}cluster {k}; batch {j['batch']}] " + (j["review"]["rationale"] or "")[:600],
        "confidence": j["review"]["confidence"],
    })
seen, dedup = set(), []
for r in verdict_rows:
    if r["org_id"] in seen:
        continue
    seen.add(r["org_id"])
    dedup.append(r)
json.dump({"cohort": COHORT, "verdicts": dedup}, open(f"{S}/357-post-body.json", "w"), ensure_ascii=False)
json.dump({"joined": joined, "samples": sample_v, "missing": missing}, open(f"{S}/357-campaign.json", "w"), ensure_ascii=False)
print("floored highs:", len(floored), dict(Counter(f for _, _, f in floored)))
print(f"POST body: {len(dedup)} rows ({dict(Counter(r['confidence'] for r in dedup))}); skipped (org/from mismatch or junk destination): {skipped}")
hi = [r for r in dedup if r["confidence"] == "high" and r["action"] == "move"]
print("HIGH moves by (from -> to):", Counter(f"{r['from_country']}->{r['to_country']}" for r in hi).most_common(15))
print("HIGH by evidence:", Counter(r["rationale"].split(";")[0].strip("[") for r in hi))
