# 440 — a TED/OJ S notice number is accepted as an organization identifier, and 425 org rows are keyed by one

Status: ready-for-agent — DEPLOYED 2026-09-27 14:40 UTC (rev `9dedf49`, commit `db8ed6d`); `repair-placeholder-orgs` DRY queued as job 1601 (after 432's dry, 1600); wet after 432's wet. Was: BUILT 2026-09-27 (uncommitted, not deployed, see the foot): `idgate::condemns` now refuses the
OJ S publication number as a class. What remains is deploy → `repair-placeholder-orgs` dry → wet → `project`, then the
Verify. Filed 2026-09-27 from a prod read of org 13782393.
Kind: defect (org layer: identifier admission, `crates/ingest/src/idgate.rs`)
Relates to: 365 (listed `2018S150345644`, `2018S025054890`, `2017S204421475` and `2023S198622496` as exhibits in its
Observed and closed without a unit for them), 300 (the gate, and the `repair-placeholder-orgs` dissolve this reuses),
312 (the GUID class: why "it does not look like a register number" is not enough), 432 (the dissolve re-resolves on its
`org_name_norm` key), 393/397 (the re-parse fold; ordering below)

## Observed (2026-09-27, `/v1/sql`, bounded seeks)

**The exhibit.** Org 13782393: `DE`, `national`, `2018S025054890`, name `Deutsche Bahn AG`, `provisional = 0`. It holds
129 mentions on 129 `ted-export-r209` notices (ids 20542859–21187762). Every mention's `raw_identifier` is
`2018/S 025-054890` (from `TED-NATIONALID`, scheme `national`). They carry **30 distinct names**: `Deutsche Bahn AG`
(78), `DB Netz AG (Bukr 16)` (12), `DB Netz AG` (12), and 27 of the form `Knoten Lindau Kabeltiefbau NT nn`. The
27 are the procedure's Nachtrag titles, published in the name slot.

`2018/S 025-054890` is the number of notice 19668712 (`054890-2018`): DB Netz AG (Bukr 16)'s contract notice
"Knoten Lindau Kabeltiefbau". DB reused the procedure's original notice number as the contracting body's NATIONALID on
every later notice. Only **1** of the 129 notices (20988916) cites it as its prior notice (`TED-REF_NOTICE.NO_DOC_OJS`).
The notice's own numbers are different: on 20542859 `TED-NO_DOC_OJS` is `2019/S 143-351238` and `TED-NOTICE_NUMBER_OJ`
is `2018/S 122-277874`.

**The class.** The stored key is the normaliser's compaction (ASCII alphanumerics, upper-cased), `YYYYS` followed by
digits. Per-year prefix seeks on `organizations_identifier_id` give the following:

| stored shape | org rows | mentions | values with ≥2 names | worst |
|---|---:|---:|---:|---:|
| `YYYYS` + 9 digits (`2018/S 025-054890`) | 403 | 966 | 35 | 30 |
| `YYYYS` + 8 digits (a two-digit issue: 19 are 2011–2012, r208's unpadded `2011/S 79-130403`; 3 are 2018–2019) | 22 | 42 | 1 | 2 |
| **total** | **425** | **1,008** | **36 (8.5 %)** | **30** |

- **Every value is a notice number.** 425 of 425 resolve to a TED notice in this corpus with exactly that publication
  number. 424 match the legacy `NNNNNN-YYYY` id. The one eForms-era value (org 23174070 `JILITI`, BT-501
  `2025S056090000`, sdk-1.13) matches `00090000-2025`.
- **No country owns the shape.** The 403 spread over 18 country codes: DE 297, PL 20, FR 20, GR 11, NL 10, GB 10,
  IT 9, AT 7, SE 4, PT 3, ES 3, RO 2, FI 2, and TZ/MK/LI/CZ/BG 1 each. All are `national` and `provisional = 0`. No
  register issues `YYYYS` + 7–9 digits. A register number never puts a year and a literal `S` in front of a serial.
- **More than the exhibit fuses.** For example org 1507020 `2010S251385627` holds Kliniken München Pasing Perlach GmbH
  together with its planning consultant IBDG Ingenieurbüro für Haustechnik GmbH.
- **One key per notice.** Each value keys exactly one row (425 values, 425 rows).

**Near the shape and left alone.** The same seeks list 63 `YYYYS…` rows outside the 9-digit form. 22 of them are the
8-digit class above. The other 41 are left alone (all measured, all one row each):
- ten digits after the `S` (2): `2011S0213347880`, `2018S0736162207`
- a number glued to another number or to text (4): `2011S175287091UND2011S183298253`,
  `2023S0270776402023S053156199`, `2019S175427242E26139926`, `2012S19030620FASSADENARBEITEN`
- buyers' own `…S…` file references (35): `2024S076079`, `2026SG340201`, `2014SER26`, `2018S637` and 31 others
- label-prefixed OJS numbers: **0 rows** under the prefixes `OJS`, `OJEU`, `OJ2`, `TED`, `ABL`, `EU2`, `JOUE`, `DOUE`,
  `GUUE`, `AMTSBLATT` and `SUPPLEMENT`

**TED's other spelling, the publication id `NNNNNN-YYYY`: measured, not a class.** I scanned four 20k-notice windows
of `organization_mentions` (notice ids 12.53M, 16.99M, 20.54M and 24.06M: r208 2011, r208 2012, r209 2019 and eForms
2024; 307,200 mentions). **51** raw identifiers contain the shape. **43** of them are real Swedish organisationsnummer
and Icelandic kennitölur, for example Boden Taxi AB `556364-2023`, Karolinska Institutet `202100-2973` and Árborg
`650598-2029`. **5** are a Thüringen agency's own `16900621-1000-50`. Only **3** are TED ids (DE: `200653-2011`,
`136766-2011`, `122232-2012-DE`), and none of the three is the notice's own id. After normalisation the dash is gone, so
the value is ten bare digits: the shape of an SE orgnr, a PL NIP or a BE KBO.

## Mechanism

1. Legacy `TED-NATIONALID` (`ORG_NATIONALID_FIELD`, `project.rs:539`) lands on `mention.raw_identifier`
   (`project.rs:3563`). An eForms BT-501 lands on the same field.
2. `normalise_identifier` (`project.rs:3594`, gated only by `scheme_never_keys`; `DENIED_SCHEMES` is empty) compacts
   `2018/S 025-054890` to `2018S025054890`.
3. The value passes the shape filters (≥4 chars, has a digit, not all-zero, not one repeated char;
   `normalise_identifier_with`, `project.rs:6141`). It matches no register prefix and no VAT country (`20` is not
   one), so it falls to the catch-all `gated(national())` (`project.rs:6295`).
4. `idgate::condemns` had no rule for it. The census scores `DE`/13 digits as scheme `other`, checksum `Unknown`; no
   lexicon, sequence, phone, four-digit or routing hit. So it becomes a merge key, and every notice of the procedure
   binds to one row, whoever the named party is.

## Fix (decided, built)

**Principle: an OJ S publication number identifies a NOTICE, never an organization, in any country or register.** So
this is a condemning class in the plausibility gate, blind to country and kind, and not a publisher-specific pattern.

`idgate::ojs_notice_number(value)`: compact the value (ASCII alphanumerics, upper-cased) and match the whole of it
against `^(19|20)\d\dS\d{7,9}$`. The published spelling is year, `/S`, issue number (1–3 digits, unpadded in r208),
`-`, and a six-digit serial, so the compact form has 7–9 digits after the `S`. Spaces, the separators and a lowercase
`s` all disappear in the compaction. Because the predicate compacts by itself, it gives the same answer for the
published string and the stored key. It becomes `GateCensus::notice_number`, and `condemns` ORs it in.

**Admitted on category, not on fusion rate.** By `condemns`' per-value standard the class runs 8.5 % ≥2 names, worst
30. That is above the 5.8 % / 93 baseline, but not by much. (It is also a different scope: the whole corpus here, while
the baseline was read over `notice_id > 30000000`.) The 425-of-425 resolution to real notices is what settles it. The
doc on `condemns` now records that a category argument needs PROOF that the value names something other than a party,
and records why "it doesn't look like a register number" is not that proof (the GUID class, issue 312).

Deliberately not built:

- **The `NNNNNN-YYYY` publication id.** It is the Swedish orgnr's own shape (measured above), and the gate only ever
  sees it as ten bare digits.
- **The glued and ten-digit residue.** Reaching those means guessing at what else the value contains. This is the same
  reason the routing rule leaves `LEITID` alone.
- **A Deutsche Bahn pattern.** See the open question below.

## Open question — Deutsche Bahn's reference-number family (`YY{T,F,G}E{A,I}nnnnn`)

DB also publishes its own procurement file number as the NATIONALID: `14TEI10487`, `17FEI28567, Los 2`,
`18GEI32660`. Prefix seeks over `05`–`26` × `{T,F,G}E{A,I}` find the following:

- **197 org rows** (196 DE, 1 FR), 85 distinct row names, all DB entities: DB Netz AG 65 rows, Deutsche Bahn AG 21,
  regional DB Netz spellings, DB Station&Service, DB Energie. Procedure titles also appear in the name slot.
- **Per value:** 196 values, 47 (24 %) with ≥2 names, worst **12**, 1,059 mentions.

This class mostly SPLITS DB across roughly 180 procedure-scoped rows; it fuses little. A per-procedure file number is
not an organization identifier either. But no shape states "a procurement file reference" in general. A DB-shaped
pattern would be exactly the publisher-specific bolt-on this issue refuses. Open, not built.

**The principled alternative checked: "a NATIONALID equal to the notice's own published reference is not an org id".**

- **Feasibility:** feasible, and cheap. The mention loop in `project.rs` walks every value of the notice, including
  PROCEDURE's `TED-REFERENCE_NUMBER`, `TED-NO_DOC_OJS` and `TED-NOTICE_NUMBER_OJ` / `TED-REF_NOTICE.NO_DOC_OJS` (BT-22
  and the notice ids in eForms). It could collect those into a set and refuse a NATIONALID whose normalised form is in
  it, in the same `.filter` that applies `scheme_never_keys` (`project.rs:3594`). That needs no DB read and no schema
  change.
