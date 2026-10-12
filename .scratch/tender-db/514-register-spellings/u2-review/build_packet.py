#!/usr/bin/env python3
"""Issue 514 U2.1, step 5: join groups.json, the prod re-reads and the register records into
packet.json (one entry per E0 group and per lone mover) and packet.md.

The pre-read in each entry applies unit2-decision.md §3.2 (a)-(c) mechanically over hand-read
names (NOTES below). It is an aid for the reviewer and the challenger, NOT a verdict: (d), the two
readers agreeing, is theirs, and a name marked unsourced counts for (b) only once they source it.

Usage: python3 -I build_packet.py
"""
import collections
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import namematch  # noqa: E402

# ---- hand readings of every name the mechanical match does not call exact (2026-10-12) ---------------
# reading: holder | holder-typo | holder-brand | holder-service | holder-branch | holder-former |
#          holder-statutory | other-entity
# sourced: the source that makes the name the holder's (rule (b) needs a register name, a former
#          name on the register, or a SOURCED brand or service). None = not sourced yet.
NOTES = {
    ("GBCHC1088835", "No Limits Safe Haven"): (
        "holder-service", None,
        "Safe Haven read as a No Limits service (unit-1 hand read, splits.json MANUAL). CC lists only NO LIMITS "
        "(SOUTH) / NO LIMITS SOUTH / NO LIMITS (SOUTHAMPTON): the reviewer must source it, or (b) fails for 31537211 "
        "(2 of its 2 mentions)."),
    ("GBCHC1122831", "Arcacia Family Support"): (
        "holder-typo", "CC 1122831 ACACIA FAMILY SUPPORT", "Misspelling of the register name (Arcacia / Acacia)."),
    ("GBCHC1185092", "First Steps Eating Disorder (ED)"): (
        "holder", "CC 1185092 FIRST STEPS ED", "The register name with 'ED' spelled out."),
    ("GBCHC216250", "Barnados"): ("holder-typo", "CC 216250 BARNARDO'S", "Misspelling."),
    ("GBCHC216250", "Barnardo's Central Region"): ("holder-branch", "CC 216250 BARNARDO'S", "A region of the holder."),
    ("GBCHC271028", "Rethink Mental Illness Brighton and Hove"): (
        "holder-branch", "CC 271028 Rethink Mental Illness", "A local branch of the holder."),
    ("GBCHC271028", "Rethink - Redwoods"): (
        "holder-service", None,
        "Redwoods read as a Rethink service (unit-1 hand read). Not register-sourced: the reviewer must source it, or "
        "(b) fails for mover 31594839 (1 of 1)."),
    ("GBMPR16337R", "Metropolitan Thames Valley Housing Limited"): (
        "holder-brand", "FCA 16337R transfers: Thames Valley Charitable HA (25301) TE 2018-10-08; 469 U6 verdict "
        "GB-MPR:GBMPR16337R~PMCL4972LYCY", "The merged body's working name; Companies House has no company of that name "
        "(469 U6)."),
    ("GBMPR16337R", "Metropolitan Thames Valley Housing"): (
        "holder-brand", "as above", "The merged body's working name."),
    ("GBMPR17437R", "Gold Hill HA"): ("holder", "FCA 17437R Gold Hill Housing Association Limited", "HA = Housing Association."),
    ("GBMPR17437R", "GOLD HILL HA"): ("holder", "as above", "HA = Housing Association."),
    ("GBMPR17437R", "GOLD HILL HA (B2G)"): ("holder", "as above", "(B2G) is a framework tag (unit-1 hand read)."),
    ("GBMPR17437R", "Gold Hill HA (B2G)"): ("holder", "as above", "(B2G) is a framework tag."),
    ("GBMPR17437R", "Gold Hill Care (B2G)"): (
        "holder-brand", None,
        "Not on the FCA record (no name changes). A care brand of the holder or a separate body: unsourced. 1 of 7 "
        "(14%) of mover 31593260's mentions, below the 1/5 floor either way; (b) holds without it (6/7)."),
    ("GBMPR28038R", "Clarion Housing Association Ltd"): (
        "other-entity", "469 U6 verdict GB-MPR:GBMPR28038R~PQND5289JNDN: 'the separate member society Clarion Housing "
        "Association Limited (7686) is not the body here'", "A separate society; 1 of 138 (0.7%), a stray below 1/5."),
    ("GBMPR30938R", "The Riverside Group trading as One Housing Group"): (
        "holder-brand", "FCA 30938R transfers: One Housing Group Limited (20453) Full 2023-03-31",
        "One Housing Group transferred its engagements to Riverside in 2023."),
    ("GBUKPRN10005790", "Sheffield Polytechnic"): (
        "holder-former", None,
        "Sheffield City Polytechnic became Sheffield Hallam University in 1992; not on UKRLP (UKRLP shows no former "
        "names). 2 of 182."),
    ("GBUKPRN10005790", "Access UK Ltd"): ("other-entity", None, "Another company; 1 of 182, a stray."),
    ("GBUKPRN10005790", "Centre for Behavioural Science and Applied Psychology, Sheffield Hallam University"): (
        "holder-branch", "UKRLP 10005790 SHEFFIELD HALLAM UNIVERSITY", "A centre of the holder."),
    ("GBUKPRN10005790", "Sheffield Hallam University Enterprises Ltd"): (
        "other-entity", None, "The university's trading company, a separate legal entity; 1 of 182, a stray below 1/5."),
    ("GBUKPRN10005790", "University of Sheffield"): (
        "other-entity", "UKRLP 10007157 University of Sheffield", "A different university (UKPRN 10007157); 1 of 182, "
        "a stray."),
    ("GBUKPRN10007157", "Sheffield Hallam University"): (
        "other-entity", "UKRLP 10005790 SHEFFIELD HALLAM UNIVERSITY", "A different university; 1 of 16 (6%), a stray. "
        "It still won 31558903's head name; organization_names reads UNIVERSITY OF SHEFFIELD."),
    ("GBUKPRN10007788", "THE CHANCELLOR MASTERS AND SCHOLARS OF THE UNIVERSITY OF CAMBRIDGE"): (
        "holder-statutory", None,
        "The University's corporate (statutory) name. UKRLP lists only UNIVERSITY OF CAMBRIDGE / University of "
        "Cambridge / Cambridge University: the reviewer must accept or source it."),
    ("GBUKPRN10007788", "The Chancellor, Masters and Scholars of the University of Cambridge"): (
        "holder-statutory", None,
        "As above. It is mover 31633713's only name (1 of 1), so (b) for the mover rests on it."),
    ("GBCHC1077607", "Daybreak Family Group Conference"): ("holder", "CC 1077607", "Singular of the register name."),
    ("GBCHC1133510", "Karis Neighbourhood Scheme"): ("holder", "CC 1133510 KARIS NEIGHBOUR SCHEME", "Near-spelling."),
    ("GBCHC1205036", "Ghamkol Sharif Central Jamia Charity"): (
        "holder", "CC 1205036 CENTRAL JAMIA MOSQUE GHAMKOL SHARIF", "The register name's words reordered."),
    ("GBCHC279057", "Birmingham Citizens Advice Bureaux"): (
        "other-entity", "CC extract 2026-10-12: 519639 BIRMINGHAM CITIZENS ADVICE BUREAU SERVICE LTD, Registered, "
        "company 02202427",
        "279057 is THE NATIONAL ASSOCIATION OF CITIZENS ADVICE BUREAUX (Citizens Advice). Birmingham's bureau is its "
        "own registered charity, 519639. A local member on the national body's number: a G3 register mismatch."),
    ("GBCHC518610", "Age Connect West Midlands"): ("holder", "CC 518610 Age Connects West Midlands", "Near-spelling."),
    ("GBCHC518753", "Solihull Bereavement Counselling Services"): (
        "holder", "CC 518753 SOLIHULL BEREAVEMENT COUNSELLING SERVICE", "Plural."),
}
HOLDER_READINGS = {"holder", "holder-typo", "holder-brand", "holder-service", "holder-branch", "holder-former",
                   "holder-statutory"}


