"""Issue 567 unit 3: a Python port of the rule now in crates/ingest/src/project.rs (`LegacyLotNumbers`,
`normalise_lot_number`), compared with the pre-567 mapping (mapping.py's faithful copy, primary-key value order)
over /content JSON files (the unit-1 sample's cache; the files are not committed).

Usage: python3 -I new_mapping.py OUT.json 'GLOB' ['GLOB' ...]
Result on the 694 cached legacy award notices of unit 1 (2026-10-17): 3,413 award blocks, 3,362 unchanged, 51 changed:
45 from the wrong lot to the lot their LOT_NO names (all 15 verified notices) and 6 from no lot to it through the
normalised number (20004819, 20356321); none moved elsewhere or dropped to the Tender (new_mapping_result.json)."""
import json, os, re, sys, glob, collections
sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mapping  # today's mapping, faithful copy

LOT_KINDS = mapping.LOT_KINDS
RESULT_KINDS = mapping.RESULT_KINDS
SEPS = " .:-#°º"

def norm(raw):
    s = raw.strip().lower()
    for p in ("lotto", "lote", "lot", "los"):
        if s.startswith(p) and not (s[len(p):len(p)+1].isalpha()):
            s = s[len(p):].lstrip(SEPS)
            for m in ("nr", "no", "n"):
                if s.startswith(m) and not s[len(m):len(m)+1].isalpha():
                    s = s[len(m):].lstrip(SEPS)
                    break
            break
    s = s.strip()
    while len(s) > 1 and s[0] == "0" and s[1].isdigit():
        s = s[1:]
    return s or None

def new_map(content):
    secs = {s["section_id"]: s for s in content["sections"]}
    kind = {k: s["kind"] for k, s in secs.items()}
    parent = {k: s["parent_section_id"] for k, s in secs.items()}
    def enclosing(sid, ks):
        cur = sid
        for _ in range(max(len(secs), 1)):
            if cur not in secs: return None
            if kind[cur] in ks: return cur
            cur = parent[cur]
            if cur is None: return None
    numbered, items = [], []
    for s in content["sections"]:
        if s["kind"] not in LOT_KINDS: continue
        for v in s["values"]:
            if v["type"] != "id": continue
            val = v["value"].strip()
            if not val: continue
            if v["field_id"] in ("TED-LOT_NO", "TED-LOT_NUMBER"): numbered.append((s["section_id"], val))
            elif v["field_id"] == "TED-ITEM": items.append((s["section_id"], val))
    by_item = not numbered
    keyed = items if by_item else numbered
    # a numbered notice whose every lot section publishes its ITEM as its number: ordinal == number
    nums = collections.defaultdict(set); its = collections.defaultdict(set)
    for sid, val in numbered: nums[sid].add(norm(val))
    for sid, val in items: its[sid].add(norm(val))
    lot_ids = {s["section_id"] for s in content["sections"] if s["kind"] in LOT_KINDS}
    agree = (not by_item) and all(nums[l] and its[l] and nums[l] == its[l] and len(its[l]) == 1 and None not in its[l] for l in lot_ids)
    ordinal_is_key = by_item or agree
    exact = collections.defaultdict(set); normd = collections.defaultdict(set)
    for sid, val in keyed:
        exact[val].add(sid)
        n = norm(val)
        if n is not None: normd[n].add(sid)
    def lookup(v):
        v = v.strip()
        if v in exact:
            return (next(iter(exact[v])), "exact") if len(exact[v]) == 1 else (None, "exact-ambiguous")
        n = norm(v)
        if n is None or n not in normd: return (None, "unmatched")
        return (next(iter(normd[n])), "norm") if len(normd[n]) == 1 else (None, "norm-ambiguous")
    refs = collections.defaultdict(list)
    for s in content["sections"]:
        owner = enclosing(s["section_id"], RESULT_KINDS)
        if owner is None or kind.get(owner) != "LotResult": continue
        for v in s["values"]:
            if v["type"] == "id" and v.get("is_ref") and v.get("scheme") != "ojs": continue
            if v["type"] == "id" and v["field_id"] in ("TED-LOT_NO", "TED-LOT_NUMBER", "TED-ITEM"):
                refs[owner].append((v["field_id"], v["value"].strip()))
    out = {}
    for r in [s["section_id"] for s in content["sections"] if s["kind"] == "LotResult"]:
        rr = refs.get(r, [])
        # LegacyLotNumbers::resolve: the first field the block carries decides; an ITEM only where the ordinal is a key
        field = next((f for f in ("TED-LOT_NO", "TED-LOT_NUMBER", "TED-ITEM") if any(g == f for g, _ in rr)), None)
        if field is None:
            out[r] = (None, "no-key"); continue
        if field == "TED-ITEM" and not ordinal_is_key:
            out[r] = (None, "ordinal-not-key"); continue
        found, how = None, None
        for g, v in rr:
            if g != field: continue
            lot, h = lookup(v)
            if lot is None: found, how = None, h; break
            if found is None: found, how = lot, (h if field != "TED-ITEM" or by_item else "item-agree")
            elif found != lot: found, how = None, "disagree"; break
        out[r] = (found, how)
    return out, by_item

def main(paths):
    stats = collections.Counter(); changes = []
    seen = set()
    for p in paths:
        try:
            c = json.load(open(p))
        except Exception:
            continue
        if not isinstance(c, dict) or "sections" not in c or "notice_id" not in c: continue
        if c["notice_id"] in seen: continue
        if not any(s["kind"] == "LotResult" for s in c["sections"]): continue
        if not any(v["field_id"].startswith("TED-") for s in c["sections"] for v in s["values"]): continue
        seen.add(c["notice_id"])
        old = mapping.analyze(c, "lex")
        new, by_item = new_map(c)
        stats["notices"] += 1
        stats["by_item" if by_item else "numbered"] += 1
        has_lots = any(s["kind"] in LOT_KINDS for s in c["sections"])
        for a in old["awards"]:
            r = a["res"]; o = a["code_lot"]; n, how = new[r]
            stats["blocks"] += 1
            stats["how:" + str(how)] += 1
            if o == n: stats["same"] += 1; continue
            kind = ("fix" if n == a["correct_lot"] and a["correct_lot"] is not None else
                    "to-none" if n is None else "from-none" if o is None else "moved")
            stats["chg:" + kind] += 1
            changes.append({"notice_id": c["notice_id"], "res": r, "old": o, "new": n, "how": how, "correct": a["correct_lot"],
                            "award_lot_no": a["award_lot_no"], "award_item": a["award_item"], "kind": kind, "by_item": by_item})
    return stats, changes

if __name__ == "__main__":
    paths = []
    for g in sys.argv[2:]:
        paths += glob.glob(g)
    stats, changes = main(sorted(paths))
    json.dump({"stats": stats, "changes": changes}, open(sys.argv[1], "w"), indent=1)
    print(json.dumps(stats, indent=1))
    for ch in changes:
        if ch["kind"] != "fix": print(ch)