- **Measured yield on the exhibits: near zero.**
  - OJS exhibit: 1 of 129 notices cites the number. The NATIONALID is the ORIGINAL notice's number, which later notices
    do not otherwise carry.
  - DB family: **0 of 18** sampled mentions match. On `18GEI32660` ×14 and `17FEI28567, Los 2` ×3 the notice's own
    `TED-REFERENCE_NUMBER` is an OJS number (`2018/S 120-274470`, `2017/S 183-375832`), so DB swapped the two fields.
    On 20542859 it is the other way round (`REFERENCE_NUMBER` `17FEI30205`, NATIONALID the OJS number).
    `14TEI10487`'s notice carries no reference number.
- **Conclusion:** a same-notice comparison cannot see a swap or a cross-notice reuse. What would work is "a NATIONALID
  equal to ANY notice's reference number in the corpus". That is a corpus-level lookup (a `notice_ids` value index over
  non-ref ids), not a projection-time check. Not built; recorded for whoever takes the DB family.

## The repair (nothing new to build)

`repair-placeholder-orgs` works as follows:

- supervisor arm `Spec::RepairPlaceholderOrgs`, `supervisor.rs:4027`
- store `Db::repair_placeholder_orgs_batch` → `dissolve_condemned`, `canonical.rs:20293` / `:20432`

