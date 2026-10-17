"""Issue 568: replay the proposed cross-notice lot match over the 71 measured Tenders.

Mirrors, in Python, the rule decision.md states, on the parsed values the projection reads
(`/v1/notices/{id}/content`, cached by the measurement's skeptics) and the Tender's version chain
(`/v1/tenders/{id}` `versions`, fold order = seq):

  1. the lots the CAN's version inherits: every lot section (Lot / LotsGroup / Part) of every
     notice folded BEFORE the CAN, keyed by section id, each carrying the number its LATEST section
     printed (TED-LOT_NO, else TED-LOT_NUMBER; first value; None included) -- the fold's carry of
     `LotState::published_lot_no`;
  2. the CAN publishes no lot section of its own (else 567's same-notice map is the authority);
  3. a block with no same-notice lot and a printed number (`award_block_lot_number`: first
     TED-LOT_NO, else first TED-LOT_NUMBER, in primary-key value order) is matched against the
     inherited lots of kind `Lot` that print a number: exact (trimmed) first; only if no lot prints
     it exactly, `normalise_lot_number`; exactly one lot -> that key, else no lot;
  4. G1 (unit 3): an award notice whose two or more numbered blocks all normalise to one number, on
     a Tender whose inherited `Lot`s print two or more distinct (normalised) numbers, matches none
     of its blocks (5476747's CAN 18543084: `LOT_NUMBER 1` on all three);
  5. G2 (unit 3): once a notice folded before the award republished an inherited lot key under a
     different normalised number, or with none (the carried one had a number), no award block is
     matched -- `fold`'s `lots_renumbered` flag, set in the lot carry (`lot_renumbered`).

Usage: python3 -I cross_notice_port.py <measurement.json> <out.json> [--extra <extra_port.jsonl>] <cache dir>...
With --extra, the judge's extra strata are replayed too: every award notice of each listed chain
(the jsonl's `chain` rows), compared with the jsonl's pre-guard `port`; a block that G1 or G2 now
refuses is listed under `extra.guard_refusals`.
Each cache dir is searched recursively for <id>.json, tenders_<id>.json, v1_tenders_<id>.json,
notices_<id>_content.json and v1_notices_<id>_content.json.
"""
import json
import os
import re
import sys
from collections import Counter

LOT_KINDS = ("Lot", "LotsGroup", "Part")
RESULT_KINDS = ("LotResult", "LotTender", "TenderingParty", "SettledContract")
PUBLISHED_LOT_NO_FIELDS = ["TED-LOT_NO", "TED-LOT_NUMBER", "BT-137-Lot", "BT-137-Part", "BT-137-LotsGroup",
                           "DE1-ProcurementProjectLot-ID"]


def normalise_lot_number(published):
    """crates/ingest/src/project.rs normalise_lot_number, line for line."""
    seps = " .:-#°º"
    rest = published.strip().lower()
    for word in ("lotto", "lote", "lot", "los"):
        if rest.startswith(word) and not rest[len(word):][:1].isalpha():
            rest = rest[len(word):].lstrip(seps)
            for mark in ("nr", "no", "n"):
                if rest.startswith(mark) and not rest[len(mark):][:1].isalpha():
                    rest = rest[len(mark):].lstrip(seps)
                    break
            break
    rest = rest.strip()
    while len(rest) > 1 and rest[0] == "0" and rest[1].isdigit():
        rest = rest[1:]
    return rest or None


def index_cache(dirs):
    tenders, notices = {}, {}
    for d in dirs:
        for root, _, files in os.walk(d):
            for f in files:
                p = os.path.join(root, f)
                m = re.fullmatch(r"(?:v1_)?tenders_(\d+)\.json", f)
                if m:
                    tenders.setdefault(int(m.group(1)), p)
                    continue
                m = re.fullmatch(r"(?:v1_)?notices_(\d+)_content\.json", f)
                if m:
                    notices.setdefault(int(m.group(1)), p)
                    continue
                m = re.fullmatch(r"(\d+)\.json", f)
                if m:
                    n = int(m.group(1))
                    (tenders if "tenders" in root.split(os.sep)[-1] else notices).setdefault(n, p)
    return tenders, notices


