#!/usr/bin/env python3
"""W7 (issue 551) for issue 577 unit 2: build set A (ids 577 U2 can move, plus every id of 577's
shadow/overlap pair lists), set B (every field id or stem the fold reads, W inputs and non-W reads
labelled), and intersect them. Read-only. 0 box statements.

Usage: python3 -I build_sets.py <repo-root> <w7-dir> <head-src-root> [<worktree-literals.tsv>]
  <w7-dir> holds moves_bind.tsv / moves_extra.tsv (emulate_577u2.py) and fold_literals.tsv
  (scan_fold_literals.py over <head-src-root>, i.e. `git show HEAD:` copies of the fold sources).
Writes <w7-dir>/set_A.tsv, set_B.tsv, intersection.tsv, intersection_broad_stem_only.tsv,
  nonW_reads_in_A.tsv
"""
import re, sys
from collections import defaultdict

ROOT, W7, HEAD = sys.argv[1], sys.argv[2], sys.argv[3]
WT_LITERALS = sys.argv[4] if len(sys.argv) > 4 else None
TRIAGE = f"{ROOT}/.scratch/tender-db/577-triage"


def stem(fid):
    """project.rs `stem` (9311): cut at the SECOND '-'."""
    idx = [i for i, c in enumerate(fid) if c == "-"]
    return fid[: idx[1]] if len(idx) >= 2 else fid


# ------------------------------------------------------------------ set A
A = defaultdict(set)  # id -> tags
for line in open(f"{W7}/moves_bind.tsv").read().splitlines()[1:]:
    old, new, prov, vers = line.split("\t")
    A[old].add("code:bind-old")
    A[new].add("code:bind-new")
for line in open(f"{W7}/moves_extra.tsv").read().splitlines()[1:]:
    old, new, branch, vers = line.split("\t")
    A[old].add("code:extra-old")
    A[new].add("code:extra-new")

shadow = open(f"{TRIAGE}/shadow.out").read()
head, _, overlap = shadow.partition("== non-nested predicate overlaps involving a graft (manual look) ==")
n_shadow = n_overlap = 0
for m in re.finditer(r"^- served (\S+)  instead of  (\S+) \((\w+)\)", head, re.M):
    A[m.group(1)].add("shadow.out:shadow")
    A[m.group(2)].add("shadow.out:shadow")
    n_shadow += 1
ov_pairs = set()
for m in re.finditer(r"^- (\S+) \((\w+)\) (\S+)\n  vs (\S+) \((\w+)\) (\S+)\n  versions: (\S+)", overlap, re.M):
    A[m.group(1)].add("shadow.out:overlap")
    A[m.group(4)].add("shadow.out:overlap")
    ov_pairs.add((m.group(1), m.group(4)))
    n_overlap += 1
for m in re.finditer(r"^- (\S+) \((\w+)\) over (\S+) at ", open(f"{TRIAGE}/shadow_extra.out").read(), re.M):
    A[m.group(1)].add("shadow_extra.out")
    A[m.group(3)].add("shadow_extra.out")
assert n_overlap == 4811, n_overlap  # 577:135 "the rest of shadow.out's overlap section (4,811 pairs)"

