"""Companies House register names for the altid campaign (issue 448 unit 4).

usage: ch_fetch.py <altid-merge-plan.json> <cache.json>
Reads every company number in pairs + denied + conflict_listing, fetches the public
company page (~1.5 req/s), and caches {number: {name, status, previous: [...], http}}.
Resumable: numbers already in the cache are skipped.
"""
import json, re, sys, time, html, subprocess

def fetch(number):
    url = f"https://find-and-update.company-information.service.gov.uk/company/{number}"
    for attempt in range(4):
        p = subprocess.run(["curl", "-sS", "-m", "30", "-w", "\n%{http_code}", url], capture_output=True, text=True)
        body, _, code = p.stdout.rpartition("\n")
        code = code.strip()
        if code == "429" or code.startswith("5") or p.returncode != 0:
            time.sleep(10 * (attempt + 1))
            continue
        break
    out = {"http": code}
    if code != "200":
        return out
    m = re.search(r'<h1 class="heading-xlarge"[^>]*>(.*?)</h1>', body, re.S)
    out["name"] = html.unescape(m.group(1).strip()) if m else None
    m = re.search(r'id="company-status">(.*?)</dd>', body, re.S)
    out["status"] = re.sub(r"\s+", " ", re.sub(r"<[^>]+>", " ", m.group(1))).strip() if m else None
    out["previous"] = [html.unescape(re.sub(r"\s+", " ", re.sub(r"<[^>]+>", " ", x)).strip())
                       for x in re.findall(r'<td id="previous-name-\d+">(.*?)</td>', body, re.S)]
    return out

def main(plan_path, cache_path):
    d = json.load(open(plan_path))
    b = d.get("body", d)
    if isinstance(b, str):
        b = json.loads(b)
    numbers = {k.split("~")[0] for k in b["pairs"]}
    for name in ("denied", "conflict_listing"):
        numbers |= {e["key"].split("~")[0] for e in b.get(name, [])}
    try:
        cache = json.load(open(cache_path))
    except FileNotFoundError:
        cache = {}
    todo = sorted(n for n in numbers if n not in cache or cache[n].get("http") not in ("200", "404"))
    print(f"{len(numbers)} numbers, {len(todo)} to fetch", flush=True)
    from concurrent.futures import ThreadPoolExecutor
    def one(n):
        r = fetch(n)
        time.sleep(1.5)
        return n, r
    with ThreadPoolExecutor(4) as pool:
        for i, (n, r) in enumerate(pool.map(one, todo)):
            cache[n] = r
            if i % 50 == 0:
                json.dump(cache, open(cache_path, "w"))
                print(f"{i}/{len(todo)} {n} {r.get('http')} {r.get('name')}", flush=True)
    json.dump(cache, open(cache_path, "w"))
    codes = {}
    for v in cache.values():
        codes[v.get("http")] = codes.get(v.get("http"), 0) + 1
    print("done", codes, flush=True)

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
