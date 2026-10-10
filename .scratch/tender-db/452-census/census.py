#!/usr/bin/env python3
"""The GB company-number census (issue 452, made re-runnable by issue 466).

Joins a Companies House "BasicCompanyDataAsOneFile" snapshot against GB `national`
orgs and splits the company-number orgs three ways:

  match     the number is a live company and the org's HEAD name matches its
            current or a previous register name (448's matcher);
  mismatch  the number is a live company and the head matches none of its names;
  absent    the number is not in the snapshot (dissolved, or never issued).

Of the mismatches, the ones whose head shares no distinctive token with any register
name and is no glued-word or accent variant of one are the "live-name-disjoint"
candidates, the reviewer + challenger cases of 452.

Input orgs are JSON lines `[identifier, org_id, head_name]` (452's `page_orgs.sh`
writes them; 466's incremental run pages the public listing above a watermark).
A company-number shape, reproducing 452's count of 30,857 orgs / 30,844 numbers on
the 2026-09-30 input exactly:
  `GBCOH` followed by any 8 characters of [A-Z0-9], or
  a bare 8 digits, or a bare 2 letters + 6 digits.

Usage:
  census.py SNAPSHOT.zip ORGS.jsonl OUT_DIR [--min-org-id N] [--pad-short] [--skip-verdicts verdicts.json]

Writes OUT_DIR/counts.json, OUT_DIR/live-name-disjoint-candidates.json and
OUT_DIR/absent.json, in 452's shapes.
"""
import csv, io, json, os, re, sys, unicodedata, zipfile

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "448-campaign"))
from altid_cases import core, matches  # noqa: E402  (448's matcher, imported, not copied)

ALNUM8 = re.compile(r"[A-Z0-9]{8}")
BARE = re.compile(r"\d{8}|[A-Z]{2}\d{6}")


def company_number(identifier, pad_short=False):
    """The company number an org identifier carries, or None (452's shape rule).

    `pad_short` (issue 466) also reads a `GBCOH` number of 6 or 7 digits as the company
    number with its leading zeros restored (`GBCOH3433043` is company 03433043). 452's rule
    left those out: 1,291 seven-digit and 117 six-digit `GBCOH` orgs on the 2026-09-30
    input were never checked. Off by default, so the 452 reproduction stays exact."""
    if identifier.startswith("GBCOH"):
        rest = identifier[5:]
        if pad_short and rest.isdigit() and 6 <= len(rest) < 8:
            rest = rest.zfill(8)
        return rest if ALNUM8.fullmatch(rest) else None
    return identifier if BARE.fullmatch(identifier) else None


def load_register(zip_path, wanted):
    """{number: {"name", "status", "previous"}} for the wanted numbers only."""
    out = {}
    with zipfile.ZipFile(zip_path) as z:
        member = next(n for n in z.namelist() if n.lower().endswith(".csv"))
        with z.open(member) as raw:
            reader = csv.reader(io.TextIOWrapper(raw, encoding="utf-8", newline=""))
            header = [h.strip() for h in next(reader)]
            col = {h: i for i, h in enumerate(header)}
            prev = [col[f"PreviousName_{k}.CompanyName"] for k in range(1, 11)]
            for row in reader:
                if len(row) < len(header):
                    row = row + [""] * (len(header) - len(row))
                number = row[col["CompanyNumber"]].strip()
                if number not in wanted:
                    continue
                out[number] = {
                    "name": row[col["CompanyName"]].strip(),
                    "status": row[col["CompanyStatus"]].strip(),
                    "previous": [row[i].strip() for i in prev if row[i].strip()],
                }
    return out


def folded(name):
    """`core` after stripping accents, so Acumé and ACUME compare equal."""
    s = unicodedata.normalize("NFKD", name or "")
    return core("".join(ch for ch in s if not unicodedata.combining(ch)), loose=True)


