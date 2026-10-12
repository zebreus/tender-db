#!/usr/bin/env python3
"""Issue 514 U2.1, step 3: fetch the register record for every canonical number.

Reuses issue 469 U2's fetch methods (scratchpad u2fetch/{mpr,ukrlp,cc,oscr-ccni}):
- MPR   FCA Mutuals register: the register-wide SocietyList.csv, plus per society the website
        search, the society page, GetNameChanges and GetTransfers (mutuals.fca.org.uk).
- UKPRN UKRLP: /search/provider/<v> and the active / deactivated search lists.
- CHC   Charity Commission bulk JSON extract (publicextract.charity.zip and
        publicextract.charity_other_names.zip): every row whose registered_charity_number is one
        of ours, linked charities included.
- NIC   CCNI: the charity-details page, the CSV export and the search API.

Small responses are cached verbatim under registers/raw/<family>/. The three bulk files (57 MB,
~12 MB, 7 MB) are downloaded into fresh empty directories under DL (the session scratchpad, not
the repo) and only the matching records are copied out verbatim; registers/fetch-manifest.json
records url, http status, bytes, sha256 and time for every request.

Usage: python3 -I fetch_registers.py <download-base-dir>
"""
import csv
import hashlib
import io
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
REG = os.path.join(HERE, "registers")
RAWD = os.path.join(REG, "raw")
UA = "tender-db-research/1.0 (issue 514 register-spelling review; polite, ~1 req/s)"
FCA = "https://mutuals.fca.org.uk"
FCA_CSV = "https://fcastoragemprprod.blob.core.windows.net/societylist/SocietyList.csv"
UKRLP = "https://ukrlp.education.gov.uk"
CC_ZIP = "https://ccewuksprdoneregsadata1.blob.core.windows.net/data/json/publicextract.charity.zip"
CC_NAMES_ZIP = "https://ccewuksprdoneregsadata1.blob.core.windows.net/data/json/publicextract.charity_other_names.zip"
CCNI_PAGE = "https://www.charitycommissionni.org.uk/charity-search/charity-details-page/?regId={}&subId=0"
CCNI_CSV = "https://www.charitycommissionni.org.uk/api/charity-details/exportToCsv?regId={}&subId=0"
CCNI_API = ("https://www.charitycommissionni.org.uk/api/charity-search/results?"
            "homePageId=c0884e5e-b621-4563-a378-2b313e15c74a&searchText={}&pageNumber=1")

MANIFEST = []
_last = [0.0]


def get(url, dest, xhr=False, accept=None):
    """GET url into dest (a file path). Returns the manifest entry."""
    wait = 1.2 - (time.time() - _last[0])
    if wait > 0:
        time.sleep(wait)
    hdr = {"User-Agent": UA}
    if xhr:
        hdr["X-Requested-With"] = "XMLHttpRequest"
    if accept:
        hdr["Accept"] = accept
    code, err, final, body = None, None, url, b""
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers=hdr), timeout=300) as r:
            code, final = r.status, r.geturl()
            with open(dest, "wb") as f:
                h = hashlib.sha256()
                n = 0
                while True:
                    chunk = r.read(1 << 20)
                    if not chunk:
                        break
                    f.write(chunk)
                    h.update(chunk)
                    n += len(chunk)
            ent = dict(url=url, final=final, http=code, bytes=n, sha256=h.hexdigest(),
                       file=os.path.relpath(dest, HERE) if dest.startswith(HERE) else dest,
                       at=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()))
            _last[0] = time.time()
            MANIFEST.append(ent)
            return ent
    except urllib.error.HTTPError as e:
        code, body = e.code, e.read() or b""
    except Exception as e:  # network failure
        err = repr(e)
    _last[0] = time.time()
    with open(dest, "wb") as f:
        f.write(body)
    ent = dict(url=url, final=final, http=code, bytes=len(body), sha256=hashlib.sha256(body).hexdigest(),
               file=os.path.relpath(dest, HERE) if dest.startswith(HERE) else dest, error=err,
               at=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()))
    MANIFEST.append(ent)
    return ent


