# 572 unit 1 — decision (synthesis of the three judges, 2026-10-19; amended after the critic the same day)

The critic's amendments are adopted in the body below, and the last section, "Critic amendments", says what each one
changed, what was rejected and why, and which files carry the new evidence.

Inputs:
- issue 572, 545 (the decision against a resolver rule, units 3 and 4) and 456 (design 3, deployed);
- `measure.md`, `measure.json`, `q4_class_refs.summary.json` (the 191 class mentions by (notice, section)),
  `q6_current.summary.json`, `q2_roles.txt` and `a_exhaustive.json`;
- the amendment pass's reads: `q9a_principal_orgs`, `q9b_routed` and `q9_share.summary.json` (every principal's own
  `?buyer=` list against its routed tenders), `q10_class_dates` and `growth.summary.json` (the monthly rate and the
  Good–Turing estimate), and 12 notice and org bodies in `bodies/` (see "Critic amendments");
- the three judges' verdicts (identity, user-value, operations). Their notes were never written to a file, so every
  figure this decision uses is re-derived from, or saved in, this directory: `judges/user-value/` (the user-value
  judge's 9 bodies) and `judges/critic/` (the critic's 7);
- these code reads: `crates/ingest/src/fts/parse.rs:249` (`buyer | procuringEntity` → OPT-300-Procedure-Buyer),
  `crates/ingest/src/fts/checklist.rs:48-63` (the `parties` container entry covers `roleDetails`; `details` is Ignored),
  `crates/store/src/canonical.rs:15578` (`RecordedMention::publishes` compares name, country, raw identifier and scheme,
  never the organization), `:1239` (`v_tender_buyers` keys on `role LIKE '%uyer%'`), `:21970` / `:22969` (519's
  name-conditional exact-triple arm, the model for any reopen), and `crates/app/src/v1/docs.rs` (the detail paragraph
  and test at `:1817`, "Codes and identities" at `:1110`).

The synthesis made 0 `/v1/sql` requests and 0 GETs. The critic made 7 GETs. The amendment pass made **3 `/v1/sql`
requests and 12 GETs**, every one plan-probed (SQL) and logged with the tag "(572 amend)". The stage total is **10 SQL
requests (cap 60) and 65 GETs (cap 150)**: 35 from the measure, 9 from the user-value judge, 2 from the operations
judge, 7 from the critic, 12 from the amendment pass. Every GET (and the measure's 13 Find a Tender package GETs)
returned 200 and every SQL request rc=0; there was no 408 and no retry. Everything is in `requests.log`.

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
1. **A /docs caveat**, pinned by a test, that covers both directions, the host case, the `?buyer=` behaviour, the SQL
   views (`v_tender_buyers` serves the head; the printed name is in `organization_mentions`), and the
   `procuringEntity` / `roleDetails` drop.
2. **An honesty fix to the FTS checklist.** `parties[].roleDetails` and `parties[].details.classifications` are
   listed as Ignored and "owed", and `parties[].roles` says that `procuringEntity` becomes Procedure-Buyer.
3. **Five 545-F-shaped mentions move to issue 545's follow-up cohort.** The measure counted them in class A, but
   none of them is an agent buying for a member:
   - Pagabo ×2 on CNTW's PPON;
   - Riverside Group as Tenderer on an MoD notice, under NHC's PPON;
   - Hywel Dda as Tenderer on a Carmarthenshire notice, under NWSSP's PPON;
   - NWSSP's name on 46768416 under GB-PPON-PTNV-7472-GRTW, the PPON of org 30913750, headed "Swansea University"
     (160 mentions), on a Swansea Bay University Health Board contract (critic B1; read by the critic and again here).
4. **A class-A triage rule**, so that 545, 456 and 524 stray reviews stop re-surfacing these keys. It works from two
   frozen, dated lists, each entry backed by a reviewed fact, never from the identifier alone (critic B1, S4).
5. **A bounded monitor and dated reopen triggers.** The identity judge's design is pre-registered as what a reopen
   builds.

Vote: two judges of three (user-value and operations) chose leave-as-published with a caveat. The identity judge
dissented for a standing reviewed alias on the P@A half. Its structural finding is adopted below as an invariant, and
its design is recorded as the reopen design, not built.

**The vote was taken on a wrong harm figure.** The judges weighed "about 1% of a council's buyer list". The amendment
pass measured every principal (q9, see "Why"): councils are about 1%, but 16 smaller or non-council principals lose
more than 10% of their own list, and 7 principals' own records were not found at all. The judges' vote was not re-run.
This synthesis keeps 572-L on the corrected numbers, for the reasons given under "Why", and says openly that the old
reopen trigger 2(d) is already met on the day of adoption; that trigger is rewritten.

What each judge contributed:

| from | taken |
|---|---|
| user-value | The caveat's content: both directions, the host case, "read `mention_name` for display", the `?buyer=` workaround, the `procuringEntity` / `roleDetails` sentence, and a dated size. The rule that every number in the caveat is already in `measure.md` (amended: or in this decision's "Critic amendments" section, which is where the post-transfer count lives). The R2 buyer-filter read and its pairs. The reopen triggers (a consumer asks; 2× the current P@A rows; routed tenders above 10% for 5 or more principals, rewritten after the critic because 16 already were; a structured on-behalf field). The checklist honesty fix as part of the ship. |
| operations | The build surface: the docs change is in `crates/app` only, outside 551's drift rule, and the checklist change waits for 551's flip. R1's served-state read: the exemplars plus 30 salted rows, salt `572-u2-`, 0 W. The known-A record. The cost case against the three other paths (a role split yields 0 for P@A; a fold gate lands principals on provisional rows; a per-mention rehome has no rebuild parity and spreads the agent's PPON as a raw identifier). The reopen triggers on 300 §10 / GB principal consolidation and on a general reviewed alias. The stop rule. |
| identity | The decisive structural fact. In A@P the identifier is the real buyer in most mentions (amended: 30 of 38 proven by one read notice per (agent, holder) pair, 2 not proven by their read, 1 found F-shaped (Swansea University), 3 host, 2 Pagabo); in P@A it is the agent's publishing account. So any future rule must key on "this identifier is an agent's account", never on name-wins or identifier-wins. This is kept as invariant I1. Also: the AGENT_ACCOUNTS list with its proof bar (B2), the negative controls (B5), and the list of non-principal names on agent accounts, all as the pre-registered reopen design. The bounded 8-org read (its B6) as the monitor and as the Verify's monitored count. The two Tenderer slips, which this synthesis routes to 545 (see "Stock"). |

