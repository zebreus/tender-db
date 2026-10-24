# 572 — a buying agent's or consortium's identifier binds the member bodies it buys for (and vice versa)

Status: ready-for-agent — **unit 2 BUILT 2026-10-24** (wf_9f0ea8c2-9c7). Read bars first: R1 40 R / 0 W / 0 N and R2 6 of 6, re-scored independently (1 SQL, 54 GETs; `572-triage/u2/`). Then U2a: the /docs caveat, the sql.rs v_tender_buyers note, the test and the CONTEXT.md "Buying agent" entry. U2b: checklist.rs owes roleDetails and details.classifications, rewords parties[].roles, and adds a unit test; metadata only, 0 notices re-queued, and 551's C1 code-drift ledger records it. Review confirmed 3 findings, all fixed. One commit, gate pending. NEXT: deploy with 551's 10-28 batch (docs.rs and sql.rs are flip files; U2b touches crates/ingest, which P1 covers), then R3 (the /docs bullet GET, 0 notices re-queued) and monitor M0, then done. Was: ready-for-agent — unit 1 MEASURED and DECIDED 2026-10-19 (rule 572-L: leave as published under 456 design 3, so a buying agent's identifier keeps binding the member body printed beside it, and document that; 5 545-F-shaped mentions move to 545's follow-up cohort; adopted after the critic's amendments, harm re-measured per principal; .scratch/tender-db/572-triage/decision.md). NEXT: unit 2 — U2a the /docs caveat, the `v_tender_buyers` note in sql.rs and their test (crates/app, any deploy), U2b the FTS checklist's two "owed" entries and the `parties[].roles` reword (crates/ingest, after 551's flip or held as a gated patch), U2c the tracker edits DONE 2026-10-19 (545's cohort of 5 plus guard G-A, 456, the 513/519/473/524 side notes appended), then read bars R1–R3 and monitor M0. Was: ready-for-agent — filed 2026-10-19 from issue 545 unit 2a (wf_fde95594-dbf: `.scratch/tender-db/545-triage/u2a/`, class "A", 52 kept rows / 53 keys / 79 mentions in the GB-PPON half alone). Next: unit 1, measure the class and decide its semantics (it is NOT 545's one-off copy-paste class, so 545's reviewed rehoming does not apply).
Kind: data quality (organization identity, buyer side)
Relates to:
- 545 (the GB-identifier-names-another-organization class; 2a split this pattern out as "A", agent and principal);
- 456 (a mention that publishes another org's identifier binds to that org);
- 482 / 481 (buyer clusters and links), 513 / 519 (NHS bodies).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/8641682 | jq -c '[.parties[] | select(.role=="Procedure-Buyer") | [.organization_id, .organization_name, .mention_name]]'
    curl -s https://tenders.zebreus.click/docs | grep -c 'issue 572'

(Rewritten by unit 1, 2026-10-19; `572-triage/decision.md` "Verify".)

- **open (2026-10-19)**:
  - Line 1 reads `[[30953692,"Northern Housing Consortium Ltd","Birmingham City Council"]]`: the member body is served
    under the agent's identifier.
  - Line 2 reads `0`: /docs does not say so.
- **done**:
  - Line 1 is **unchanged**, by decision 572-L: the agent stays the bound organization and the member is the
    `mention_name`.
  - Line 2 reads at least `1`: the caveat is live.
  - R1 reads 40 R / 0 W (at most 5 replaced N) and R2 reads 6 of 6.
  - U2b (the FTS checklist lists `parties[].roleDetails` and `parties[].details.classifications` as Ignored, "owed")
    has landed on main, or is held as patch `<sha>` gated green (`ops/check.sh` `GATE-EXIT=0`) until 551's flip.
  - The 5 F-shaped mentions are on 545's NEXT line.
  - Monitor M0 is recorded, with the 8 agent accounts resolved by identifier and their heads checked.
- **monitored, not driven to 0** (monitor M, quarterly from 2027-01-19):
  - NHC member mentions under GB-PPON-PJXZ-4423-NDBT: 97 mentions over 52 bodies on 2026-10-19;
  - P@A mentions on the 8 agent accounts: 153;
  - principals whose routed current tenders exceed 10% of their own `?buyer=` list: 16.
- A reopen trigger firing reopens 572 as "serve the principal as buyer". It does not fail this Verify.

## What was found

In the GB-PPON half of 545's candidate listing, 52 kept rows are a buying agent or consortium and the body it buys for:
Northern Housing Consortium's member councils and housing associations (34 keys), NWSSP on Welsh health boards, Star,
Ardal, Welland, and brokers. The publisher prints the principal's name with the agent's identifier (or the reverse), so
the mention binds to the identifier's holder whatever its name says. Each such pair is systematic (the same agent across
many notices), unlike 545's one-off copy-pastes, so rehoming each mention would fight the publisher every week.

## Proposed units

1. **Measure and decide (S–M).** Size the A class across GBPPON and GBCOH (reuse 545's candidates.py with the A labels) and
   read how the buyer/agent relation is published (eForms BT-765/role codes, "on behalf of" fields, the agent as
   Organization with a "procurement service provider" role). Decide: serve the principal as buyer and the agent as a
   separate role (if the data carries the role), park, or leave as published. Record the decision.
2. **Build (M)** per the decision, then a cohort drain.

## Unit 1 — measured and decided (2026-10-19)

Work dir: `.scratch/tender-db/572-triage/`. Measure: `measure.md`. Decision, with the critic's amendments adopted:
`decision.md`. Reads for the whole stage: 10 `/v1/sql` requests (cap 60) and 65 public GETs (cap 150), plus 13 Find
a Tender package GETs. Every SQL request was plan-probed, every request returned 200 (SQL rc=0), there was no 408 and
no retry. All are logged in `requests.log`.

### Size

- **545's detectors saw 129 mentions.** Its judged candidates hold 65 class-A rows (64 keys, 129 mentions).
- **The exhaustive count is 191.** Read for the same 15 agents over all 45,824 GBPPON/GBCOH-keyed orgs, the class is
  191 mentions, 85 keys and 190 notices, every one a Find a Tender (OCDS) notice.
- **P@A, 153 mentions (70 keys):** the principal's name under the agent's identifier.
  - Northern Housing Consortium: 97 mentions over 52 member councils and housing associations.
  - NWSSP: 41.
  - Gibbs Laidler: 8.
  - Others: 7.
- **A@P, 38 mentions:** the agent's name under someone else's identifier.
  - 32 sit on the real buyer's own identifier. 30 of them are proven by one read notice per (agent, holder) pair.
  - 3 sit on a host council's identifier: STAR on Trafford, Ardal on Cardiff.
  - 3 are unrelated holders: Pagabo ×2 on CNTW, and NWSSP on Swansea University (30913750) for a Swansea Bay UHB
    contract.
- **5 move to 545.** These are the 3 unrelated holders and 2 Tenderer slips (Riverside under NHC's PPON, Hywel Dda
  under NWSSP's). They are 545-F-shaped and go to 545's follow-up cohort. **That leaves 186 mentions on 185 notices,
  from 14 agents.**
- **What a principal loses today:** 95 current tenders carry a routed P@A buyer row. On 93 of them the principal's
  own org is absent, so those 93 are missing from 67 principals' own `?buyer=` lists, 1 to 7 each.
  - The share of the principal's own list depends on the kind of body. Councils have a median of 1.2% (1 of 30 above
    10%). Housing associations have a median of 10% (10 of 21 above). Health bodies have a median of 9.7% (2 of 6
    above: Hywel Dda 7 of 27, Public Health Wales 5 of 23). School trusts are 25–100% (3 of 3 above).
  - That makes 16 principals above 10%. 7 principals have no identified row an exact-name seek finds.
  - Source: `q9_share.summary.json`. Its counts equal `?buyer=` on all 8 lists read by GET.
- **Growth:** 9.7 class mentions a month over the PPON era, and 13 a month over the last 12 months (NHC 8)
  (`growth.summary.json`).

### How the relation is published

- **On Find a Tender, the mechanism is the publishing account's PPON.** In the P@A shape the `buyer` party carries
  the agent's PPON under the principal's typed name.
- **The agent appears as a separate party only sometimes.** It is a separate `procuringEntity` party, with a
  free-text `roleDetails` "on behalf of" sentence, on 7 of the 13 exemplar notices. NHC and Welland never print the
  agent.
- **Most notices have no separate agent party at all.** 127 of the 151 P@A buyer-role mentions (84%) sit on a notice
  with one buyer section, so the agent and the principal are one fused party.
- **tender-db keeps none of the relation.** The FTS parser maps both `buyer` and `procuringEntity` to
  `OPT-300-Procedure-Buyer` (`crates/ingest/src/fts/parse.rs:249`). It drops `roleDetails` and the buyer's UK_CA_TYPE
  classification, and the FTS checklist wrongly marks `roleDetails` as mapped.
- **No structured on-behalf-of field exists in the source.**
- **Served today (456 design 3):** `organization_id` and `organization_name` are the identifier's holder, and
  `mention_name` is the printed body. The SQL views `v_tender_buyers` and `v_awards` serve the holder's head name only.

### Decision: rule 572-L, leave as published and document it

A mention keeps binding the holder of its identifier, in both directions. 572 adds none of these:
- a resolver rule;
- a fold-time gate;
- a role remap;
- a class-A rehoming cohort.

The vote was 2 of 3 judges for leaving as published (user-value and operations). The identity judge dissented for a
reviewed standing alias on the P@A half. That design is pre-registered as what a reopen builds, under invariant I1: any
future rule must key on a reviewed fact that the identifier is an agent's publishing account, never on name
disagreement alone.

The critic found that the vote's harm figure ("about 1% of a council's list") does not hold outside councils. The
decision is kept on the corrected numbers for four reasons:
1. The absolute harm is 93 tenders, 1 to 7 per body.
2. The high shares sit on principals whose own record is itself a fragment (a 1-tender COH or MPR row, a bare
   number, a PPON/NHS split, or no row), so moving the mention would not make the record whole. That is reopen
   trigger 4.
3. 0 of NHC's 58 member call-offs are biddable.
4. The alias's costs are unchanged.

The old reopen trigger "5 or more principals above 10%" was already met on the day of adoption, so it is rewritten.

### Unit 2 (S), the build

- **U2a (crates/app, any deploy):**
  - a /docs bullet in "Codes and identities" covering both directions, the host case, `?buyer=`, the SQL views,
    `procuringEntity` / `roleDetails`, and the dated size;
  - a clause in the detail section;
  - a sentence in `sql.rs`'s `v_tender_buyers` note;
  - the test `the_docs_say_a_buying_agents_identifier_binds_its_member_bodies`;
  - a CONTEXT.md line for "buying agent".
- **U2b (crates/ingest, after 551's flip or held as a gated patch):**
  - `parties[].roleDetails` and `parties[].details.classifications` become Ignored("owed: …");
  - `parties[].roles` says that `procuringEntity` becomes Procedure-Buyer;
  - a unit test pins both. This is metadata only: 0 notices are re-queued.
- **U2c (tracker):**
  - 545's NEXT gains the 5 F-shaped mentions (Pagabo 46844727 and 46844778, Riverside 46798381, Hywel Dda 46776118,
    NWSSP on Swansea University 46768416) and the class-A triage rule as guard G-A;
  - 456 records that design 1 stays rejected for this class;
  - side notes go to 513/519 (the Welsh principals' routed shares, Velindre with no identified row, the Public
    Health Wales / NHS Wales Performance and Improvement question), 470/473 (Gleeds' split) and 524 (RPJ3's head).
- **The class-A triage rule** (for 545, 456 and 524 stray reviews). It skips only names and pairs on two frozen,
  dated lists in `572-triage/triage_frozen_lists.json`: 70 reviewed principal keys plus 10 non-principal names on
  the 8 agent accounts, and 11 (agent, holder) pairs, each proven by a read notice. Any new name gets normal review
  and lands in monitor M's "unreviewed key" bucket.

### Bars

- **Budget:** at most 3 `/v1/sql` requests and 75 GETs, at 1 GET per 2 s. Every SQL request is plan-probed, SEARCH
  only. A 408 is never retried.
- **R1, served state, before U2a merges:**
  - read the 10 current exemplar tenders plus 30 salted current class rows (salt `572-u2-`; P@A 22, A@P 8);
  - each must serve the recorded holder in the recorded role, with `mention_name` byte-equal to the printed name;
  - bar: 40 R, 0 W, at most 5 replaced N.
- **R2, `?buyer=`:** 6 of 6 include/exclude pairs exactly as stated (for example, 8641682 is on NHC's list and not on
  Birmingham's).
- **R3, after the deploys:**
  - /docs carries the bullet;
  - `ops/check.sh` reads `GATE-EXIT=0` for each commit;
  - U2b re-queues 0 notices;
  - M0 is recorded.
- **Stop rule:** any W in R1 or R2 stops U2a. A caveat sentence is false somewhere, most likely through a regression
  of 456 design 3. Diagnose it, correct the text or reopen, then re-read a fresh 30 at salt `572-u2b-`.
- **Monitor M:** one bounded `/v1/sql` over the 8 agent accounts, resolved by identifier first, with a head check.
  - It runs at unit 2's close, then quarterly from 2027-01-19.
  - It counts P@A on the agent accounts (153), NHC member mentions (97 over 52), Velindre on NWSSP's PPON (47), and
    unreviewed keys.
- **Reopen as "serve the principal as buyer"** when any of these holds:
  - a consumer asks;
  - P@A on the agent accounts reaches 306;
  - NHC member mentions reach 194;
  - current P@A rows reach 190;
  - 32 or more principals are above 10% of their own list;
  - any principal with a list of 20 or more tenders is above 30% (today's highest is Hywel Dda at 25.9%);
  - FTS publishes a structured on-behalf field, or F1 adopts a distinct role;
  - a GB principal consolidation lands (or 456 design 1 or 300 §10 reopens);
  - a general reviewed alias arm exists;
  - `mention_name` regresses.

  At the last 12 months' rate, the NHC and P@A size triggers are about 12–15 months away.

### Follow-up

F1 is a new issue to file with unit 2: "Find a Tender's `procuringEntity` is served as Procedure-Buyer, and its
`roleDetails` is dropped". Its first step is to measure with a census over the archived FTS releases, then decide
whether `procuringEntity` gets a distinct role. It also owns the host-case role (STAR on Trafford) and Gleeds'
duplicate College of Policing party.

### Next

Unit 2 as above: U2a and U2c first, because they are free of 551. U2b follows after 551's flip, or is held as a gated
patch. Then R1–R3 and M0, then close 572 against the Verify above.

## Unit 2 — read and built (2026-10-24, wf_9f0ea8c2-9c7; `572-triage/u2/`)

- **R1 PASS: 40 R, 0 W, 0 replaced N.**
  - q8_rows: 1 SQL, 66 SEARCH steps, 263 rows. The class now has 193 rows: all 191 mentions of 10-19 plus 2 new
    current P@A rows.
    - COMMUNITY GATEWAY on NHC (8853907/47418946) is a new key, not on frozen list (a), so it goes to monitor M's
      unreviewed bucket.
    - Cardiff and Vale on NWSSP (47428982) is an existing key.
    - Current rows: P@A 99, A@P 13.
  - The sample was the 10 exemplars (R) plus a salted 30 (A@P 8, P@A 22), every row R. The 3 v1-only exemplars are N,
    as expected.
- **R2 PASS: 6 of 6.** 11 lists, each a complete single page.
- **Re-score.** An independent re-score reproduced the salted draw exactly and every score.
- **Build.**
  - U2a, docs.rs: the "Codes and identities" bullet in the decided wording, plus the clause in "A party names its
    organization twice".
  - U2a, sql.rs: the v_tender_buyers note, with a needle in the plans test.
  - U2a, CONTEXT.md: "Buying agent", limited after review to procurement service providers, not eSenders; the host
    binding applies only where the host's identifier is printed.
  - U2b: checklist.rs, both "owed" entries citing (issue 572), since F1 is not filed. The classifications reason
    names UK_CA_TYPE, UK_CA_DEVOLVED_REGULATIONS, TED_CA_TYPE, COFOG, TED_CE_ACTIVITY and UVDB, and
    `parties[].details` no longer claims them.
- **Tests.** Focused runs: 150 passed, then checklist 3 and the census 1, all EXIT=0.
- **Budget left for unit 2:** 2 SQL statements and 21 GETs, for M0 and R3.