# ------------------------------------------------------------------ set B
# Per fold context (the const/fn the literal sits in): (category, W tier, match mode).
#   W tier: "W" direct W input; "W-ind" indirect (winner identity, chain/version order); "nonW".
#   match: "stem" = canonical_name / stem-table semantics (full id OR stem(id) equals the key);
#          "exact" = full-id comparison; "prefix" = starts_with.
CTX = {
    # amounts (Fact::Amount: procedure figures, lot estimates/awards, partner-only result values)
    "AMOUNTS": ("AMOUNT", "W", "stem"),
    "RESULT_LOT_AMOUNTS": ("AMOUNT(result framework, partner-only)", "W", "stem"),
    "RESULT_AMOUNT_STEMS": ("AMOUNT/LOT_RESULT(BT-720 award, BT-161)", "W", "stem"),
    "amount_target": ("AMOUNT", "W", "exact"),
    "TAX_BASIS_FIELDS": ("AMOUNT(tax basis)", "W", "exact"),
    "withheld_source": ("AMOUNT(withheld marker BT-195(src))", "W", "prefix"),
    "F14_VALUE_FIELD": ("AMOUNT(F14 corrected value, legacy)", "W", "exact"),
    "F14_OLD_VALUE_FIELD": ("AMOUNT(F14 corrected value, legacy)", "W", "exact"),
    "F14_TARGET_FIELD": ("AMOUNT/DATE(F14 target, legacy)", "W", "exact"),
    # stated totals (551 B1 frozen table)
    "stated_total_kind": ("STATED_TOTAL", "W", "exact"),
    "notice_stated_totals": ("STATED_TOTAL(TED-FORM cn_form)", "W", "exact"),
    # results graph: lot results, winners, tenderer refs, bids, contracts
    "read_results": ("LOT_RESULT/WINNER/TENDERER_REF", "W", "stem"),
    "RESULT_ID_STEMS": ("LOT_RESULT/TENDERER_REF", "W", "stem"),
    "RESULT_CODE_STEMS": ("LOT_RESULT", "W", "stem"),
    "RESULT_DATE_STEMS": ("LOT_RESULT", "W", "stem"),
    "RESULT_NUMBER_STEMS": ("LOT_RESULT", "W", "stem"),
    "read_legacy_results": ("LOT_RESULT(legacy)", "W", "exact"),
    "read_move_award": ("LOT_RESULT(legacy MOVE)", "W", "exact"),
    "MOVE_OPERATOR_FIELD": ("LOT_RESULT(legacy MOVE winner)", "W", "exact"),
    "LEGACY_NON_WINNER_REFS": ("LOT_RESULT(legacy)", "W", "exact"),
    "LEGACY_AWARDED_MARKER": ("LOT_RESULT(legacy)", "W", "exact"),
    "LEGACY_NO_AWARD_MARKER": ("LOT_RESULT(legacy)", "W", "exact"),
    "LEGACY_DISCONTINUED_MARKER": ("LOT_RESULT(legacy)", "W", "exact"),
    "LEGACY_UNSUCCESSFUL_MARKER": ("LOT_RESULT(legacy)", "W", "exact"),
    "LEGACY_GROUP_MARKER": ("LOT_RESULT(group, legacy)", "W", "exact"),
    "LEGACY_NO_GROUP_MARKER": ("LOT_RESULT(group, legacy)", "W", "exact"),
    "LEGACY_AWARD_DATE_FIELD": ("LOT_RESULT(legacy)", "W", "exact"),
    "LEGACY_AWARD_DATE_FIELD_R207": ("LOT_RESULT(legacy)", "W", "exact"),
    "LEGACY_AWARD_DATE_FIELD_R209": ("LOT_RESULT(legacy)", "W", "exact"),
    "LEGACY_BID_COUNT_FIELDS": ("LOT_RESULT(statistics, legacy)", "W", "exact"),
    "SDK01_RESULT_CODE_FIELD": ("LOT_RESULT(sdk-0.1)", "W", "exact"),
    "SDK01_AWARD_DATE_FIELD": ("LOT_RESULT(sdk-0.1)", "W", "exact"),
    "AWARD_TITLE_FIELDS": ("LOT_RESULT(legacy award placement)", "W", "exact"),
    "resolve": ("LOT_NUMBER(legacy)", "W", "exact"),
    # published lot numbers (551 B1)
    "PUBLISHED_LOT_NO_FIELDS": ("LOT_NUMBER", "W", "exact"),
    "award_block_lot_number<": ("LOT_NUMBER(legacy)", "W", "exact"),
    # winner identity (R1 / R6 / n_winners read organization ids)
    "ORG_NAME_FIELD": ("WINNER_IDENTITY", "W-ind", "exact"),
    "ORG_IDENTIFIER_FIELD": ("WINNER_IDENTITY", "W-ind", "exact"),
    "ORG_NATIONALID_FIELD": ("WINNER_IDENTITY", "W-ind", "exact"),
    "ORG_COUNTRY_FIELD": ("WINNER_IDENTITY", "W-ind", "exact"),
    "ORG_NAME_FIELDS": ("WINNER_IDENTITY", "W-ind", "exact"),
    "ORG_COUNTRY_FIELDS": ("WINNER_IDENTITY", "W-ind", "exact"),
    "ORG_NAME_FIELD_IDS": ("WINNER_IDENTITY", "W-ind", "exact"),
    "SDK01_PARTY_NAME_FIELDS": ("WINNER_IDENTITY", "W-ind", "exact"),
    "SDK01_PARTY_COUNTRY_FIELDS": ("WINNER_IDENTITY", "W-ind", "exact"),
    "TOUCHPOINT_NAME_FIELD": ("WINNER_IDENTITY(576 U2 touchpoint, worktree only)", "W-ind", "exact"),
    "TOUCHPOINT_COUNTRY_FIELD": ("WINNER_IDENTITY(576 U2 touchpoint, worktree only)", "W-ind", "exact"),
    # role references: OPT-300-/OPT-301- any suffix (role_name); 530-T's role guard decides a
    # GUID-twin group's identity, so even non-winner roles reach winner identity
    "role_name": ("ROLE_REF (530-T role guard -> winner identity)", "W-ind", "prefix"),
    "de1_mark_reference": ("ROLE_REF", "W-ind", "prefix"),
    "guid_twin_role_ref": ("ROLE_REF", "W-ind", "prefix"),
    "role_kind": ("ROLE_REF(legacy)", "W-ind", "prefix"),
    # chain membership and version order (539's precedent: version order sets current_seq)
    "PROCEDURE_KEY_FIELD": ("CHAIN", "W-ind", "exact"),
    "LOGICAL_NOTICE_FIELD": ("CHAIN", "W-ind", "exact"),
    "PREVIOUS_NOTICE_FIELD": ("CHAIN", "W-ind", "exact"),
    "GROUP_ID_FIELD": ("CHAIN", "W-ind", "exact"),
    "GROUP_MEMBER_FIELD": ("CHAIN", "W-ind", "exact"),
    "SDK01_FOLDER_FIELD": ("CHAIN", "W-ind", "exact"),
    "DE1_FOLDER_FIELD": ("CHAIN", "W-ind", "exact"),
    "LEGACY_OWN_NUMBER_FIELDS": ("CHAIN", "W-ind", "exact"),
    "PUBLICATION_DATE_FIELDS": ("CHAIN(version order)", "W-ind", "exact"),
    "DISPATCH_DATE_FIELDS": ("CHAIN(version order)", "W-ind", "exact"),
    "SUBTYPE_FIELD": ("CHAIN(fold order)", "W-ind", "exact"),
    "EFORMS_NOTICE_TYPE_FIELDS": ("CHAIN(fold order)", "W-ind", "exact"),
    "LEGACY_DOC_TYPE_FIELDS": ("CHAIN(fold order)", "W-ind", "exact"),
    # lot titles (R6 reads the served lot title) and the original language that picks it
    "ORIGINAL_LANG_FIELDS": ("LOT_TITLE(R6 language pick)", "W", "exact"),
    # non-W reads
    "CLASSIFICATIONS": ("cpv/nuts", "nonW", "stem"),
    "DATES": ("deadline/duration dates", "nonW", "stem"),
    "CONTRACTING_SYSTEM_FIELDS": ("dps", "nonW", "exact"),
    "PLACE_COUNTRY_FIELDS": ("place country", "nonW", "exact"),
    "PROCEDURE_FIELDS": ("procedure type", "nonW", "exact"),
    "PROCEDURE_MARKERS": ("procedure type (legacy)", "nonW", "exact"),
    "procedure_type": ("procedure type", "nonW", "exact"),
    "PROCEDURE_TYPE_FIELD": ("procedure type (role census)", "nonW", "exact"),
    "NATURE_STEMS": ("contract nature", "nonW", "stem"),
    "contract_nature": ("contract nature", "nonW", "stem"),
    "F14_DATE_FIELD": ("F14 date (legacy)", "nonW", "exact"),
    "LEGACY_ADDRESS_NUTS_FIELDS": ("address nuts (skipped)", "nonW", "exact"),
    "LEGACY_SECTION_NUTS_FIELD": ("nuts (legacy)", "nonW", "exact"),
    "LEGACY_DPS_MARKER": ("dps (legacy)", "nonW", "exact"),
    "SDK01_DEADLINE_FIELD": ("deadline (sdk-0.1)", "nonW", "exact"),
    "OJ_HEADING_FIELD": ("tender title fallback (legacy)", "nonW", "exact"),
}
TEXTS_TITLE = ("LOT_TITLE(R6 reads the served lot title)", "W", "stem")
TEXTS_OTHER = ("text fact (description/selection_criterion/tenderer_requirement/...)", "nonW", "stem")
READ_LINES = {  # NoticeState::read's own literals, by HEAD line
    "SDK01-NoticeTypeCode": ("sdk-0.1 award deadline filter", "nonW", "exact"),
    "BT-749": ("selection_criterion section filter (577 U2)", "nonW", "stem"),
    "TED-NO_DOC_OJS": ("tender title fallback guard (legacy)", "nonW", "exact"),
    "TED-LOT_NO": ("LOT_NUMBER(legacy)", "W", "exact"),
    "TED-LOT_NUMBER": ("LOT_NUMBER(legacy)", "W", "exact"),
    "TED-ITEM": ("LOT_NUMBER(legacy ordinal)", "W", "exact"),
}