One thing is new in the synthesis. q4 shows that the two Tenderer slips are not agent call-offs:
- 46798381 has Ministry of Defence as buyer;
- 46776118 has Carmarthenshire County Council as buyer.

So they have 545's harm, an agent body served as a bidder, and they go to 545 rather than being parked.

The critic found a fifth (B1). Notice 46768416 is a UK6 for "(MAINTENANCE OF) SUPPORT OF CELLMA SYSTEMS" at "Neath
Port Talbot Hospital within Swansea Bay University Health Board". It prints NWSSP's name under PTNV-7472-GRTW, which
binds to org 30913750, headed "Swansea University" (`bodies/v1_organizations_30913750.json`: identifier
GBPPONPTNV7472GRTW, 160 mentions, no merged identifiers). A university is not the buyer of an NHS contract, so this is
545's F shape (an unrelated holder), not an agent on its principal. The row is on an older tender version only (q6
`cur = 0`), so no current `parties[]` shows it, but the mention binds the wrong org all the same.

## Rule 572-L (the semantics, wording for the issue)

1. **Binding is unchanged.** A mention binds by its gated identifier first (456, 545). A buying agent's or
   consortium's identifier binds whatever name the notice printed beside it. Each subtype stays as follows:

   | subtype | mentions (2026-10-19) | served | stays because |
   |---|---|---|---|
   | P@A: a principal's name under the agent's identifier | 151 (153 less the 2 Tenderer slips) | the agent org, with `mention_name` = the principal | it is the publisher's account; the agent's part in the call-off stays findable (`?buyer=<agent>`); the principal's own record is often split or thin anyway (Birmingham: 347 mentions on PPON row 30914707 against 10,165 on provisional 1291257, re-derived in `q9a_principal_orgs.out.json`; see "Why" for the small bodies) |
   | A@P own buyer: the agent's name under the real buyer's identifier | 32 (30 proven, 2 not proven by their read) | **usually the right buyer**, with `mention_name` = the agent | proven per (agent, holder) pair by one read notice that names the holder as the buying body (list in rule 4 (b)); NWSSP/Public Health Wales (46823552 names "NHS Wales Performance and Improvement") and PaLS/Northern HSCT (46762943 names no body) are presumed, not proven |
   | A@P host: an agent under its host council's identifier | 3 (STAR ×2 on Trafford, Ardal ×1 on Cardiff) | the host as a buyer of another body's procurement | a buying service with no legal personality of its own is keyed by its host (545's LASER / Kent precedent); the wrong *role* belongs to the role follow-up (F1) |
   | F-shaped (an unrelated holder, or a Tenderer slip) | 5 | — | **leave 572** for 545's follow-up cohort (see "Stock") |

   Left as published: **186 mentions on 185 notices, from 14 agents** (P@A 151, A@P 35). Outside the GBPPON/GBCOH
   slices, one more A@P own-buyer mention is known and proven: Gibbs Laidler printed under YMCA Saint Pauls Group's
   PPON on 46801228 (org 15069937, a swap with YMCA's name under Gibbs' PPON on the same notice). It is left as
   published and is not in the 186.

2. **Roles are unchanged.**
   - FTS `procuringEntity` stays OPT-300-Procedure-Buyer (`parse.rs:249`).
   - `roleDetails` and `details.classifications` stay unread.
   - The checklist now says so (unit 2b). Whether `procuringEntity` should become a role of its own is not decided
     here. That goes to the new issue F1.

3. **Invariant I1, binding on any reopen.** A rule that moves class-A mentions must be keyed on a reviewed fact about
   the identifier: that it is an agent's publishing account. It must never be keyed on name disagreement alone.
   Otherwise it moves the 30 proven (32 presumed) A@P buyers, plus about 40 non-principal names on the agent accounts:
   - NWSSP's 32 own variants that the agent-name pattern misses, including "NHS Wales" ×10 and "NHS" ×2;
   - NHC's framework title ×4;
   - RPJ3's four "… on Behalf of <trust>" names.

   It would also move the 47 Velindre-named mentions on NWSSP's PPON (Velindre University NHS Trust ×46, Welsh Blood
   Service ×1), whose right destination is unsettled. The measure labelled them "host" without reading them. 3 of 3
   read now are Velindre's own procurements (46856148 and 46744906 for the Welsh Blood Service, 47382186 for the
   trust's PR contract), so they are P@A-shaped, but Velindre has no identified row (provisional 13080608, 94
   mentions, per the critic's read in `judges/critic/`). They are counted in neither the 40 nor the 151 (critic N5).

