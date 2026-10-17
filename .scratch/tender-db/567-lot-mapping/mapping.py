"""Re-implementation of crates/ingest/src/project.rs read_legacy_results' lot mapping
(lot_by_no first-wins over TED-LOT_NO|TED-LOT_NUMBER|TED-ITEM of Lot-kind sections; an award
block's lot_key = lot_by_no.get(v) for EACH of its TED-LOT_NO|TED-LOT_NUMBER|TED-ITEM values in
turn, so the last one decides, None when the last one misses) against the correct mapping
(award LOT_NO -> the lot section publishing that LOT_NO)."""
import re

LOT_KINDS = ("Lot", "LotsGroup", "Part")
RESULT_KINDS = ("LotResult", "LotTender", "TenderingParty", "SettledContract")
KEYF = ("TED-LOT_NO", "TED-LOT_NUMBER", "TED-ITEM")
NOF = ("TED-LOT_NO", "TED-LOT_NUMBER")


def sec_sort_key(sid):
    m = re.match(r"^([A-Z]+)-(\d+)$", sid)
    if m:
        return (m.group(1), int(m.group(2)), sid)
    return (sid, 0, sid)


def flat(content, order):
    """Values as (section_id, field_id, ordinal, value-dict) in one of two orders:
    'lex'  — (section_id, field_id, ordinal): the notice_ids PRIMARY KEY order the projection's
             parsed_chunk / parsed_by_ids reads return (and the API content order);
    'doc'  — the walker's emission order: sections LOT-1..n numerically (document order),
             TED-ITEM first in a section (emitted at section open), then the rest by ordinal."""
    rows = []
    for s in content["sections"]:
        for v in s["values"]:
            rows.append((s["section_id"], v["field_id"], v["ordinal"], v))
    if order == "lex":
        rows.sort(key=lambda r: (r[0], r[1], r[2]))
    else:
        rows.sort(key=lambda r: (sec_sort_key(r[0]), 0 if r[1] == "TED-ITEM" else 1, r[2], r[1]))
    return rows


def norm(x):
    x = x.strip()
    digits = re.findall(r"\d+", x)
    if len(digits) == 1:
        return str(int(digits[0]))
    return re.sub(r"[^0-9a-z]", "", x.lower())


