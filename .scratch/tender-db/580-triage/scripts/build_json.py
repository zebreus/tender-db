"""Assemble 580-triage/per-archive-counts.json from retire.json + tally.json + the job ledger."""
import json, sys
SP, OUT = sys.argv[1], sys.argv[2]
R = json.load(open(f"{SP}/retire.json"))
T = json.load(open(f"{SP}/tally.json"))
J = json.load(open(f"{SP}/jobs.json"))
S = ".scratch/tender-db/551-lot-sum"

trigger = {  # writer job -> (class, trigger jobs, what, tenders re-folded/written, source of that count)
    2630: ("campaign", "refold-lot-sum 2629", "551 B5 switch-off drain-back (W=false)", 3502, "2629 stamped"),
    2632: ("campaign", "refold-fields 2631", "541 TED-COST_PARAMETERS", 753, "2631 stamped"),
    2636: ("campaign", "refold-fields 2635", "571 TED-ADDRESS_PARTY", 67, "2635 stamped"),
    2645: ("daily", "chain 2637-2646 (probe/process ted, doe, fts; fetch-rates 2643; rederive-eur-recent 2644; project 2645; reveal-recheck 2646)", "daily chain 2026-10-19", 4070, "2645 tenders written"),
    2651: ("other", "apply-rehoming 2651", "545 cohort rehomes (org mentions)", None, None),
    2652: ("campaign", "apply-rehoming 2651", "545 project behind the rehomes", 48, "2652 tenders written"),
    2654: ("other", "refresh-org-names 2654", "548 v13 renames (organizations)", None, None),
    2668: ("daily-rederive", "chain 2661-2670", "rederive-eur-recent 2026-10-20 (D5 rows)", None, None),
    2669: ("daily", "chain 2661-2670 (doe probe 2663 errored 504; fetch-rates 2667; rederive 2668; project 2669)", "daily chain 2026-10-20", 4974, "2669 tenders written"),
    2672: ("other", "dissolve-guid-twins 2672", "530 Build 2 wet (organizations)", None, None),
    2673: ("other", "dissolve-guid-twins 2673", "530 Build 2 wet (organizations)", None, None),
    2687: ("daily-rederive", "chain 2680-2689", "rederive-eur-recent 2026-10-21", None, None),
    2688: ("daily", "chain 2680-2689 (fetch-rates 2686; rederive 2687; project 2688)", "daily chain 2026-10-21", 5887, "2688 tenders written"),
    2692: ("campaign", "refold-lot-sum 2691", "551 U7 stage1", 426, "2691 stamped"),
    2695: ("campaign", "refold-lot-sum 2694", "551 U7 stage2", 3100, "2694 stamped"),
    2698: ("campaign", "refold-lot-sum 2697", "551 U7 lc01", 489, "2697 stamped"),
    2704: ("campaign", "refold-lot-sum 2703", "551 U7 lc03", 33, "2703 stamped"),
    2715: ("daily-rederive", "chain 2708-2717", "rederive-eur-recent 2026-10-22", None, None),
    2716: ("daily", "chain 2708-2717 (fetch-rates 2714; rederive 2715; project 2716)", "daily chain 2026-10-22", 5785, "2716 tenders written"),
    2725: ("daily-rederive", "chain 2718-2727", "rederive-eur-recent 2026-10-23", None, None),
    2726: ("daily", "chain 2718-2727 (fetch-rates 2724; rederive 2725; project 2726)", "daily chain 2026-10-23", 5739, "2726 tenders written"),
    2730: ("campaign", "refold-lot-sum 2729", "551 U9 p0 (546 U3 probe)", 3537, "2729 stamped"),
    2734: ("campaign", "refold 2733 eforms:eforms-sdk-0.1", "539 run a", 676936, "2733 stamped"),
    2738: ("campaign", "refold 2737 eforms:eforms-de-1.0/1.1/1.2", "539 run b", 126812, "2737 stamped"),
    2741: ("campaign", "refold 2740 eforms:eforms-de-2.0/2.1", "539 run c", 147825, "2740 stamped"),
}
# folds inside a covered window that wrote ZERO change rows (so 0 retired, by the window's completeness)
silent = [
    {"job_id": 2649, "trigger": "refold-lot-sum 2648", "what": "551 re-run W=false", "tenders": 3502, "window": "u6-lchanged"},
    {"job_id": 2658, "trigger": "refold-lot-sum 2657", "what": "551 re-run", "tenders": 3502, "window": "u6-lchanged"},
    {"job_id": 2679, "trigger": "refold-lot-sum 2678", "what": "551 re-run", "tenders": 3502, "window": "u6-lchanged"},
    {"job_id": 2674, "trigger": "dissolve-guid-twins 2673", "what": "530 project, 0 notices", "tenders": 0, "window": "u6-lchanged"},
    {"job_id": 2701, "trigger": "refold-lot-sum 2700", "what": "551 U7 lc02", "tenders": 206, "window": "u9-days12"},
    {"job_id": 2707, "trigger": "refold-lot-sum 2706", "what": "551 U7 lc04", "tenders": 1, "window": "u9-days12"},
    {"job_id": 2731, "trigger": "repair-notice-instants 2731 (dry)", "what": "z539-1", "tenders": 0, "window": "u9-seg-z539-1 (K == CA, empty)"},
    {"job_id": 2732, "trigger": "repair-notice-instants 2732 (wet, 16,343 notices; no versions)", "what": "z539-1", "tenders": 0, "window": "u9-seg-z539-1 (K == CA, empty)"},
]