4. **Class-A triage rule** (for 545, 456 and 524 stray reviews, and 524's `stray_mentions` census readers). Both
   lists are frozen and dated 2026-10-19, and both are in `572-triage/triage_frozen_lists.json` (written by
   `build_frozen.py`). A candidate matches by its bound org and its `name_norm` (lowercase, whitespace collapsed,
   trailing `.,;` stripped, as `org_name_norm`). Nothing is skipped on the identifier alone.
   - **Class A, skipped and not a rehome candidate:** a candidate in either of these two cases.
     - (a) It is bound to one of the **8 agent-account orgs** in a buyer-family role, and its `name_norm` is on the
       frozen list of **reviewed names** for that org: the 70 principal keys (151 mentions) and the 10 non-principal
       names (the agents' own variants, NHC's framework title, and the two Velindre spellings). The 8 orgs are:
       - 30953692 NHC;
       - 30913738 NWSSP;
       - 30913798 Welland;
       - 30913722 Gibbs Laidler;
       - 30930308 Invictus;
       - 30968739 Ginger;
       - 30914820 RPJ3;
       - 30916628 NHS London Commercial Hub.

       A name that matches the agent's own pattern (the same file) is the agent's own and is not a stray at all.
     - (b) It is an agent name on a holder in one of the **11 proven (agent, holder) pairs**, each backed by a read
       notice that names the holder as the buying body (or, for the 2 hosts, as the host of the service):

       | agent | holder | proof notice |
       |---|---|---|
       | NWSSP | 30919377 Aneurin Bevan UHB | 46816840 (E4) |
       | NWSSP | 30926792 Cwm Taf Morgannwg UHB | 46784422 |
       | NWSSP | 30926924 Cardiff and Vale UHB | 46742484 |
       | NWSSP | 30929723 Betsi Cadwaladr UHB | 46697948 |
       | TMT | 30916821 Maidstone BC | 46750905 (E10) |
       | TMT | 30967593 Tunbridge Wells BC | 46789370 |
       | Gleeds | 30914436 College of Policing | 46693105 (E11) |
       | Scape | 30974616 Arc Property Services Partnership | 46806701 |
       | Gibbs Laidler | 15069937 YMCA Saint Pauls Group (outside the slices) | 46801228 |
       | STAR (host) | 30917275 Trafford Council | 46817944 (E6) |
       | Ardal (host) | 30913763 Cardiff Council | 46765017 (E7) |
   - **Not class A, reviewed as usual:**
     - a Tenderer or winner role under an agent identifier;
     - any **new name** on the 8 orgs, not on list (a). It is reviewed as a normal stray, and monitor M lists it in an
       "unreviewed key" bucket until a reviewer adds it to list (a) (a principal) or sends it to 545 (an F);
     - an agent name on a holder outside list (b), including the 4 reviewed and not frozen: NWSSP on 30928312 Public
       Health Wales (46823552 names "NHS Wales Performance and Improvement"), PaLS on 30917832 Northern HSCT
       (46762943 names no body), NWSSP on 30913750 Swansea University and Pagabo on 31546270 CNTW (both F, to 545);
     - any agent outside the 14.

   The Riverside, Hywel Dda and Swansea University rows are the reason for these exceptions: an identifier-only rule
   would have hidden all three from 545 for good.

## Why (the evidence that decides)

- **Information is not lost; only the org link is.**
  - All 191 mentions are visible today, with the printed body as `mention_name` (over `/v1/sql`, through
    `organization_mentions.name`; the views serve the head, see U2a).
  - The 30 proven own-buyer A@P rows already serve the right org.
  - NHC call-off titles name the member ("Birmingham City Council TCI May 26").
  - The alias option would trade one link for another. The alias moves only P@A mentions, and 127 of the 151 P@A
    buyer-role mentions (84%) sit on a notice with a single buyer section, so the agent would stop being a served
    party at all. That covers NHC 97 of 97. NHC and Welland print no agent party on any class notice; Gibbs prints
    itself once, on 46801228, as a swap under YMCA Saint Pauls Group's PPON. "Which tenders went through NHC's
    frameworks" would become unanswerable without a new `?agent=` filter.
- **The harm is small in absolute terms, but not small for every body** (amended, critic B2; `q9_share.summary.json`).
  - 95 current tenders carry a routed P@A buyer row (NHC 58, NWSSP 25, others 12; the 2 Tenderer slips excluded).
    On 93 of them the principal's own org is not a buyer too, so 93 tenders are missing from 67 principals' own
    `?buyer=` lists, 1 to 7 each. The other 2 are swaps where the principal's own PPON is printed as well (ABUHB on
    46819472, YMCA Saint Pauls on 46801228).
  - Measured against each principal's own list (the count q9a computes is exactly `?buyer=`'s; it matched all 8
    lists read by GET, from Solihull Community Housing's 6 to Birmingham's 198), the share depends on the kind of body:

    | kind | principals measured | median share | range | above 10% |
    |---|---|---|---|---|
    | councils | 30 | 1.2% | 0.1–12.5% | 1 (Warwick DC, 1 of 8) |
    | housing associations | 21 | 10.0% | 0.5–100% | 10 |
    | health bodies | 6 | 9.7% | 0.3–25.9% | 2 (Hywel Dda 7 of 27, Public Health Wales 5 of 23) |
    | school trusts | 3 | 25% | 25–100% | 3 |

    That is 16 principals above 10%, including Solihull Community Housing (5 of 6) and six housing associations or
    trusts whose identified row holds a single tender (Arches, LIVIN, One Vision, Onward, Newlon, The Keys Academy
    Trust). Another 7 principals have no identified row that an exact-name seek finds (City of Lincoln, RBKC, NWJCC,
    William Farr School, Remarkable Autism, English Rural HA ×2, Hull Churches HA), so their 8 routed tenders reach no
    record of theirs at all.
  - The high shares sit on principals whose own record is itself a fragment: a 1–10-tender COH, MPR or bare-number
    row, a PPON/NHS split (Hywel Dda, HEIW, Cardiff and Vale, ABUHB), or no identified row. A moved mention would add 1
    to 7 tenders to a fragment; it would not make the body's record whole.
  - Opportunities are mostly not lost. NHC's own list holds 74 tenders: 60 UK7, 8 UK1, 3 UK2, 1 UK5, 1 UK6, 1 UK11,
    and no UK4 at all. Its 58 member call-offs are 56 UK7, 1 UK6 (South Lakes Housing) and 1 UK11 (Newcastle), so 0
    are biddable (`judges/user-value/v1_tenders_buyer_30953692_limit_1000.json`). NWSSP is the exception: of 4 sampled
    principal notices, 1 is a UK4 tender notice (46803631, HEIW "Mental Health Education and Training"), and 3 are
    UK5, UK6 and UK7 (`judges/user-value/n_*.json`).
  - NHC's own list (74 tenders, 58 of them member call-offs) is the one large visible distortion. It is exactly what
    one documented paragraph explains.
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
  - Unreviewed new keys run at about 20% of new P@A mentions (Good–Turing: 30 of the 70 P@A keys were seen once over
    151 buyer-role mentions, N1/N = 0.199; in the last 12 months 60 of 125 P@A mentions were on a key first seen
    then; `growth.summary.json`).
  - 551's drift ledger is open.
  - The yield is 93 tenders spread over 67 bodies, 1 to 7 each, most of them onto a fragment of the body's record.
- **545's rejection of a resolver rule (456 design 1) stands for this class too.** The identity judge's variant does
  answer 545's three objections by keying on the identifier, and that is why it is the pre-registered reopen design.
  It is not built now. The gain is about 1% recall on council lists, which is what the judges voted on. It is 10% or
  more for 16 smaller or non-council principals, but for those the destination is a fragment (see above), so the
  alias would move the share without making the record whole. That is reopen trigger 4's condition: build when a
  moved mention would reach the principal's whole record. The Welsh health bodies, the largest real lists among the
  high shares (Hywel Dda 27, Public Health Wales 23, HEIW 31, Cardiff and Vale 76), go to 513/519 as a side note,
  because their PPON/NHS split is that work's subject.
