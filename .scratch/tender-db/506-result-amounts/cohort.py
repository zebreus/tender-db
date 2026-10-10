#!/usr/bin/env python3
"""Issue 506 unit 3, step 0: the cohort read BEFORE the deploy (the unit-2 decision's precondition).

usage: cohort.py OUT.json   (run on the box; bounded /v1/sql reads through /root/sq.sh, paced)

The cohort is `refold-value-band`'s: every Tender whose stored head, or any stored lot value of any version, is
EUR 1 bn or more (two index range reads, `tenders_current_value_eur` and the partial
`tender_version_lots_value_eur`), plus the adjudicated ids of 492 and 505 (the ones a path switch could re-admit).

Per batch of <= 100 Tenders it reads the versions' notices, the stored amounts (all versions) and lot awards, the
lots' keys, and the notice layer's result-level framework values (BT-709 / BT-660, their DE1 spellings) with each
result's BT-13713, plus the bare DE1-FrameworkMaximumAmount. A result figure counts as a new partner only where its
BT-13713 names a lot the Tender has (the build drops the rest). For every stored amount F worth EUR 1 bn or more it
classifies the NEW exact partners F = R x 10^k (k >= 2, R > 1,000 cents, same currency):
  new-x100  F had no partner and gains a x100 one (and no k >= 3 one);
  switch    F had only a x100 partner and gains a k >= 3 one (moves to the corroboration test);
  new-k3    F had no partner and gains a k >= 3 one;
and whether F is corroborated (a second field of its version carries the same cents). It also lists every bare DE1
framework maximum above the Tender's stored head, and counts result rows dropped (no BT-13713, or no such lot).
"""
import collections, json, subprocess, sys, time

RESULT_FIELDS = {
    "BT-709-LotResult": "result_framework_maximum",
    "BT-660-LotResult": "result_framework_reestimate",
    "DE1-NoticeResult-LotResult-FrameworkAgreementValues-MaximumValueAmount": "result_framework_maximum",
    "DE1-NoticeResult-LotResult-FrameworkAgreementValues-ReestimatedValueAmount": "result_framework_reestimate",
}
LOT_REFS = ("BT-13713-LotResult", "DE1-NoticeResult-LotResult-TenderLot-ID")
BARE = "DE1-FrameworkMaximumAmount"
GATE = 100_000_000_000  # EUR 1 bn in cents
FLOOR = 1_000
ADJUDICATED = [395737, 627219]  # 506's two slips; the 492/505 ids are added from their files below


class Truncated(Exception):
    pass


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
        raise SystemExit(f"query failed: {d['error']}: {q[:160]}")
    if d.get("truncated"):
        raise Truncated(q)
    return d["rows"]


SEQS = {}  # tender -> its version seqs, for a single Tender too big for one read


def sql_in(template, ids):
    try:
        return sql(template.format(ids=",".join(map(str, ids))))
    except Truncated:
        if len(ids) == 1:
            seqs = SEQS.get(ids[0])
            if not seqs or "tender_id IN" not in template:
                raise SystemExit(f"one id overflows the row cap: {ids[0]}")
            # One Tender with more rows than the cap: read it one version at a time.
            out = []
            for seq in seqs:
                out += sql(template.format(ids=ids[0]) + f" AND seq = {seq}")
            return out
        half = len(ids) // 2
        return sql_in(template, ids[:half]) + sql_in(template, ids[half:])


def keyset(template):
    """`template` has `{after}`; pages of 1000 by ascending id."""
    out, after = [], 0
    while True:
        rows = sql(template.format(after=after))
        if not rows:
            return out
        out.extend(r[0] for r in rows)
        after = rows[-1][0]


def partners(f, figures):
    """(has x100, has k>=3) for figure f against an iterable of figures."""
    x100 = k3 = False
    for r in figures:
        if r <= FLOOR:
            continue
        if f == r * 100:
            x100 = True
        for k in range(3, 9):
            if f == r * 10**k:
                k3 = True
    return x100, k3