It walks EVERY identifier-bearing org row by PK (`SELECT id, country, identifier_kind, identifier … WHERE id > ? AND
identifier IS NOT NULL`) and calls the injected `ingest::idgate::condemns` on the STORED triple. There is no fixed
placeholder list. So once this deploys it condemns the 425 rows. Each row is dissolved:

- every mention re-resolves through the post-234 provisional path: reuse on `(org_name_norm(name), country)`, else a
  fresh provisional row
- party and bid-party rows follow their mention
- winner rows follow tiers 1–3. Otherwise they are deleted, their tender is stamped epoch-stale and its notices are
  re-queued (tier 5)
- change events are emitted, and a dissolved row leaves the scan, so a rerun is a no-op

The stored identifier is the compact form, which `ojs_notice_number` matches directly.

**Order.** With the gate live, `normalise_identifier` returns `None` for these values, so no fold or re-parse can bind a
mention to a notice-number key again. The 393/397 re-parse chain is therefore safe on either side of this deploy. Run
the dissolve after the deploy. If it lands in the same window as 432, run it after 432's repair so the re-resolve and the
stock agree on `org_name_norm`.

    /root/aj.sh /admin/jobs '{"kind":"repair-placeholder-orgs","dry_run":true}'
    # expect: "… 425 condemned, 425 dissolved, 0 skipped, ~1008 mentions re-resolved"
    #
    # RECONCILE before going wet. Every earlier class was dissolved to 0: 365's re-census after jobs 835-840 read
    # 0 lexicon / 0 sequence / 0 phone / 0 bare-4-digit / 0 routing-scope. So "condemned" should be this class
    # (425 plus any arrivals before the deploy) and nothing else. A much larger number means a rule over-reached:
    # do not go wet, read the plan.
    /root/aj.sh /admin/jobs '{"kind":"repair-placeholder-orgs","dry_run":false}'
    # expect: condemned/dissolved/mentions equal to the dry run. The fresh/reused split differs (a dry-run artifact,
    # 365 "The dissolve, verified").
    /root/aj.sh /admin/jobs '{"kind":"project","rebuild":false}'           # re-derive the stamped tenders
    /root/aj.sh /admin/jobs '{"kind":"backfill-org-name-variants"}'        # the dissolve's own doc: repopulate variants