- **Park is not an option.** It leaves today's behaviour unexplained, against /docs' "served as published …
  documents the pattern here", and there is nothing to wait for.

## Unit 2 — what is built (S)

There is no supervisor change and no `Spec` arm, so `Box::pin` and the `server`-gated `supervisor.rs` do not come up.
`docs.rs` lives in the `server`-gated `v1` module, so a bare `cargo test -p tender-db` skips its tests. Gate through
`ops/check.sh`, and read the echoed `GATE-EXIT=`, never through a pipe.

### U2a — the /docs caveat (`crates/app/src/v1/docs.rs`; outside 551's drift rule, any deploy)

1. **A new bullet in "Codes and identities"** (`<h3>Codes and identities</h3>`, after the organizations bullet).
   Wording (amended, critic B1, S2, S3), numbers from `measure.md` or this decision's "Critic amendments" only:

   > **A buying agent's identifier can stand for the body it buys for.** On Find a Tender a buyer party can carry the
   > identifier of the account that published the notice instead of its own. When a buying agent, consortium or shared
   > procurement service publishes for a member body, the notice can print the member's name with the agent's
   > identifier. Northern Housing Consortium does this regularly for its member councils and housing associations;
   > NHS Wales Shared Services Partnership (for health boards), Welland Procurement, Gibbs Laidler Consulting and some
   > school and energy brokers do it occasionally. Such a party binds by the published identifier (issue 456):
   > `organization_id` and `organization_name` are the agent's, and the printed name stays in `mention_name`, so read
   > `mention_name` for display. `?buyer=` with the agent's id returns these Tenders; with the member's own
   > organization it does not, and for a smaller member body (a housing association, a school trust, a Welsh health
   > board) they can be a tenth or more of what its own list shows. Over `/v1/sql`, `v_tender_buyers` and `v_awards`
   > name the agent too (`buyer_name` is the organization's head); the printed name is `organization_mentions.name` at
   > the party's `mention_notice_id` and `mention_section_id` in `tender_version_parties`. The reverse also occurs. An
   > agent printed under the buyer's own identifier binds to that buyer, usually the right organization. An agent
   > printed under its host council's identifier binds to the host, so the host appears as buyer of another body's
   > procurement (STAR Procurement under Trafford Council). Find a Tender's `procuringEntity` party is served as
   > `Procedure-Buyer` beside the buyer, and its free-text `roleDetails` ("on behalf of …") is not kept. Measured on
   > 2026-10-19: about 186 party mentions on 185 Find a Tender notices, from 14 agents (issue 572).

2. **One clause in the detail section's "A party names its organization twice" paragraph**, after "(issue 456).":
   "A buying agent that publishes for a member body under its own account is one systematic case (see
   `<a href="#caveats">caveats &rarr; Codes and identities</a>`)."

3. **A test**, `the_docs_say_a_buying_agents_identifier_binds_its_member_bodies`, in the 456 test's style:
   - It reads only the Codes and identities list, from `<h3>Codes and identities</h3>` to its `</ul>`.
   - It asserts these needles: `buying agent`, `mention_name`, `buyer=`, `v_tender_buyers`, `host`,
     `procuringEntity`, `Procedure-Buyer`, `roleDetails`, `issue 572`.
   - It also asserts that the detail section (`<h2 id="detail">` to the next `<h2`) contains `buying agent`.
   - The existing `the_docs_say_which_name_a_party_serves` stays green.
4. **One sentence on the SQL surface** (critic S3), in `crates/app/src/v1/sql.rs`'s `v_tender_buyers` table note
   (`:972`): "`buyer_name` is the bound organization's head; when a buying agent published under its own identifier
   for a member body (issue 572), the printed name is `organization_mentions.name` at the party's
   `(mention_notice_id, mention_section_id)` in `tender_version_parties`." It is the same crate as the docs change,
   outside 551's drift rule. If a test pins that note's text, it gains the needle `issue 572`.
5. **The glossary gap** (critic N6). CONTEXT.md has no term for a buying agent, a principal (member body) or a host.
   When U2a lands, add one line to CONTEXT.md, for example "Buying agent: an Organization that publishes a notice
   for another; FTS `procuringEntity`, eForms OPT-300-Procedure-SProvider; its identifier can stand for the body it
   buys for (issue 572)", or note the gap for `/domain-modeling` if the owner prefers.

### U2b — the checklist honesty fix (`crates/ingest/src/fts/checklist.rs`; after 551's U6 flip)

- **Add an entry** (amended, critic N1: "owed", the file's own convention for a mapping not built yet, since F1
  decides whether it gets modelled): `("parties[].roleDetails", Ignored("owed: free-text 'on behalf of' prose beside
  a procuringEntity or buyer role; not read — the agent relation it states is not modelled yet (issues 572, F1)"))`.
- **Add an entry for the buyer classification** (critic N1): `("parties[].details.classifications",
  Ignored("owed: UK_CA_TYPE, the buyer's contracting-authority type (the BT-11 analogue the eForms profile stores);
  not folded for FTS (issues 572, F1)"))`. The longer path wins over `parties[].details` (longest match), so that
  entry's "outside the canonical model" reason no longer covers UK_CA_TYPE. Its unit test also checks this entry.
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
  - NWSSP on Swansea University 46768416 `ORG-GB-PPON-PTNV-7472-GRTW` (org 30913750);
  - the class-A triage rule, with its two frozen lists (`triage_frozen_lists.json`), as guard G-A.
- **456:** 572 decided that class A stays as published under design 3, and design 1 stays rejected (545). 456's
  measurement readers apply the triage rule.
- **Side notes, not 572 work:**
  - 513/519: NWSSP is split across 30913738, provisional 15641552 (2,123 mentions, its `GB-NHS-RQF` procuringEntity
    sections), 31537662 and 114 provisional spellings. The Welsh principals each have a PPON row and an NHS row.
    Their current tenders routed through NWSSP's PPON are missing from their own PPON-row lists: Hywel Dda 7 of 27,
    Public Health Wales 5 of 23, HEIW 3 of 31, Cardiff and Vale 6 of 76, Betsi 2 of 42 (`q9_share.summary.json`).
    Velindre's own procurements (47 mentions) sit on NWSSP's PPON and Velindre has no identified row. Notice 46823552
    puts NHS Wales Performance and Improvement's contract under Public Health Wales's PPON; which body is the
    contracting authority is unsettled.
  - 470/473: Gleeds is split between 11550220 `06472932` and 31615833 `GBCOH6472932`.
  - 524: RPJ3's head is "RPJ3 Group Limited on Behalf of Aspirational Futures MAT".