B = []  # (key, match, category, tier, source)


def add(key, match, cat, tier, src):
    B.append((key, match, cat, tier, src))


src_head = open(f"{HEAD}/crates/ingest/src/project.rs").read()
texts_block = src_head[src_head.index("const TEXTS: &[(&str, &str)] = &["):]
texts_block = texts_block[: texts_block.index("\n];")]
texts = dict(re.findall(r'\(\s*"([^"]+)",\s*"([^"]+)",?\s*\)', texts_block))


def literal_rows(path):
    for line in open(path).read().splitlines():
        if line.startswith("#"):
            continue
        loc, ctx, lit = line.split("\t")
        yield loc, ctx, lit


seen_ctx = set()
for loc, ctx, lit in literal_rows(f"{W7}/fold_literals.tsv"):
    if ctx == "DE1_FIELD_ALIASES":
        continue  # handled below with the target's own category
    if ctx == "withheld_source":
        # withheld_source_fields reads EVERY `BT-195(<src>)-<ctx>` code into (section, src); the set
        # is looked up only for amounts (stem of the amount id), BT-720 bids and BT-759/760 stats.
        # So a declaration is a W input exactly when <src> is one of those stems.
        for src in ("BT-27", "BT-271", "BT-161", "BT-118", "BT-1118", "BT-709", "BT-660", "BT-720", "BT-759", "BT-760"):
            add(f"BT-195({src})", "prefix", f"AMOUNT/LOT_RESULT quality (withheld marker on {src})", "W", f"{loc} {ctx}")
        add(lit, "prefix", "withheld declaration of a non-W source (read, never looked up for W)", "nonW", f"{loc} {ctx}")
        seen_ctx.add(ctx)
        continue
    if ctx == "TEXTS":
        cat, tier, match = TEXTS_TITLE if texts.get(lit) == "title" else TEXTS_OTHER
    elif ctx == "read":
        cat, tier, match = READ_LINES[lit]
    elif ctx in CTX:
        cat, tier, match = CTX[ctx]
    else:
        raise SystemExit(f"unclassified fold context {ctx} ({lit} at {loc})")
    seen_ctx.add(ctx)
    add(lit, match, cat, tier, f"{loc} {ctx}")
