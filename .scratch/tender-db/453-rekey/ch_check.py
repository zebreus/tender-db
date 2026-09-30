#!/usr/bin/env python3
"""Issue 453 plan review: read each planned re-key's wrong and right company numbers
off the public Companies House register and compare them with the org's names.

    python3 ch_check.py rekey-plan.json > ch-check.json

Input is the stored `rekey-plan` report body. For each listing in `plan` (and in
`denied`, for context) it fetches the register name and status of the right number
and of the wrong number, and scores the org's head name (and the merge target's
head name) against them with a token Jaccard over legal-form-stripped words.

A planned re-key reads SUPPORTED when the right number's register name agrees with
the org's name (score >= 0.5) and the wrong number's does not (it names some other
company or none). Everything else is listed for a human-grade read.
"""
import html
import json
import re
import sys
import time
import urllib.request

FORMS = {"ltd", "limited", "plc", "llp", "lp", "the", "and", "co", "company", "uk", "cic", "group", "holdings"}


def words(name):
    name = html.unescape(name or "").lower().replace("&", " and ")
    return {w for w in re.split(r"[^a-z0-9]+", name) if w and w not in FORMS}


def score(a, b):
    wa, wb = words(a), words(b)
    if not wa or not wb:
        return 0.0
    return len(wa & wb) / len(wa | wb)


def body(number):
    n = re.sub(r"[^A-Z0-9]", "", number.upper())
    for prefix in ("GBCOH", "GB"):
        if n.startswith(prefix) and len(n) - len(prefix) == 8:
            n = n[len(prefix):]
    return n


CACHE = {}


def register(number):
    n = body(number)
    if n in CACHE:
        return CACHE[n]
    url = f"https://find-and-update.company-information.service.gov.uk/company/{n}"
    out = {"number": n, "name": None, "status": None, "http": None}
    for attempt in range(3):
        try:
            with urllib.request.urlopen(url, timeout=30) as r:
                page = r.read().decode("utf-8", "replace")
                out["http"] = r.status
            m = re.search(r'<h1 class="heading-xlarge"[^>]*>([^<]*)</h1>', page)
            out["name"] = html.unescape(m.group(1).strip()) if m else None
            m = re.search(r'id="company-status">\s*([^<]*?)\s*<', page)
            out["status"] = m.group(1).strip() if m else None
            break
        except urllib.error.HTTPError as e:
            out["http"] = e.code
            if e.code == 404:
                break
            time.sleep(2 * (attempt + 1))
        except Exception as e:  # network hiccup
            out["http"] = str(e)
            time.sleep(2 * (attempt + 1))
    time.sleep(0.4)
    CACHE[n] = out
    return out


def check(listing):
    right = register(listing["right"])
    wrong = register(listing["wrong"])
    name = listing.get("name") or ""
    target = listing.get("target") or {}
    s_right = score(name, right["name"])
    s_wrong = score(name, wrong["name"])
    s_target = score(target.get("name"), right["name"]) if target else None
    supported = s_right >= 0.5 and s_wrong < 0.5 and (s_target is None or s_target >= 0.5)
    return {
        "key": listing["key"],
        "shape": listing["shape"],
        "org": listing["org"],
        "name": name,
        "target": target or None,
        "right_register": right,
        "wrong_register": wrong,
        "score_right": round(s_right, 2),
        "score_wrong": round(s_wrong, 2),
        "score_target": None if s_target is None else round(s_target, 2),
        "supported": supported,
    }


def main():
    plan = json.load(open(sys.argv[1]))
    rows = [check(l) for l in plan.get("plan", [])]
    denied = [check(l) for l in plan.get("denied", [])]
    json.dump(
        {
            "planned": len(rows),
            "supported": sum(r["supported"] for r in rows),
            "rows": rows,
            "denied": denied,
        },
        sys.stdout,
        indent=1,
    )


if __name__ == "__main__":
    main()