**Follow-up, not built.** The `org-merge-health` census (`supervisor.rs`, beside `routing_scope`) does not yet count
`notice_number`, so the weekly report cannot show the class going to 0; the Verify below reads it directly instead. It
needs one counter, one JSON key and one format slot. I left it out: changing `supervisor.rs` needs a
`--features server` build, which was outside this unit's disk budget.

## Verify

    printf '%s' "SELECT COUNT(*) FROM generate_series(1990, 2030) y CROSS JOIN organizations o WHERE o.identifier >= y.value || 'S' AND o.identifier < y.value || 'T' AND length(o.identifier) BETWEEN 12 AND 14 AND substr(o.identifier, 6) NOT GLOB '*[^0-9]*'" | ssh -o BatchMode=yes root@zebreus.click 'curl -s --max-time 15 -H "Authorization: Bearer $(cat /root/tender-sql-token)" --data-binary @- https://tenders.zebreus.click/v1/sql'

The plan was read locally with `plan-probe`: `SCAN generate_series AS y` / `SEARCH o USING INDEX
organizations_identifier_id (identifier>=? AND identifier<?)`, one range seek per year. It takes 1.7 s on prod.

- **done**: `"rows":[[0]]`, read after the wet dissolve
- **open**: `"rows":[[425]]` (read 2026-09-27)

## Built (2026-09-27)

Uncommitted, not deployed, not run on prod. Only one file changed: `crates/ingest/src/idgate.rs`. `ops/check.sh` was
not run (disk budget); the focused runs below are green.

- `pub fn ojs_notice_number(value) -> bool`: the compaction, then the whole value must match `YYYY` (`19xx`/`20xx`) +
  `S` + 7–9 digits. Its doc records the three exclusions and the `NNNNNN-YYYY` measurement.
- `GateCensus::notice_number`: set in `census`, blind to country and kind.
- `condemns`: `|| c.notice_number`. Its doc now lists the later classes (units 2-3, 440) and states the category route
  with its proof requirement. The measurement table gains the class's row.
- Test `idgate::tests::an_ojs_notice_number_is_never_an_organization_identifier`:
  - **condemned**, under six country codes: `2018/S 025-054890`, `2018S025054890`, `2018/s 025-054890`,
    `2018 / S 025 - 054890`, `2018/S025-054890`, `2018/S 025 - 054890`, the r208 `2011/S 79-130403` / `2011S79130403`,
    the fixtures' `2010/S 66-098284`, a one-digit issue, the eForms `2025S056090000`, and a 1999 number
  - **end to end through `project::normalise_identifier`**: the exhibit's published NATIONALID (DE) and the r208
    spelling (GB) both return `None`
  - **not condemned**: the measured near misses (ten digits, glued, `…S…` references, a non-TED year, a digit or a
    letter where the `S` belongs), plus a new `REAL_REGISTRANTS` table built from the specimens this module's tests
    already pin: every live checksum PASS, the labelled ids, org 28's key, a GUID, the compound NIP/REGON. It also holds
    the Swedish and Icelandic `NNNNNN-YYYY` registrants, and `556364-2023` must still normalise to `5563642023`.
- **Mutation-checked:** with `|| c.notice_number` removed the new test FAILS (GATE-EXIT=101); restored, it passes.
- **Runs** (`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`):
  - `cargo test -p ingest --lib idgate`: 19 passed, GATE-EXIT=0
  - `cargo test -p ingest --lib`: 276 passed, GATE-EXIT=0
  - no fixture carries an OJS-shaped NATIONALID or CompanyID (grepped), so the integration suites' org keys cannot move
- **Other readers of `condemns`/the normaliser, checked:**
  - the E0/R2/R3 merge arms receive `condemns` injected (`supervisor.rs` ~4867/5059/5294), so they skip the class too
  - the one-shot 325/328/345 re-normalisation repairs return `None` for the class, the same way they already do for
    every earlier condemned class
