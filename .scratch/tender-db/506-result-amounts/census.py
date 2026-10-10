#!/usr/bin/env python3
"""Issue 506 unit 1, census half: a SAMPLED per-field read of the eForms result-level money BTs.

usage: census.py OUT.json [WINDOWS] [WIDTH]   (run on the box; bounded /v1/sql reads through /root/sq.sh)

Per eForms profile, WINDOWS start ids evenly spaced over the profile's id span; at each, the next WIDTH
notices of that profile through the `notices_profile` index (ids are sparse, so id windows miss), then
their `notice_amounts` and BT-13713 refs by `notice_id IN (...)` (primary-key seeks). EU SDK profiles are
measured; eForms-DE profiles (their own `DE1-*` field ids) get a field-id inventory only. For every row of a candidate
field it records, per SDK profile: whether it equals the SAME lot's BT-271/BT-27 (through the result's
BT-13713), any lot figure, a procedure figure (BT-27/271-Procedure, BT-161), any BT-720 tender value;
whether it exceeds the notice's largest mapped figure (BT-27*/271*/161, same currency — the head proxy) or
the notice has none; and whether it makes a 10^k (k>=2) partner of a mapped figure.
"""
import collections, json, subprocess, sys, time

FIELDS = ["BT-709-LotResult", "BT-660-LotResult", "BT-118-NoticeResult", "BT-1118-NoticeResult",
          "BT-156-NoticeResult", "BT-1561-NoticeResult", "BT-157-LotsGroup", "BT-710-LotResult", "BT-711-LotResult"]
MAPPED = ("BT-27-", "BT-271-", "BT-161-")
PROFILES = {  # profile -> (min id, max id), read 2026-10-10
    "eforms:eforms-sdk-1.3": (22799746, 23730731), "eforms:eforms-sdk-1.6": (23027710, 24187964),
    "eforms:eforms-sdk-1.7": (23135969, 44505457), "eforms:eforms-sdk-1.8": (23334784, 24813446),
    "eforms:eforms-sdk-1.9": (23460350, 24825134), "eforms:eforms-sdk-1.10": (23596968, 44559190),
    "eforms:eforms-sdk-1.11": (23924000, 44559188), "eforms:eforms-sdk-1.12": (7, 47380888),
    "eforms:eforms-sdk-1.13": (1, 47380898), "eforms:eforms-sdk-1.14": (3, 47380900),
}
DE_PROFILES = {"eforms:eforms-de-1.1": (26195620, 26644178), "eforms:eforms-de-1.2": (26311110, 26722031)}
DE_FRAGMENTS = ("MaximumValueAmount", "Reestimated", "OverallMaximum", "OverallApproximate",
                "FrameworkMaximum", "EstimatedOverall", "LowerTender", "HigherTender")


def sql(q):
    # Paced: /v1/sql rate-limits (429). A 429 is retried after a pause; a 408 never is.
    for attempt in range(8):
        time.sleep(0.3)
        p = subprocess.run(["/root/sq.sh"], input=q.encode(), capture_output=True)
        d = json.loads(p.stdout)
        if isinstance(d.get("error"), dict) and d["error"].get("status") == 429:
            time.sleep(2 + 2 * attempt)
            continue
        break
    if "error" in d:
        raise SystemExit(f"query failed: {d['error']}: {q[:120]}")
    if d.get("truncated"):
        raise SystemExit(f"truncated: {q[:120]}")
    return d["rows"]