## Stock: how it drains

- **Class A: by decision, nothing drains.**
  - 0 of the 186 mentions move, there is no refold and there is no project.
  - This is stable across refolds and rebuilds by construction, because the fold already does exactly this.
- **The 5 F-shaped mentions drain through 545's machinery, as a 545 follow-up cohort (not a 572 cohort).** The steps:
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
  - **NWSSP on Swansea University (46768416):** the printed name is NWSSP's, so the destination is NWSSP's
    identified row 30913738, or park. The review should also ask whether PTNV-7472-GRTW is Swansea University's PPON
    (a copy-paste, a rehome) or Swansea Bay UHB's PPON sitting on the university's org (an identifier verdict). The
    row is on an older tender version only, so the project step corrects version history, not current parties.
- **Not in any cohort:**
  - Gleeds ×4: College of Policing is served twice, the right org;
  - Essex / Bolton Impact Trust on Embark MAT, and Create PT on Howard PT: both stay parked under 545
    (`545-triage/u3/guards.md` (b)), and Embark's possible PYLY-4964-TXGH altid defect stays with 545.

## Read bars (R-style)

The unit 2 read budget is at most **3 `/v1/sql` requests** (amended, critic N3: q8 1, monitor M0's endpoint EXPLAIN
probe 1, the M0 run 1) and at most **75 GETs**, at 1 per 2 s, logged to `572-triage/requests.log`. The expected spend
is about 63 GETs: R1 43 plus up to 5 replacements, R2 11 lists plus any pages, R3 1. Every SQL request is
plan-probed locally first; the plan must be SEARCH only, saved as `.plan.txt`. A 408 is never retried.

**R1 — served state (before U2a merges; it proves the caveat true today).**
- **Rows.** One read, `q8_rows.sql`: q6's 22-arm UNION ALL, ungrouped, selecting `p.tender_id`,
  `p.seq = t.current_seq`, `p.organization_id`, `p.role`, `m.notice_id`, `m.section_id`, `m.name`. The plan is the same
  as q6's (`q6_current.plan.txt`): a `tender_version_parties_org` seek, a `tenders` PK lookup, and an
  `organization_mentions` PK lookup per arm.
- **Sample.**
  - (a) The 10 exemplar tenders whose class party is current: 8641682, 8636304, 8677647, 8698064, 8655231, 8677040,
    8639202, 8580124, 8596944, 8687157.
  - The 3 v1-only exemplars (8654651, 8621643, 8613154) are read and expected N.
  - (b) 30 current class rows, excluding the 5 transferred mentions and the exemplar tenders, one row per tender:
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
- **Resolve the 8 accounts first** (critic N3), so a merge or re-key (455, 470, 481, 514) cannot zero the count
  silently. Key M on the 8 orgs' identifiers (`measure.md` (c): GBPPONPJXZ4423NDBT NHC, GBPPONPXWV6492CGMN NWSSP,
  GBPPONPVMQ1845NYPP Welland, GBCOHOC322760 Gibbs Laidler, GBPPONPGGP8622HHRX Invictus, GBPPONPLMN9829JXLV Ginger,
  GBPPONPRYY6263NQHG RPJ3, GBPPONPBWD3998QHRQ NHS London Commercial Hub), not on the org ids: one arm per
  identifier, `SELECT id, name FROM organizations WHERE identifier = ?` (an `organizations_identifier_id` seek), in
  the same request as the count below. If an identifier no longer resolves to the listed org, follow the merge through
  `/v1/organizations/{old id}` (1 GET each) and record the new id. **Head check:** each resolved org's `name` must
  still be the agent's (it matches the agent's pattern in `triage_frozen_lists.json`). RPJ3's head today is "RPJ3
  Group Limited on Behalf of Aspirational Futures MAT", which already names a principal; it is recorded as known
  (524), and any other head that stops matching is a finding.
- The count: `SELECT organization_id, name, count(*) FROM organization_mentions WHERE organization_id IN (<the 8
  resolved ids>) GROUP BY organization_id, name`. This is a single-table aggregate over 8 keys, about 1,880 mentions
  today. A local probe on `plan.db` gives `SEARCH organization_mentions USING INDEX organization_mentions_org
  (organization_id=?)`. It is re-probed through the endpoint and saved as `.plan.txt` before the first run.
- Sort every name into the frozen lists (`triage_frozen_lists.json`). That gives:
  - **P@A on agent accounts** (list (a) principal keys): 153 on 2026-10-19, 151 once 545 moves the slips;
  - **NHC member mentions:** 97 over 52 bodies, 96 over 51 once Riverside moves;
  - **Velindre on NWSSP's PPON:** 47, counted on its own line;
  - **the agents' own names and the other non-principal names:** subtracted;
  - **unreviewed keys:** every name on list neither (a) nor the agent's pattern. A reviewer clears each one, into
    list (a) as a principal or into 545 as an F (critic S4).
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
  - U2b (the FTS checklist lists `parties[].roleDetails` and `parties[].details.classifications` as Ignored,
    "owed") has **landed on main, or is held as patch `<sha>` gated green** (`ops/check.sh` `GATE-EXIT=0`) until
    551's flip (critic N2).
  - The 5 F-shaped mentions are on 545's NEXT line.
  - M0 is recorded, with the 8 accounts resolved and their heads checked.
- **monitored, not driven to 0:**
  - NHC member mentions under GB-PPON-PJXZ-4423-NDBT: 97 mentions / 52 bodies on 2026-10-19 (monitor M);
  - P@A on the 8 agent accounts: 153;
  - principals whose routed current tenders exceed 10% of their own `?buyer=` list: 16 on 2026-10-19
    (`q9_share.summary.json`).
- A reopen trigger (below) firing reopens 572 as "serve principal as buyer". It does not fail this Verify.

## What is declined, and why

