"""Issue 570: replay today's legacy lot keying and the proposed fold-time remap (option B) over cached chains.

Mirrors, in Python, on the parsed values the projection reads (`/v1/notices/{id}/content`) and the Tender's
version chain (`/v1/tenders/{id}` `versions`, fold order = seq):

  TODAY (crates/ingest/src/project.rs at 86f2745):
    - each notice's lot sections (Lot / LotsGroup / Part) are keyed by their own section id `LOT-k` and
      merged into the Tender's lots by that key (`fold`'s lot carry); the carried lot's number is the latest
      section's `published_lot_no` (TED-LOT_NO, else TED-LOT_NUMBER; first value), None included;
    - an award block resolves to a section of ITS OWN notice (issue 567, `LegacyLotNumbers::read/resolve`);
    - an award notice with no lot section takes the inherited lot printing the block's number (issue 568,
      `match_inherited_lots`, G1, G2 = `lot_renumbered` over the carry).

  B (decision.md, rule steps 1-8): before the carry, a legacy notice's lot sections are remapped onto the
    Tender's existing keys by printed number:
      target(s) = the ONE carried `Lot` printing s's number (exact trimmed first, else normalise_lot_number;
                  `LegacyLotNumbers::inherited` + `lookup`), else s's own key;
      R-guard 1 (injective): two sections of the notice on one key -> the notice keeps its own keys;
      R-guard 2 (no silent merge): a numbered section that matched nothing may keep its own key only when
                  the carried lot there prints no number or the same one; else the notice keeps its own keys;
      an unnumbered section keeps its own key (as today; nothing to check);
    the notice's award blocks follow their section's target; G2 is read after the remap.

Usage: python3 -I remap_port.py <out.json> <groups.json> <cache dir>... [--exhibits id,id,...]
Each cache dir is searched recursively for tender_<id>.json / v1_tenders_<id>.json and
notice_<id>.json / v1_notices_<id>_content.json; the first file found wins, dirs in argument order.

Run of 2026-10-18 (port_result.json): the k*_skeptic/live dirs first (the skeptics' re-fetches), then cache_k1..12,
then the issue exhibits' chains from ../567-lot-mapping/readback/raw-api-cache.tar.gz extracted to a directory, with
--exhibits 8375276,8387180,8480304,7415397,7025548,6120045,7025760. Result: today's port equals the served lot on
1,316 of 1,316 placed blocks (the other 280 are R2.0.8 blocks 568's pending drain places); B remaps 11 of 384
sample lot-publishing notices (2 refused by K2, 3 with nothing to match), moves 32 of 1,596 blocks in 8 Tenders,
mints or sweeps no lot key in 211 chains, and every one of its 124 moved sections is titled like its target.
"""
import difflib
import json
import os
import sys
import unicodedata
from collections import Counter, defaultdict

LOT_KINDS = ("Lot", "LotsGroup", "Part")
RESULT_KINDS = ("LotResult", "LotTender", "TenderingParty", "SettledContract")
PUBLISHED_LOT_NO_FIELDS = ["TED-LOT_NO", "TED-LOT_NUMBER", "BT-137-Lot", "BT-137-Part", "BT-137-LotsGroup",
                           "DE1-ProcurementProjectLot-ID"]


def norm(published):
    """normalise_lot_number, line for line."""
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
            for f in sorted(files):
                if not f.endswith(".json"):
                    continue
                p = os.path.join(root, f)
                stem = f[:-5]
                if stem.startswith("v1_tenders_"):
                    tenders.setdefault(int(stem[len("v1_tenders_"):]), p)
                elif stem.startswith("tender_") and stem[7:].isdigit():
                    tenders.setdefault(int(stem[7:]), p)
                elif stem.startswith("v1_notices_") and stem.endswith("_content"):
                    notices.setdefault(int(stem[len("v1_notices_"):-len("_content")]), p)
                elif stem.startswith("notice_") and stem[7:].isdigit():
                    notices.setdefault(int(stem[7:]), p)
    return tenders, notices


def ordered_values(section):
    # the store's primary-key order inside one section: (field_id, ordinal)
    return sorted(section["values"], key=lambda v: (v["field_id"], v.get("ordinal", 0)))


def is_legacy(content):
    return any(v["field_id"].startswith("TED-") for s in content["sections"] for v in s["values"])