archives = []
for a in R:
    t = next(x for x in T["archives"] if x["name"] == a["name"])
    span = a["hi"] - a["lo"]
    jobs = []
    for j in a["jobs"]:
        jid = j["job_id"]
        tr = trigger.get(jid)
        attached = j["retire_attached"]
        jobs.append({
            "job_id": jid, "job_kind": j["kind"],
            "class": tr[0] if tr else ("campaign" if jid is None else None),
            "trigger": tr[1] if tr else ("refold-lot-sum 2626 (551 B5 corpus drain) -> project 2627" if jid is None else None),
            "what": tr[2] if tr else ("551 B5 corpus drain" if jid is None else None),
            "tenders_refolded_or_written": tr[3] if tr else (251526 if jid is None else None),
            "tenders_count_source": tr[4] if tr else ("issue 551:1105 (2626 stamped 251,526)" if jid is None else None),
            "changed_at": j["stamps"], "cursor": j["cursor"], "rows": j["rows"],
            "tender_changed": j["tender_changed"],
            "tender_removed": j["tender_removed"],
            "removed_alongside_retirement": attached,
            "tender_added": j["tender_added"], "minted_new_ids": j["minted"], "minted_id_range": j["minted_range"],
            "added_on_existing_id_seq1": j["added_existing"],
            "other_removed_rows": j["other_removed"],
            "retired_ids": sorted(j["removed_ids"]),
        })
    archives.append({
        "name": a["name"], "path": a["path"].replace("/home/user/tender-db/", ""),
        "cursor_range": f"({a['lo']}, {a['hi']}]",
        "kinds": t["kinds"], "rows": a["rows"],
        "complete_no_gap": (a["rows"] == span) if t["kinds"] == "all" else "tender-only read; manifest complete",
        "counts": t["counts"], "counts_by_version": t["counts_vnull"],
        "tender_removed": t["tender_removed"], "tender_added": t["tender_added"],
        "minted_new_ids": sum(j["minted"] for j in a["jobs"]),
        "removed_alongside_retirement": {k: sum(j["retire_attached"].get(k, 0) for j in a["jobs"]) for k in ("lot", "lot_result", "bid", "contract")},
        "jobs": jobs,
    })

summary_only = [{
    "name": "568-u8-fresh568 (summary only, raw pages not retained)",
    "path": ".scratch/tender-db/568-r208-lots/readback-u8/fresh568/changes_summary.json + readback-u8/readback_result.json items (e),(g)",
    "cursor_range": "(957854566, 958764334]",
    "kinds": "tender feed + lot feed (counted, not archived)",
    "jobs": [{"job_id": 2617, "trigger": "refold-fields 2616 (568 unit 8 + 570)", "class": "campaign",
              "tenders_refolded_or_written": 1250447, "tenders_count_source": "issue 570:4 (1,250,447 written)",
              "tender_changed": 46879, "tender_removed": 0, "tender_added": 0,
              "lot_events": 862889, "lot_removed": 0, "lot_added": 0}],
}]