def fresh_dir(base, tag):
    n = 1
    while os.path.exists(os.path.join(base, "dl-%s-%d" % (tag, n))):
        n += 1
    d = os.path.join(base, "dl-%s-%d" % (tag, n))
    os.makedirs(d)
    return d


def numbers():
    g = json.load(open(os.path.join(HERE, "groups.json")))
    out = {"MPR": [], "UKPRN": [], "CHC": [], "NIC": []}
    for x in g["groups"] + g["lone_movers"]:
        n = x["number"]
        if x["scheme"] == "NIC":
            n = re.sub(r"^(NIC|NI)", "", n)
        if n not in out[x["scheme"]]:
            out[x["scheme"]].append(n)
    return {k: sorted(v) for k, v in out.items()}


# ---- MPR ----------------------------------------------------------------------------------------
def parse_search(path):
    import html
    t = open(path, encoding="utf-8", errors="replace").read()
    res = []
    for m in re.finditer(r'<a class="SearchLink" href="/Search/Society/(\d+)">(.*?)</a></div>\s*'
                         r'<div>Status:\s*(.*?)</div>\s*<div>Registration Number:\s*(.*?)</div>', t, re.S):
        res.append({"society_id": int(m.group(1)), "name": html.unescape(m.group(2)).strip(),
                    "status": html.unescape(m.group(3)).strip(),
                    "reg_no": re.sub(r"\s+", "", html.unescape(m.group(4)))})
    return res