def lot_sections(content):
    """[(section_id, kind, printed number or None, item, title)] in section-id order (the fold's BTreeMap)."""
    out = []
    for s in sorted(content["sections"], key=lambda s: s["section_id"]):
        if s["kind"] not in LOT_KINDS:
            continue
        best = None
        item = title = None
        for v in ordered_values(s):
            if v["type"] == "id" and v["field_id"] in PUBLISHED_LOT_NO_FIELDS:
                rank = PUBLISHED_LOT_NO_FIELDS.index(v["field_id"])
                if best is None or rank < best[0]:
                    best = (rank, v["value"])
            if v["type"] == "id" and v["field_id"] == "TED-ITEM" and item is None:
                item = v["value"]
            if v["type"] == "text" and v["field_id"] == "TED-TITLE" and title is None:
                title = v["value"]
        out.append((s["section_id"], s["kind"], best[1] if best else None, item, title))
    return out


def same_notice_map(content):
    """Issue 567's LegacyLotNumbers::read + resolve: {block key: (lot section key or None, printed number)}."""
    secs = {s["section_id"]: s for s in content["sections"]}

    def enclosing(sid, kinds):
        cur = sid
        for _ in range(max(len(secs), 1)):
            if cur not in secs:
                return None
            if secs[cur]["kind"] in kinds:
                return cur
            cur = secs[cur]["parent_section_id"]
            if cur is None:
                return None
        return None

    numbered, items = [], []
    lot_ids = set()
    for s in content["sections"]:
        if s["kind"] not in LOT_KINDS:
            continue
        lot_ids.add(s["section_id"])
        for v in s["values"]:
            if v["type"] != "id":
                continue
            val = v["value"].strip()
            if not val:
                continue
            if v["field_id"] in ("TED-LOT_NO", "TED-LOT_NUMBER"):
                numbered.append((s["section_id"], val))
            elif v["field_id"] == "TED-ITEM":
                items.append((s["section_id"], val))
    by_item = not numbered
    nums, its = defaultdict(set), defaultdict(set)
    for sid, val in numbered:
        nums[sid].add(norm(val))
    for sid, val in items:
        its[sid].add(norm(val))
    agree = (not by_item) and all(nums[l] and its[l] and nums[l] == its[l] and len(its[l]) == 1 and None not in its[l]
                                  for l in lot_ids)
    ordinal_is_key = by_item or agree
    exact, normd = defaultdict(set), defaultdict(set)
    for sid, val in (items if by_item else numbered):
        exact[val].add(sid)
        n = norm(val)
        if n is not None:
            normd[n].add(sid)

    def lookup(v):
        v = v.strip()
        if v in exact:
            return next(iter(exact[v])) if len(exact[v]) == 1 else None
        n = norm(v)
        if n is None or n not in normd:
            return None
        return next(iter(normd[n])) if len(normd[n]) == 1 else None

    refs = defaultdict(list)
    for s in sorted(content["sections"], key=lambda s: s["section_id"]):
        owner = enclosing(s["section_id"], RESULT_KINDS)
        if owner is None or secs[owner]["kind"] != "LotResult":
            continue
        for v in ordered_values(s):
            if v["type"] == "id" and v.get("is_ref") and v.get("scheme") != "ojs":
                continue
            if v["type"] == "id" and v["field_id"] in ("TED-LOT_NO", "TED-LOT_NUMBER", "TED-ITEM"):
                refs[owner].append((v["field_id"], v["value"]))
    out = {}
    for r in sorted(s["section_id"] for s in content["sections"] if s["kind"] == "LotResult"):
        rr = refs.get(r, [])
        printed = next((v for f in ("TED-LOT_NO", "TED-LOT_NUMBER") for g, v in rr if g == f), None)
        field = next((f for f in ("TED-LOT_NO", "TED-LOT_NUMBER", "TED-ITEM") if any(g == f for g, _ in rr)), None)
        lot = None
        if field is not None and not (field == "TED-ITEM" and not ordinal_is_key):
            for g, v in rr:
                if g != field:
                    continue
                hit = lookup(v)
                if hit is None or (lot is not None and lot != hit):
                    lot = None
                    break
                lot = hit
        out[r] = (lot, printed)
    return out


