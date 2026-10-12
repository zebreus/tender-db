#!/usr/bin/env python3
"""Issue 514 U2.1, step 3b: parse the cached register responses into registers/records.json.

Offline only: reads registers/raw/* (written by fetch_registers.py) and records what each
register says for each canonical number -- name, status, former / other names and the
facts a reviewer needs (company number, linked charities, transfers). It judges nothing.

Usage: python3 -I build_registers.py
"""
import csv
import html
import json
import os
import re
from html.parser import HTMLParser

HERE = os.path.dirname(os.path.abspath(__file__))
REG = os.path.join(HERE, "registers")
RAWD = os.path.join(REG, "raw")
FCA = "https://mutuals.fca.org.uk"
UKRLP = "https://ukrlp.education.gov.uk"


def clean(s):
    return re.sub(r"\s+", " ", html.unescape(s or "")).strip() or None


# ---- MPR ----------------------------------------------------------------------------------------
def fca_page(path):
    t = open(path, encoding="utf-8", errors="replace").read()
    out = {}
    m = re.search(r"<h1>(.*?)</h1>", t, re.S)
    out["name"] = clean(re.sub(r"<[^>]+>", "", m.group(1))) if m else None
    m = re.search(r"<strong>Registration Number:\s*(.*?)</strong>", t, re.S)
    out["registration_number"] = clean(m.group(1)) if m else None
    for m in re.finditer(r'<label class="[^"]*" for="(\w+)">(.*?)</label>\s*<div class="[^"]*">(.*?)</div>', t, re.S):
        key, val = m.group(1), m.group(3)
        if key == "Address":
            val = ", ".join(p for p in (clean(re.sub(r"<[^>]+>", "", x)) for x in re.split(r"<br\s*/?>", val)) if p)
        else:
            val = clean(re.sub(r"<[^>]+>", "", val))
        out[key] = val
    return out


def fca_json(path):
    try:
        return json.load(open(path, encoding="utf-8-sig"))["aaData"]
    except Exception:
        return None


def build_mpr(nums_rows):
    out = {}
    rows = {}
    p = os.path.join(RAWD, "mpr", "SocietyList-rows.csv")
    with open(p, encoding="utf-8", newline="") as f:
        for r in csv.DictReader(f):
            rows[r["Full Registation Number"].strip().upper()] = r
    for n in nums_rows:
        r = rows.get(n)
        d = os.path.join(RAWD, "mpr")
        page = fca_page(os.path.join(d, n + "-page.html")) if os.path.exists(os.path.join(d, n + "-page.html")) else {}
        names = fca_json(os.path.join(d, n + "-namechanges.html")) or []
        transfers = fca_json(os.path.join(d, n + "-transfers.html")) or []
        search = open(os.path.join(d, n + "-search.html"), encoding="utf-8", errors="replace").read()
        sid = re.search(r'href="/Search/Society/(\d+)">[^<]*</a></div>\s*<div>Status:[^<]*</div>\s*<div>Registration Number:\s*'
                        + re.escape(n) + r"\s*<", search)
        out["GBMPR" + n] = dict(
            scheme="MPR", register="FCA Mutuals Public Register", number=n, found=r is not None,
            name=clean(r["Society Name"]) if r else page.get("name"),
            fca_society_name=clean(r["Society Name"]) if r else page.get("name"),
            status=r["Society Status"] if r else page.get("Status"),
            registered_as=r["Registered As"] if r else None,
            registration_act=r["Registration Act"] if r else None,
            registration_date=r["Registration Date"] if r else None,
            deregistration_date=(r["Deregistration Date"] or None) if r else None,
            sic=r["Standard Industrial Classification Code (SIC)"] if r else None,
            address=clean(r["Society Address"]) if r else None,
            page=page,
            former_names=[{"changed": a, "former_name": clean(b)} for a, b in names],
            transfers_in=[{"date": t[0], "from_society": clean(t[1]), "from_society_number": t[2], "type": t[3]}
                          for t in transfers],
            society_id=int(sid.group(1)) if sid else None,
            sources=dict(csv="https://fcastoragemprprod.blob.core.windows.net/societylist/SocietyList.csv",
                         page="%s/Search/Society/%s" % (FCA, sid.group(1)) if sid else None),
            raw=[os.path.join("registers/raw/mpr", n + s) for s in ("-search.html", "-page.html", "-namechanges.html",
                                                                     "-transfers.html")] + ["registers/raw/mpr/SocietyList-rows.csv"])
    return out