def fetch_mpr(nums, dl):
    d = os.path.join(RAWD, "mpr")
    os.makedirs(d, exist_ok=True)
    csv_dir = fresh_dir(dl, "fca-csv")
    ent = get(FCA_CSV, os.path.join(csv_dir, "SocietyList.csv"))
    print("FCA CSV", ent["http"], ent["bytes"], flush=True)
    rows_out = []
    with open(os.path.join(csv_dir, "SocietyList.csv"), encoding="cp1252", errors="replace", newline="") as f:
        rd = csv.reader(f)
        hdr = next(rd)
        i_full = hdr.index("Full Registation Number")
        want = set(nums)
        for r in rd:
            if len(r) > i_full and r[i_full].strip().upper() in want:
                rows_out.append(r)
    with open(os.path.join(d, "SocietyList-rows.csv"), "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(hdr)
        w.writerows(rows_out)
    print("FCA CSV rows kept", len(rows_out), flush=True)
    for n in nums:
        s = get(FCA + "/Search/Search?" + urllib.parse.urlencode({"SearchTerm": n}), os.path.join(d, n + "-search.html"))
        res = parse_search(os.path.join(d, n + "-search.html")) if s["http"] == 200 else []
        sid = next((r["society_id"] for r in res if r["reg_no"].upper() == n), None)
        print("FCA", n, s["http"], "society", sid, flush=True)
        if sid is None:
            continue
        get("%s/Search/Society/%d" % (FCA, sid), os.path.join(d, n + "-page.html"))
        get("%s/Societies/GetNameChanges?societyId=%d" % (FCA, sid), os.path.join(d, n + "-namechanges.html"), xhr=True)
        get("%s/Societies/GetTransfers?societyId=%d" % (FCA, sid), os.path.join(d, n + "-transfers.html"), xhr=True)


# ---- UKPRN ----------------------------------------------------------------------------------------
def fetch_ukrlp(nums):
    d = os.path.join(RAWD, "ukrlp")
    os.makedirs(d, exist_ok=True)
    for v in nums:
        for kind, url in (("detail", "%s/search/provider/%s" % (UKRLP, v)),
                          ("active", "%s/search?SearchTerm=%s&UKPRNStatusFilter=active&Page=1" % (UKRLP, v)),
                          ("deactivated", "%s/search?SearchTerm=%s&UKPRNStatusFilter=deactivated&Page=1" % (UKRLP, v))):
            e = get(url, os.path.join(d, "%s_%s.html" % (v, kind)), accept="text/html")
            print("UKRLP", v, kind, e["http"], e["bytes"], flush=True)


# ---- CHC ----------------------------------------------------------------------------------------
def zip_records(zpath):
    """Stream the one JSON array inside a CC extract zip, one record at a time (469 U2's index.py)."""
    with zipfile.ZipFile(zpath) as z:
        names = z.namelist()
        assert len(names) == 1, names
        with z.open(names[0]) as raw:
            f = io.TextIOWrapper(raw, encoding="utf-8-sig", newline="")
            buf = ""
            for line in f:
                s = line.strip()
                if not s:
                    continue
                if s.startswith("["):
                    s = s[1:]
                if s.startswith(","):
                    s = s[1:]
                if s.endswith("]"):
                    try:
                        yield json.loads(s[:-1])
                        continue
                    except Exception:
                        pass
                if not s:
                    continue
                buf = buf + s if buf else s
                try:
                    obj = json.loads(buf)
                except json.JSONDecodeError:
                    continue
                buf = ""
                yield obj


def fetch_cc(nums, dl):
    want = {int(n) for n in nums}
    out = {"charity": [], "other_names": []}
    for url, key, fn in ((CC_ZIP, "charity", "publicextract.charity.zip"),
                         (CC_NAMES_ZIP, "other_names", "publicextract.charity_other_names.zip")):
        dd = fresh_dir(dl, "cc-" + key)
        e = get(url, os.path.join(dd, fn))
        print("CC", key, e["http"], e["bytes"], flush=True)
        n = 0
        for r in zip_records(os.path.join(dd, fn)):
            n += 1
            if r.get("registered_charity_number") in want:
                out[key].append(r)
        e["records_streamed"] = n
        print("CC", key, "records", n, "kept", len(out[key]), flush=True)
    os.makedirs(os.path.join(RAWD, "cc"), exist_ok=True)
    json.dump(out, open(os.path.join(RAWD, "cc", "cc-extract-rows.json"), "w"), indent=1, ensure_ascii=False)


# ---- NIC ----------------------------------------------------------------------------------------
def fetch_ccni(nums):
    d = os.path.join(RAWD, "ccni")
    os.makedirs(d, exist_ok=True)
    for n in nums:
        for kind, url, ext, acc in (("page", CCNI_PAGE.format(n), "html", "text/html"),
                                    ("csv", CCNI_CSV.format(n), "csv", None),
                                    ("search", CCNI_API.format(n), "json", "application/json")):
            e = get(url, os.path.join(d, "%s-%s.%s" % (n, kind, ext)), accept=acc)
            print("CCNI", n, kind, e["http"], e["bytes"], flush=True)


def main():
    dl = sys.argv[1]
    os.makedirs(dl, exist_ok=True)
    nums = numbers()
    print({k: len(v) for k, v in nums.items()}, flush=True)
    os.makedirs(RAWD, exist_ok=True)
    only = sys.argv[2:] or ["NIC", "UKPRN", "MPR", "CHC"]
    try:
        if "NIC" in only:
            fetch_ccni(nums["NIC"])
        if "UKPRN" in only:
            fetch_ukrlp(nums["UKPRN"])
        if "MPR" in only:
            fetch_mpr(nums["MPR"], dl)
        if "CHC" in only:
            fetch_cc(nums["CHC"], dl)
    finally:
        mp = os.path.join(REG, "fetch-manifest.json")
        prev = json.load(open(mp)) if os.path.exists(mp) else {"numbers": nums, "requests": []}
        prev["numbers"] = nums
        prev["requests"].extend(MANIFEST)
        json.dump(prev, open(mp, "w"), indent=1)


if __name__ == "__main__":
    main()