class Inherited:
    """LegacyLotNumbers::inherited + lookup over carried lots {key: {kind, num}}."""

    def __init__(self, lots):
        self.exact, self.normd = defaultdict(set), defaultdict(set)
        for key, lot in lots.items():
            if lot["kind"] != "Lot":
                continue
            num = (lot["num"] or "").strip()
            if not num:
                continue
            self.exact[num].add(key)
            n = norm(num)
            if n is not None:
                self.normd[n].add(key)

    def lookup(self, p):
        p = p.strip()
        if p in self.exact:
            return next(iter(self.exact[p])) if len(self.exact[p]) == 1 else None
        n = norm(p)
        if n is None or n not in self.normd:
            return None
        return next(iter(self.normd[n])) if len(self.normd[n]) == 1 else None


def lot_renumbered(carried, republished):
    c = norm(carried) if carried else None
    if c is None:
        return False
    return (norm(republished) if republished else None) != c


def match_inherited(results, lots):
    """568's match_inherited_lots over [(block, lot, printed)] -> new list."""
    numbered = [(i, p.strip()) for i, (_, lot, p) in enumerate(results) if lot is None and p and p.strip()]
    if not numbered:
        return results
    inh = Inherited(lots)
    if not inh.exact:
        return results
    block_numbers = {norm(p) for _, p in numbered}
    lot_numbers = {norm(n) for n in inh.exact}
    if len(numbered) >= 2 and len(block_numbers) == 1 and len(lot_numbers) >= 2:
        return results
    out = list(results)
    for i, p in numbered:
        out[i] = (results[i][0], inh.lookup(p), results[i][2])
    return out


def remap(sections, lots, legacy):
    """Option B: {own key: tender key}, how ('identity'|'remapped'|'refused:<guard>'|'anchor'|'no-numbers')."""
    own = {s[0]: s[0] for s in sections}
    if not legacy or not lots:
        return own, "anchor"
    inh = Inherited(lots)
    if not inh.exact:
        return own, "no-numbers"
    target, matched = {}, {}
    for key, kind, num, _, _ in sections:
        p = (num or "").strip()
        t = inh.lookup(p) if p and kind == "Lot" else None
        target[key] = t if t is not None else key
        matched[key] = t is not None
    if len(set(target.values())) != len(target):
        return own, "refused:injective"
    for key, kind, num, _, _ in sections:
        p = (num or "").strip()
        if matched[key] or not p:
            continue
        carried = lots.get(key)
        if carried and (carried["num"] or "").strip() and norm(carried["num"]) != norm(p):
            return own, "refused:silent-merge"
    return target, ("identity" if all(target[k] == k for k in target) else "remapped")


def fold(chain, contents, mode):
    """chain: [notice_id] in seq order. Returns per-version state and per-notice remap info."""
    lots = {}  # key -> {kind, num, sources: [(notice, section, num, title)]}
    renumbered = False
    results = {}  # (notice, block) -> (lot, printed)
    info = []
    for nid in chain:
        c = contents.get(nid)
        if c is None:
            info.append({"notice": nid, "missing": True})
            continue
        legacy = is_legacy(c)
        secs = lot_sections(c)
        if mode == "B":
            target, how = remap(secs, lots, legacy)
        else:
            target, how = {s[0]: s[0] for s in secs}, "today"
        moved = {k: v for k, v in target.items() if k != v}
        info.append({"notice": nid, "sections": len(secs), "how": how, "moved": moved,
                     "map": {s[0]: s[2] for s in secs}})
        for key, kind, num, item, title in secs:
            tk = target[key]
            if tk in lots:
                carried = lots[tk]
                carried["kind"] = kind
                renumbered |= lot_renumbered(carried["num"], num)
                carried["num"] = num
                carried["sources"].append((nid, key, num, title))
            else:
                lots[tk] = {"kind": kind, "num": num, "sources": [(nid, key, num, title)]}
        same = same_notice_map(c)
        rs = [(b, (target.get(lot, lot) if lot else None), p) for b, (lot, p) in sorted(same.items())]
        if not secs and not renumbered:
            rs = match_inherited(rs, {k: v for k, v in lots.items()})
        for b, lot, p in rs:
            results[(nid, b)] = (lot, p)
    return lots, results, info