# ---- UKPRN ----------------------------------------------------------------------------------------
class DL(HTMLParser):
    """Collect dt -> dd rows (dd paragraphs as lines); 469 U2's parse_ukrlp.py."""

    def __init__(self):
        super().__init__()
        self.rows, self.cur_key, self.cur_dd, self.buf, self.title, self.in_title, self.last_h2 = [], None, None, "", "", False, None
        self.h1 = []

    def handle_starttag(self, tag, attrs):
        if tag == "title":
            self.in_title = True
        if tag in ("h1", "h2", "dt", "dd", "p", "li"):
            if tag == "p" and self.cur_dd is not None:
                self.flush()
            if tag in ("h1", "h2", "dt", "dd"):
                self.buf = ""
            if tag == "dd":
                self.cur_dd = []
        if tag == "br":
            self.buf += " "

    def flush(self):
        t = " ".join(self.buf.split())
        if t:
            self.cur_dd.append(t)
        self.buf = ""

    def handle_endtag(self, tag):
        if tag == "title":
            self.in_title = False
        t = " ".join(self.buf.split())
        if tag == "h1":
            self.h1.append(t)
        elif tag == "h2":
            self.last_h2 = t
        elif tag == "dt":
            self.cur_key = t
        elif tag == "p" and self.cur_dd is not None:
            self.flush()
        elif tag == "dd":
            self.flush()
            self.rows.append((self.last_h2, self.cur_key, self.cur_dd))
            self.cur_dd = None

    def handle_data(self, data):
        if self.in_title:
            self.title += data
        self.buf += data


def ukrlp_search(raw, v):
    links = re.findall(r'href="/search/provider/(\d+)\?', raw)
    names = {}
    for blk in re.findall(r'<div class="search-result.*?</div>', raw, re.S):
        mm = re.search(r'href="/search/provider/(\d+)\?[^"]*">\s*(.*?)<span', blk, re.S)
        if mm:
            names[mm.group(1)] = html.unescape(" ".join(mm.group(2).split()))
    return {"hit": v in links, "name": names.get(v)}


def build_ukrlp(nums):
    out = {}
    for v in nums:
        d = os.path.join(RAWD, "ukrlp")
        p = DL()
        p.feed(open(os.path.join(d, v + "_detail.html"), encoding="utf-8", errors="replace").read())
        rows = {k: lines for _, k, lines in p.rows}
        act = ukrlp_search(open(os.path.join(d, v + "_active.html"), encoding="utf-8", errors="replace").read(), v)
        de = ukrlp_search(open(os.path.join(d, v + "_deactivated.html"), encoding="utf-8", errors="replace").read(), v)
        legal = (rows.get("Legal name") or [None])[0]
        trading = []
        for x in rows.get("Trading names") or []:
            for part in str(x).split("\n"):
                q = " ".join(part.split())
                if q and q.lower() not in ("not applicable", "see more trading names") and q not in trading:
                    trading.append(q)
        out["GBUKPRN" + v] = dict(
            scheme="UKPRN", register="UKRLP", number=v,
            found=bool(" ".join(p.title.split()).startswith("Provider details for") and (act["hit"] or de["hit"])),
            name=legal, status="active" if act["hit"] else ("deactivated" if de["hit"] else None),
            trading_names=trading, legal_address=", ".join(rows.get("Legal address") or []) or None,
            search_list_name=act["name"] or de["name"], page_title=" ".join(p.title.split()),
            sources=dict(detail="%s/search/provider/%s" % (UKRLP, v)),
            raw=["registers/raw/ukrlp/%s_%s.html" % (v, k) for k in ("detail", "active", "deactivated")])
    return out