def load(p):
    return json.load(open(os.path.join(HERE, p)))


def register_names(r):
    return ([r.get("name")] + [y["former_name"] for y in r.get("former_names", [])]
            + [y["name"] for y in r.get("other_names", [])] + r.get("trading_names", [])
            + ([r["other_name"]] if r.get("other_name") else []))


def alnum(s):
    return "".join(c for c in (s or "") if c.isascii() and c.isalnum()).upper()


def main():
    g = load("groups.json")
    reg = load("registers/records.json")
    rec = reg["records"]
    journal = [json.loads(l) for l in open(os.path.join(HERE, "prod", "journal.jsonl"))]
    orgs = {r[0]: dict(zip(["id", "country", "identifier_kind", "identifier", "name", "provisional"], r))
            for r in load("prod/01-orgs.out")["rows"]}
    mrows = collections.defaultdict(list)
    for f in ("prod/02-mnames-members.out", "prod/03-mnames-movers.out"):
        for oid, name, raw, sch, cty, n in load(f)["rows"]:
            mrows[oid].append((name, raw, sch, cty, n))
    onames = collections.defaultdict(list)
    for oid, lang, name in load("prod/04-onames.out")["rows"]:
        onames[oid].append({"lang": lang, "name": name})
    hold_c = collections.defaultdict(list)
    for oid, ident, name, prov in load("prod/05-holders-canonical.out")["rows"]:
        hold_c[ident].append({"org_id": oid, "name": name, "provisional": prov})
    hold_s = collections.defaultdict(list)
    for oid, ident, name, prov in load("prod/06-holders-stored.out")["rows"]:
        hold_s[ident].append({"org_id": oid, "name": name, "provisional": prov})
    u6 = json.load(open(os.path.join(HERE, "..", "..", "469-registry-pairs", "u6", "verdicts-confirmed.json")))
    u6_by_org = collections.defaultdict(list)
    for v in u6["verdicts"]:
        for m in v["members"]:
            u6_by_org[m].append({"cohort": u6["cohort"], **{k: v[k] for k in ("country", "scheme", "key", "members",
                                                                                 "action", "confidence")}})

    unused_notes = set(NOTES)
    entries, anomalies = [], []
    drift = []
    seq = {"group": 0, "lone": 0}
    for kind, items in (("group", g["groups"]), ("lone", g["lone_movers"])):
        for x in items:
            seq[kind] += 1
            eid = ("G%02d" if kind == "group" else "L%02d") % seq[kind]
            canon = x["canonical"]
            r = rec[canon]
            rnames = register_names(r)
            members = []
            for m in x["members"]:
                oid = m["org_id"]
                cur = orgs.get(oid)
                rows = mrows.get(oid, [])
                total = sum(n for *_, n in rows)
                by_name = collections.OrderedDict()
                for name, raw, sch, cty, n in sorted(rows, key=lambda t: -t[-1]):
                    e = by_name.setdefault(name, {"name": name, "n": 0, "raws": collections.Counter(),
                                                  "schemes": collections.Counter(), "countries": collections.Counter()})
                    e["n"] += n
                    e["raws"][raw] += n
                    e["schemes"][sch] += n
                    e["countries"][cty] += n
                names = []
                for e in sorted(by_name.values(), key=lambda e: -e["n"]):
                    cls, via = namematch.classify(e["name"], rnames)
                    note = NOTES.get((canon, e["name"]))
                    if note:
                        unused_notes.discard((canon, e["name"]))
                        reading, sourced, text = note
                    elif cls in ("exact", "close"):
                        reading, sourced, text = "holder", "%s %s %s" % (r["register"], r["number"], via), None
                    else:
                        reading, sourced, text = "UNREAD", None, "no hand note: reviewer must read"
                    names.append(dict(name=e["name"], n=e["n"], share=round(e["n"] / total, 4) if total else None,
                                      raws=dict(e["raws"]), schemes=dict(e["schemes"]), countries=dict(e["countries"]),
                                      mechanical_match=cls, matched_register_name=via, reading=reading,
                                      sourced=sourced, note=text))
                holder_n = sum(e["n"] for e in names if e["reading"] in HOLDER_READINGS)
                holder_src_n = sum(e["n"] for e in names if e["reading"] in HOLDER_READINGS and e["sourced"])
                other = [e for e in names if e["reading"] == "other-entity"]
                is_drift = (cur is None or cur["identifier"] != m["stored_identifier"] or cur["name"] != m["head_name"]
                            or cur["provisional"] != m["provisional"] or total != m["mentions"])
                if is_drift:
                    drift.append(oid)
                raws_spellings = sorted({alnum(raw) for _, raw, *_ in rows})
                member = dict(
                    org_id=oid, role="mover" if m["moves"] else "standing twin", rule=m["rule"],
                    stored_identifier=m["stored_identifier"], canonical_after_rewrite=canon,
                    census=dict(head_name=m["head_name"], mentions=m["mentions"], provisional=m["provisional"],
                                rows=m["census"]),
                    current=dict(identifier=cur and cur["identifier"], name=cur and cur["name"],
                                 provisional=cur and cur["provisional"], country=cur and cur["country"],
                                 identifier_kind=cur and cur["identifier_kind"], mentions=total,
                                 drift_since_census=is_drift),
                    organization_names=onames.get(oid, []),
                    mention_names=names,
                    mention_raw_spellings=raws_spellings,
                    polluted_spelling_only=(raws_spellings == [m["stored_identifier"]]) if m["moves"] else None,
                    share_naming_holder=round(holder_n / total, 4) if total else None,
                    share_naming_holder_sourced=round(holder_src_n / total, 4) if total else None,
                    largest_other_entity=(max(other, key=lambda e: e["n"])["name"] if other else None),
                    largest_other_entity_share=(round(max(e["n"] for e in other) / total, 4) if other else 0.0),
                    other_verdicts_local=u6_by_org.get(oid, []),
                )
                members.append(member)
            # ---- the mechanical pre-read of §3.2 / §3.3 -------------------------------------------------
            flags = []
            a_ok = bool(r["found"]) and (r["status"] in ("Registered", "active") or r["scheme"] == "NIC")
            if not a_ok:
                flags.append("(a) register record missing or not registered")
            for mm in members:
                if mm["share_naming_holder"] is None or mm["share_naming_holder"] <= 0.5:
                    flags.append("(b) %s: %.0f%% of mentions name the holder" % (mm["org_id"], 100 * (mm["share_naming_holder"] or 0)))
                elif mm["share_naming_holder_sourced"] <= 0.5:
                    flags.append("(b) %s: majority rests on an unsourced name (%.0f%% sourced)" % (
                        mm["org_id"], 100 * mm["share_naming_holder_sourced"]))
                if mm["largest_other_entity_share"] >= 0.2:
                    flags.append("(c) %s: %r on %.0f%%" % (mm["org_id"], mm["largest_other_entity"],
                                                          100 * mm["largest_other_entity_share"]))
                if any(e["reading"] == "UNREAD" for e in mm["mention_names"]):
                    flags.append("unread name on %s" % mm["org_id"])
            if kind == "group":
                pre = "meets (a)-(c) on the hand read" if not flags else "reviewer attention"
            else:
                pre = "register agrees (rule b)" if not flags else "REGISTER MISMATCH candidate (hold list)" if any(
                    f.startswith("(b)") and "name the holder" in f for f in flags) else "reviewer attention"
            entry = dict(
                id=eid, kind=x["kind"], canonical=canon, scheme=x["scheme"], number=x["number"], rules=x["rules"],
                e0_verdict_key=dict(country="GB", scheme="E0", key="national:" + canon) if kind == "group" else None,
                members_ascending=[mm["org_id"] for mm in members],
                survivor_if_merged=x.get("survivor_if_merged"),
                survivor_is_mover=(next(mm for mm in members if mm["org_id"] == x.get("survivor_if_merged"))["role"] == "mover")
                if kind == "group" else None,
                mentions=sum(mm["current"]["mentions"] for mm in members),
                register=r,
                holders_today=dict(canonical=hold_c.get(canon, []),
                                   stored={mm["stored_identifier"]: hold_s.get(mm["stored_identifier"], [])
                                           for mm in members if mm["role"] == "mover"}),
                verdicts=dict(
                    e0_key_listing="not readable: org_merge_verdicts is not on /v1/sql's allow-list (prod call 7, HTTP 400)",
                    identifier_verdicts="not readable: org_identifier_verdicts is not on /v1/sql's allow-list (prod call 8, HTTP 400)",
                    local_record_e0_key="none (no verdict file under .scratch/tender-db names this key)",
                    local_record_on_members=[v for mm in members for v in mm["other_verdicts_local"]]),
                prescreen=dict(result=pre, flags=flags,
                               note="mechanical over hand-read names; not a verdict (rule (d) is the two readers')"),
                members=members,
            )
            entries.append(entry)
    assert not unused_notes, unused_notes

    # ---- anomalies --------------------------------------------------------------------------------------
    s = g["summary"]
    mism = [e for e in entries if e["id"].startswith("L") and e["prescreen"]["result"].startswith("REGISTER MISMATCH")]
    attn = [e for e in entries if e["prescreen"]["result"] == "reviewer attention"]
    mover_ids = [mm["org_id"] for e in entries for mm in e["members"] if mm["role"] == "mover"]
    not_single = [mm["org_id"] for e in entries for mm in e["members"]
                  if mm["role"] == "mover" and mm["mention_raw_spellings"] != [mm["stored_identifier"]]]
    anomalies.append(dict(
        id="A1", kind="count", title="§1.4's lone-mover breakdown sums to 29, not 27",
        detail="§1.4 lists 20 CHC + 1 UKPRN + 8 IP+R lone moves = 29. The rules give 18 CHC + 1 UKPRN + 8 MPR-a = 27 "
               "(groups.json). The 20 counts 1122831's two movers, which reunite with each other and so form E0 group "
               "G03, as §0's 'Reach' line says (29 = 27 lone + 1122831's 2). Totals agree: 54 rows, 26 groups (25 onto a "
               "standing twin + 1122831), 52 members, 27 lone movers, 743 group mentions."))
    anomalies.append(dict(
        id="A2", kind="count", title="'54 canonical numbers' are 53 distinct numbers; CCNI 102575 is one of them",
        detail="54 rewritten rows map to 53 distinct canonical values: 1122831's two rows share one. CCNI 102575 is the "
               "NIC group's canonical (G%02d), not a 55th number. 53 register records were fetched (MPR 19, CHC 27, "
               "UKPRN 6, NIC 1); all 53 were found." % next(i + 1 for i, e in enumerate(entries) if e["scheme"] == "NIC")))
    anomalies.append(dict(
        id="A3", kind="scope", title="1003906's mover is a lone mover, and its census split stays open",
        detail="31562667 (GBCHCREGISTEREDCHARITYNO1003906, Birmingham Mind, 7 mentions) moves to GBCHC1003906, but its "
               "census twin 31594757 (GBCHCREGISTRATIONNO1003906, a generic label, deferred by ruling 9) does not. The "
               "split goes from label-vs-label to bare-vs-generic. Its 8 mentions are in the 109 deferred, not the 743. "
               "No other lone mover belongs to a census split."))
    for e in mism:
        mm = e["members"][0]
        anomalies.append(dict(
            id="A4", kind="register mismatch (G3)", title="%s: lone mover %s names another charity" % (e["canonical"], mm["org_id"]),
            detail="Mover %s (%s, %d mention) reads %r. The CC record for %s is %r (Registered, company %s). "
                   "Birmingham's bureau is its own charity, 519639 BIRMINGHAM CITIZENS ADVICE BUREAU SERVICE LTD "
                   "(Registered, company 02202427, same extract). Under §3.3, %s goes on REGISTER_SPELLING_HOLDS: the "
                   "rewrite must not mint the national body's identity for a local member. This is ONE mismatch, so "
                   "S11 (two or more) does not stop the unit. The census measured this hazard at 0 of 36 splits; it "
                   "sits in a lone mover, which the census never name-checked." % (
                       mm["org_id"], mm["stored_identifier"], mm["current"]["mentions"], mm["mention_names"][0]["name"],
                       e["number"], e["register"]["name"], e["register"]["company_number"], e["canonical"])))
    srcs = [(e["id"], e["canonical"], mm["org_id"], nm["name"], nm["n"], mm["current"]["mentions"])
            for e in entries for mm in e["members"] for nm in mm["mention_names"]
            if nm["reading"] in HOLDER_READINGS and not nm["sourced"]]
    anomalies.append(dict(
        id="A5", kind="name gate", title="(b) rests on unsourced names in %d entries" % len({x[0] for x in srcs if x[4] / x[5] > 0.5}),
        detail="Names read as the holder's but not register-sourced (rule (b) needs a sourced brand or service): "
               + "; ".join("%s %s org %s %r %d/%d" % x for x in srcs)
               + ". Where the name carries the member's majority the reviewer must source it or post keep: G01 "
                 "1088835 (31537211, 2/2 'No Limits Safe Haven'), G09 271028 (31594839, 1/1 'Rethink - Redwoods'), "
                 "G25 10007788 (31633713, 1/1 statutory name; also 6/13 of 31549754)."))
    strays = [(e["canonical"], mm["org_id"], nm["name"], nm["n"], mm["current"]["mentions"])
              for e in entries for mm in e["members"] for nm in mm["mention_names"] if nm["reading"] == "other-entity"
              and not e["id"].startswith("L")]
    anomalies.append(dict(
        id="A6", kind="name gate", title="Other-entity names on group members, all below the 1/5 floor",
        detail="; ".join("%s org %s %r %d/%d" % x for x in strays)
               + ". 31558903 still carries the head name 'Sheffield Hallam University' from its single stray mention "
                 "(organization_names reads UNIVERSITY OF SHEFFIELD); it is the lower id, so it survives a G24 merge with "
                 "that head name (risk 9, cosmetic) unless the head is re-chosen."))
    anomalies.append(dict(
        id="A7", kind="drift", title="No drift since the census" if not drift else "Drift on %s" % drift,
        detail="All 79 orgs re-read with the census identifier, head name, provisional = 0 and mention total. Each "
               "canonical value is held only by its standing twin (25 holders; none for 1122831 or the 27 lone "
               "movers' values); each of the 54 stored polluted values is held only by its census org. Every mover's "
               "mention raws still normalise to its own stored value only%s. All 785 mentions are country GB." % (
                   "" if not not_single else " except %s" % not_single)))
    alt = sorted({(v["key"], tuple(v["members"])) for e in entries for v in e["verdicts"]["local_record_on_members"]})
    anomalies.append(dict(
        id="A8", kind="verdicts", title="The existing verdicts on the 26 E0 keys cannot be listed through /v1/sql",
        detail="org_merge_verdicts and org_identifier_verdicts are not on /v1/sql's allow-list (prod calls 7-8, HTTP "
               "400, no data pages read). Identifier verdicts surface on /v1/organizations as identifier_status, which "
               "is outside this unit's read path. P0 item 3 stays open and needs an operator read before the verdicts "
               "are posted (S8). Local record: no verdict file under .scratch/tender-db names any of the 26 E0 keys. "
               "The only verdicts naming these orgs are 469 U6's %d GB:altid merges (cohort "
               "altid-registry-confirmed-2026-10-11, HIGH merge, applied), one per G4 holder: %s. They are a different "
               "(scheme, key) and do not deny an E0 group." % (len(alt), "; ".join("%s %s" % (k, list(m)) for k, m in alt))))
    anomalies.append(dict(
        id="A9", kind="survivor", title="In %d groups the survivor (lower id) is the mover" % sum(
            1 for e in entries if e["survivor_is_mover"]),
        detail="; ".join("%s %s survivor %s" % (e["id"], e["canonical"], e["survivor_if_merged"]) for e in entries
                         if e["survivor_is_mover"])
               + ". After the 345 re-key both members hold the canonical value, so only head names are affected "
                 "(risk 9). The 5 G4 holders (Metropolitan, Clarion, Creative Support, Hallam, Cambridge) are each the "
                 "lower id and survive."))

    traps = [31563025, 31552235, 30917450, 31547203, 31611321, 31600646, 31614503, 31605441, 31623228, 31536226,
             31613164, 31587933, 31550331, 31596036]  # unit2-decision.md §1.4 "Untouched"
    touched = sorted(set(traps) & {mm["org_id"] for e in entries for mm in e["members"]})
    counts = dict(
        rows_rewritten=s["rows_rewritten"], by_scheme=s["rows_rewritten_by_scheme"], by_rule=s["rows_rewritten_by_rule"],
        e0_groups=s["e0_groups"], reunions_onto_twin=s["e0_groups_reunion_onto_twin"],
        movers_reunite=s["e0_groups_movers_reunite"], group_members=s["group_members"],
        group_mentions=s["group_mentions"], lone_movers=s["lone_movers"], lone_movers_by_rule=s["lone_movers_by_rule"],
        lone_mover_mentions=s["lone_mover_mentions"], orgs_read=len(orgs),
        mentions_read=sum(n for rows in mrows.values() for *_, n in rows),
        distinct_canonical_numbers=len(rec), register_records_found=reg["found"],
        register_requests=reg["requests"], register_non_200=len(reg["non_200"]),
        prod_calls=len(journal), prod_408=sum(1 for j in journal if j.get("is_408")),
        decision_check=dict(rows=(s["rows_rewritten"], 54), groups=(s["e0_groups"], 26), lone=(s["lone_movers"], 27),
                            group_mentions=(s["group_mentions"], 743), members=(s["group_members"], 52)),
        prescreen=dict(collections.Counter(e["prescreen"]["result"] for e in entries)),
        trap_orgs_in_packet=touched,
    )
    packet = dict(
        issue=514, unit="U2.1 review packet ('Measure first' #3 and #4)", date="2026-10-12",
        decision="unit2-decision.md §1.3 rules, §3.2 rule (a)-(d), §3.3 lone movers",
        inputs=dict(groups="groups.json (materialise.py over census raw/001-004, 016-019, 021-029)",
                    prod="prod/01-08 + prod/journal.jsonl (prod_reads.py, plus the two allow-list probes)",
                    registers="registers/records.json (build_registers.py over registers/raw, fetched by "
                              "fetch_registers.py; manifest registers/fetch-manifest.json)"),
        counts=counts, anomalies=anomalies, entries=entries)
    json.dump(packet, open(os.path.join(HERE, "packet.json"), "w"), indent=1, ensure_ascii=False)
    write_md(packet, journal, reg)
    print(json.dumps(counts, indent=1))
    for a in anomalies:
        print(a["id"], a["title"])