def sample(profile, lo, hi, windows, width):
    ids = set()
    step = max((hi - lo) // windows, 1)
    for w in range(windows):
        x = lo + w * step
        for (n,) in sql(f"SELECT id FROM notices WHERE profile = '{profile}' AND id >= {x} ORDER BY id LIMIT {width}"):
            ids.add(n)
    return sorted(ids)


def chunks(ids, k=150):
    for i in range(0, len(ids), k):
        yield ids[i:i + k]


def main(argv):
    out = argv[1]
    windows = int(argv[2]) if len(argv) > 2 else 12
    width = int(argv[3]) if len(argv) > 3 else 150
    stats = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    notices_seen = collections.Counter()
    examples = collections.defaultdict(list)
    de_inventory = collections.defaultdict(collections.Counter)
    for profile, (lo, hi) in DE_PROFILES.items():
        ids = sample(profile, lo, hi, windows, width)
        notices_seen[profile] = len(ids)
        for ch in chunks(ids):
            for (f, n) in sql(
                "SELECT field_id, count(*) FROM notice_amounts WHERE notice_id IN (" + ",".join(map(str, ch)) + ") GROUP BY field_id"
            ):
                if any(x in f for x in DE_FRAGMENTS):
                    de_inventory[profile][f] += n
        print(f"{profile}: {len(ids)} notices", file=sys.stderr)
    for profile, (lo, hi) in PROFILES.items():
        ids = sample(profile, lo, hi, windows, width)
        notices_seen[profile] = len(ids)
        for ch in chunks(ids):
            inlist = ",".join(map(str, ch))
            amounts = collections.defaultdict(list)
            for n, sec, f, c, cur in sql(
                f"SELECT notice_id, section_id, field_id, cents, currency FROM notice_amounts WHERE notice_id IN ({inlist})"
            ):
                amounts[n].append((sec, f, c, cur))
            lot_of = {}
            for n, sec, v in sql(
                f"SELECT notice_id, section_id, value FROM notice_ids WHERE notice_id IN ({inlist}) "
                f"AND field_id = 'BT-13713-LotResult'"
            ):
                lot_of[(n, sec)] = v
            for n, rows in amounts.items():
                p = profile
                mapped = [(s, f, c, cur) for s, f, c, cur in rows if f.startswith(MAPPED)]
                tenders = {(c, cur) for s, f, c, cur in rows if f == "BT-720-Tender"}
                for sec, f, c, cur in rows:
                    if f not in FIELDS:
                        continue
                    st = stats[f][p]
                    st["rows"] += 1
                    lot = lot_of.get((n, sec))
                    same_lot = {(c2, cur2) for s2, f2, c2, cur2 in mapped if lot and s2 == lot and f2 in ("BT-271-Lot", "BT-27-Lot")}
                    any_lot = {(c2, cur2) for s2, f2, c2, cur2 in mapped if f2 in ("BT-271-Lot", "BT-27-Lot")}
                    proc = {(c2, cur2) for s2, f2, c2, cur2 in mapped if f2 in ("BT-271-Procedure", "BT-27-Procedure", "BT-161-NoticeResult")}
                    key = (c, cur)
                    hit = []
                    if key in same_lot:
                        st["eq_same_lot_figure"] += 1; hit.append("same_lot")
                    if key in any_lot:
                        st["eq_any_lot_figure"] += 1; hit.append("any_lot")
                    if key in proc:
                        st["eq_procedure_figure"] += 1; hit.append("proc")
                    if key in tenders:
                        st["eq_tender_value"] += 1; hit.append("tender")
                    if not hit:
                        st["eq_nothing"] += 1
                    same_cur = [c2 for s2, f2, c2, cur2 in mapped if cur2 == cur and c2 > 0]
                    if not same_cur:
                        st["no_mapped_figure"] += 1
                    elif c > max(same_cur):
                        st["exceeds_head_proxy"] += 1
                        if len(examples[(f, "exceeds")]) < 8:
                            examples[(f, "exceeds")].append([n, sec, c, cur, max(same_cur)])
                    if c > 0 and any(
                        c == m * 10**k or m == c * 10**k for m in same_cur for k in range(2, 7)
                    ):
                        st["pow10_partner"] += 1
                        if len(examples[(f, "pow10")]) < 8:
                            examples[(f, "pow10")].append([n, sec, c, cur, sorted(set(same_cur))[:6]])
        print(f"{profile}: {len(ids)} notices", file=sys.stderr)
    json.dump({
        "windows": windows, "width": width,
        "de_inventory": {p: dict(c) for p, c in de_inventory.items()},
        "notices_seen": notices_seen,
        "stats": {f: {p: dict(c) for p, c in ps.items()} for f, ps in stats.items()},
        "examples": {f"{f}:{k}": v for (f, k), v in examples.items()},
    }, open(out, "w"), indent=1)


if __name__ == "__main__":
    main(sys.argv)