def analyze(content, order="lex"):
    secs = {s["section_id"]: s for s in content["sections"]}
    kind = {k: s["kind"] for k, s in secs.items()}
    parent = {k: s["parent_section_id"] for k, s in secs.items()}

    def enclosing(sid, ks):
        cur = sid
        for _ in range(max(len(secs), 1)):
            if cur not in secs:
                return None
            if kind[cur] in ks:
                return cur
            cur = parent[cur]
            if cur is None:
                return None
        return None

    rows = flat(content, order)
    results = [s["section_id"] for s in content["sections"] if s["kind"] == "LotResult"]
    lots = [s["section_id"] for s in content["sections"] if s["kind"] in LOT_KINDS]
    lots.sort(key=sec_sort_key)
    results.sort(key=sec_sort_key)

    # code: lot_by_no, first wins
    lot_by_no = {}
    for sid, fid, _o, v in rows:
        if kind.get(sid) in LOT_KINDS and fid in KEYF and v["type"] == "id":
            lot_by_no.setdefault(v["value"].strip(), sid)
    code_key = {r: None for r in results}
    award_vals = {r: [] for r in results}
    for sid, fid, _o, v in rows:
        owner = enclosing(sid, RESULT_KINDS)
        if owner is None or owner not in code_key:
            continue
        if v["type"] == "id" and v.get("is_ref") and v.get("scheme") != "ojs":
            continue  # the winner arm comes first in the match
        if fid in KEYF and v["type"] == "id":
            code_key[owner] = lot_by_no.get(v["value"].strip())
            award_vals[owner].append((fid, v["value"]))

    # per-section published numbers
    lot_item = {l: [] for l in lots}
    lot_no = {l: [] for l in lots}
    for sid, fid, _o, v in rows:
        if sid in lot_item and v["type"] == "id":
            if fid == "TED-ITEM":
                lot_item[sid].append(v["value"].strip())
            elif fid in NOF:
                lot_no[sid].append(v["value"].strip())

    out_awards = []
    for r in results:
        nos = [val.strip() for f, val in award_vals[r] if f in NOF]
        items = [val.strip() for f, val in award_vals[r] if f == "TED-ITEM"]
        correct, how = None, None
        if nos:
            no = nos[-1]
            exact = [l for l in lots if no in lot_no[l]]
            if len(exact) >= 1:
                correct, how = exact[0], ("exact" if len(exact) == 1 else "exact-dup")
            else:
                nn = norm(no)
                fuzzy = [l for l in lots if any(norm(x) == nn for x in lot_no[l])]
                if fuzzy:
                    correct, how = fuzzy[0], ("norm" if len(fuzzy) == 1 else "norm-dup")
                else:
                    how = "unmatched"
        else:
            how = "no-award-lotno"
        if how in ("no-award-lotno", "unmatched"):
            status = "undefined" if code_key[r] is None else "code-guess"
        elif code_key[r] == correct:
            status = "ok"
        elif code_key[r] is None:
            status = "unresolved"
        else:
            status = "WRONG"
        out_awards.append({
            "res": r, "award_item": items, "award_lot_no": nos,
            "code_lot": code_key[r], "correct_lot": correct, "correct_how": how, "status": status,
        })

    lot_rows = [{"lot": l, "item": lot_item[l], "lot_no": lot_no[l]} for l in lots]
    # at risk: a lot section whose ITEM differs from its LOT_NO, where that ITEM string is another
    # section's LOT_NO or some award block's LOT_NO (a collision is possible)
    all_lot_nos = {}
    for l in lots:
        for x in lot_no[l]:
            all_lot_nos.setdefault(x, set()).add(l)
    award_nos = {x for a in out_awards for x in a["award_lot_no"]}
    item_ne_no = False
    at_risk = False
    for l in lots:
        if lot_item[l] and lot_no[l] and set(lot_item[l]) != set(lot_no[l]):
            item_ne_no = True
            for it in lot_item[l]:
                if (all_lot_nos.get(it, set()) - {l}) or it in award_nos and it not in lot_no[l]:
                    at_risk = True
    # an award block whose ITEM differs from its LOT_NO (harmless: LOT_NO is read last)
    award_item_ne_no = any(a["award_item"] and a["award_lot_no"] and set(a["award_item"]) != set(a["award_lot_no"]) for a in out_awards)
    return {
        "lots": lot_rows,
        "awards": out_awards,
        "lot_by_no": lot_by_no,
        "item_ne_lotno": item_ne_no,
        "at_risk": at_risk,
        "award_item_ne_lotno": award_item_ne_no,
        "wrong": [a for a in out_awards if a["status"] == "WRONG"],
    }


def td_type(content):
    for s in content["sections"]:
        if s["section_id"] == "PROCEDURE":
            for v in s["values"]:
                if v["field_id"] == "TED-TD_DOCUMENT_TYPE":
                    return v.get("code")
    return None


def classify(content):
    """multi-lot CAN: TD_DOCUMENT_TYPE 7, >= 2 LotResult sections, >= 2 distinct award LOT_NO values."""
    td = td_type(content)
    res = [s for s in content["sections"] if s["kind"] == "LotResult"]
    nos = set()
    for s in content["sections"]:
        pass
    a = analyze(content)
    for aw in a["awards"]:
        if aw["award_lot_no"]:
            nos.add(norm(aw["award_lot_no"][-1]))
    if td != "7":
        return "not-can", td, len(res), len(nos)
    if len(res) < 2:
        return "can-single-award", td, len(res), len(nos)
    if len(nos) >= 2:
        return "multilot-can", td, len(res), len(nos)
    if not nos:
        return "can-multi-award-no-lotno", td, len(res), len(nos)
    return "can-multi-award-one-lotno", td, len(res), len(nos)