def mixed_keys(lots):
    """Tender keys whose merged sections print two or more different normalised numbers (a conflated lot)."""
    out = []
    for key, lot in lots.items():
        nums = {norm(s[2]) for s in lot["sources"] if s[2] and s[2].strip()}
        if len(nums) > 1:
            out.append(key)
    return sorted(out)


def squeeze(text):
    text = unicodedata.normalize("NFKD", (text or "").lower())
    return " ".join("".join(ch for ch in text if not unicodedata.combining(ch)).split())


def title_check(chain, contents, info_b):
    """Each section B moves: is its own title closer to the anchor notice's title at the target key than at its own
    key? 'target' / 'own' / 'tie' (within 0.02) / 'untitled'. The anchor is the first lot-publishing notice."""
    anchor = next((lot_sections(contents[n]) for n in chain if n in contents and lot_sections(contents[n])), [])
    at = {s[0]: s[4] for s in anchor}
    # Where the anchor prints no titles, the first notice that keeps the anchor's numbering (B outcome identity)
    # and prints them is the reference instead (8375276: CN 18679338 is untitled, 21026735 agrees and is titled).
    if not any(at.values()):
        for i in info_b:
            if i.get("how") == "identity":
                ref = {s[0]: s[4] for s in lot_sections(contents[i["notice"]])}
                if any(ref.values()):
                    at = ref
                    break
    out = []
    for i in info_b:
        if not i.get("moved"):
            continue
        own_titles = {s[0]: s[4] for s in lot_sections(contents[i["notice"]])}
        for own, tgt in sorted(i["moved"].items()):
            t, a_t, a_o = own_titles.get(own), at.get(tgt), at.get(own)
            if not (t and a_t and a_o):
                verdict, st, so = "untitled", None, None
            else:
                st = difflib.SequenceMatcher(None, squeeze(t), squeeze(a_t)).ratio()
                so = difflib.SequenceMatcher(None, squeeze(t), squeeze(a_o)).ratio()
                verdict = "target" if st > so + 0.02 else "own" if so > st + 0.02 else "tie"
            out.append({"notice": i["notice"], "section": own, "target": tgt, "verdict": verdict,
                        "title": t, "anchor_title_at_target": a_t, "anchor_title_at_own": a_o,
                        "sim_target": None if st is None else round(st, 3), "sim_own": None if so is None else round(so, 3)})
    return out


def option_a(chain, contents, keys_today):
    """Option A's churn: key every numbered legacy section `LOT-<normalised number>` (unnumbered: its own key)."""
    keys, rekeyed, nonint = set(), 0, 0
    for nid in chain:
        if nid not in contents:
            continue
        for key, kind, num, item, title in lot_sections(contents[nid]):
            n = norm(num) if num and num.strip() else None
            keys.add("LOT-" + n if n else key)
            if n and n != norm(key.split("-", 1)[-1]):
                rekeyed += 1
            if n and not n.isdigit():
                nonint += 1
    return {"sections_rekeyed": rekeyed, "keys_minted": sorted(keys - set(keys_today)),
            "keys_swept": sorted(set(keys_today) - keys), "non_integer_numbers": nonint}


def lot_no_for(lots, key):
    if key is None or key not in lots:
        return None
    return lots[key]["num"]