- **Serve the principal as buyer through a reviewed standing alias** (the identity judge's Rule 572-A, P@A only).
  Declined now and pre-registered as the reopen design. What it would cost:
  - a new name-conditional resolver arm plus an explicit drain;
  - a review of 70 keys, more than one session's GET cap;
  - split destinations;
  - overlap with 513/519 on NWSSP's principals;
  - losing the agent as a served party on 84% of the P@A mentions it would move (127 of 151 sit on single-section
    notices).

  What it would buy (amended, critic B2): 93 tenders onto 67 principals' lists. That is about 1% recall on council
  lists (median 1.2%), but 10% or more for 16 smaller or non-council principals, most of whose own records are a
  fragment (a tiny, split or missing identified row). Its I1 finding is adopted.
- **The agent as a separate role** (map `procuringEntity` to a non-buyer role). Declined in 572.
  - It yields 0 for P@A: 127 of 151 P@A buyer-role mentions sit on single-section notices; NHC and Welland print no
    agent party, and Gibbs prints itself once, as a swap (46801228).
  - Even where a `procuringEntity` is printed (E5, E9, E13), the buyer party still carries the agent's PPON.
  - It changes the ADR-0015 surface (`v_tender_buyers` keys on `role LIKE '%uyer%'`) on an unmeasured number of FTS
    notices.
  - Its census belongs to F1.
- **A fold-time refusal of the agent identifier on non-agent names.** Declined.
  - About 40 non-principal names would move wrongly against 151 right ones, plus 47 Velindre mentions whose right
    destination is unsettled (critic N5).
  - A refused name lands on the 234 provisional row: Birmingham on 1291257, not 30914707; Hywel Dda on 4863254.
  - The stock would still need a planner, because `publishes()` keeps recorded binds.
  - It contradicts 545.
- **A per-mention 317 rehome of class A.** Declined.
  - The flow recurs at about 13 mentions a month over the last 12 months (9.7 over the whole PPON span), and every
    new notice binds the agent again.
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
   - current P@A buyer-role party rows at 190 or more (2× 95, the baseline once both Tenderer slips, which are both
     current rows, move to 545; critic N4 said 96, but the Riverside slip is current too);
   - **(rewritten, critic B2)** principals with an identified row whose routed current tenders exceed 10% of their
     own `?buyer=` list at 32 or more (2× the 16 measured on 2026-10-19), **or** any principal whose own list holds
     20 or more tenders with routed current tenders above 30% of it (today's highest is Hywel Dda at 25.9%, 7 of 27).
     The first form of this trigger ("5 or more principals above 10%") was already met on the day of adoption, by 16
     principals; it measured small bodies' thin records more than harm, so it is replaced, not counted as fired.
3. **Structure.** FTS publishes a structured agent or on-behalf-of field, or F1's census adopts a distinct role.
4. **The ground moves.** 456 design 1 is reopened, 300 §10 is amended, or a GB principal consolidation lands, so that a
   moved mention would reach the principal's whole record.
5. **A general reviewed `(identifier, name key) → org` alias arm exists for another issue.** 572 then rides it with
   the frozen AGENT_ACCOUNTS list below.
6. **`mention_name` regresses** on these parties. This voids 572-L's premise immediately.

The class grew at 9.7 mentions a month over the whole PPON span (2025-03-11 to 2026-10-19, 191 mentions over 19.6
months, NHC 4.9), but at 13.1 a month over the last 12 months and 13.3 over the last 6 (NHC 8.1; P@A buyer-role
125 in 12 months, 10.4 a month) (`growth.summary.json`). At that rate the first two size triggers are about a year
away, not years: NHC member mentions reach 194 in roughly 12 months, P@A on the agent accounts reaches 306 in roughly
15. Monitor M's quarterly run (first 2027-01-19) is therefore a real check, not a formality. Triggers 1, 3, 4 and 5
stay live as well.

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
  - NWSSP on the 5 health-body orgs (14: ABUHB 3, Cwm Taf Morgannwg 2, Cardiff and Vale 7, Public Health Wales 1,
    Betsi 1); NWSSP's 1 on Swansea University (30913750), a university and not a health body, goes to 545 instead
    (critic B1);
  - Gibbs Laidler on YMCA Saint Pauls (1, outside the slices);
  - STAR on Trafford (2);
  - Ardal on Cardiff (1).
- **B6, recurrence:** after 7 or more daily folds, monitor M shows 0 reviewed keys left on the agent orgs.

**Known costs to re-measure at reopen.**
- About 20% of new P@A mentions arrive on an unreviewed key (Good–Turing: 30 singleton keys over 151 P@A buyer-role
  mentions, 0.199), and 60 of the last 12 months' 125 P@A mentions were on a key first seen in that window
  (`growth.summary.json`).
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
- **No other new issues.** The 5 F-shaped mentions go to 545's existing follow-up cohort. The side notes (513/519,
  470/473, 524) go to those issues as one-line appends (U2c).

## Risks

1. **The harm stays and grows** by about 13 mentions a month over the last 12 months (NHC about 8):
   - NHC stays credited as the buyer of 58 of its 74 listed tenders;
   - 93 current tenders stay missing from 67 principals' own records; for 16 principals that is more than 10% of
     their own `?buyer=` list (Solihull Community Housing 5 of 6, Hywel Dda 7 of 27, Public Health Wales 5 of 23,
     and thirteen smaller housing associations, school trusts and one council), and 7 principals have no identified
     row an exact-name seek finds;
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

## Critic amendments (adopted 2026-10-19)

The critic's verdict was "AMEND BEFORE ADOPTING": 2 blocking, 4 should and 6 nit amendments, checked against 10
claims and 7 GETs of its own. Every blocking and should amendment held on the evidence and is adopted above. The nits
are adopted too. Five points are rejected or corrected in part; they are listed at the end of this section with the
reason for each.

**Reads made for this pass** (all logged "(572 amend)" in `requests.log`):
- 12 GETs: 8 notice contents for B1 and N4 (46784422, 46823552, 46697948, 46742484, 46789370, 46806701, 46762943,
  46801228), the org 30913750 for B1, and 3 Velindre-named notices for N5 (46856148, 46744906, 47382186). Bodies in
  `bodies/`.
- 3 `/v1/sql` requests, each plan-probed locally with python sqlite3 and turso `plan-probe` on `plan.db` (SEARCH
  only, saved as `.plan.txt`) before it ran:
  - `q9a_principal_orgs`: every identified org an exact `name_norm` seek finds for each of the 69 P@A principals on a
    current class row (115 spellings), plus 21 known ids, with each org's current buyer count; plus S1's two
    Birmingham mention counts. 135 rows.
  - `q9b_routed`: per principal, the current tenders routed through the agent's identifier, and how many of those do
    not also carry one of the principal's own identified orgs as buyer. 69 rows. (The read is split in two because
    `/v1/sql` takes at most 64 KiB of body, `sql.rs` MAX_BODY.)
  - `q10_class_dates`: `published_at` of the 190 class notices, by primary key.