if WT_LITERALS:
    head_rows = {(c, l) for _, c, l in literal_rows(f"{W7}/fold_literals.tsv")}
    for loc, ctx, lit in literal_rows(WT_LITERALS):
        if (ctx, lit) not in head_rows:
            cat, tier, match = CTX[ctx]
            add(lit, match, cat, tier, f"worktree {loc} {ctx} (uncommitted, not in HEAD)")

# Non-literal reads (constants imported or built, suffix rules)
add("VALUE_COST", "suffix", "AMOUNT(legacy VALUE_COST, AMOUNT_ELEMENT)", "W", "project.rs AMOUNT_ELEMENT / has_destination")
add("OCDS-ContractValue", "exact", "AMOUNT/LOT_RESULT(FTS contract value)", "W", "fts::parse::CONTRACT_VALUE via read_results")
add("OPT-300-", "prefix", "ROLE_REF (530-T role guard -> winner identity)", "W-ind", "role_name / guid_twin_role_ref")
add("OPT-301-", "prefix", "ROLE_REF (530-T role guard -> winner identity)", "W-ind", "role_name / guid_twin_role_ref")
add("OPT-300-Tenderer", "exact", "TENDERER_REF/WINNER", "W", "read_results TenderingParty")
add("OPT-301-Tenderer-SubCont", "exact", "TENDERER_REF", "W", "read_results TenderingParty")

