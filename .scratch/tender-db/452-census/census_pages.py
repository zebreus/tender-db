#!/usr/bin/env python3
"""Register pages for the census's absent numbers (issue 466 unit 3).

usage: census_pages.py CACHE.json ABSENT.json [ABSENT.json ...] [--exclude OLD_ABSENT.json ...]

Fetches the public Companies House page of every number in the ABSENT lists (census.py's
`absent.json`) not already in CACHE with a final answer (200 or 404), using 448's
`ch_fetch.fetch` (imported, ~1.5 req/s per worker, 4 workers, back-off on 429/5xx).
`--exclude` skips numbers an earlier census already read (452's
`absent-register-pages-2026-09-30.json` covers its 1,009). A 404 is a number Companies
House never issued; a 200 carries the name, status and previous names.
"""
import json, os, sys, time
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "448-campaign"))
from ch_fetch import fetch  # noqa: E402


def numbers_of(path):
    d = json.load(open(path))
    if isinstance(d, dict):
        return set(d)
    return {r["number"] for r in d}


def main(argv):
    cache_path, rest = argv[1], argv[2:]
    wanted, exclude, mode = set(), set(), "in"
    for a in rest:
        if a == "--exclude":
            mode = "ex"
            continue
        (wanted if mode == "in" else exclude).update(numbers_of(a))
    try:
        cache = json.load(open(cache_path))
    except FileNotFoundError:
        cache = {}
    todo = sorted(n for n in wanted - exclude if cache.get(n, {}).get("http") not in ("200", "404"))
    print(f"{len(wanted)} absent numbers, {len(wanted & exclude)} read before, {len(todo)} to fetch", flush=True)

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
    json.dump(cache, open(cache_path, "w"), indent=0, ensure_ascii=False)
    codes = {}
    for n in wanted - exclude:
        c = cache.get(n, {}).get("http")
        codes[c] = codes.get(c, 0) + 1
    print("done", codes, flush=True)


if __name__ == "__main__":
    main(sys.argv)