def main():
    args = sys.argv[1:]
    exhibits = []
    if "--exhibits" in args:
        i = args.index("--exhibits")
        exhibits = [int(x) for x in args[i + 1].split(",") if x]
        args = args[:i] + args[i + 2:]
    out_path, groups_path, dirs = args[0], args[1], args[2:]
    groups = json.load(open(groups_path))
    sample = [it["tender_id"] for g in groups["groups"] for it in g["items"]]
    tenders, notices = index_cache(dirs)
    report = {"sample": [], "exhibits": []}
    stats = Counter()
    for population, ids in (("sample", sample), ("exhibits", exhibits)):
        for tid in ids:
            if tid not in tenders:
                report[population].append({"tender_id": tid, "error": "no tender in cache"})
                stats[population + ":no-tender"] += 1
                continue
            t = json.load(open(tenders[tid]))
            chain = [v["caused_by_notice_id"] for v in sorted(t["versions"], key=lambda v: v["seq"])]
            contents = {}
            for nid in chain:
                if nid in notices:
                    contents[nid] = json.load(open(notices[nid]))
            missing = [n for n in chain if n not in contents]
            lots_t, res_t, info_t = fold(chain, contents, "today")
            lots_b, res_b, info_b = fold(chain, contents, "B")
            served = {(r["notice_id"], r["key"]): r.get("lot") for r in t["lot_results"]}
            parity = Counter()
            for k, (lot, _) in res_t.items():
                if k in served:
                    parity["same" if served[k] == lot else "differ"] += 1
            moved = []
            for k in sorted(res_t):
                lt, p = res_t[k]
                lb, _ = res_b.get(k, (None, None))
                if lt != lb:
                    moved.append({"notice": k[0], "block": k[1], "printed": p, "today": lt, "B": lb,
                                  "today_lot_no": lot_no_for(lots_t, lt), "B_lot_no": lot_no_for(lots_b, lb)})
            notice_hows = [i.get("how") for i in info_b if i.get("sections")]
            row = {
                "tender_id": tid,
                "chain": chain,
                "missing_notices": missing,
                "lot_publishing_notices": sum(1 for i in info_b if i.get("sections")),
                "maps": {str(i["notice"]): i["map"] for i in info_b if i.get("sections")},
                "B_notice_outcomes": {str(i["notice"]): i["how"] for i in info_b if i.get("sections")},
                "B_moved_sections": {str(i["notice"]): i["moved"] for i in info_b if i.get("moved")},
                "keys_today": sorted(lots_t),
                "keys_B": sorted(lots_b),
                "keys_minted_by_B": sorted(set(lots_b) - set(lots_t)),
                "keys_swept_by_B": sorted(set(lots_t) - set(lots_b)),
                "conflated_keys_today": mixed_keys(lots_t),
                "conflated_keys_B": mixed_keys(lots_b),
                "head_lot_numbers_today": {k: v["num"] for k, v in sorted(lots_t.items())},
                "head_lot_numbers_B": {k: v["num"] for k, v in sorted(lots_b.items())},
                "results": len(res_t),
                "results_moved_by_B": moved,
                "parity_today_vs_served": dict(parity),
            }
            row["B_title_check"] = title_check(chain, contents, info_b)
            row["A"] = option_a(chain, contents, row["keys_today"])
            report[population].append(row)
            stats[population + ":tenders"] += 1
            for h in notice_hows:
                stats[population + ":notice:" + str(h)] += 1
            stats[population + ":results"] += len(res_t)
            stats[population + ":results_moved_by_B"] += len(moved)
            stats[population + ":tenders_moved_by_B"] += bool(moved or row["B_moved_sections"])
            stats[population + ":tenders_conflated_today"] += bool(row["conflated_keys_today"])
            stats[population + ":tenders_conflated_B"] += bool(row["conflated_keys_B"])
            stats[population + ":keys_minted_by_B"] += len(row["keys_minted_by_B"])
            stats[population + ":keys_swept_by_B"] += len(row["keys_swept_by_B"])
            stats[population + ":A_tenders_rekeyed"] += bool(row["A"]["sections_rekeyed"])
            stats[population + ":A_sections_rekeyed"] += row["A"]["sections_rekeyed"]
            stats[population + ":A_tenders_minting_or_sweeping"] += bool(row["A"]["keys_minted"] or row["A"]["keys_swept"])
            stats[population + ":A_keys_minted"] += len(row["A"]["keys_minted"])
            stats[population + ":A_keys_swept"] += len(row["A"]["keys_swept"])
            stats[population + ":A_tenders_with_non_integer_numbers"] += bool(row["A"]["non_integer_numbers"])
            stats[population + ":B_sections_moved"] += sum(len(v) for v in row["B_moved_sections"].values())
            for t in row["B_title_check"]:
                stats[population + ":B_title_" + t["verdict"]] += 1
            stats[population + ":parity_same"] += parity["same"]
            stats[population + ":parity_differ"] += parity["differ"]
            stats[population + ":missing_notices"] += len(missing)
    report["stats"] = dict(sorted(stats.items()))
    json.dump(report, open(out_path, "w"), indent=1, ensure_ascii=False)
    print(json.dumps(report["stats"], indent=1))


if __name__ == "__main__":
    main()