GENERIC = {
    "and", "of", "for", "in", "on", "at", "to", "by", "a", "an",
    "limited", "ltd", "plc", "llp", "lp", "cic", "company", "co", "the", "uk", "u", "k",
    "group", "holdings", "services", "service", "solutions", "international", "management",
}


def tokens(name):
    return {t for t in folded(name).split() if len(t) >= 3 and t not in GENERIC}


def related_spelling(head, register):
    """Whether a mismatched head is still a spelling of a register name: it shares a
    distinctive token (3+ letters, not a legal form or a generic word), or its glued
    form equals a register name's, extends it (`BT Business…` over `BT`), is a 5+ letter
    prefix of it (`Torus` / `TORUS62`), or shares a 5+ letter ending with it
    (`The Logically` / `THELOGICALLY`). A head with no distinctive token at all (`CiC`,
    `EY UK LLP`) is not judged disjoint: there is nothing to compare.

    Reproduces 452's 354 live-name-disjoint candidates on the 2026-09-01 snapshot to
    within 3 (2 more, 1 fewer; recorded on issue 466)."""
    ht = tokens(head)
    if not ht:
        return True
    hg = folded(head).replace(" ", "")
    for r in register:
        if ht & tokens(r):
            return True
        rg = folded(r).replace(" ", "")
        if not hg or not rg:
            continue
        if hg == rg or (len(rg) >= 2 and hg.startswith(rg)) or (len(hg) >= 5 and rg.startswith(hg)):
            return True
        if min(len(hg), len(rg)) >= 5 and (hg.endswith(rg) or rg.endswith(hg)):
            return True
    return False


def main(argv):
    zip_path, orgs_path, out_dir = argv[1:4]
    min_org = 0
    pad_short = False
    skip = set()
    rest = argv[4:]
    while rest:
        flag = rest.pop(0)
        if flag == "--min-org-id":
            min_org = int(rest.pop(0))
        elif flag == "--pad-short":
            pad_short = True
        elif flag == "--skip-verdicts":
            # A triple that already carries a verdict is not re-reviewed (466 unit 3).
            for v in json.load(open(rest.pop(0))):
                skip.add((v.get("org") or v.get("org_id"), v.get("number") or v.get("identifier")))
        else:
            raise SystemExit(f"unknown flag {flag}")
    orgs = []
    for line in open(orgs_path):
        identifier, org, head = json.loads(line)
        number = company_number(identifier, pad_short)
        if number and org > min_org and (org, number) not in skip:
            orgs.append((org, number, head))
    register = load_register(zip_path, {n for _, n, _ in orgs})
    counts = {"match": 0, "mismatch": 0, "absent": 0}
    disjoint, absent, mismatch = [], [], []
    for org, number, head in sorted(orgs, key=lambda o: (o[1], o[0])):
        reg = register.get(number)
        if reg is None:
            counts["absent"] += 1
            absent.append({"org": org, "number": number, "head": head})
            continue
        names = [reg["name"]] + reg["previous"]
        if matches(head, names):
            counts["match"] += 1
            continue
        counts["mismatch"] += 1
        mismatch.append({"org": org, "number": number, "head": head, "register": reg})
        if not related_spelling(head, names):
            disjoint.append({"org": org, "number": number, "head": head, "register": reg})
    os.makedirs(out_dir, exist_ok=True)
    json.dump(counts, open(os.path.join(out_dir, "counts.json"), "w"), indent=1)
    json.dump(disjoint, open(os.path.join(out_dir, "live-name-disjoint-candidates.json"), "w"), indent=0, ensure_ascii=False)
    json.dump(absent, open(os.path.join(out_dir, "absent.json"), "w"), indent=0, ensure_ascii=False)
    json.dump(mismatch, open(os.path.join(out_dir, "mismatch.json"), "w"), indent=0, ensure_ascii=False)
    print(json.dumps({"orgs": len(orgs), "numbers": len({n for _, n, _ in orgs}), **counts, "disjoint": len(disjoint)}))


if __name__ == "__main__":
    main(sys.argv)