- Scripts: `build_q9.py`, `analyze_q9.py` (writes `q9_share.summary.json`), `analyze_growth.py` (writes
  `growth.summary.json`), `build_frozen.py` (writes `triage_frozen_lists.json`), and `get_amend.sh` (`get.sh` with
  the log tag).

### B1 (blocking): the A@P own-buyer presumption — adopted

The critic's misbind holds. Notice 46768416 is a Swansea Bay University Health Board contract ("Neath Port Talbot
Hospital within Swansea Bay University Health Board"). It prints NWSSP's name under PTNV-7472-GRTW, and that binds
org 30913750, headed "Swansea University", 160 mentions (`bodies/v1_organizations_30913750.json`). One qualification:
the row is on an older tender version only (q6 `cur = 0`). So "served as Procedure-Buyer" is true of the version
history, not of any current `parties[]`. The misbind stands either way.

One notice was read for each A@P pair that had no read notice:

| pair | notice | what the notice says | verdict |
|---|---|---|---|
| NWSSP on 30926792 Cwm Taf Morgannwg UHB | 46784422 | "Cwm Taf Morgannwg University Health Board (CTMUHB) has awarded …" | proven |
| NWSSP on 30926924 Cardiff and Vale UHB | 46742484 | "Cardiff and Vale University Health Board, acting on behalf of the All Wales Genomics Service …" | proven |
| NWSSP on 30929723 Betsi Cadwaladr UHB | 46697948 | "Betsi Cadwaladr University Health Board is seeking …" | proven |
| NWSSP on 30928312 Public Health Wales | 46823552 | "NHS Wales Performance and Improvement are awarding …"; Public Health Wales is not named | not proven |
| TMT on 30967593 Tunbridge Wells BC | 46789370 | "Tunbridge Wells Borough Council is looking to have the large Pond … relined" | proven |
| Scape on 30974616 Arc | 46806701 | "SCAPE, on behalf of Arc Partnership (the Client) …" | proven |
| PaLS on 30917832 Northern HSCT | 46762943 | "Patient information for consent leaflets …"; no body named | not proven |
| TMT on 30916821 Maidstone BC | 46750905 (E10, no new GET) | "Maidstone Borough Council (MBC) is looking to appoint …" | proven (already) |

With E4 (NWSSP on ABUHB) and E11 (Gleeds on College of Policing), which were read before, the 38 A@P mentions now
split like this:
- **own buyer, 32:** 30 proven and 2 not proven;
- **host, 3;**
- **F-shaped, 3:** Pagabo ×2 and Swansea University ×1.

All F-shaped rows, the 2 Tenderer slips included, now total 5. **Left as published: 186 mentions on 185 notices,
from 14 agents (P@A 151, A@P 35).**

What changed:
- the Decision list (5 transfers) and "Stock";
- the subtype table, now "usually the right buyer";
- the identity judge's row and I1, now "30 proven (32 presumed)";
- rule 4 (b), now a frozen list of 11 pairs, each with its proof notice;
- B5, now 5 health-body orgs with 14 mentions, plus Gibbs on YMCA;
- the caveat, now "usually the right organization";
- U2c and the 513/519 note.

### B2 (blocking): the harm figure and trigger 2(d) — adopted, with 572-L kept and the trigger rewritten

The critic's figures hold exactly. q9 measured every principal on a current class row:
- **The count is right.** q9a's own count equals `?buyer=` for all 8 lists read by GET: Darlington 172, Birmingham
  198, NHC 74, Hywel Dda 27, HEIW 31, Public Health Wales 23, Cardiff and Vale 76, Solihull Community Housing 6.
- **The critic's 5 shares reproduce:** Solihull Community Housing 5 of 6, Public Health Wales 5 of 23, Hywel Dda 7
  of 27, HEIW 3 of 31, Cardiff and Vale 6 of 76.
- **Size:** 95 routed current tenders. On 93 of them the principal is absent, across 67 principals. The other 2
  are swaps: ABUHB and YMCA are printed under their own PPON too.
- **Distribution:**
  - councils: median 1.2%, 1 of 30 above 10%;
  - housing associations: median 10.0%, 10 of 21 above 10%;
  - health bodies: median 9.7%, 2 of 6 above 10%;
  - school trusts: 3 of 3 above 10%.
- **Unmatched:** 7 principals have no identified row an exact-name seek finds. Better spellings may find a row for
  City of Lincoln or RBKC; they were not tried.

So 16 principals are above 10%. The old trigger 2(d) (5 or more) was met on the day of adoption.

The decision text changed in seven places:
- the vote note;
- "Why": the table, the fragments point and the opportunities point;
- the reason the alias is not built;
- "What is declined";
- trigger 2(d), rewritten as 2× the measured 16, or 30% for a principal with 20 or more listed tenders;
- "today only Hywel Dda", deleted;
- Risks, and the 513/519 note.

**Why 572-L still stands on the corrected numbers.** The judges were not reconvened in this pass. Keeping 572-L is
this synthesis's call, recorded here so a reviewer can overturn it on these numbers.
1. **The absolute harm is unchanged:** 93 tenders, 1 to 7 per body.
2. **The high shares sit on fragments.** The six bodies whose identified row holds 1 tender, Solihull Community
   Housing's bare-number row, the PPON/NHS splits (Hywel Dda, HEIW, Cardiff and Vale, ABUHB) and the 7 with no row
   found are where the share is high. Moving 1 to 7 tenders onto such a row does not make the body's record whole.
   That is reopen trigger 4's condition, which stays live.
3. **Opportunities are mostly not lost.** 0 of NHC's 58 member call-offs are biddable. NWSSP is the exception, with
   1 of 4 sampled.
4. **The cost case is unchanged:** the resolver arm, 70 keys to review, split destinations, and the overlap with
   513/519.

The Welsh health bodies have the largest real lists among the high shares. They go to 513/519 with their figures.

### S1 (should): the evidence trail — adopted in part