# DE1_FIELD_ALIASES: normalise_de1 rewrites the DE-1.x id to its eForms target before any read;
# the DE-1.x id takes the categories its target matches in B (fold semantics).
consts = dict(re.findall(r'^(?:pub )?const ([A-Z0-9_]+): &str = "([^"]+)";', src_head, re.M))
al_block = src_head[src_head.index("const DE1_FIELD_ALIASES: &[(&str, &str)] = &["):]
al_block = al_block[: al_block.index("\n];")]
aliases = []
for m in re.finditer(r'\(\s*"([^"]+)",\s*(?:"([^"]+)"|([A-Z0-9_]+)),?\s*\)', al_block):
    aliases.append((m.group(1), m.group(2) or consts[m.group(3)]))
assert len(aliases) >= 70, len(aliases)


def fold_match(fid, key, match):
    if match == "exact":
        return fid == key
    if match == "stem":
        return fid == key or stem(fid) == key
    if match == "prefix":
        return fid.startswith(key)
    if match == "suffix":
        return fid.endswith(key)
    raise ValueError(match)


base_B = list(B)
for de1, target in aliases:
    hits = [(cat, tier) for key, match, cat, tier, _ in base_B if fold_match(target, key, match)]
    if not hits:
        hits = [("alias target read nowhere?", "nonW")]
    for cat, tier in sorted(set(hits)):
        add(de1, "exact", f"{cat} [via DE1 alias -> {target}]", tier, "project.rs DE1_FIELD_ALIASES")

with open(f"{W7}/set_B.tsv", "w") as fh:
    fh.write("key\tmatch\ttier\tcategory\tsource\n")
    for key, match, cat, tier, src in sorted(B, key=lambda r: (r[3] != "W", r[3], r[0])):
        fh.write(f"{key}\t{match}\t{tier}\t{cat}\t{src}\n")

with open(f"{W7}/set_A.tsv", "w") as fh:
    fh.write("id\tstem\ttags\n")
    for fid in sorted(A):
        fh.write(f"{fid}\t{stem(fid)}\t{','.join(sorted(A[fid]))}\n")

# ------------------------------------------------------------------ intersection
TIER_ORDER = {"W": 0, "W-ind": 1, "nonW": 2}
fold_hits = defaultdict(list)  # id -> [(tier, cat, key, match, src)]
for fid in A:
    for key, match, cat, tier, src in B:
        if fold_match(fid, key, match):
            fold_hits[fid].append((tier, cat, key, match, src))

# The task's literal rule: {a, stem(a)} against {b, stem(b)} for every B key (generous).
broad_only = defaultdict(list)
for fid in A:
    sa = {fid, stem(fid)}
    for key, match, cat, tier, src in B:
        if match in ("prefix", "suffix"):
            continue
        if sa & {key, stem(key)} and not fold_match(fid, key, match):
            broad_only[fid].append((tier, cat, key, match, src))