def load(p):
    with open(p, encoding="utf-8") as fh:
        return json.load(fh)


def sections_of(content):
    secs = content.get("sections") or []
    return {s["section_id"]: s for s in secs}


def published_number(section):
    """`published_lot_numbers`: first value of the first PUBLISHED_LOT_NO_FIELDS field the section carries."""
    best = None
    for v in sorted(section.get("values", []), key=lambda v: (v["field_id"], v.get("ordinal", 0))):
        if v.get("type") != "id" or v["field_id"] not in PUBLISHED_LOT_NO_FIELDS:
            continue
        rank = PUBLISHED_LOT_NO_FIELDS.index(v["field_id"])
        if best is None or rank < best[0]:
            best = (rank, v["value"])
    return None if best is None else best[1]


def enclosing(secs, sid, kinds):
    cur = sid
    for _ in range(max(len(secs), 1)):
        s = secs.get(cur)
        if s is None:
            return None
        if s["kind"] in kinds:
            return s["section_id"]
        cur = s.get("parent_section_id")
        if cur is None:
            return None
    return None


def award_refs(secs):
    """Per LotResult section: its (field, value) lot references, in primary-key value order."""
    rows = []
    for s in secs.values():
        for v in s.get("values", []):
            rows.append((s["section_id"], v["field_id"], v.get("ordinal", 0), v))
    rows.sort(key=lambda r: (r[0], r[1], r[2]))
    refs = {sid: [] for sid, s in secs.items() if s["kind"] == "LotResult"}
    for sid, field, _, v in rows:
        owner = enclosing(secs, sid, RESULT_KINDS)
        if owner not in refs:
            continue
        if field in ("TED-LOT_NO", "TED-LOT_NUMBER", "TED-ITEM") and v.get("type") == "id":
            refs[owner].append((field, v["value"]))
    return refs


def award_block_lot_number(refs):
    for field in ("TED-LOT_NO", "TED-LOT_NUMBER"):
        for f, val in refs:
            if f == field:
                return val
    return None


def cross_notice_lookup(carried, printed):
    """carried: {lot key: (kind, published number or None)} -> (lot key or None, reason)."""
    numbered = [(k, n.strip()) for k, (kind, n) in carried.items() if kind == "Lot" and n is not None and n.strip()]
    p = printed.strip()
    exact = sorted(k for k, n in numbered if n == p)
    if exact:
        return (exact[0], "exact") if len(exact) == 1 else (None, "ambiguous")
    key = normalise_lot_number(p)
    if key is None:
        return None, "no_match"
    norm = sorted(k for k, n in numbered if normalise_lot_number(n) == key)
    if len(norm) == 1:
        return norm[0], "normalised"
    return None, ("ambiguous" if norm else "no_match")


def lot_renumbered(carried, republished):
    """G2, `project.rs` `lot_renumbered`: the carried section printed a number and the republished
    one prints another once normalised, or none (a blank number is none)."""
    old = normalise_lot_number(carried) if carried is not None else None
    if old is None:
        return False
    new = normalise_lot_number(republished) if republished is not None else None
    return new != old


def carry(carried, secs):
    """The fold's lot carry for one notice: each lot section (re)places its key's (kind, number).
    Returns (the notice's own lot sections, whether it renumbered an inherited lot -- G2)."""
    lots = [s for s in secs.values() if s["kind"] in LOT_KINDS]
    renumbered = False
    for s in lots:
        number = published_number(s)
        old = carried.get(s["section_id"])
        if old is not None and lot_renumbered(old[1], number):
            renumbered = True
        carried[s["section_id"]] = (s["kind"], number)
    return lots, renumbered