**Copied:**
- `judges/user-value/`: the user-value judge's 5 `?buyer=` lists and 4 notice bodies.
- `judges/critic/`: the critic's 7 bodies.

**Re-derived:**
- Birmingham's split, 347 mentions on 30914707 against 10,165 on 1291257: exact (q9a, "S1 mentions").
- Darlington 172 / 2, Birmingham 198 / 1, NHC 74 / 58 and Hywel Dda 27 / 7: from q9a, q9b and the copied lists.
- **The growth rate** (`growth.summary.json`, from q10). Over the whole span, 191 mentions in 19.6 months
  (2025-03-11 to 2026-10-19) give 9.7 a month, NHC 4.9. That confirms "9–10, NHC about 5" as a whole-span average.
  But the last 12 months run at 13.1 a month (NHC 8.1) and the last 6 at 13.3. So "trigger 2 is years away" was
  wrong: about a year. This is corrected in "Reopen conditions" and in Risks.
- **Good–Turing:** 30 of the 70 P@A `name_norm` keys were seen once, over 151 buyer-role mentions, which gives
  0.199. That confirms "about 20%".
- **"Misses almost no opportunities":**
  - NHC's list holds 60 UK7, 8 UK1, 3 UK2, 1 UK5, 1 UK6, 1 UK11 and no UK4.
  - Its 58 member call-offs are 56 UK7, 1 UK6 and 1 UK11.
  - NWSSP's 4 sampled notices: 46803631 UK4 (HEIW), 46754741 UK6, 46808676 UK5, 46828235 UK7.

**Not possible:** the three judges' notes themselves. They were never written to a file; the session scratchpad
holds the user-value judge's bodies and the critic's bodies and nothing else for 572. The operations judge's two
"stdout only" GETs (the Birmingham and Hywel Dda `name_prefix` searches) cannot be recovered either. What they
showed is re-derived by q5 and q9a. The decision's "Why" now rests only on files in this directory.

### S2 (should): the caveat overgeneralised — adopted

The new wording is in U2a item 1, and it changes five things:
- "a buyer party can carry the identifier of the account that published the notice instead of its own";
- NHC "regularly", and NWSSP, Welland, Gibbs Laidler and the brokers "occasionally" (NWSSP 41 of 1,170 mentions,
  Welland 1 of 287, per `measure.md` (c));
- "binds by the published identifier (issue 456), and the printed name stays in `mention_name`", in place of
  "served as published";
- the host example is STAR/Trafford alone, because 8621643 v5 serves Cardiff as "Cardiff Council";
- the count is after the transfers: about 186 mentions on 185 notices, from 14 agents.

### S3 (should): the SQL surface — adopted

Verified:
- `v_tender_buyers` serves `buyer_name = o.name`, the head (`canonical.rs:1231-1241`).
- `v_awards`' representative `buyer_name` is the same head (`canonical.rs:1263-1266`).

What was added:
- a caveat sentence pointing to `organization_mentions.name` at the party's mention keys;
- U2a item 4, a sentence for `sql.rs:972`'s `v_tender_buyers` note;
- the test needle `v_tender_buyers`.

### S4 (should): triage rule (a) too broad — adopted

Rule 4 (a) now skips only a name on a frozen, dated list of reviewed names: 70 principal keys covering 151 mentions,
and 10 non-principal names (`triage_frozen_lists.json`). Any new name on the 8 orgs gets normal review and lands in
monitor M's "unreviewed key" bucket until a reviewer clears it.

### Nits — adopted

- **N1:** the `roleDetails` entry now reads "owed", and a `parties[].details.classifications` entry for UK_CA_TYPE
  is added (U2b).
- **N2:** the done line reads "landed on main, or held as patch `<sha>` gated green".
- **N3:** the unit-2 budget allows 3 SQL requests. Monitor M is keyed on the 8 identifiers, resolved first through
  merges, and checks each org's head. RPJ3's head is recorded as known.
- **N4:** three corrections.
  - The alias share is now 127 of 151 P@A buyer-role mentions (84%).
  - The Gibbs sentence is corrected. Gibbs on YMCA (46801228, "… as required by YMCA Saint Pauls Group") is read,
    proven, and recorded outside the slices, so it is not in the 186.
  - Trigger 2(c) is re-baselined to 95.
- **N5:** 3 of 3 Velindre-named notices read are Velindre's own procurements: two for the Welsh Blood Service and
  one PR contract. Velindre has no identified row. The 47 mentions are left out of both I1 counts (now about 40
  non-principal names, against 151) and are counted on their own line in monitor M.
- **N6:** CONTEXT.md has no term for a buying agent, a principal or a host. Adding one is U2a item 5, because this
  pass edits no repo file outside `572-triage`.

### Rejected or corrected in part

1. **B1's read of TMT on 30916821:** not made. Exemplar E10 (46750905) already proves that pair, so the critic's
   list counted it as unread by mistake.
2. **S1's "copy the judges' notes":** cannot be done, because the notes do not exist as files. Re-derivation
   replaces them (above).
3. **N4's baseline of 96:** corrected to 95. Both Tenderer slips are current rows (q6: Riverside `cur = 1` and the
   Hywel Dda Tenderer `cur = 1`), and q9b's routed total is 95.
4. **B2's other option, re-running the judges' vote:** not taken. The amendment allows either path. This pass took
   "keep 572-L, say openly that 2(d) is met, rewrite the trigger".
5. **B1's wording that Swansea University "is served as Procedure-Buyer":** qualified. The row is on an older
   version only. The transfer is adopted unchanged.

### Found in this pass, beyond the critic

- **The class is growing faster than the decision said:** 13 a month over the last 12 months, against 9.7 over the
  whole span (S1).
- **Two routed rows are not missing.** For ABUHB (46819472) and YMCA Saint Pauls (46801228) the routed tender still
  lists the principal under its own PPON, so their routed count does not mean "missing". A 545 or 572 reviewer must
  not read every P@A row as a missing principal.
- **Velindre has no identified row, and NWSSP's PPON carries its own procurements.** This goes to 513/519.
- **The Public Health Wales pair is open.** Notice 46823552 puts an "NHS Wales Performance and Improvement" contract
  under Public Health Wales's PPON, and the read does not settle which body is the contracting authority. That
  question goes to 513/519. The pair stays out of rule 4 (b).

`measure.md` is left as it was. Where it says "33 own buyer (3 read, 30 presumed)", this decision supersedes it.
