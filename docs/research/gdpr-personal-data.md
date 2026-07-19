# Personal data in procurement notices — GDPR stance for tender-db

Status: research pass, 2026-07-19. **Not legal advice** — an engineering-decision
document assembled from primary sources. Every claim is marked
**[established]** (primary source fetched/verified), **[reported]** (secondary
source or search-level evidence, not independently verified), or
**[judgment]** (our engineering/legal judgment, needs user sign-off and, where
flagged, real legal advice).

Companion gap: SUMMARY.md item A1 / decision C26.

---

## 0. TL;DR

- Notices contain natural-person data by design; the eForms regulation itself
  acknowledges it (its field descriptions for BT-502/503/506 warn against
  unnecessary personal data) and TED's own privacy statement admits notices
  carry contact details of authorities and winners.
- There is **no erasure right against official-register-style publication**
  (*Manni*), but there IS a case-by-case right to have access **limited**
  (*Manni* ¶60; BGH II ZB 2/25 orders exactly a redacted-copy remedy for the
  Handelsregister). The engineering translation is a redaction-tombstone
  mechanism, not archive destruction.
- The CJEU has already struck down **unconditional public access to UBO
  data** (C-37/20). Republishing eForms UBO fields through an unauthenticated
  API and a public SQL endpoint is the one thing we should proactively not do.
- Recommended stance: three exposure tiers (open / API-only / not-public),
  personal-leaning fields isolated in dedicated satellite tables so the SQL
  gate can exclude them by table name, one append-only `redaction` table wired
  into projection and the change cursor, a public privacy notice
  (Art 14(5)(b)) and a documented takedown process. All of this is schema
  design, zero migrations later.
- The promises reword to: *append-only except documented redaction events*;
  *canonical layer = deterministic function of (archive − redaction log)*.

---

## 1. What personal data actually flows through notices

All from our own research docs (docs/research/eforms-data-model.md,
ted-legacy-mapping.md, ted-empirical-checks.md) plus the eForms SDK; the field
inventory is **[established]** against SDK 1.15.

### eForms era (2023→) — structured, explicit

| Data | Where | Nature |
|---|---|---|
| Ultimate beneficial owners | `ND-UBO` (OPT-202, `UBO-XXXX`), referenced from Organization via `ND-OrganizationUboReference` | **Natural person by definition**: name, nationality (repeatable), residence address. Appears on result notices for winners. |
| Contact-point person names | BT-502 (Organisation/TouchPoint Contact Point) | Free text; buyers routinely put an employee's full name ("Frau Dr. X Y") |
| Contact emails | BT-506 | Frequently personal mailboxes (`firstname.lastname@authority.de`), not functional addresses |
| Contact phones | BT-503 | Often direct-dial numbers of a named employee |
| Organization name | BT-500 | Usually a legal entity — but **sole traders** appear as winners, where the "organization name" *is* a natural person's name, and BT-501 registration ids can be personal identifiers |
| Addresses | BT-51x block | Legal-entity addresses normally; a sole trader's business address may be a home address |
| Free text | descriptions, BT-196 withheld-reason text, review sections | Can embed arbitrary personal data |

Scale note: eForms notices carry these per mention; Organization entities have
up to ~25 fields, UBO 17, Touchpoint 20 (eforms-data-model.md §"counts").

### The withheld-fields mechanism (BT-195…BT-198) is NOT a privacy channel

61 fields carry a `privacy` block; a notice can withhold a value with a reason
code (BT-197), free-text justification (BT-196) and an available-from date
(BT-198). The reason codelist is **procurement secrecy**, not data protection:
`eo-int` (economic-operator interest), `fair-comp` (fair competition),
`oth-int` (other public interest), `max-val` etc. Contact/UBO fields are *not*
among the privacy-capable fields — the mechanism withholds prices and
statistics, not people. Empirically 0.3–2 % of notices use it, and a
withheld-then-revealed transition has **never been observed** (82/82 archived
withheld notices unchanged when re-fetched, including 3 whose BT-198 date had
passed) — ted-empirical-checks.md §5. **[established]**

Consequence: there is no in-band mechanism in the data by which personal data
gets redacted. If TED redacts, it must happen out-of-band (see §2).

