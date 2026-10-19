# 572 unit 1 — decision (synthesis of the three judges, 2026-10-19)

Inputs:
- issue 572, 545 (the decision against a resolver rule, units 3 and 4) and 456 (design 3, deployed);
- `measure.md`, `measure.json`, `q4_class_refs.summary.json` (the 191 class mentions by (notice, section)),
  `q6_current.summary.json`, `q2_roles.txt` and `a_exhaustive.json`;
- the three judges' notes (identity, user-value, operations);
- these code reads: `crates/ingest/src/fts/parse.rs:249` (`buyer | procuringEntity` → OPT-300-Procedure-Buyer),
  `crates/ingest/src/fts/checklist.rs:48-63` (the `parties` container entry covers `roleDetails`; `details` is Ignored),
  `crates/store/src/canonical.rs:15578` (`RecordedMention::publishes` compares name, country, raw identifier and scheme,
  never the organization), `:1239` (`v_tender_buyers` keys on `role LIKE '%uyer%'`), `:21970` / `:22969` (519's
  name-conditional exact-triple arm, the model for any reopen), and `crates/app/src/v1/docs.rs` (the detail paragraph
  and test at `:1817`, "Codes and identities" at `:1110`).

This synthesis made **0 `/v1/sql` requests and 0 GETs**. The stage total is unchanged: 7 SQL requests and 46 GETs
(35 from the measure, 9 from the user-value judge, 2 from the operations judge), all in `requests.log`.

## Decision

**Rule 572-L: leave as published, and document it.** A mention keeps binding the holder of its identifier. That holds
in both directions, and for every agent, host and principal in the class.

572 adds none of these:
- a resolver rule;
- a fold-time identifier gate;
- an FTS role remap;
- a rehoming cohort for class A.

The served answer is 456 design 3. `organization_id` / `organization_name` is the identifier's holder, and
`mention_name` is the body the notice printed.

Unit 2 ships five things:
1. **A /docs caveat**, pinned by a test, that covers both directions, the host case, the `?buyer=` behaviour, and
   the `procuringEntity` / `roleDetails` drop.
2. **An honesty fix to the FTS checklist.** `parties[].roleDetails` is listed as Ignored, and `parties[].roles` says
   that `procuringEntity` becomes Procedure-Buyer.
3. **Four 545-F-shaped mentions move to issue 545's follow-up cohort.** The measure counted them in class A, but
   none of them is an agent buying for a member:
   - Pagabo ×2 on CNTW's PPON;
   - Riverside Group as Tenderer on an MoD notice, under NHC's PPON;
   - Hywel Dda as Tenderer on a Carmarthenshire notice, under NWSSP's PPON.
4. **A class-A triage rule**, so that 545, 456 and 524 stray reviews stop re-surfacing these keys.
5. **A bounded monitor and dated reopen triggers.** The identity judge's design is pre-registered as what a reopen
   builds.

Vote: two judges of three (user-value and operations) chose leave-as-published with a caveat. The identity judge
dissented for a standing reviewed alias on the P@A half. Its structural finding is adopted below as an invariant, and
its design is recorded as the reopen design, not built.

What each judge contributed:

| from | taken |
|---|---|
| user-value | The caveat's content: both directions, the host case, "read `mention_name` for display", the `?buyer=` workaround, the `procuringEntity` / `roleDetails` sentence, and a dated size. The rule that every number in the caveat is already in `measure.md`. The R2 buyer-filter read and its pairs. The reopen triggers (a consumer asks; 2× the current P@A rows; routed tenders above 10% for 5 or more principals; a structured on-behalf field). The checklist honesty fix as part of the ship. |
| operations | The build surface: the docs change is in `crates/app` only, outside 551's drift rule, and the checklist change waits for 551's flip. R1's served-state read: the exemplars plus 30 salted rows, salt `572-u2-`, 0 W. The known-A record. The cost case against the three other paths (a role split yields 0 for P@A; a fold gate lands principals on provisional rows; a per-mention rehome has no rebuild parity and spreads the agent's PPON as a raw identifier). The reopen triggers on 300 §10 / GB principal consolidation and on a general reviewed alias. The stop rule. |
| identity | The decisive structural fact. In A@P the identifier is the real buyer in 33 of 38 mentions; in P@A it is the agent's publishing account. So any future rule must key on "this identifier is an agent's account", never on name-wins or identifier-wins. This is kept as invariant I1. Also: the AGENT_ACCOUNTS list with its proof bar (B2), the negative controls (B5), and the list of non-principal names on agent accounts, all as the pre-registered reopen design. The bounded 8-org read (its B6) as the monitor and as the Verify's monitored count. The two Tenderer slips, which this synthesis routes to 545 (see "Stock"). |