def match_notice(carried, renumbered, own_lots, refs):
    """`project.rs` `fold` + `match_inherited_lots` for one award notice: {result key: (lot, why)}."""
    printed = {res: award_block_lot_number(r) for res, r in refs.items()}
    numbered = {res: p.strip() for res, p in printed.items() if p is not None and p.strip()}
    lot_numbers = {normalise_lot_number(n.strip()) for kind, n in carried.values()
                   if kind == "Lot" and n is not None and n.strip()}
    g1 = (len(numbered) >= 2 and len({normalise_lot_number(p) for p in numbered.values()}) == 1
          and len(lot_numbers) >= 2)
    out = {}
    for res, p in printed.items():
        if own_lots:
            out[res] = (None, "can_has_own_lots")
        elif p is None:
            out[res] = (None, "no_number")
        elif renumbered:
            out[res] = (None, "g2")
        elif g1 and res in numbered:
            out[res] = (None, "g1")
        else:
            out[res] = cross_notice_lookup(carried, p)
    return out


def res_order(k):
    return int(k.split("-")[1]) if k.split("-")[1].isdigit() else 0


def replay_extra(extra_path, tenders_idx, notices_idx):
    """The judge's extra strata: every award notice of each chain, with the guards."""
    chains, pre = {}, {}
    with open(extra_path, encoding="utf-8") as fh:
        for line in fh:
            row = json.loads(line)
            if "chain" in row:
                chains[row["tender"]] = row["chain"]
            else:
                pre[(row["tender"], row["notice"], row["res"])] = row["port"]
    blocks, refusals, missing = [], [], []
    for tid, chain in chains.items():
        carried, renumbered = {}, False
        for n in chain:
            if n not in notices_idx:
                missing.append(n)
                continue
            secs = sections_of(load(notices_idx[n]))
            own_lots, renumbers = carry(carried, secs)
            renumbered = renumbered or renumbers
            refs = award_refs(secs)
            if not refs:
                continue
            for res, (port, why) in sorted(match_notice(carried, renumbered, own_lots, refs).items(),
                                           key=lambda kv: res_order(kv[0])):
                before = pre.get((tid, n, res), "absent")
                blocks.append({"tender_id": tid, "notice": n, "result_key": res, "port_lot": port,
                               "port_reason": why, "pre_guard_port": before, "agree_pre_guard": port == before})
                if why in ("g1", "g2"):
                    refusals.append(blocks[-1])
    return {
        "tenders": len(chains),
        "blocks": len(blocks),
        "agree_with_pre_guard_port": sum(b["agree_pre_guard"] for b in blocks),
        "matched": sum(b["port_lot"] is not None for b in blocks),
        "guard_refusals": refusals,
        "disagreements_other_than_guards": [b for b in blocks if not b["agree_pre_guard"] and b["port_reason"] not in ("g1", "g2")],
        "missing_notices": missing,
        "tender_5476747": [b["port_lot"] for b in blocks if b["tender_id"] == 5476747],
        "rows": blocks,
    }