crosschecks = {
    "u2_api_tender_feed (958787061, 958848784]": {"path": f"{S}/b6/u2/api/changes/000-008.json", "tender_removed": 11, "equals_job_2645_retired_ids": True},
    "u9_replay_e1 tender pages (959004090, 959180852]": {"path": f"{S}/b6/u9/replay/e1/", "rows": 18216, "removed": 24, "added": 5182, "changed": 13010, "removed_ids_equal_days12": True},
    "u7_d5 stage1/stage2/lc01/lc03 (959004090, 959026689]": {"path": f"{S}/b6/u7/d5/*/changes/", "tender_removed": 0, "matches_days12_jobs_2692_2695_2698_2704": True},
    "u9 switchoff.json retired": {"path": f"{S}/b6/u9/20261023/switchoff.json", "n": 24, "equals_days12_retired_ids": True},
    "u9 registry R.json retired": {"path": f"{S}/b6/u9/registry/R.json", "n": 117, "equals_days12_plus_w539c_retired_ids": True},
    "u6 box/raw + evidence-u6-pre.tar.xz": {"path": f"{S}/b6/u6/box/raw/, {S}/b6/u6/evidence-u6-pre.tar.xz", "note": "raw responses of the same chunks as u6/lchanged and u9-days12 (not re-counted separately)"},
}

re = T["removed_reappear"]
reappear = {
    "retired_ids_checked": len(re),
    "covered_through_cursor": 959223268,
    "with_any_later_row": sum(1 for v in re.values() if v["later"]),
    "retired_more_than_once": sum(1 for v in re.values() if len(v["removed_at"]) > 1),
    "retired_ids_minted_inside_the_window": {k: v["archive"] for k, v in re.items() if "added" in v["earlier_ops"]},
    "schema": "tenders.id INTEGER PRIMARY KEY AUTOINCREMENT (crates/store/src/canonical.rs:110): a deleted id is never re-issued",
}

daily = [j for a in archives for j in a["jobs"] if j["class"] == "daily"]
camp = [j for a in archives for j in a["jobs"] if j["class"] == "campaign"]
camp_t = sum(j["tenders_refolded_or_written"] or 0 for j in camp) + 1250447 + sum(s["tenders"] for s in silent)
est = {
    "daily_chain": {
        "chains": [(j["what"], j["job_id"], j["tender_removed"], j["minted_new_ids"], j["tenders_refolded_or_written"]) for j in daily],
        "retired_total": sum(j["tender_removed"] for j in daily),
        "mean_per_chain": sum(j["tender_removed"] for j in daily) / len(daily),
        "min": min(j["tender_removed"] for j in daily), "max": max(j["tender_removed"] for j in daily),
        "per_100k_tenders_written": 1e5 * sum(j["tender_removed"] for j in daily) / sum(j["tenders_refolded_or_written"] for j in daily),
        "retired_per_minted": sum(j["tender_removed"] for j in daily) / sum(j["minted_new_ids"] for j in daily),
    },
    "campaign": {
        "539_run_c_2741": {"retired": 93, "tenders": 147825, "per_100k": 1e5 * 93 / 147825, "minted": 93},
        "539_runs_abc": {"retired": 93, "tenders": 676936 + 126812 + 147825, "per_100k": 1e5 * 93 / (676936 + 126812 + 147825)},
        "all_campaign_folds_in_window": {"retired": 93, "tenders": camp_t, "per_100k": 1e5 * 93 / camp_t},
        "grouping_neutral_folds": {"retired": 0, "tenders": camp_t - 147825, "rule_of_three_95pct_upper_per_100k": 1e5 * 3 / (camp_t - 147825)},
    },
}
json.dump({"issue": 580, "built": "2026-10-24", "archives": archives, "summary_only_archives": summary_only,
           "silent_folds_in_covered_windows": silent, "crosschecks": crosschecks, "reappearance": reappear,
           "estimates": est}, open(OUT, "w"), indent=1)
print(json.dumps(est, indent=1))