def pct(x):
    return "%d%%" % round(100 * x) if x is not None else "-"


def write_md(p, journal, reg):
    c = p["counts"]
    L = []
    w = L.append
    w("# Issue 514 U2.1: review packet (2026-10-12)")
    w("")
    w("This packet covers \"Measure first\" #3 (register records for the canonical numbers) and #4 (the mention-name "
      "packet for the 52 group members and the 27 lone movers) of `../unit2-decision.md`. The data is in "
      "`packet.json`, with one entry per E0 group (`G01`-`G26`) and per lone mover (`L01`-`L27`). The pre-read applies "
      "§3.2 (a)-(c) mechanically to hand-read names. **It is not a verdict.** Rule (d), agreement between the reviewer "
      "and the challenger, is still to come.")
    w("")
    w("## Counts")
    w("")
    w("| | materialised | decision §1.4 |")
    w("|---|---:|---:|")
    w("| rows rewritten | %d (UKPRN %d, CHC %d, NIC %d, MPR %d: %d IP, %d REGNO) | 54 (6 / 28 / 1 / 19: 18 IP, 1 REGNO) |" % (
        c["rows_rewritten"], c["by_scheme"]["UKPRN"], c["by_scheme"]["CHC"], c["by_scheme"]["NIC"], c["by_scheme"]["MPR"],
        c["by_rule"]["MPR-a"], c["by_rule"]["MPR-b"]))
    w("| E0 groups | %d (%d onto a standing twin, %d movers reunite: 1122831) | 26 (25 + 1122831) |" % (
        c["e0_groups"], c["reunions_onto_twin"], c["movers_reunite"]))
    w("| group members | %d (every group has 2) | 52 |" % c["group_members"])
    w("| group mentions | %d | 743 |" % c["group_mentions"])
    w("| lone movers | %d (CHC %d, MPR IP+R %d, UKPRN %d), %d mentions | 27 (listed as 20 + 1 + 8: see A1) |" % (
        c["lone_movers"], c["lone_movers_by_rule"]["CHC"], c["lone_movers_by_rule"]["MPR-a"],
        c["lone_movers_by_rule"]["UKPRN"], c["lone_mover_mentions"]))
    w("| distinct canonical numbers | %d (all found on their register) | \"54\" (see A2) |" % c["distinct_canonical_numbers"])
    w("")
    w("The rewrite reaches none of §1.4's 14 trap orgs: they appear neither as members nor as movers (S6), %s." % (
        "confirmed" if not c["trap_orgs_in_packet"] else "EXCEPT %s" % c["trap_orgs_in_packet"]))
    w("")
    w("**Reads.**")
    w("- Prod: %d sequential calls through `/root/sq.sh`, with no 408 and no retry:" % c["prod_calls"])
    w("  - 6 bounded index-served SELECTs:")
    w("    - organizations by primary key (79);")
    w("    - mentions by `organization_mentions_org`, grouped, in 2 calls;")
    w("    - organization_names by primary-key prefix;")
    w("    - holders of the 53 canonical and the 54 stored values on `organizations_identity`;")
    w("  - 2 allow-list probes, both refused at parse with HTTP 400.")
    w("  - The SQL, timings and status of each call are in `prod/journal.jsonl`.")
    w("- Registers: %d GETs, all HTTP 200 (`registers/fetch-manifest.json`):" % c["register_requests"])
    w("  - FCA SocietyList.csv, plus each society's search result, page, name changes and transfers;")
    w("  - UKRLP detail and status lists;")
    w("  - the CC JSON extracts, dated 2026-10-12, of which only the 27 numbers' rows are kept: 163 charity rows "
      "including linked charities, and 48 other-name rows;")
    w("  - the CCNI page, CSV and search.")
    w("  - The bulk files stay in the session scratchpad. Their sha256 is in the manifest.")
    w("")
    w("**Pre-read** (mechanical, not a verdict): %s." % ", ".join("%s %d" % (k, v) for k, v in sorted(c["prescreen"].items())))
    w("")
    w("## Anomalies")
    w("")
    for a in p["anomalies"]:
        w("- **%s %s.** %s" % (a["id"], a["title"], a["detail"]))
    w("")
    w("## E0 groups")
    w("")
    w("Members are listed ascending, and the first one is the survivor (`*` marks a mover). Each name share is "
      "weighted by mention count. \"holder\" is the share of mentions that name the register holder; unsourced brands "
      "are counted, and the flags show which ones they are.")
    w("")
    w("| id | canonical | register (status) | members: stored, mentions, holder share | pre-read |")
    w("|---|---|---|---|---|")
    for e in p["entries"]:
        if not e["id"].startswith("G"):
            continue
        ms = "<br>".join("%s%s `%s` %d, %s" % ("*" if m["role"] == "mover" else "", m["org_id"], m["stored_identifier"],
                                               m["current"]["mentions"], pct(m["share_naming_holder"])) for m in e["members"])
        w("| %s | `%s` | %s (%s) | %s | %s%s |" % (e["id"], e["canonical"], e["register"]["name"], e["register"]["status"],
                                                ms, e["prescreen"]["result"],
                                                ("<br>" + "<br>".join(e["prescreen"]["flags"])) if e["prescreen"]["flags"] else ""))
    w("")
    w("## Lone movers")
    w("")
    w("| id | canonical | org, stored, mentions | mention names | register (status) | pre-read |")
    w("|---|---|---|---|---|---|")
    for e in p["entries"]:
        if not e["id"].startswith("L"):
            continue
        m = e["members"][0]
        nm = "; ".join("%s ×%d" % (x["name"], x["n"]) for x in m["mention_names"])
        w("| %s | `%s` | %s `%s` %d | %s | %s (%s) | %s |" % (e["id"], e["canonical"], m["org_id"], m["stored_identifier"],
                                                          m["current"]["mentions"], nm, e["register"]["name"],
                                                          e["register"]["status"], e["prescreen"]["result"]))
    w("")
    w("## Register facts a reviewer will want")
    w("")
    w("- **MPR (FCA).**")
    w("  - All 19 are Registered Societies under the 2014 Act.")
    w("  - Name changes and transfers are in each record. Examples:")
    w("    - 28038R was Affinity Sutton Group Limited until 2016-11-29.")
    w("    - 30938R took One Housing Group (Full) on 2023-03-31.")
    w("    - 16337R took Thames Valley Charitable HA (TE) on 2018-10-08.")
    w("- **CHC (CC extract 2026-10-12).**")
    w("  - All 27 are Registered, with no removal.")
    w("  - `company_number`, other names and linked charities are recorded.")
    w("  - Linked charities: 216250 has 93 (11 registered), 221124 has 33 (14), 517077 has 5, 271028 has 3, 279057 has 1, "
      "1104951 has 1.")
    w("- **UKPRN (UKRLP).**")
    w("  - All 6 are active.")
    w("  - 10007157 is the University of Sheffield, with trading names THE UNIVERSITY OF SHEFFIELD and University of "
      "Sheffield.")
    w("  - 10007788 is UNIVERSITY OF CAMBRIDGE, with trading names University of Cambridge and Cambridge University.")
    w("- **NIC (CCNI).**")
    w("  - 102575 is Radius Housing Association Limited, registered 2016-04-04.")
    w("  - CCNI's status column reads \"Received: on time\", which is its reporting-status label.")
    w("")
    w("## Not covered here")
    w("")
    w("- The verdict listing on the 26 keys (A8) needs an operator path. It is outside `/v1/sql`.")
    w("- \"Measure first\" #5 is not covered: fresh range re-reads just before deploy, to catch movers minted since "
      "2026-10-11. #6 (XI) is not covered either.")
    w("- Cross-scheme twins of the same bodies stand outside unit 2, and the E0 groups leave them as they are. For "
      "example, `519-nhs-digit-codes/name-twins-2026-10-12.json` shows Cruse on `GBCOH00638709` and `GBCOH638709`, "
      "plus a PPON and an NHS code, and St Andrew's on NHS `NYA01`.")
    w("")
    w("## Files")
    w("")
    w("- `materialise.py` produces `groups.json`, re-implementing §1.3 and §1.2's I1-I3 over census `raw/001-004`. "
      "Mentions come from `raw/016-019` and `raw/021-029`.")
    w("- `prod_reads.py` writes `prod/01-06*.out` and `prod/journal.jsonl`. The two probes are `prod/07-08`.")
    w("- `fetch_registers.py` writes `registers/raw/**` and `registers/fetch-manifest.json`. `build_registers.py` "
      "turns them into `registers/records.json`.")
    w("- `namematch.py` holds the mechanical routing only. `build_packet.py` writes `packet.json` and `packet.md`, and "
      "holds the hand notes on every non-exact name.")
    open(os.path.join(HERE, "packet.md"), "w").write("\n".join(L) + "\n")


if __name__ == "__main__":
    main()