# ---- CHC ----------------------------------------------------------------------------------------
def build_cc(nums):
    d = json.load(open(os.path.join(RAWD, "cc", "cc-extract-rows.json")))
    out = {}
    for n in nums:
        rows = [r for r in d["charity"] if r["registered_charity_number"] == int(n)]
        main = [r for r in rows if r["linked_charity_number"] == 0]
        linked = [r for r in rows if r["linked_charity_number"] != 0]
        names = [r for r in d["other_names"] if r["registered_charity_number"] == int(n)]
        m = main[0] if main else None
        out["GBCHC" + n] = dict(
            scheme="CHC", register="Charity Commission for England and Wales (register extract)", number=n,
            found=m is not None, name=m["charity_name"] if m else None,
            status=m["charity_registration_status"] if m else None,
            date_of_registration=(m["date_of_registration"] or "")[:10] or None if m else None,
            date_of_removal=(m["date_of_removal"] or "")[:10] or None if m else None,
            company_number=m["charity_company_registration_number"] if m else None,
            charity_type=m["charity_type"] if m else None,
            is_cio=m["charity_is_cio"] if m else None,
            insolvent=m["charity_insolvent"] if m else None,
            in_administration=m["charity_in_administration"] if m else None,
            latest_income=m["latest_income"] if m else None,
            web=m["charity_contact_web"] if m else None,
            other_names=[{"type": x["charity_name_type"], "name": x["charity_name"], "linked_charity_number":
                          x["linked_charity_number"]} for x in names],
            linked_charities=dict(total=len(linked), registered=sum(1 for r in linked if r["charity_registration_status"] == "Registered"),
                                  sample=[{"linked_charity_number": r["linked_charity_number"], "name": r["charity_name"],
                                           "status": r["charity_registration_status"]} for r in linked[:12]]),
            extract_date=(m or (rows[0] if rows else {})).get("date_of_extract"),
            sources=dict(extract="https://ccewuksprdoneregsadata1.blob.core.windows.net/data/json/publicextract.charity.zip",
                         page="https://register-of-charities.charitycommission.gov.uk/charity-search/-/charity-details/%s" % n),
            raw=["registers/raw/cc/cc-extract-rows.json"])
    return out


# ---- NIC ----------------------------------------------------------------------------------------
def build_ccni(nums):
    out = {}
    for n in nums:
        d = os.path.join(RAWD, "ccni")
        raw = open(os.path.join(d, n + "-csv.csv"), "rb").read().decode("utf-8-sig")
        rr = list(csv.reader(raw.splitlines()))
        row = dict(zip(rr[0], rr[1])) if len(rr) > 1 else {}
        srch = json.load(open(os.path.join(d, n + "-search.json")))["results"]["pageItems"]
        hit = [h for h in srch if h.get("regNo") == n]
        out["GBNICNIC" + n] = dict(
            scheme="NIC", register="Charity Commission for Northern Ireland", number=n, found=bool(row),
            name=row.get("Charity name"), status=row.get("Status"), date_registered=row.get("Date registered"),
            company_number=row.get("Company number"), other_name=row.get("Other name") or None,
            public_address=row.get("Public address"), search_hit=hit,
            sources=dict(page="https://www.charitycommissionni.org.uk/charity-search/charity-details-page/?regId=%s&subId=0" % n),
            raw=["registers/raw/ccni/%s-%s" % (n, s) for s in ("page.html", "csv.csv", "search.json")])
    return out


def main():
    man = json.load(open(os.path.join(REG, "fetch-manifest.json")))
    nums = man["numbers"]
    rec = {}
    rec.update(build_mpr(nums["MPR"]))
    rec.update(build_ukrlp(nums["UKPRN"]))
    rec.update(build_cc(nums["CHC"]))
    rec.update(build_ccni(nums["NIC"]))
    bad = [r for r in man["requests"] if r.get("http") != 200]
    out = dict(issue=514, unit="U2.1 register records", fetched=sorted({r["at"][:10] for r in man["requests"]}),
               requests=len(man["requests"]), non_200=bad, count=len(rec),
               found=sum(1 for v in rec.values() if v["found"]), records=rec)
    json.dump(out, open(os.path.join(REG, "records.json"), "w"), indent=1, ensure_ascii=False)
    for k, v in sorted(rec.items()):
        print(k, v["found"], v["status"], "|", v["name"],
              "| other:", [x.get("former_name") or x.get("name") for x in v.get("former_names", []) + v.get("other_names", [])][:6],
              v.get("trading_names") or "", v.get("company_number") or "", "linked" if v.get("linked_charities", {}).get("total") else "")


if __name__ == "__main__":
    main()