One thing is new in the synthesis. q4 shows that the two Tenderer slips are not agent call-offs:
- 46798381 has Ministry of Defence as buyer;
- 46776118 has Carmarthenshire County Council as buyer.

So they have 545's harm, an agent body served as a bidder, and they go to 545 rather than being parked.

## Rule 572-L (the semantics, wording for the issue)

1. **Binding is unchanged.** A mention binds by its gated identifier first (456, 545). A buying agent's or
   consortium's identifier binds whatever name the notice printed beside it. Each subtype stays as follows:

   | subtype | mentions (2026-10-19) | served | stays because |
   |---|---|---|---|
   | P@A: a principal's name under the agent's identifier | 151 (153 less the 2 Tenderer slips) | the agent org, with `mention_name` = the principal | it is the publisher's account; the agent's part in the call-off stays findable (`?buyer=<agent>`); the principal's own record is split anyway (Birmingham 347 mentions on PPON row 30914707 against 10,165 on provisional 1291257) |
   | A@P own buyer: the agent's name under the real buyer's identifier | 33 | **the right buyer**, with `mention_name` = the agent | correct identity; the agent's name is extra information |
   | A@P host: an agent under its host council's identifier | 3 (STAR ×2 on Trafford, Ardal ×1 on Cardiff) | the host as a buyer of another body's procurement | a buying service with no legal personality of its own is keyed by its host (545's LASER / Kent precedent); the wrong *role* belongs to the role follow-up (F1) |
   | F-shaped (an unrelated holder, or a Tenderer slip) | 4 | — | **leave 572** for 545's follow-up cohort (see "Stock") |

   Left as published: **187 mentions** (P@A 151, A@P 36).

2. **Roles are unchanged.**
   - FTS `procuringEntity` stays OPT-300-Procedure-Buyer (`parse.rs:249`).
   - `roleDetails` and `details.classifications` stay unread.
   - The checklist now says so (unit 2b). Whether `procuringEntity` should become a role of its own is not decided
     here. That goes to the new issue F1.

3. **Invariant I1, binding on any reopen.** A rule that moves class-A mentions must be keyed on a reviewed fact about
   the identifier: that it is an agent's publishing account. It must never be keyed on name disagreement alone.
   Otherwise it moves the 33 correct A@P buyers, plus about 90 non-principal names on the agent accounts:
   - NWSSP's host, Velindre, ×46, and Welsh Blood Service ×1;
   - NWSSP's 32 own variants that the agent-name pattern misses, including "NHS Wales" ×10 and "NHS" ×2;
   - NHC's framework title ×4;
   - RPJ3's four "… on Behalf of <trust>" names.

4. **Class-A triage rule** (for 545, 456 and 524 stray reviews, and 524's `stray_mentions` census readers).
   - **Class A, skipped and not a rehome candidate:** a candidate in either of these two cases.
     - (a) It is bound to one of the **8 agent-account orgs** in a buyer-family role, and its name is not the agent's
       own. The 8 orgs are:
       - 30953692 NHC;
       - 30913738 NWSSP;
       - 30913798 Welland;
       - 30913722 Gibbs Laidler;
       - 30930308 Invictus;
       - 30968739 Ginger;
       - 30914820 RPJ3;
       - 30916628 NHS London Commercial Hub.
     - (b) It is one of the **(agent, holder) pairs** listed as A@P in `572-triage/q4_class_refs.summary.json`, other
       than Pagabo/CNTW.
   - **Not class A, reviewed as usual:**
     - a Tenderer or winner role under an agent identifier;
     - an agent name on a holder outside those pairs;
     - any agent outside the 15.

   The Riverside and Hywel Dda slips are the reason for these exceptions. The agents' name patterns and own orgs are in
   `a_exhaustive.json`.

## Why (the evidence that decides)

- **Information is not lost; only the org link is.**
  - All 191 mentions are visible today, with the printed body as `mention_name`.
  - The 33 own-buyer A@P rows already serve the right org.
  - NHC call-off titles name the member ("Birmingham City Council TCI May 26").
  - The alias option would trade one link for another. On 143 of 191 mentions (75%) the notice has a single buyer
    section, so the agent would stop being a served party at all. That covers NHC 97 of 97, and NHC, Welland and
    Gibbs publish no agent party. "Which tenders went through NHC's frameworks" would become unanswerable without a
    new `?agent=` filter.
- **The harm is small and concentrated.**
  - About 97 current tenders are involved: NHC 59, NWSSP 26. That is about 1% of a council's buyer list:
    - Darlington: 172 tenders, 2 missing (routed through NHC);
    - Birmingham: 198 tenders, 1 missing.
  - Hywel Dda is the outlier, at about 7 against 27.
  - Exemplar notices are mostly award and contract notices: NHC 3 of 3 are UK6/UK7, and 1 of 4 sampled NWSSP
    principal notices is biddable. So a supplier watching a council misses almost no opportunities.
  - NHC's own list (74 tenders, about 58 of them member call-offs) is the one large visible distortion. It is exactly
    what one documented paragraph explains.
- **The fixes cost more than they buy.**
  - The alias needs a new name-conditional resolver arm, plus an explicit drain (`publishes()` never re-decides a
    recorded bind).
  - Reviewing 70 keys takes more than one session's 150 GETs.
  - The principals' destinations are split:
    - HEIW: 30917293 vs 31539418 GBNHST1530;
    - Karbon: PPON row plus CHC/MPR rows;
    - Hywel Dda, ABUHB: a PPON row and an NHS row each;
    - Solihull CH: bare number 5539040.
  - NWSSP's principals sit inside 513/519's NHS-body consolidation.
  - Unreviewed new keys run at about 20% or more of new P@A mentions.
  - 551's drift ledger is open.
  - The yield is about 97 tenders spread over about 70 bodies, each of whose own record is already split.
- **545's rejection of a resolver rule (456 design 1) stands for this class too.** The identity judge's variant does
  answer 545's three objections by keying on the identifier, and that is why it is the pre-registered reopen design.
  It is not built while the measured gain is about 1% recall.
- **Park is not an option.** It leaves today's behaviour unexplained, against /docs' "served as published …
  documents the pattern here", and there is nothing to wait for.

## Unit 2 — what is built (S)

There is no supervisor change and no `Spec` arm, so `Box::pin` and the `server`-gated `supervisor.rs` do not come up.
`docs.rs` lives in the `server`-gated `v1` module, so a bare `cargo test -p tender-db` skips its tests. Gate through
`ops/check.sh`, and read the echoed `GATE-EXIT=`, never through a pipe.

### U2a — the /docs caveat (`crates/app/src/v1/docs.rs`; outside 551's drift rule, any deploy)

1. **A new bullet in "Codes and identities"** (`<h3>Codes and identities</h3>`, after the organizations bullet).
   Wording, numbers from `measure.md` only:

   > **A buying agent's identifier can stand for the body it buys for.** On Find a Tender a party carries the
   > identifier of the account that published the notice. When a buying agent, consortium or shared procurement service
   > publishes for a member body, the notice often prints the member's name with the agent's identifier: Northern
   > Housing Consortium call-offs for member councils and housing associations, NHS Wales Shared Services Partnership
   > for health boards, Welland Procurement, Gibbs Laidler Consulting for housing associations, school and energy
   > brokers. Such a party binds to the agent: `organization_id` and `organization_name` are the agent's, and
   > `mention_name` names the member, so read `mention_name` for display. `?buyer=` with the agent's id returns these
   > Tenders; with the member's own organization it does not. The reverse also occurs. An agent printed under the
   > buyer's own identifier binds to that buyer, which is the right organization. An agent printed under its host
   > council's identifier binds to the host, so the host appears as buyer of another body's procurement (STAR
   > Procurement under Trafford Council, Ardal under Cardiff Council). Find a Tender's `procuringEntity` party is served
   > as `Procedure-Buyer` beside the buyer, and its free-text `roleDetails` ("on behalf of …") is not kept. Measured on
   > 2026-10-19: about 190 party mentions on 190 Find a Tender notices, from 15 agents. They are served as published
   > (issue 572).

2. **One clause in the detail section's "A party names its organization twice" paragraph**, after "(issue 456).":
   "A buying agent that publishes for a member body under its own account is one systematic case (see
   `<a href="#caveats">caveats &rarr; Codes and identities</a>`)."

3. **A test**, `the_docs_say_a_buying_agents_identifier_binds_its_member_bodies`, in the 456 test's style:
   - It reads only the Codes and identities list, from `<h3>Codes and identities</h3>` to its `</ul>`.
   - It asserts these needles: `buying agent`, `mention_name`, `buyer=`, `host`, `procuringEntity`, `Procedure-Buyer`,
     `roleDetails`, `issue 572`.
   - It also asserts that the detail section (`<h2 id="detail">` to the next `<h2`) contains `buying agent`.
   - The existing `the_docs_say_which_name_a_party_serves` stays green.

### U2b — the checklist honesty fix (`crates/ingest/src/fts/checklist.rs`; after 551's U6 flip)

- **Add an entry:** `("parties[].roleDetails", Ignored("free-text 'on behalf of' prose beside a procuringEntity or
  buyer role; not read — the agent relation it states is not modelled (issue 572)"))`.
- **Reword `parties[].roles`:** `Mapped("OPT-300-Procedure-Buyer — from both `buyer` and `procuringEntity`, so an agent
  acting for the buyer is served as a second buyer (issue 572) / OPT-300-Procedure-CPB / OPT-301-Lot-ReviewOrg /
  OPT-301-Lot-Mediator; suppliers and tenderers are reached through the results graph instead")`.
- **Add a unit test in `checklist.rs`:** `disposition("parties[].roleDetails")` is `Ignored`, and the `parties[].roles`
  reason contains `procuringEntity`. No fixture carries `roleDetails`, so the `tests/fts.rs` census never sees it. The
  pin is the unit test.
- **Effect:** metadata only. It is not a fold input, so no job runs and 0 notices are re-queued.
- **Timing.** It is a `crates/ingest` commit, so it lands after 551's flip. If unit 2 closes earlier, hold U2b as a
  patch (546 U3's precedent). Alternatively, land it with a line in 551's B6 code-drift ledger reading "metadata only,
  0 notices re-queued". The owner chooses at merge time. Either way, 551's held flip patch re-gates on whatever base
  main has by then; it must do that anyway.

### U2c — records (tracker edits, no code)

- **572:** Status, this rule, the Verify below, the triage rule, the monitor and the reopen triggers.
- **545:** add to NEXT, for the follow-up cohort beside Wyre Forest 46842882:
  - Pagabo 46844727 and 46844778;
  - Riverside 46798381 `ORG-GB-PPON-PJXZ-4423-NDBT`;
  - Hywel Dda 46776118 `ORG-GB-PPON-PXWV-6492-CGMN`;
  - the class-A triage rule as guard G-A.
- **456:** 572 decided that class A stays as published under design 3, and design 1 stays rejected (545). 456's
  measurement readers apply the triage rule.
- **Side notes, not 572 work:**
  - 513/519: NWSSP is split across 30913738, provisional 15641552 (2,123 mentions, its `GB-NHS-RQF` procuringEntity
    sections), 31537662 and 114 provisional spellings. The Welsh principals each have a PPON row and an NHS row.
  - 470/473: Gleeds is split between 11550220 `06472932` and 31615833 `GBCOH6472932`.
  - 524: RPJ3's head is "RPJ3 Group Limited on Behalf of Aspirational Futures MAT".

## Stock: how it drains

- **Class A: by decision, nothing drains.**
  - 0 of the 187 mentions move, there is no refold and there is no project.
  - This is stable across refolds and rebuilds by construction, because the fold already does exactly this.
- **The 4 F-shaped mentions drain through 545's machinery, as a 545 follow-up cohort (not a 572 cohort).** The steps:
  1. Per-mention review: a reviewer and a skeptic, each reading `/v1/notices/{id}/content` and both orgs.
  2. `POST /admin/rehoming` for the HIGH verdicts.
  3. `apply-rehoming` dry, whose plan must equal the cohort's HIGH rehomes exactly.
  4. Wet, held against the plan tuple by tuple.
  5. `project`.
  6. If this runs before 551's flip, the touched tender ids join 551 B6 `L_changed`.
- **Destination notes for that review:**
  - **Riverside Group:** the Tenderer section printed "Riverside Group Limited". Find its own identified row by a
    bounded identifier or `name_norm` seek, in 545's q4/q9 shape. 545's G1 (shared section) applies, and it is
    clean in q4: the section binds Tenderer only.
  - **Hywel Dda (Tenderer):** the destination is split: PPON 30926960, GBNHS7A2 31544733, provisional 4863254. Use
    545's tiebreak (identified, most-referenced) coordinated with 519's Welsh 7A\* work. Park if 519 is about to fold
    those rows.
  - **Pagabo ×2:** the printed name is "Pagabo", so the destination is Pagabo's own org if one exists, or 545's exact
    provisional rule.
    - **Not The Education Alliance**, as the operations judge proposed. TEA is already printed in its own buyer section
      on both notices (31587300 and 31578022), so moving Pagabo's mention there would serve TEA twice and lose Pagabo.
    - CNTW's org 31546270 holds 7 CNTW and 2 Pagabo mentions. If review shows that PDBW-5798-GPLT is really
      Pagabo's account, this is an identifier verdict (545's CUH / Accenture precedent), not a rehome.
- **Not in any cohort:**
  - Gleeds ×4: College of Policing is served twice, the right org;
  - Essex / Bolton Impact Trust on Embark MAT, and Create PT on Howard PT: both stay parked under 545
    (`545-triage/u3/guards.md` (b)), and Embark's possible PYLY-4964-TXGH altid defect stays with 545.

## Read bars (R-style)

The unit 2 read budget is at most **2 `/v1/sql` requests** and at most **75 GETs**, at 1 per 2 s, logged to
`572-triage/requests.log`. The expected spend is about 63 GETs: R1 43 plus up to 5 replacements, R2 11 lists plus any
pages, R3 1. Every SQL request is plan-probed locally first; the plan must be SEARCH only, saved as
`.plan.txt`. A 408 is never retried.

**R1 — served state (before U2a merges; it proves the caveat true today).**
- **Rows.** One read, `q8_rows.sql`: q6's 22-arm UNION ALL, ungrouped, selecting `p.tender_id`,
  `p.seq = t.current_seq`, `p.organization_id`, `p.role`, `m.notice_id`, `m.section_id`, `m.name`. The plan is the same
  as q6's (`q6_current.plan.txt`): a `tender_version_parties_org` seek, a `tenders` PK lookup, and an
  `organization_mentions` PK lookup per arm.
- **Sample.**
  - (a) The 10 exemplar tenders whose class party is current: 8641682, 8636304, 8677647, 8698064, 8655231, 8677040,
    8639202, 8580124, 8596944, 8687157.
  - The 3 v1-only exemplars (8654651, 8621643, 8613154) are read and expected N.
  - (b) 30 current class rows, excluding the 4 transferred mentions and the exemplar tenders, one row per tender:
    - stratum P@A 22, stratum A@P 8 (all of them if fewer, with the rest from P@A);
    - each stratum ordered by `sha256('572-u2-' + tender_id + '/' + notice_id + '/' + section_id)`.
- **Scoring**, on `/v1/tenders/{id}` `parties[]`:
  - **R:** a party in the recorded role, with `organization_id` = the recorded holder and `mention_name`
    byte-equal to the printed name.
  - **N:** the tender gained a version since 2026-10-19 and no longer carries the row. The next row in salted order
    replaces it, and the N is recorded.
  - **W:** anything else: the party is missing on an unchanged version, `mention_name` is null or different, or the org
    is different.
- **Bar:** 40 R, 0 W, at most 5 replaced Ns.

**R2 — `?buyer=` semantics (the caveat's buyer sentence).**
- Lists are read in full at `limit=1000`, paginated where needed; each page counts as a GET.

  | tender | `?buyer=` must include | `?buyer=` must exclude |
  |---|---|---|
  | 8641682 | 30953692 NHC | 30914707 Birmingham |
  | 8636304 | 30953692 NHC | 30914393 Darlington |
  | 8677040 | 30913798 Welland | 30915335 Rutland |
  | 8698064 | 30913738 NWSSP | 30917293 HEIW |
  | 8596944 | 30913722 Gibbs Laidler | 2291108 English Rural HA (provisional) |
  | 8655231 | 30917275 Trafford (host) and 30916230 Rochdale | — |

- **Bar:** 6 of 6 rows exactly as stated, 0 W.

**R3 — after the deploy(s).**
- `/docs` carries the bullet: 1 GET, Verify line 2.
- `ops/check.sh` reads `GATE-EXIT=0` for each commit, and the U2a docs test and the U2b checklist test pass.
- U2b re-queues 0 notices, and no job runs.
- The monitor baseline M0 is recorded (below).

**Stop rule.** Any W in R1 or R2 stops U2a. A sentence of the caveat is false somewhere. The likeliest cause is a
regression of 456 design 3, which voids this decision's premise. Diagnose it, correct the text or reopen, then re-read
a fresh 30 at salt `572-u2b-`.

**Rider bars (owned by 545's follow-up cohort; listed so the hand-over is complete).**
- Reviewer and skeptic both HIGH, with an existing destination under 545's G3.
- The dry plan's eligible set equals the cohort's HIGH rehomes exactly; nothing else is eligible beyond the standing
  `missing_target` rows of that day.
- The wet run moves exactly that set.
- Read-back n of n: each target serves the printed role, and the holder (CNTW, NHC, NWSSP) is gone from that party.
- Cohort 545's 12 keep verdicts still bind their holders, 12 of 12.

**Monitor M** (1 `/v1/sql`, plan-probed).
- The query: `SELECT organization_id, name, count(*) FROM organization_mentions WHERE organization_id IN (30953692,
  30913738, 30913798, 30913722, 30930308, 30968739, 30914820, 30916628) GROUP BY organization_id, name`. This is a
  single-table aggregate over 8 keys, about 1,880 mentions today. A local probe on `plan.db` gives `SEARCH
  organization_mentions USING INDEX organization_mentions_org (organization_id=?)`. It is re-probed through the endpoint
  and saved as `.plan.txt` before the first run.
- Subtract the agents' own patterns and the I1 non-principal names. That gives:
  - **P@A on agent accounts:** 153 on 2026-10-19, 151 once 545 moves the slips;
  - **NHC member mentions:** 97 over 52 bodies, 96 over 51 once Riverside moves.
- **When it runs:**
  - M0 at unit 2's close;
  - then on or after each quarter (first 2027-01-19), adding the q6 shape (1 more SQL) for current P@A rows;
  - on demand, whenever a stray review meets class-A rows.

## Verify (to replace the block in issue 572)

    curl -s https://tenders.zebreus.click/v1/tenders/8641682 | jq -c '[.parties[] | select(.role=="Procedure-Buyer") | [.organization_id, .organization_name, .mention_name]]'
    curl -s https://tenders.zebreus.click/docs | grep -c 'issue 572'

- **open (2026-10-19):**
  - Line 1 reads `[[30953692,"Northern Housing Consortium Ltd","Birmingham City Council"]]`. The member body is
    served under the agent's identifier.
  - Line 2 reads `0`: /docs does not say so.
- **done:**
  - Line 1 is **unchanged**. This is by decision 572-L: the agent stays the bound organization, and the member is the
    `mention_name`.
  - Line 2 reads at least `1`: the caveat is live.
  - R1 is 40 R / 0 W and R2 is 6 of 6.
  - The FTS checklist lists `parties[].roleDetails` as Ignored (`ops/check.sh` `GATE-EXIT=0`).
  - The 4 F-shaped mentions are on 545's NEXT line.
  - M0 is recorded.
- **monitored, not driven to 0:**
  - NHC member mentions under GB-PPON-PJXZ-4423-NDBT: 97 mentions / 52 bodies on 2026-10-19 (monitor M);
  - P@A on the 8 agent accounts: 153.
- A reopen trigger (below) firing reopens 572 as "serve principal as buyer". It does not fail this Verify.

## What is declined, and why

- **Serve the principal as buyer through a reviewed standing alias** (the identity judge's Rule 572-A, P@A only).
  Declined now and pre-registered as the reopen design. What it would cost:
  - a new name-conditional resolver arm plus an explicit drain;
  - a review of 70 keys, more than one session's GET cap;
  - split destinations;
  - overlap with 513/519 on NWSSP's principals;
  - losing the agent as a served party on 75% of the class.

  What it would buy: about 1% recall on councils' buyer lists, whose own records are already split. Its I1 finding
  is adopted.
- **The agent as a separate role** (map `procuringEntity` to a non-buyer role). Declined in 572.
  - It yields 0 for P@A: 129 of 153 P@A mentions sit on single-section notices, and NHC, Welland and Gibbs print no
    agent party.
  - Even where a `procuringEntity` is printed (E5, E9, E13), the buyer party still carries the agent's PPON.
  - It changes the ADR-0015 surface (`v_tender_buyers` keys on `role LIKE '%uyer%'`) on an unmeasured number of FTS
    notices.
  - Its census belongs to F1.
- **A fold-time refusal of the agent identifier on non-agent names.** Declined.
  - About 90 non-principal names would move wrongly against 153 right ones.
  - A refused name lands on the 234 provisional row: Birmingham on 1291257, not 30914707; Hywel Dda on 4863254.
  - The stock would still need a planner, because `publishes()` keeps recorded binds.
  - It contradicts 545.
- **A per-mention 317 rehome of class A.** Declined.
  - The flow recurs at about 9–10 mentions a month, and every new notice binds the agent again.
  - It has no rebuild parity: targets are recorded as of the review.
  - It puts NHC's PPON as a raw identifier on about 52 principal orgs, which then feeds the mention-identifier
    planners (469 `mention_sides`, the wall's `evidence_keys`).
- **Park.** Declined. Nothing is pending that would decide it, and silence breaks /docs' as-published promise.
- **The operations judge's Pagabo destination (The Education Alliance).** Not adopted; see "Stock".
- **Parking the two Tenderer slips (identity judge).** Superseded. They are 545-F-shaped and go to 545's review.

## Reopen conditions

Reopen as "serve principal as buyer" when **any** of these holds:
1. **A consumer asks.** A user or consumer asks for member-body attribution or per-council buyer statistics, or
   reports the agent being credited as buyer.
2. **Size.** Monitor M shows any of:
   - P@A on the agent accounts at 306 or more (2× 153);
   - NHC member mentions at 194 or more (2× 97);
   - current P@A party rows at 194 or more (2× 97);
   - routed current tenders above 10% of the principal's own `?buyer=` list for 5 or more principals that have an
     identified row (today only Hywel Dda, at about 7 of 27).
3. **Structure.** FTS publishes a structured agent or on-behalf-of field, or F1's census adopts a distinct role.
4. **The ground moves.** 456 design 1 is reopened, 300 §10 is amended, or a GB principal consolidation lands, so that a
   moved mention would reach the principal's whole record.
5. **A general reviewed `(identifier, name key) → org` alias arm exists for another issue.** 572 then rides it with
   the frozen AGENT_ACCOUNTS list below.
6. **`mention_name` regresses** on these parties. This voids 572-L's premise immediately.

The current state grows at about 9–10 class mentions a month, so trigger 2 is years away at today's rate. Triggers 1,
3, 4 and 5 are the live ones.

## Pre-registered reopen design (Rule 572-A, condensed from the identity judge; not built)

**The design.**
- **AGENT_ACCOUNTS.** A frozen, reviewed list of identifier bodies, in every spelling idgate keys:
  - GBPPONPJXZ4423NDBT (NHC);
  - GBPPONPXWV6492CGMN (NWSSP);
  - GBPPONPVMQ1845NYPP (Welland);
  - GBPPONPGJB3773RCMH (Gibbs: the mention's PPON, not the org's COH);
  - GBPPONPGGP8622HHRX (Invictus);
  - GBPPONPLMN9829JXLV (Ginger);
  - GBPPONPRYY6263NQHG (RPJ3);
  - GBPPONPBWD3998QHRQ (NHS London Commercial Hub).

  Proof per identifier (B2): the holder's dominant name is the agent, AND either at least 2 distinct principal keys
  appear under it, or a read FTS `procuringEntity` says "on behalf of" the printed buyer. Ginger and RPJ3 rest on a
  holder majority and need that read.
- **Reviewed principal keys.** A table of (identifier, `altid_name_key`) → destination org, with merges followed. The
  destination is the identified org first, then a reviewed tiebreak, then an exact provisional row; otherwise park.
- **The arm.** It runs at the head of the identified branch, modelled on 519's `nhs_body_triple_bind`:
  - it is name-conditional (`altid_keys_agree` with a reviewed key);
  - it is never cached in `org_of`;
  - a refusal falls through exactly as if the arm were unarmed;
  - a (notice, section) bound by more than one party or role parks (545 G1).
- **What the arm leaves.** Every other name under a listed identifier binds the holder: own names, the host, variants,
  the title, "on behalf of" names, and unreviewed keys. An identifier not on the list is never touched, which is I1.
- **The stock** moves through a dry-planned, tuple-held wet apply (the 317/519 pattern) followed by a project.

**Bars.**
- **B1, review:** all keys read by a reviewer and a skeptic, with a second skeptic on every HIGH; 0 W. Parked:
  sub-brands (Karbon / 54North), hosts, titles, and splits not settled by review.
- **B2, proof:** 8 of 8 identifiers carry a recorded proof.
- **B3, dry plan:** planned equals HIGH exactly; 0 planned off the list; 0 of the A@P rows planned.
- **B4, read-back:** the 13 exemplars plus 30 salted rows, 43 of 43, 0 W.
- **B5, negative controls, 0 moves:**
  - TMT on Maidstone and Tunbridge Wells (11);
  - NWSSP on the 6 health-body orgs (15);
  - STAR on Trafford (2);
  - Ardal on Cardiff (1).
- **B6, recurrence:** after 7 or more daily folds, monitor M shows 0 reviewed keys left on the agent orgs.

**Known costs to re-measure at reopen.**
- About 20% of new P@A mentions arrive on an unreviewed key (Good–Turing), and more than that per procedure.
- The agent is lost as a served party on single-section notices.
- Principals with no identified row: English Rural HA (provisional 2291108 only) and Remarkable Autism (none).
- NWSSP's principals interact with 513/519.
- Any new `Spec` arm needs `Box::pin` (CLAUDE.md).

## Follow-up issues

- **F1 (new issue): "Find a Tender's `procuringEntity` is served as Procedure-Buyer, and its `roleDetails` is
  dropped".**
  - **Unit 1, measure first:** a dry census over the archived FTS releases. It counts:
    - parties by role set ({buyer}, {procuringEntity}, {buyer + procuringEntity}, `centralPurchasingBody`);
    - notices whose only buying party is a `procuringEntity`;
    - `roleDetails` presence and shapes;
    - `procuringEntity` parties that share a buyer's identifier.

    It is a corpus walk, so it runs as a census job, not a `/v1/sql` loop.
  - **Decide after:** a distinct role (and what it does to `v_tender_buyers` under ADR-0015 and to `?buyer=`), or
    keep the fold.
  - **It owns:** the host-case role (STAR on Trafford, Ardal on Cardiff), Gleeds' duplicate College of Policing party,
    and NWSSP's 2,123 `GB-NHS-RQF` procuringEntity mentions on provisional 15641552.
  - U2b's checklist entry records the drop until F1 decides.
- **No other new issues.** The 4 F-shaped mentions go to 545's existing follow-up cohort. The side notes (513/519,
  470/473, 524) go to those issues as one-line appends (U2c).

## Risks

1. **The harm stays and grows** by about 9–10 mentions a month (NHC about 5):
   - NHC stays credited as the buyer of about 58 of its 74 listed tenders;
   - about 97 current tenders stay missing from their principals' `?buyer=` lists;
   - HEIW's UK4 opportunities published through NWSSP's account are not on HEIW's list.

   This is accepted by decision, with triggers.
2. **The premise is 456 design 3.** The docs tests pin the text, not the data. R1 pins the data once, and trigger 6
   voids the decision if `mention_name` regresses.
3. **The class is undercounted.** Agents keyed by bare number, VAT or a provisional row are unmeasured; STAR alone has
   24 provisional rows. The caveat therefore describes a pattern and promises no list, and its size is dated and
   approximate.
4. **Visible asymmetry with 545.** 545 rehomed 62 one-off F mentions, and 572 leaves the systematic A class. Without
   the triage rule, reviewers will rehome A rows piecemeal. The rule and the issue text say why.
5. **The host case stays a wrong-role credit** (Trafford ×2, Cardiff ×1 on an older version) until F1.
6. **Caveat drift.** The figure is dated (2026-10-19) and says "about". If a reopen later builds Rule 572-A, the
   caveat and its test are retired in the same change.
