"""Draw: for each month of the part's years, a random OJ S publication day (Tue-Sat); list the TED
tenders whose current version was published that day (/v1/tenders?source=ted&published_after=D&
published_before=D+1, up to 2 pages of 1000); drop rows whose submission deadline is on/after the
publication day (the current version is then a contract notice); probe the rest in random order
(publication_id -> /v1/notices -> /v1/notices/{id}/content) until `per_month` multi-lot CANs
(r2.0.8/r2.0.9) are found; a day that runs dry after `cap` probes is replaced by another day."""
import datetime, json, os, random, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import api, mapping

part = int(sys.argv[1])
years = {0: range(2012, 2016), 1: range(2016, 2020), 2: range(2020, 2024)}[part]
rng = random.Random(56700 + part)
extra_months = {y: rng.randrange(1, 13) for y in years}  # one month per year draws a 3rd notice -> 100
cap = 60
OUT = os.path.join(api.DIR, f"part{part}_probes.jsonl")
done = set()
if os.path.exists(OUT):
    for line in open(OUT):
        r = json.loads(line)
        done.add((r["month"], r["publication_id"]))

def log(rec):
    with open(OUT, "a") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")

def day_rows(day):
    nxt = day + datetime.timedelta(days=1)
    rows, cursor = [], None
    for page in range(2):
        q = f"/v1/tenders?source=ted&published_after={day.isoformat()}&published_before={nxt.isoformat()}&limit=1000"
        if cursor:
            q += f"&cursor={cursor}"
        d = api.get(q, cache=f"raw/day_{day.isoformat()}_p{page}.json")
        rows += d["items"]
        if not d.get("more") or not d.get("next_cursor"):
            break
        cursor = d["next_cursor"]
    return rows

for y in years:
    for m in range(1, 13):
        month = f"{y}-{m:02d}"
        target = 3 if extra_months[y] == m else 2
        hits = 0
        # resume: count hits already logged for this month
        if os.path.exists(OUT):
            for line in open(OUT):
                r = json.loads(line)
                if r["month"] == month and r["class"] == "multilot-can":
                    hits += 1
        days_tried = set()
        ndays = (datetime.date(y + (m == 12), m % 12 + 1, 1) - datetime.date(y, m, 1)).days
        cands = [datetime.date(y, m, d) for d in range(1, ndays + 1) if datetime.date(y, m, d).weekday() in (1, 2, 3, 4, 5)]
        rng_m = random.Random(f"{part}-{month}")
        rng_m.shuffle(cands)
        for day in cands:
            if hits >= target:
                break
            rows = day_rows(day)
            pool = [t for t in rows if not (t.get("submission_deadline") and t["submission_deadline"][:10] >= t["published_at"])]
            print(f"{month} day {day} rows {len(rows)} pool {len(pool)}", flush=True)
            if len(rows) < 100:
                continue
            rng_d = random.Random(f"{part}-{day}")
            rng_d.shuffle(pool)
            probes = 0
            for t in pool:
                if hits >= target or probes >= cap:
                    break
                pub = t["publication_id"]
                if (month, pub) in done:
                    continue
                probes += 1
                rec = {"month": month, "day": day.isoformat(), "tender_id": t["id"], "publication_id": pub,
                       "tender_lots": t["lots"], "tender_version": t["version"]}
                try:
                    lk = api.get(f"/v1/notices?source=ted&publication_id={pub}", cache=f"lookup/{pub}.json")
                except Exception as e:
                    rec.update({"class": "error", "error": str(e)}); log(rec); continue
                items = [n for n in lk["items"] if n["source"] == "ted"]
                if not items:
                    rec.update({"class": "no-notice"}); log(rec); continue
                n = items[0]
                rec.update({"notice_id": n["id"], "profile": n["profile"], "notice_published": n["published_at"]})
                if n["profile"] not in ("ted-export-r208", "ted-export-r209"):
                    rec.update({"class": "other-profile"}); log(rec); continue
                try:
                    c = api.get(f"/v1/notices/{n['id']}/content", cache=f"content/{n['id']}.json")
                except Exception as e:
                    rec.update({"class": "error", "error": str(e)}); log(rec); continue
                cls, td, nres, nnos = mapping.classify(c)
                rec.update({"class": cls, "td": td, "n_res": nres, "n_award_lotnos": nnos,
                            "n_lot_sections": sum(1 for s in c["sections"] if s["kind"] in mapping.LOT_KINDS)})
                log(rec)
                done.add((month, pub))
                if cls == "multilot-can":
                    hits += 1
                    print(f"  HIT {month} {pub} notice {n['id']} tender {t['id']} res {nres} lots {rec['n_lot_sections']}", flush=True)
            print(f"  {month}: probes {probes} hits {hits}/{target}", flush=True)
