# DR premise at current scale + the unrebuildable state — re-check for the 2026-07-19 "no off-box backups" decision

Research date: 2026-08-09. Closes **gap 4** of `docs/research/research-gaps-2026-08.md` (issue 170) and the **D4/D5 half of gap 8b** (issue 173's probe-status question). This is a repo + light-metadata study: sources are the issue board, the research corpus, the store schema (every `CREATE TABLE` in `crates/store/src/`), plus exactly two light production reads on 2026-08-09 — `df`/`du` on `/data` and the `/admin/jobs` recent-run log (both metadata/bounded, free under `docs/agents/prod-box-reads.md`). No data pages were read; row counts of user-state tables are schema-reasoned, not queried. Everything **[verified]** was read or measured today; **[measured]** cites the original measurement; **[estimated]** is derived; **[from issue N]** cites the board.

---

## 0. Bottom line

The 2026-07-19 decision — *"no off-box backups — everything rebuildable at the cost of roughly a day"* (SUMMARY.md §5, CONTEXT.md) — is wrong today on both halves, in opposite directions:

1. **"Roughly a day" is really ≈3–6 days** (scenario-dependent), because the number traced to pilot-sizing's ~2.2 h figure, which was **notice-layer parse+load only, projection explicitly excluded** — and projection turned out to be the dominant cost (~24–30 h measured at full scale, *after* a month of optimization work). Issue 23's counter-claim of "weeks" is also wrong now: it predates the issue 58/62/66/91 projection fixes. The honest number sits between the two myths.
2. **"Everything" is no longer everything.** Accounts, API tokens, webhook registrations, and change-cursor continuity exist now, are *not derivable from any source*, and — since the 2026-08-06 snapshot removal (commits 39c0e08/aa9f2a1/1faf9d9: DR = re-ingest, no ring exists) — have **zero copies anywhere**. Losing the DB file loses them permanently: there is no email on accounts by design, so not even a re-claim path exists.

The exposure is small in bytes (KBs today, MBs at launch scale) and cheap to close. The recommendation menu is §7; the minimum honest move is (b) — a tiny off-box user-state copy — plus two small fixes ((§3 finding) an archive re-register path, and scheduling the D4 probe) regardless of which backup posture Lennart picks.

---

## 1. What the box holds today [verified 2026-08-09]

| Item | Size | Note |
|---|---|---|
| `/data` volume | 1000 GB, **781 GB used (79 %)**, 219 GB free | handover's "87 %" predates the 08-06 ring deletion |
| `/data/db/tender-db.db` | **560 GB** (`du`) | the 2026-08-06 snapshot job reported 455.57 GB copied; handover says "455–558 GB" — 560 GB is the current allocated size |
| `/data/archive/ted` | 176 GB | immutable raw packages, 1993→ |
| `/data/archive/doe` | 3.3 GB | 2022-12→ |
| `/data/snapshots` + `/data/db/snapshots` | **0 / empty** | the ring is really gone, not just disabled |
| swapfile / scratch-lots / tmp | 16 + 12 + ~0 GB | issue-57 swap band-aid still present |
| Corpus | 14,249,656 notices, ~8.1 M tenders | notice count from the last-ever snapshot job's own log line [verified, job 570]; tenders from HANDOVER-2026-08-07 |

Two structural facts that shape every scenario below:

- **A plain on-box copy of the DB is no longer possible**: 219 GB free < 560 GB DB. Only an XFS **reflink** copy fits (the deleted ring was measured "near-fully reflink-shared" — freeing it recovered only ~9 GB [from HANDOVER-2026-08-07]).
- **The last-ever snapshot job is the only at-scale backup measurement that exists**: 2026-08-06 07:53 UTC, job 570 — *"455.57 GB, 14,249,656 notices, integrity ok · frozen 941 s, verify 606 s"* [verified from `/admin/jobs`]. Checkpoint+copy at half a terabyte = **~16 min writer freeze + ~10 min verify**. Keep this number; nothing else like it will be produced now that the feature is deleted.

---

## 2. Stage rates — the evidence table

Every recovery scenario is composed from these stages. Rates are production measurements unless marked otherwise.

| Stage | Measured rate / wall-clock | Source |
|---|---|---|
| **Re-fetch full archive** (~180 GB, 397 TED + 44 DÖE monthlies + dailies) | **~1–1.5 days** wall (started 2026-07-20, `DONE; df used=200GB` logged 2026-07-21 morning; ≤3 concurrent downloads, disk-guard-paced) | [measured, from issue 15] — wire burst is 760 Mbit/s [measured, pilot-sizing §3.4], so the bound is politeness + package walking, not bandwidth |
| **Process** (parse + ingest whole archive) | **70–120 notices/s era-dependent** (peaks ~390 n/s in the 1990s text era, lows 51–62 n/s in late XML at scale) ⇒ 14.25 M notices = **33–57 h pure compute**; the July run took ~2.5–3 days wall including incidents | [measured, from issues 28 + 15]. NB: pilot-sizing's 1,047–1,472 n/s was the throwaway pilot tool with a rough schema; the real exhaustive-consumption parser + full satellite writes is ~10× slower — part of why "2.2 h" was never going to survive contact |
| **Projection, full rebuild** (`rebuild=true`, 14.2 M notices → 8.1 M tenders) | plan build ~2 h [scoped plan measured 2h17m, issue 91] + bucketed pre-pass **7 h 13 m** [measured 2026-08-01, issue 91] + fold+apply **11 h 51 m** / 8.1 M tenders [measured, issue 91] + end index builds ~1–3 h [estimated from issue 82's 49-min single-index build] ⇒ **~24–30 h**; issue 91's own total for `rebuild=true`: *"~30 h with the tender layer empty throughout"* | [measured, from issues 91/62/82, ADR-0009] |
| **Incremental projection** (for scale feel, from the current job log) | 1.1 K notices → 186 s; 4.5 K → 740 s; the 59 K reclaim fold → **8 h 17 m** (job 578, 2026-08-06, sub-threshold ParsedFold) | [verified, `/admin/jobs` 2026-08-09] |
| **ANALYZE** | 11 s / 10 GB ⇒ **~10 min** at 560 GB | [measured turso-scale §1; extrapolated] |
| **Verification** | `verify` + `data-quality` binaries (external ground truth + row counts): **~0.5 day** operator time. No full `integrity_check` path exists at this scale in-app (turso-only rule; the snapshot's row-count verify took 606 s) | [estimated; mechanism from docs/operations.md + turso-scale §1] |
| **Box re-provision** (code + unit + nginx + TLS + secrets) | **~0.5–1 day**: cold nix build is tens of minutes, everything else is documented in operations.md and regenerable | [estimated from operations.md build times] |

**Finding (new, load-bearing): the archive alone cannot drive a rebuild — the `fetches` registry lives in the DB.** `crates/ingest/src/fetch.rs::fetch()` decides idempotency from `db.latest_fetch(...)`, never from on-disk bytes, and `process` walks packages via `fetches` rows. So a lost DB forces a **full ~180 GB re-download even when `/data/archive` is intact** — there is no register-existing-files path. This is a small, cheap feature (hash the on-disk file, `record_fetch` it) that removes ~1–1.5 days from the most likely bad scenario and makes the archive actually function as the "source of truth" ADR-0001 and the removal commit call it.

---

## 3. Scenario estimates — the honest totals

All totals assume the current (2026-08-09) codebase with the issue-91 fold routing; the single writer serializes the stages. Throughout any recovery, **the public instance serves an empty or partial tender layer until the rebuild's fold completes** (ADR-0009) — "service degraded" lasts the whole wall-clock, not just the fetch.

| Scenario | Stages | Wall-clock | Source-data loss | **User-state loss** |
|---|---|---|---|---|
| **S1 — canonical layer damaged** (bad projection, fat-fingered layer wipe; DB file otherwise healthy) | `project rebuild=true` + verify | **~1–1.5 days** | none | **none** (same DB file) |
| **S2 — DB file lost/corrupt** (archive + box intact) | re-fetch (forced, §2 finding) + process + project + ANALYZE/verify | **~4–6 days** today; **~3–4.5 days** with a re-register-from-disk path | none (re-derived) | **TOTAL** — accounts, tokens, webhooks, cursor continuity, all provenance history |
| **S3 — volume lost** (`/data` gone; box intact) | provision volume + S2-with-refetch | **~4–6 days** | none *if upstream still serves everything* — see caveat below | **TOTAL** |
| **S4 — box lost, volume survives** (Hetzner volume is a separate device) | re-provision box, reattach, redeploy, regenerate secrets/TLS | **~0.5–1 day** | none | **none** |
| **S5 — box + volume lost** (account-level/datacenter event) | S4 + S3 | **~4–7 days** | as S3 | **TOTAL** |

Caveats, honestly stated:

- **These are babysat-compute estimates, and the record says first attempts run long.** Every multi-day production run so far collected incidents (WAL pinning — issue 42/52; projection OOM — 57/61; index hangs — 62/82; tmpfs sort spill — 83). The 2026-08-01 full rebuild *was itself* the recovery from such an incident. Plan for the top of each range on a first-attempt DR.
- **No DR drill has ever been run.** turso-scale's "restore drill" open question was never closed, and the snapshot-era drill died with the feature. The re-ingest path's *component stages* have all run at full scale in production (which is better evidence than most DR plans have), but never end-to-end from an empty volume.
- **S3's re-fetch has an unverified premise**: that TED/DÖE still serve the full history and serve it *equal to what we ingested*. TED has no checksums/ETags, the 1996 package carries baked-in upstream rot [from issue 15], and the D4 immutability probes that would measure re-fetchability drift **have never run** (§6). DÖE history exists only back to 2022-12 upstream. Re-fetch also cannot restore the original `sha256` provenance — the proof of what was originally ingested is itself DB-resident (§4, class C).
- Issue 23's header ("weeks of processing time to rebuild") should be read as superseded by this table: it was written before the projection optimizations (58/62/66/91) landed.

---

## 4. The unrebuildable-state enumeration

Sweep of **every** `CREATE TABLE` in `crates/store/src/{lib,accounts,webhooks,jobs,canonical}.rs` (35 durable tables + transient `plan_*` + views + `sqlite_sequence`), classified into three classes. Sizes are schema-reasoned where row counts would need a data read (out of scope for this study).

### Class A — derivable from the archive (re-process; ~1.5–2.5 days)

`notices`, `notice_sections`, `notice_texts`, `notice_codes`, `notice_classifications`, `notice_amounts`, `notice_dates`, `notice_integers`, `notice_numbers`, `notice_ids`; `quarantine` (rows re-derive — the same parser re-quarantines the same members); `fetches` (rows re-derive, but **only via re-download** today — §2 finding). This is the bulk of the 560 GB.

### Class B — derived but expensive (projection; ~24–30 h)

`tenders`, `tender_versions`, `lots`, `tender_version_{lots,texts,amounts,dates,classifications,parties}`, `organizations`, `organization_mentions`, `lot_results`, `bids`, `contracts`, `tender_version_{lot_results,result_winners,result_stats,bids,bid_parties,contracts}`, `changes` (**rows** regenerate — see C3 for what does not), `projection_state`, `layer_presence` (re-observes), transient `plan_*`.

### Class C — NOT derivable from anything

| # | State | Where | Size today [estimated] | What loss means |
|---|---|---|---|---|
| C1 | **`users`** (username, argon2id PHC, created_at) | accounts.rs | a handful of rows — the handover names `diag-svc` + `acceptance-verify`, plus operator accounts; **KBs** | Every account gone. **No email exists by product decision** (CONTEXT/C21), so there is no re-claim path even in principle — "lost password = lost account" scales up to "lost table = lost user base". Post-launch this is every customer. |
| C2 | **`api_tokens`** (SHA-256 of `tdb_` tokens) | accounts.rs | few rows; KBs | Every programmatic integration breaks simultaneously. Tokens are shown once at mint; nothing can regenerate them — every client must re-onboard. |
| C3 | **Change-cursor continuity + epoch** — `changes.cursor` numbering, its `sqlite_sequence` row, and the implicit "generation" (ADR-0009 `clear_changes`) | canonical.rs | one integer, effectively | A fresh-DB rebuild restarts the cursor and assigns entirely new tender ids: **every externally held cursor** (SSE `Last-Event-Id`, poll `since=`, webhook slots) is silently invalid. The documented reset/epoch protocol (issue 46) is **designed but not built**, so consumers cannot even be told properly. Any DR event is therefore also a mandatory, currently-unimplementable epoch reset. |
| C4 | **`webhook_endpoints`** (URL, **plaintext** signing secret, `last_delivered_cursor`, backoff state) | webhooks.rs | 0–few rows pre-launch; KBs | Every subscriber integration destroyed; signing secrets must be re-exchanged out of band with each consumer. Cursor slots are C3-invalid anyway. |
| C5 | `sessions` (SHA-256 session ids, 30-day lifetime) | accounts.rs | ephemeral; KBs | Acceptable loss by design — users re-login. Listed for completeness. |
| C6 | `webhook_delivery_log`, `job_log` (~588 rows [verified: max job id]), `job_queue` (pending jobs) | webhooks.rs, jobs.rs | tens of KBs | Operational history + in-flight work. Acceptable loss, but `job_log` is the only record of what jobs ran when — the provenance layer of every "verified in prod" claim on the board. |
| C7 | **Provenance columns inside otherwise-derivable tables**: `fetches.sha256/fetched_at` (the *only* proof of originally-ingested bytes — also D4's input), `notices.ingested_at`, `quarantine.first_seen/reprocessed_at/skipped_at/skipped_reason` (the row-level resolution history behind the "zero unexplained quarantine" claim — the committed `quarantine-ledger.json` keeps the summary, the dates live only in the DB) | lib.rs | — | Re-ingest stamps everything with recovery-day timestamps: arrival history, the reclaim audit trail, and the ability to ever detect upstream package mutation are gone. Not service-affecting; honesty-affecting. |
| C8 | Off-DB box state: `/root/tender-admin-secret`, diag token/password, systemd drop-ins, nginx vhost, TLS | box root disk | — | All regenerable from operations.md in ~an hour (S4). Not a data-loss item; listed so the enumeration is complete. |

**Total genuinely unrecoverable state today: well under 1 MB** (C1+C2+C4+C3's manifest); at a launch scale of thousands of users + tokens + webhooks, **single-digit MBs**. This is the "MBs, not issue 23's 500 GB problem" the gap register predicted — confirmed by schema.

---

## 5. The delta — what was decided vs. what is true

| 2026-07-19 decision record | 2026-08-09 reality |
|---|---|
| "Everything rebuildable **at the cost of roughly a day**" — traced to pilot-sizing §3.4: ~2.2 h XML-era parse+load (+40 min eForms, +20 min text, +~15 min download), with *"canonical-layer projection is not included (not designed yet)"* written right under it | Projection alone is **~24–30 h measured** [issue 91]; the real app's parse rate is ~10× the pilot tool's cost; scope grew (full history + DÖE + 1.8 M reclaimed quarantine → 14.25 M notices, DB 560 GB vs. the 100–300 GB planning band). Honest end-to-end: **~1–1.5 d (S1) to ~4–6 d (S2/S3)** |
| "No off-box backups for now (accepted risk)" — with C24 (off-box target) left open, and issue 23 then *building* the snapshot machinery | 2026-07-21: Lennart declined the Storage Box → **local ring only**. 2026-07-22: volume grown 500 G→1 TB largely *for ring headroom*. **2026-08-06: owner removed the snapshot feature entirely** (storage pressure; ring + code deleted, snapwatch off). Since then **DR = re-ingest, and there is no copy of anything, anywhere off `/data`** — code excepted (private GitHub mirror) |
| "Everything" = the notice + canonical layers; **user state did not exist** at decision time | `users`/`api_tokens`/`sessions`/`webhook_endpoints` shipped (issues 06/08) and production carries live accounts; `changes` is the public contract's spine. **§4 class C is not rebuildable and is not covered by the decision that governs it** |
| Issue 23 (2026-08 wording): rebuild costs "weeks of processing time" | Superseded: predates the 58/62/66/91 projection fixes. The measured stack says days, not weeks (§3) |
| CONTEXT.md still ships the decision verbatim ("…at the cost of roughly a day"); operations.md still documents the snapshot ring, the daily pipeline "ends with a snapshot", and the restore drill | **Doc drift** — see §8. `prod-box-reads.md` was updated on removal day; `operations.md` and `CONTEXT.md` were not |

The decision was reasonable on the evidence it had. But its two load-bearing inputs — the wall-clock and the "everything" — have both moved, and the 2026-08-06 removal *widened* the blast radius (the ring used to cover S1-class corruption and fat-fingers in minutes; now S1 costs a ~1-day re-projection). This is exactly the "different decision than the one Lennart signed off" the gap register flagged; it needs his signature again either way.

---

## 6. D4/D5 probe status (gap 8b) — **neither is implemented or scheduled**

Method: swept the supervisor's complete job-kind surface (`crates/app/src/supervisor.rs`: `fetch`, `probe`, `process`, `reprocess`, `project`, `reindex`, `refold`, `mark-skipped-siblings`, `clear-rebuild-flag` — nothing else exists), all of `nix/` (module.nix, package.nix, backup-ship.sh; no timers beyond the dormant ship script), `docs/operations.md` (no mention), a repo-wide grep for rehash/immutability/republication/BT-198-reveal machinery, and the 20 most recent `job_log` entries on prod [verified 2026-08-09: only probe/process/project/reprocess + the final snapshot appear].

- **D4 (monthly re-hash of old TED packages; DÖE monthly-export immutability)** — [from SUMMARY §2.D, ted-access-channels §8 "needs a re-hash probe over time"]: **not implemented, not scheduled**. The *mechanism* half-exists: `fetch` with `refetch:true` re-downloads, compares by sha256, and versions a changed package (`Outcome::NewVersion`, never overwriting the archive) — but it is only ever aimed at the **current day's finality window** by the scheduler. Nothing points it at historical packages on any cadence. Cost to close: a small scheduled job (or a monthly line in the daily pipeline) sampling N old packages with `refetch:true` — the fetch path already does the hard part.
- **D5 (periodic BT-198 reveal recheck)** — [from SUMMARY §2.D, ted-empirical-checks §"open": *"re-run `51_republication.py` periodically once ingestion is live"*]: **not implemented, not scheduled**. `51_republication.py` exists only as a research artifact on the VPS (`/opt/tender-db/`, outside the repo — itself a D7 fragility). The in-DB material to do it natively exists (`notice_withheld_fields` view computes `publish_after`), but no job kind, no schedule, and ingestion has been live since July.

Relevance beyond gap 8: **D4 is a DR input.** The re-ingest DR premise assumes re-fetched bytes ≈ ingested bytes; D4 is the only measurement that could ever confirm or refute that, and after an S2/S3 event the original hashes (C7) are gone — the probe must run *before* it is needed.

---

## 7. The re-decision menu

Framed as options for Lennart because the binding constraint is his standing "no external resources" rule (2026-07-21), which any off-box copy must relax in some direction. The options compose; (b) does not preclude (c).

### (a) Status quo, honestly documented

Keep zero backups; rewrite the RTO/RPO into `operations.md` and `CONTEXT.md` (§8): **RTO ~1–1.5 d (S1) / ~4–6 d (S2/S3); RPO = 0 for source-derived data, RPO = ∞ (permanent) for §4 class C.** Cost: €0 and a documentation hour. Defensible **pre-launch only** — the decision record's own escape clause ("revisit if the dataset ever stops being cheaply rebuildable") has already triggered for user state, which was never cheaply rebuildable at any price. Two no-regret hardening items belong in (a) regardless of choice:
1. **Archive re-register path** (§2 finding): hash on-disk packages into `fetches` — removes ~1–1.5 days from S2, the most likely scenario.
2. **Schedule D4** (§6): re-fetchability drift is a DR input, and it is unmeasured.

### (b) Tiny off-box user-state copy — **recommended pre-launch minimum**

*What*: `users`, `api_tokens`, `webhook_endpoints` — verbatim rows, ids included (the FK graph is internal to the set; on restore, bump `sqlite_sequence` past the max imported ids). Skip `sessions` (30-day lifetime, users re-login) and `webhook_delivery_log` (debug ring). Plus a **continuity manifest**: current `changes` head cursor, a generation/epoch marker (this is issue 46's epoch, which needs building anyway), and headline row counts (`notices`, `tenders`) for post-restore sanity. `quarantine-ledger.json` is already committed to the repo.

*Size*: **< 100 KB today; single-digit MBs at launch scale** [estimated, §4].

*How, under standing rules*: an in-app Supervisor job (`kind: "export-user-state"` — turso-only rule respected, `/admin`-visible, `job_log`-recorded) doing bounded SELECTs over KB-scale tables and writing one JSON (or SQL) file to a staging dir; a systemd timer ships it — the exact split issue 23 already established (app produces, box timer ships).

*Where* (the actual decision — pick one):
1. **Encrypted blob to the existing private GitHub mirror** (age/openssl; the mirror is the one already-sanctioned external resource; today it is "code only", so this needs an explicit scope extension from Lennart). €0.
2. **Hetzner Storage Box BX11** (~€3.81/mo, 1 TB — six orders of magnitude of headroom; rsync-native; the issue-23 recommendation revived at 1/1000th the payload).
3. **Pull-based**: Lennart's machine fetches the export over the existing ssh path on a cron. €0, but ties RPO to his laptop's uptime.

*Cadence*: daily after the 09:35 pipeline pre-launch; hourly post-launch (RPO ≤ 1 h on account/webhook churn).

*Restore path*: fresh DB → schema migrates at open → `import-user-state` job inserts the rows → accounts and tokens work the moment the app is up, **days before** the corpus finishes re-ingesting; webhook `last_delivered_cursor` slots are reset to the new log head and the epoch change announced (issue 46's protocol — build it once, it serves both DR and routine rebuilds).

*What it buys*: the only permanently-unrecoverable state (§4 C1/C2/C4, plus C3's announceability) becomes recoverable for ~€0–4/mo and about a day of implementation. It restores the *letter* of the 2026-07-19 premise: after (b), everything really is rebuildable-or-copied.

### (c) Full off-box DB copy — the economics, so the "no" is informed

- **Size**: 560 GB now; plan 0.6–0.8 TB over the next year (~30 GB/yr steady-state notice-layer growth [pilot-sizing §3.3 extrapolation] plus ~50–60 M change rows per rebuild generation [ADR-0009]; the file can never shrink — VACUUM is impossible [turso-scale §1]).
- **Creation**: checkpoint+copy measured at scale: **941 s frozen + 606 s verify at 455 GB** [verified, job 570]. An XFS reflink copy would cut the frozen window to seconds and the staging cost to divergence-only — the only staging that *can* work now (219 GB free, §1).
- **Transfer**: 1 Gb/s uplink ⇒ 560 GB ≈ 1.4 h at wire speed; realistically **3–14 h** per full copy to a Storage Box [estimated; unmeasured]. Daily full copies are not viable; **weekly** is workable. Rsync-delta on a checkpointed B-tree file is unmeasured and likely poor after projections (page churn everywhere). Litestream-style WAL shipping is off the table by the turso-only/single-process owner rule.
- **Cost**: BX21 5 TB **~€12.90/mo** (holds ~6 generations at current size); Object Storage ~€6/TB/mo. Hetzner-internal traffic free.
- **What it buys over (b)**: S2/S3 collapse from ~4–6 days to **copy-back + swap ≈ 0.5–1 day**, and the provenance history (§4 C7/C6) survives. RPO = cadence (a week), vs. RPO 0 via re-derivation for source data — so (c) is an RTO play, not an RPO play, for everything except user state.
- **What it does not buy**: it still needs (b)'s cadence (hourly) for user state if weekly RPO on accounts is unacceptable — another reason (b) comes first.

**Recommendation**: (b) + the two no-regret items from (a), decided before launch; (c) optional, revisit when the RTO of "days" first threatens a real obligation (paying users, an SLA, or the first support email that says "our integration is down").

---

## 8. Corrections owed to other documents (doc drift found by this study)

- **`docs/operations.md`** — the entire "Backups" section, the scheduler description ("…and finally one `snapshot`"), the `JobRequest` kind list, and the restore drill still document the feature **deleted on 2026-08-06**. `prod-box-reads.md` was updated that day; operations.md was not. It should state DR = re-ingest and carry §3's numbers.
- **`CONTEXT.md`** ("…everything is rebuildable … at the cost of roughly a day") — update with the honest range and the user-state carve-out once Lennart re-decides.
- **`nix/backup-ship.sh`** — ships nothing that exists anymore; delete or retarget as (b)'s shipping template.
- **Issue 23** — "weeks of processing time" header is superseded by §3; the issue is the natural home for the (b)/(c) implementation once decided.
- **`.scratch/tender-db/issues/170`** — this doc discharges the study half; the issue stays open pending the Lennart re-decision.

## Open questions

- **[needs decision — Lennart]** The re-decision itself: (a)/(b)/(c) of §7, and — if (b) — which shipping destination relaxes the no-external-resources rule.
- **[needs research, cheap]** Actual row counts of the class-C tables (three bounded `COUNT(*)` reads via `/v1/sql` on the lead's word) to replace §4's "[estimated] KBs" with numbers.
- **[needs research, conditional on (c)]** Measured rsync throughput and delta ratio box→Storage Box for the 560 GB file; unmeasured, and it decides (c)'s cadence.
- **[needs building regardless]** Issue 46's epoch/reset protocol — every scenario in §3 ends with an epoch reset nobody can currently announce.
- **[drill]** One restore drill of whatever is chosen, with measured durations recorded in operations.md — no DR path of this system has ever been exercised end-to-end.