### Legacy eras (1993–2024) — inline, unstructured

- TED_EXPORT XML (2011–2024): buyer/winner contact blocks are inline
  (`CONTACT_POINT`, `PHONE`, `E_MAIL`, plus an "attention of" person field in
  older schemas), 100 % present on buyer blocks (ted-legacy-mapping.md §
  mapping table). No UBO data — UBOs exist only in eForms result notices.
- Tagged-text era (1993–2010): contact data embedded in free text.

### What TED itself publishes openly today

- Everything above is in the public bulk packages, reusable "for commercial or
  non-commercial purposes" under Commission Decision 2011/833/EU
  (ted-access-channels.md §7, TED legal notice). **[established]**
- TED's *display* retention on the website is **10 years** ("data available as
  of 1/1/2014") per the eForms FAQ
  (<https://docs.ted.europa.eu/eforms-common/FAQ/index.html>, fetched
  2026-07-19). The bulk archive nevertheless reaches back to 1993.
  **[established]** — this asymmetry matters for our backfill decision (§5,
  Open questions).

---

## 2. How the publishers themselves handle it

### TED / Publications Office

From the TED legal notice + privacy statement
(<https://ted.europa.eu/en/legal-notice>, fetched 2026-07-19)
**[established]**:

- The Publications Office openly acknowledges that TED "collects and uses
  your personal information when publishing calls for tender of contracting
  authorities … with their contact details, as well as notices of tender
  awards to successful tenderers, with the tenderers' contact details".
- Controller: Unit C.3 "TED and EU Public Procurement" of the Publications
  Office, under **Regulation (EU) 2018/1725** (the EU-institutions regulation
  — not the GDPR). Contact: info@publications.europa.eu.
- Data subjects get the standard rights ("access, rectify or erase … restrict
  the processing") by writing to the controller. **There is no published
  notice-specific redaction/takedown procedure** — the GDPR-equivalent channel
  exists (2018/1725 Arts 17–20 rights against the OP), but it is a generic
  mailbox, not a documented workflow.
- Retention: contact details are displayed for **ten years** on the TED
  website, then archived. (Matches the eForms-FAQ 10-year display window,
  §1.) The bulk-download archive nevertheless reaches 1993 — the retention
  limit is a display policy, not a data deletion.

Do TED notices ever get redacted in the data? Empirically we have never
observed any in-place change to a published notice (82/82 withheld-notice
refetches byte-identical; corrections flow as new corrigendum notices —
ted-empirical-checks.md, ted-access-channels.md §8) **[established for our
samples; absence of redaction events is not proof none ever happen]**. If the
OP ever honours an erasure request, the mechanics are undocumented; given
"republication of historical packages: no mechanism documented", a redaction
might only ever manifest on the website/API, not in already-downloaded bulk
packages — meaning **we cannot rely on upstream to propagate redactions to
us**. We need our own channel regardless. **[judgment]**

### The eForms regulation knows the problem — and pushes it to buyers

Commission Implementing Regulation (EU) 2019/1780 (CELEX 32019R1780,
<https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX%3A32019R1780>,
fetched 2026-07-19) **[established]**: the field descriptions in its annex
carry explicit data-minimisation language for exactly the fields we flagged —

> BT-502: "To avoid unnecessary processing of personal data, the contact
> point may allow identification of a physical person **only when necessary**
> (in the sense of Regulation (EU) 2016/679 and Regulation (EU) 2018/1725)."

with the same wording on BT-506 (email), BT-503 (telephone) and BT-739 (fax).
So the legislator's model is: contact fields *should* be functional
(`vergabestelle@stadt.de`), personal only when necessary — but nothing
enforces it, and real notices carry personal names/mailboxes anyway. UBO
fields, by contrast, are deliberately natural-person data with no such
caveat. The BT-195…198 unpublished-fields mechanism mirrors the withholding
grounds of Directive 2014/24/EU Art 50(4)/55(3) — release impeding law
enforcement or contrary to the public interest, prejudicing legitimate
commercial interests, or prejudicing fair competition (the eForms reason
codes map onto these one-to-one) — i.e. **procurement secrecy; privacy/data
protection is not among the grounds**. **[established** for the codelist
correspondence; directive text quoted from the stable law, not re-fetched
this pass**]**

### DÖE / BeschA (oeffentlichevergabe.de)

Their privacy page is an SPA that returns no static content to fetchers;
their Datenschutzerklärung could **not be verified** this pass. What we know
from german-portals.md §7 **[established]**: notice data is CC0; the
Open-Data-Richtlinie places content responsibility with the publishing
authority; no notice-content redaction process was found in any of their
documentation. Cheap follow-up: add the question to the planned A4 email to
support@datenservice-oeffentlicher-einkauf.de. **[gap]**

### Comparable re-publishers

- **OCDS / Open Contracting Partnership** **[established]**: the OCDS
  guidance on organization/personal identifiers
  (<https://standard.open-contracting.org/latest/en/guidance/map/organization_personal_identifiers/>,
  fetched 2026-07-19) tells publishers to disclose personal details only for
  tenderers/suppliers **and** only where "the laws in your jurisdiction
  permit the publication of such details" — i.e. even the transparency
  movement's own standard treats natural-person data as conditional, not
  default-open. OCP's publication-policy template and its "Mythbusting
  Confidentiality" report cover the same ground **[reported]**.
- **OpenTender.eu**: imprint/privacy pages returned 403 to fetchers; their
  practice is **unverified**. **[gap]**
- Commercial German tender databases (DTAD, ibau …): not investigated (time);
  low value — their terms are not precedent. **[gap, low priority]**

---

## 3. Lawful-basis landscape for tender-db's re-publication

Unlike the Publications Office (which operates under Regulation (EU)
2018/1725, the EU-institutions data-protection regulation), tender-db is a
private controller squarely under the **GDPR** for every natural-person datum
it stores and republishes. "The data is already public" is **not** a lawful
basis; it is one factor in the Art 6(1)(f) balancing. **[established]** (GDPR
has no public-data exemption; EDPB Guidelines 1/2024 on legitimate interest
reiterate that publicly available data still requires a basis and balancing —
<https://www.edpb.europa.eu/system/files/2024-10/edpb_guidelines_202401_legitimateinterest_en.pdf>,
**[reported]**, not fetched in full.)

### The favourable core: Art 6(1)(f) legitimate interest

The processing purpose — making officially published procurement data
queryable for market analysis, opportunity discovery and public-spending
transparency — is a textbook legitimate interest, strongly reinforced by the
fact that EU law itself *mandates* the publication of these notices
(Directive 2014/24/EU Arts 49–52; the eForms Regulation). Data subjects'
reasonable expectations cut in our favour: a person named as a contact point
in an EU-wide official journal cannot be surprised that the notice circulates.
**[judgment]**, but well-anchored:

### CJEU C-398/15 *Manni* (9 Mar 2017) — registry data has no general erasure right

Fetched from EUR-Lex (CELEX 62015CJ0398,
<https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX%3A62015CJ0398>)
**[established]**:

- No right "to obtain, as a matter of principle, after a certain period of
  time … the erasure of personal data" from a companies register (¶56): legal
  certainty and third-party interests take precedence, indefinitely.
- BUT ¶60: "there may be specific situations in which the overriding and
  legitimate reasons relating to the specific case of the person concerned
  justify exceptionally that access to personal data entered in the register
  is **limited**, upon expiry of a sufficiently long period … to third parties
  who can demonstrate a specific interest in their consultation."
- Mere commercial inconvenience is not enough (¶63).

Read-across for tender-db: the European model for official-publication data is
**"no erasure, but case-by-case access restriction"**. That maps exactly onto
a tombstone/redaction-at-read design rather than archive destruction — the
archive may keep the record while public access to a specific datum is
restricted on a justified request. **[judgment]**

### CJEU C-37/20 & C-601/20 *Luxembourg Business Registers* (22 Nov 2022) — the UBO signal

Fetched from EUR-Lex (CELEX 62020CJ0037,
<https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX%3A62020CJ0037>)
**[established]**:

- Invalidated Art 1(15)(c) of Directive (EU) 2018/843 insofar as it made UBO
  information accessible "in all cases to any member of the general public".
- General public access to UBO data (name, nationality, ownership) is "a
  serious interference with the fundamental rights enshrined in Articles 7
  and 8 of the Charter" (¶44); the data "enable a profile to be drawn up"
  of a person's wealth and investments, and internet availability to "a
  potentially unlimited number of persons" aggravates the interference and
  makes later defence "increasingly difficult, or even illusory" (¶¶41–43).
- Access conditioned on demonstrating a **legitimate interest** (press, civil
  society, counterparties) remains valid; it was the *unconditional public*
  channel that fell.

Read-across: eForms UBO fields (OPT-202) are published under procurement law,
a different legal channel than the AML register the Court examined — TED
publishing them is the Publications Office's problem, not automatically ours.
But the Court's *reasoning* targets precisely what tender-db would otherwise
do: take natural-person ownership data and expose it to an unlimited public
via an internet query interface, amplified by aggregation into canonical
profiles. Re-publishing UBO data through an unauthenticated API and a public
SQL endpoint sits directly against the ratio of C-37/20. This is the
strongest single argument for a field-level exposure policy (§5).
**[judgment, high confidence]**

### Erasure and objection mechanics that will actually hit us

GDPR text **[established]** (stable law, cited from the regulation):

- **Art 21(1)**: data subjects can object to Art 6(1)(f) processing "on
  grounds relating to his or her particular situation"; we must stop unless we
  demonstrate compelling legitimate grounds. This — not Art 17 — is the
  realistic legal shape of a takedown request against us, and it is
  case-by-case by design (matching *Manni*).
- **Art 17(1)(c)**: erasure follows a successful objection. **Art 17(3)(d)**
  exempts processing for archiving in the public interest / scientific or
  historical research / statistics under Art 89(1) where erasure would
  seriously impair those purposes — partially invocable for the archive layer,
  much less so for the live public query surface. **[judgment]**
- **Art 14(5)(b)**: we collect personal data not from the data subjects and
  can't individually notify millions of mentioned persons — the
  disproportionate-effort exemption applies but *requires* public information
  measures: a privacy notice on the site describing categories, sources,
  purposes, and rights. This is a launch requirement, not optional.
- **Art 12(3)**: requests must be answered within one month.

### German context

- **BGH, 18 Feb 2026 – II ZB 2/25** (verified via legal reporting,
  <https://www.fgs.de/news-and-insights/blog/detail/bgh-bestaetigt-loeschungsanspruch-von-personenbezogenen-daten-im-handelsregister>,
  fetched 2026-07-19) **[established via secondary source]**: personal data in
  Handelsregister documents *beyond what registration law requires*
  ("überobligatorische Daten" — private residential addresses, handwritten
  signatures) can be erased under Art 17 GDPR; mandatory register content
  (name, birth date, business address) stays public. Crucially, the remedy is
  **selective/partial**: the publicly accessible document is swapped for a
  redacted version (signature → "gez." + typed name) while the original stays
  in the register file. The German highest-court model is literally our
  tombstone design: redact the public copy, keep the record.
- **Bisnode (Poland, UODO 2019; upheld by the Supreme Administrative Court)**
  **[established via UODO's own English summary,**
  <https://uodo.gov.pl/en/553/1572> **, plus concordant reporting]**: the
  ~€220k fine against an aggregator of 6M+ sole-trader records scraped from
  the public CEIDG register was for breaching the **Art 14 information duty**
  (it had postal addresses and chose not to notify), **not for the
  republication itself**. Courts narrowed the duty to persons with active or
  suspended business activity. The precise lesson for tender-db: republishing
  official-register data was not the violation; failing transparency
  obligations was. Our Art 14(5)(b) posture (public privacy notice, since we
  hold no reliable contact data for mentioned persons — unlike Bisnode) is
  the load-bearing compliance surface. **[judgment on the read-across]**
- **BDSG**: §27 BDSG privileges processing for scientific/historical research
  and statistics; tender-db is a commercial product with research uses, so we
  should not lean on §27 as primary basis — Art 6(1)(f) remains the basis,
  §27 at most supports the archival layer. **[judgment]** No German court
  decision specifically on procurement-notice re-publication was found this
  pass. **[gap]**

---

## 4. Erasure-compatible immutable stores — engineering patterns

### What the field does (survey, **[reported]** — well-known practice, links not individually verified)

Three patterns recur across event-sourcing practice for GDPR:

1. **Crypto-shredding**: encrypt personal fields with a per-data-subject key;
   erasure = destroy the key (e.g. EventSourcingDB best-practices doc,
   <https://docs.eventsourcingdb.io/best-practices/gdpr-compliance/>;
   HashiCorp Vault pattern,
   <https://www.hashicorp.com/en/resources/gdpr-compliant-event-sourcing-with-hashicorp-vault>).
2. **Tombstone / redaction events**: append an event that marks data as
   erased; projections honour it; the store may additionally support physical
   scrubbing (Kafka compaction tombstones; Rails Event Store GDPR docs,
   <https://railseventstore.org/docs/v1/gdpr/>).
3. **Personal-data-out-of-band**: keep PII in a mutable side store keyed by
   pseudonym; events carry only the pseudonym (event-driven.io,
   <https://event-driven.io/en/gdpr_in_event_driven_architecture/>).

### Why crypto-shredding is wrong for tender-db **[judgment]**

- Keys are per *data subject*, but our subjects are unknown at ingest time:
  a notice mentions people without telling us which strings are people. We
  cannot key-partition what we cannot identify.
- The raw archive is the source of truth as *received bytes* (hash-based fetch
  idempotency, quarantine reprocessing). Encrypting fields inside raw XML
  destroys the raw-payload property; encrypting whole blobs per-notice keys
  gains nothing (a notice-level key deletion would erase the whole notice, not
  a field).
- Redactions will be rare (TED-level takedowns are rare, §2); a heavyweight
  key-management plane for a handful of events a year is the wrong trade.

### Recommended mechanism: an append-only redaction log ("tombstones") **[judgment]**

The invariant rewords from "the archive is append-only and immutable" to:

> **The Notice archive is append-only, except for documented redaction
> events. A redaction is itself an appended, auditable record; it removes a
> value but never a row, and never silently.**

and the rebuild promise becomes:

> **canonical layer = deterministic function of (Notice archive − redaction
> log)**. Same archive + same log ⇒ same canonical layer, byte for byte.

Concretely:

- **`redaction` table** (append-only): `id`, `scope`
  (`notice_field` / `mention_field` / `mention` / `organization_profile`),
  target keys (notice id + field path, or mention id, or org id), `category`
  of the removed data (e.g. `contact_email`, `ubo`) — **never the removed
  value itself** — `reason`, legal basis, request/decision dates, decider.
- **Raw blobs**: the affected raw XML is rewritten with the value excised and
  flagged `redacted = 1`; the original SHA-256 is kept as metadata (identity
  survives, content doesn't). True erasure must also reach WAL (checkpoint),
  `VACUUM` if pages are to be reclaimed, and expire backups — document the
  backup-retention window in the takedown reply ("fully purged after N days").
- **Parsed / canonical layers**: re-project the affected notice(s); the
  projector consults the redaction log, so the redacted value physically
  disappears from every queryable table. This is essential because the SQL
  endpoint reads real tables — a view-time filter cannot protect against
  arbitrary SELECTs. **Redaction must be materialized, not filtered.**
- **Change cursor**: a redaction advances the ordinary cursor, emitting
  `changed` (field redacted) or `removed` (mention-level) events, so SSE,
  poll, and webhook consumers converge. Because snapshots and diffs are always
  served from current tables (api-layer.md event design), a client replaying
  from an old cursor can never re-obtain the redacted value. Webhook docs
  state that consumers are expected to apply changes (making them
  "processors informed of the erasure" in GDPR Art 19 terms — our notification
  duty to recipients is discharged through the same change feed plus a note in
  the API terms). **[judgment]**
- **Quarantine interplay** (ADR-0004): a redacted notice remains reprocessable
  — the importer parses the redacted blob; the exhaustive-parse rule is
  unaffected because redaction removes element *content*, not elements.

Cost check: this touches nothing that exists yet (schema is not built), and at
steady state it is one extra table plus one projector lookup. The promise
language in CONTEXT.md/ADR-0001 needs the rewording above — that is the whole
reason this research is pre-planning. **[judgment]**

---

## 5. Risk tiers and the exposure policy

The exposure question is per **face**: the unauthenticated REST API, the
account-gated arbitrary-SQL endpoint, SSE/webhooks, and the dashboard. The
SQL endpoint is the critical one — it is the bulk-aggregation amplifier that
C-37/20 ¶¶41–43 warns about, and view-time filtering is impossible there
(arbitrary SELECTs over real tables), so exclusion must be structural.

### Tier A — plainly fine, expose everywhere **[judgment, high confidence]**

Organization names, official identifiers, legal-entity addresses, roles,
tender values, awards, CPV codes, deadlines, procedure metadata, documents
URLs, statistics. This is the core transparency payload; *Manni* logic
(third-party interest in who contracts with the state) covers it. Sole
traders are inseparably inside this tier (their firm name is a person's
name); that residual risk is handled by the takedown process, not by
proactive suppression.

### Tier B — personal-leaning operational data: API yes, SQL endpoint no **[judgment]**

Contact-point fields: BT-502 contact-point name, BT-506 email, BT-503/739
phone/fax, and the legacy-era equivalents (`CONTACT_POINT`, "attention of",
`E_MAIL`, `PHONE`).

- **Expose on per-tender/per-notice API responses and the dashboard**: the
  publication purpose of these fields is to be contacted about the procedure;
  faithfully mirroring what TED displays is squarely within the original
  purpose and users need it to act on opportunities.
- **Exclude from the public SQL endpoint** (and from any bulk export): a
  full-corpus `SELECT email FROM …` is a spam/profiling harvest with no
  transparency payoff — exactly the "unlimited number of persons …
  aggregation" fact pattern the CJEU treats as aggravating, and contrary to
  the 2019/1780 minimisation intent (§2).
- **Never index into canonical Organization profiles or search**: profiles
  aggregate mentions across notices; building a person-centric view
  (one person's email across 50 procedures) is precisely the profiling step
  that flips the balancing test against us.

### Tier C — UBO data: not exposed on any public face **[judgment, high confidence]**

`ND-UBO` (name, nationality, residence). Stored in the archive and parsed
layer (the completeness promise stands — storage ≠ exposure), but served by
no public face in v1: not in API responses, not in SQL-visible tables, not in
profiles. C-37/20 makes unconditional public UBO access the one clearly
indefensible cell in the matrix. A later gated channel (account +
logged legitimate-interest declaration, mirroring post-C-37/20 register
regimes) is possible if a user need materialises — schema hook below keeps
that door open.

### Withheld-field metadata (BT-195–198)

BT-196 free-text justifications can embed personal data; the satellite table
is fine to expose except that BT-196 text should ride with Tier B. **[judgment,
low stakes]**

### Enforcement mechanism

The SQL gate is already a parser-based allow-list (single SELECT). Extend it
with a **table allow-list**: Tier B/C data lives only in dedicated tables
(`organization_contact`, `ubo`, `withheld_field_text`), which are simply not
on the allowed list. Corollary schema rule: **no personal-leaning column ever
lives inline in a Tier A table** — then exposure policy changes and
redactions are table-scoped and never need a migration. **[judgment]**

### Takedown process (Art 21/17 requests)

1. Published contact channel (privacy@ address on the privacy notice; the
   dashboard has no verified emails, so this is a mailbox, not a form).
2. Verify identity and locate the mentions (notice ids + field paths).
3. Balance per *Manni*/Art 21: contact-person fields → grant by default
   (removing a phone number costs transparency nothing); org-role data of
   sole traders → grant only on particular-situation grounds (Manni ¶63:
   commercial inconvenience is not enough); UBO → moot if Tier C is adopted
   (not publicly processed beyond storage; on request, redact anyway by
   default). **[judgment]**
4. Redact via the tombstone mechanism (§4): append `redaction` row,
   re-project, cursor emits change events, blob rewritten, backups age out
   within the documented window.
5. Reply within one month (Art 12(3)) stating what was restricted and the
   backup-purge horizon. Log the decision (the redaction row is the log).

---

## 6. AGPL §13 note

AGPL-3.0 §13 requires that anyone interacting with a modified tender-db over
the network be offered the Corresponding Source. For us this is one link to
the public repository, surfaced where users interact: (1) the API root
response (`/v1` index JSON: `"source": "<repo url>"`) and error/usage pages,
(2) the dashboard footer, (3) the privacy-notice/imprint page. Since we run
the unmodified published code ourselves, keeping the repo public and the link
present satisfies §13; any third party self-hosting a fork inherits the same
duty, which the footer link template makes trivial. No further action.
**[established]** (licence text) / **[judgment]** (placement).

---

## 7. Implications for tender-db

Concrete, all pre-schema (nothing built yet, so all of this is free now and
expensive later):

**Schema hooks (needed NOW so redaction never requires a migration)**

1. `redaction` table, append-only: `id`, `scope`, target keys (notice id +
   field path / mention id / organization id), removed-data `category` (never
   the value), reason, legal basis, request/decision timestamps, decider.
   Consulted by the projector; drives everything else.
2. Raw-blob rows get `redacted INTEGER NOT NULL DEFAULT 0` plus
   `original_sha256` retained as identity metadata when a blob is rewritten
   with content excised.
3. Personal-leaning fields live only in dedicated tables:
   `organization_contact` (BT-502/503/506/739 + legacy contact blocks),
   `ubo`, withheld-reason free text. Tier A tables never carry them inline.
4. The SQL-endpoint gate gains a table allow-list; `organization_contact`,
   `ubo` and the redaction table itself are off-list. (The redaction table is
   internal: its existence is documented, its rows are not public — they
   point at what was removed.)
5. Canonical-layer projector applies the redaction log; the change cursor
   emits ordinary `changed`/`removed` events for redactions.

**Promise rewording (CONTEXT.md + ADR-0001, needs user sign-off)**

- "The Notice archive is append-only, **except for documented redaction
  events**; a redaction is an appended, auditable record that removes a value,
  never a row, and never silently."
- "The canonical layer is deterministically rebuildable from **the Notice
  archive minus the redaction log**."
- "Mentions are never destroyed **by merging**; they can be redacted on a
  justified data-protection request."

**Process & site furniture (launch requirements)**

- Public privacy notice (Art 14(5)(b) requires it since we cannot notify
  mentioned persons individually): categories of data, sources (TED, DÖE),
  purposes, legal basis (Art 6(1)(f)), retention, rights, takedown mailbox.
- Documented takedown workflow (§5) with the one-month clock.
- Documented backup-retention window, quoted in takedown replies.
- Attribution lines for TED (Decision 2011/833/EU) and DÖE already required
  by reuse terms — same footer real estate as the AGPL source link (§6): API
  root JSON + dashboard footer + privacy/imprint page.

## 8. Open questions

**Needs user decision**

1. Sign off the promise rewording above (touches CONTEXT.md and ADR-0001).
2. Confirm the exposure matrix: Tier B (contact fields) API-visible but
   SQL-endpoint-excluded; Tier C (UBO) on no public face in v1. The
   alternative — mirror TED exactly on every face — is defensible for Tier B
   but not, in our judgment, for UBO-over-public-SQL.
3. Backfill depth vs data minimisation: TED itself only *displays* 10 years.
   Republishing 1993–2015 contact persons is the weakest cell of the matrix.
   Option: legacy contact fields stay Tier B (they already are) and we simply
   accept the archive depth for Tier A data. Decide together with the
   pending backfill-depth decision (SUMMARY.md).
4. Whether to get real legal advice before launch. Recommended: yes, one
   focused review of the privacy notice + exposure matrix (a public SQL
   endpoint over procurement data has no direct precedent we could find).
5. Takedown mailbox address and who decides requests (solo operator: you).

**Needs research (cheap follow-ups)**

6. DÖE privacy/redaction practice — unverifiable by fetch (SPA); fold into
   the planned A4 support email.
7. OpenTender.eu and commercial re-publishers' takedown practice (403/not
   checked) — nice-to-have comparison, not load-bearing.
8. Whether the OP ever *has* redacted a TED notice (EDPS case law on OJ
   publications was not reachable this pass) — would calibrate how often we
   should expect requests. Not blocking: the mechanism is justified
   regardless.
9. EDPB Guidelines 1/2024 final text (only summarised here) — read before
   writing the privacy notice's balancing-test paragraph.