def main():
    meas_path, out_path, *dirs = sys.argv[1:]
    extra_path = None
    if dirs[:1] == ["--extra"]:
        extra_path, dirs = dirs[1], dirs[2:]
    meas = load(meas_path)
    tenders_idx, notices_idx = index_cache(dirs)
    out, tally, disagreements = [], Counter(), []
    for t in meas["tenders"]:
        tid, can = t["tender_id"], t["can_notice_id"]
        rec = {"tender_id": tid, "can_notice_id": can}
        if tid not in tenders_idx:
            rec["error"] = "tender not cached"
            out.append(rec)
            continue
        tender = load(tenders_idx[tid])
        versions = sorted(tender["versions"], key=lambda v: v["seq"])
        chain = [v["caused_by_notice_id"] for v in versions]
        rec["chain"] = chain
        if can not in chain:
            rec["error"] = "CAN not in the Tender's versions"
            out.append(rec)
            continue
        before = chain[: chain.index(can)]
        carried, lot_publishers, missing, renumbered = {}, Counter(), [], False
        for n in before:
            if n not in notices_idx:
                missing.append(n)
                continue
            secs = sections_of(load(notices_idx[n]))
            lots, renumbers = carry(carried, secs)
            renumbered = renumbered or renumbers
            if lots:
                lot_publishers[n] = len(lots)
        rec["missing_notices"] = missing
        rec["lots_renumbered"] = renumbered
        rec["lot_publishing_notices"] = dict(lot_publishers)
        rec["carried_lots"] = {k: v[1] for k, v in sorted(carried.items())}
        if can not in notices_idx:
            rec["error"] = "CAN content not cached"
            out.append(rec)
            continue
        can_secs = sections_of(load(notices_idx[can]))
        own_lots = [s for s in can_secs.values() if s["kind"] in LOT_KINDS]
        rec["can_own_lot_sections"] = len(own_lots)
        refs = award_refs(can_secs)
        matched = match_notice(carried, renumbered, own_lots, refs)
        measured = {b["result_key"]: b for b in t["blocks"]}
        blocks = []
        for res in sorted(refs, key=res_order):
            printed = award_block_lot_number(refs[res])
            items = [v for f, v in refs[res] if f == "TED-ITEM"]
            port, why = matched[res]
            m = measured.get(res)
            exp = None if m is None else m.get("matched_lot_key") if m["cls"] in ("unique_right", "unique_unverifiable", "unique_wrong") else None
            agree = m is not None and port == exp
            lot_nos = [v for f, v in refs[res] if f == "TED-LOT_NUMBER"]
            blocks.append({"result_key": res, "printed": printed, "lot_number_values": lot_nos, "can_items": items,
                           "port_lot": port, "port_reason": why, "measured_cls": None if m is None else m["cls"],
                           "measured_lot": exp, "agree": agree})
            tally[(why, None if m is None else m["cls"], agree)] += 1
            if not agree:
                disagreements.append({"tender_id": tid, **blocks[-1]})
        unmeasured = sorted(set(measured) - set(refs))
        rec["blocks"] = blocks
        rec["measured_blocks_not_in_can"] = unmeasured
        out.append(rec)
    summary = {
        "tenders": len(out),
        "errors": [r for r in out if "error" in r],
        "blocks": sum(len(r.get("blocks", [])) for r in out),
        "tally_reason_cls_agree": {f"{a}|{b}|{c}": n for (a, b, c), n in sorted(tally.items(), key=str)},
        "disagreements": disagreements,
        "multi_valued_lot_number_blocks": [
            {"tender_id": r["tender_id"], "result_key": b["result_key"], "values": b["lot_number_values"]}
            for r in out for b in r.get("blocks", []) if len(set(v.strip() for v in b["lot_number_values"])) > 1],
        "tenders_with_two_lot_publishing_notices": [r["tender_id"] for r in out if len(r.get("lot_publishing_notices", {})) > 1],
        "tenders_where_a_can_has_own_lots": [r["tender_id"] for r in out if r.get("can_own_lot_sections")],
        "blocks_matched_only_by_normalisation": sum(1 for r in out for b in r.get("blocks", []) if b["port_reason"] == "normalised"),
        "agree": sum(1 for r in out for b in r.get("blocks", []) if b["agree"]),
        "g1_refused_blocks": sum(1 for r in out for b in r.get("blocks", []) if b["port_reason"] == "g1"),
        "g2_refused_blocks": sum(1 for r in out for b in r.get("blocks", []) if b["port_reason"] == "g2"),
        "tenders_with_a_renumbered_lot": [r["tender_id"] for r in out if r.get("lots_renumbered")],
    }
    result = {"summary": summary, "tenders": out}
    if extra_path:
        extra = replay_extra(extra_path, tenders_idx, notices_idx)
        result["extra"] = extra
        summary["extra"] = {k: v for k, v in extra.items() if k != "rows"}
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(result, fh, ensure_ascii=False, indent=1)
    print(json.dumps(summary, ensure_ascii=False, indent=1)[:8000])


if __name__ == "__main__":
    main()