with open(f"{W7}/intersection.tsv", "w") as fh:
    fh.write("id\tA_tags\ttier\tcategory\tB_key\tB_match\tB_source\n")
    for fid in sorted(fold_hits, key=lambda f: (min(TIER_ORDER[h[0]] for h in fold_hits[f]), f)):
        for tier, cat, key, match, src in sorted(set(fold_hits[fid]), key=lambda h: TIER_ORDER[h[0]]):
            fh.write(f"{fid}\t{','.join(sorted(A[fid]))}\t{tier}\t{cat}\t{key}\t{match}\t{src}\n")
with open(f"{W7}/intersection_broad_stem_only.tsv", "w") as fh:
    # One row per A id: the matches the task's generous reading ({a, stem(a)} vs {b, stem(b)}) adds
    # beyond the fold's own matching, aggregated (the colliding stems, how many B keys, which tiers).
    fh.write("id\tA_tags\tcolliding_stems\tn_B_keys\ttiers\texample_B_key\tfold_semantics_hit\n")
    for fid in sorted(broad_only):
        rows = sorted(set(broad_only[fid]))
        stems_ = sorted({stem(fid)} & {stem(r[2]) for r in rows} | ({fid} & {stem(r[2]) for r in rows}))
        tiers_ = sorted({r[0] for r in rows}, key=lambda t: TIER_ORDER[t])
        ex = next((r[2] for r in rows if r[0] == tiers_[0]), rows[0][2])
        fh.write(f"{fid}\t{','.join(sorted(A[fid]))}\t{','.join(stems_)}\t{len({r[2] for r in rows})}\t{','.join(tiers_)}\t{ex}\t{'yes' if fid in fold_hits else 'no'}\n")


def tiers(fid):
    return {h[0] for h in fold_hits.get(fid, [])}


w_hits = sorted(f for f in fold_hits if "W" in tiers(f))
wind_hits = sorted(f for f in fold_hits if "W-ind" in tiers(f) and "W" not in tiers(f))
nonw = sorted(f for f in fold_hits if tiers(f) == {"nonW"})
code_ids = {f for f in A if any(t.startswith("code:") for t in A[f])}
with open(f"{W7}/nonW_reads_in_A.tsv", "w") as fh:
    fh.write("id\tA_tags\tnon-W read\n")
    for fid in nonw:
        fh.write(f"{fid}\t{','.join(sorted(A[fid]))}\t{'; '.join(sorted({h[1] + ' (' + h[2] + ')' for h in fold_hits[fid]}))}\n")

print(f"A: {len(A)} ids ({len(code_ids)} from the code-derived moves; shadow rows {n_shadow}, overlap pairs {n_overlap}, distinct overlap pairs {len(ov_pairs)})")
print(f"B: {len(B)} entries ({sum(1 for b in B if b[3]=='W')} W, {sum(1 for b in B if b[3]=='W-ind')} W-ind, {sum(1 for b in B if b[3]=='nonW')} nonW)")
print(f"fold-semantics hits: W {len(w_hits)}, W-ind only {len(wind_hits)}, nonW-only reads {len(nonw)}; broad stem-only extra matches: {len(broad_only)} ids")
print("W hits:")
for f in w_hits:
    print("  ", f, "|", ",".join(sorted(A[f])), "|", "; ".join(sorted({h[1] for h in fold_hits[f] if h[0] == 'W'})))
print("W-ind hits:")
for f in wind_hits:
    print("  ", f, "|", ",".join(sorted(A[f])), "|", "; ".join(sorted({h[1] for h in fold_hits[f] if h[0] == 'W-ind'})))
print("code-derived ids with a W/W-ind hit:", sorted(f for f in code_ids if tiers(f) & {"W", "W-ind"}))
broad_new_w = sorted(f for f in broad_only if f not in fold_hits and any(h[0] in ("W", "W-ind") for h in broad_only[f]))
broad_stems = defaultdict(int)
for f in broad_new_w:
    broad_stems[stem(f)] += 1
print(f"broad-rule-only ids (no fold-semantics hit) matching a W/W-ind key by stem collision: {len(broad_new_w)}; by stem: {dict(broad_stems)}")