def main(argv):
    out = argv[1]
    heads = keyset("SELECT id FROM tenders WHERE current_value_eur_cents >= %d AND id > {after} ORDER BY id LIMIT 1000" % GATE)
    lots = keyset(
        "SELECT DISTINCT tender_id FROM tender_version_lots WHERE value_eur_cents IS NOT NULL AND value_eur_cents >= %d "
        "AND tender_id > {after} ORDER BY tender_id LIMIT 1000" % GATE
    )
    extra = set(ADJUDICATED)
    # The 492 x100 heads (rows lead with the Tender id) and the 505 adjudicated heads, copied to the box
    # beside this script (scp them first; a missing file only narrows the extra set, and the count says so).
    for path in ("x100-partner-hits-2026-10-09.json", "adjudication-wf_368978c8-198.json"):
        try:
            data = json.load(open(path))
        except OSError:
            print(f"missing {path}: its ids are not in the cohort", file=sys.stderr)
            continue
        rows = data["verdicts"] if isinstance(data, dict) else data
        for row in rows:
            extra.add(row["id"] if isinstance(row, dict) else row[0])
    cohort = sorted(set(heads) | set(lots) | extra)
    print(f"cohort: {len(heads)} heads, {len(lots)} lot-value Tenders, {len(extra)} adjudicated -> {len(cohort)}", file=sys.stderr)

    report = {"cohort": len(cohort), "heads": len(heads), "lot_value_tenders": len(lots), "rows": [],
              "bare_above_head": [], "dropped": collections.Counter(), "carriers": 0, "result_rows": 0}
    for i in range(0, len(cohort), 100):
        batch = cohort[i:i + 100]
        versions = sql_in("SELECT tender_id, seq, caused_by_notice_id FROM tender_versions WHERE tender_id IN ({ids})", batch)
        notice_of = {(t, s): n for t, s, n in versions}
        for t, s, _ in versions:
            SEQS.setdefault(t, []).append(s)
        notices = sorted({n for _, _, n in versions})
        amounts = sql_in(
            "SELECT tender_id, seq, lot_id, field, cents, currency, eur_cents, quality FROM tender_version_amounts "
            "WHERE tender_id IN ({ids})", batch)
        awards = sql_in(
            "SELECT tender_id, seq, awarded_cents, awarded_currency FROM tender_version_lot_results "
            "WHERE tender_id IN ({ids}) AND awarded_cents IS NOT NULL", batch)
        lot_keys = collections.defaultdict(set)
        for t, key in sql_in("SELECT tender_id, lot_key FROM lots WHERE tender_id IN ({ids})", batch):
            lot_keys[t].add(key)
        head = dict(sql_in("SELECT id, current_value_eur_cents FROM tenders WHERE id IN ({ids})", batch))
        fields = list(RESULT_FIELDS) + list(LOT_REFS) + [BARE]
        flist = ",".join(f"'{f}'" for f in fields)
        nl_amounts = sql_in(
            "SELECT notice_id, section_id, field_id, cents, currency FROM notice_amounts WHERE notice_id IN ({ids}) "
            f"AND field_id IN ({flist})", notices) if notices else []
        refs = {}
        for n, sec, v in (sql_in(
            "SELECT notice_id, section_id, value FROM notice_ids WHERE notice_id IN ({ids}) "
            f"AND field_id IN ({','.join(repr(f) for f in LOT_REFS)})".replace('"', "'"), notices) if notices else []):
            refs[(n, sec)] = v
        tender_of_notice = collections.defaultdict(set)
        for (t, s), n in notice_of.items():
            tender_of_notice[n].add(t)
        new_figures = collections.defaultdict(list)  # tender -> [(cents, currency, field)]
        for n, sec, f, c, cur in nl_amounts:
            for t in tender_of_notice[n]:
                if f == BARE:
                    # Head proxy in EUR is unknown for a non-EUR figure; list the figure beside the head.
                    report["bare_above_head"].append({"tender": t, "notice": n, "cents": c, "currency": cur, "head_eur_cents": head.get(t)})
                    continue
                report["result_rows"] += 1
                lot = refs.get((n, sec))
                if lot is None:
                    report["dropped"]["no_lot_ref"] += 1
                    continue
                if lot not in lot_keys[t]:
                    report["dropped"]["no_such_lot"] += 1
                    continue
                new_figures[t].append((c, cur, RESULT_FIELDS[f]))
        report["carriers"] += len(new_figures)
        old_by_tender = collections.defaultdict(list)
        for t, s, lot, f, c, cur, eur, q in amounts:
            old_by_tender[t].append((s, lot, f, c, cur, eur, q))
        awards_by_tender = collections.defaultdict(list)
        for t, s, c, cur in awards:
            awards_by_tender[t].append((c, cur))
        for t, new in new_figures.items():
            old = old_by_tender[t]
            for s, lot, f, c, cur, eur, q in old:
                if q is not None or eur is None or eur < GATE:
                    continue
                before = [c2 for _, _, _, c2, cur2, _, q2 in old if cur2 == cur and q2 is None] + \
                         [c2 for c2, cur2 in awards_by_tender[t] if cur2 == cur]
                bx, bk = partners(c, before)
                rnew = [c2 for c2, cur2, _ in new if cur2 == cur]
                nx, nk = partners(c, rnew)
                cls = None
                if not bx and not bk and nx and not nk:
                    cls = "new-x100"
                elif bx and not bk and nk:
                    cls = "switch"
                elif not bx and not bk and nk:
                    cls = "new-k3"
                if cls:
                    corroborated = len({f2 for s2, _, f2, c2, cur2, _, q2 in old if s2 == s and c2 == c and cur2 == cur and q2 is None}) > 1
                    report["rows"].append({"tender": t, "seq": s, "lot_id": lot, "field": f, "cents": c, "currency": cur,
                                           "eur_cents": eur, "class": cls, "corroborated": corroborated,
                                           "head_eur_cents": head.get(t), "result_figures": sorted(set(rnew))[:8]})
        print(f"batch {i // 100 + 1}/{(len(cohort) + 99) // 100}: {len(report['rows'])} rows so far", file=sys.stderr)
    report["tenders_listed"] = sorted({r["tender"] for r in report["rows"]})
    json.dump(report, open(out, "w"), indent=1)
    print(f"listed {len(report['tenders_listed'])} Tenders, {len(report['rows'])} rows; carriers {report['carriers']}; "
          f"dropped {dict(report['dropped'])}; bare {len(report['bare_above_head'])}", file=sys.stderr)


if __name__ == "__main__":
    main(sys.argv)
