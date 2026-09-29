# 447 — GB company numbers published behind an English field label (`COMPANYNO…`, `COMPANYNUMBER…`) split organizations from their bare twins

Status: done — DEPLOYED 2026-09-29 22:42 UTC (rev `40867f2`), repaired (job 1666) and folded (R2 job 1669). Verify read: 109 → 13.
Built the same day; next is to deploy, run `repair-label-prefixes` dry then wet, then an R2 fold, and read the Verify.
Kind: data quality (organization identity), small
Relates to: 328/359/363/374 (the label-prefix class and its vocabulary), 342 (where the `COMPANY…` rows were first
noticed; they are mostly TED-era, not FTS), 300 (identity gate)

## What is wrong

`ingest::countries::LABEL_PREFIXES` is the vocabulary of publisher field names that get glued in front of an
identifier. It has German, Polish, Italian, Spanish, Danish, French and Finnish labels, but no English ones. A UK
company number published as "Company No. 01628868" is keyed as `COMPANYNO01628868`, not as `01628868`. The same
company published bare gets a second organization row.

Measured on prod 2026-09-29 (bounded reads over the `(country, identifier_kind, identifier)` index, `country='GB'`):

| | rows |
| --- | --- |
| GB national identifiers starting `COMPANY…` or `REGISTEREDCOMPANY…` | 181 |
| … that strip to pure digits under the company-number labels | 121 |
| … whose bare remainder already stands as its own GB national org | **38** (Civica UK `01628868`, `03990481`, `04168336`, …) |

The biggest forms are `COMPANYNO…` (93 by 9-char prefix), `COMPANYNUMBER…` (52), `COMPANYREGISTRATIONNUMBER…` (14),
`COMPANYREGISTRATION(NO)…` (8) and `REGISTEREDCOMPANY(NUMBER|NO)…` (4). Most carriers are UK housing associations,
i.e. TED's UK years rather than FTS.

## Fix (built 2026-09-29)

Eleven English company-number labels join `LABEL_PREFIXES`, in length order. The existing `recognisable` guard in
`project::normalise_identifier_with` does the rest. A stripped remainder is kept only when it is pure digits, so these
stay as published:

- composites such as `COMPANYNO03574882HCANOLH4209CHARITYCOMMISSIONNO1074574`;
- Industrial & Provident society numbers (`COMPANYNOIP28137R`, 36 rows);
- `OC`/`RC`-led Companies House numbers.

**Charity labels are deliberately NOT listed.** `REGISTEREDCHARITYNUMBER…` / `REGISTEREDCHARITYNO…` (14 rows) name the
Charity Commission register. Stripped, a charity number would stand as a bare GB national id indistinguishable from a
company number: a false-merge hazard, not a reunion.

Tests: the strip table in `countries.rs` gains the GB forms, and the bare-label test gains `COMPANYNO` and
`COMPANYNUMBER`. `project.rs` gains `an_english_company_number_label_strips_to_the_bare_number_and_nothing_else_does`:
the labelled and bare forms are one identifier, and the composite, IP and charity forms keep what was published.

## Not this issue

- Zero-padding. `COMPANYNUMBER2296559` strips to `2296559`, and a twin published as `02296559` stays apart. The FTS GB
  arm zero-pads GB-COH scheme values. TED-era bare GB numbers carry no scheme, so padding them would be a guess about
  which register a 7-digit number belongs to. Separate question.
- The `IP…` (Mutuals Public Register) and charity forms: a scheme-aware GB arm for TED-era values, if ever.

## Verify

After deploy and `repair-label-prefixes` wet (and the R2 fold it feeds):

```sh
printf '%s' "SELECT COUNT(*) FROM organizations WHERE country = 'GB' AND identifier_kind = 'national' AND ((identifier >= 'COMPANYNO0' AND identifier < 'COMPANYNO:') OR (identifier >= 'COMPANYNUMBER0' AND identifier < 'COMPANYNUMBER:'))" | /root/sq.sh
```

Today: 109. Expect 13 afterwards, the rows whose remainder is not pure digits: composites such as
`COMPANYNO04302179HCALH4336`, and mutual-society numbers such as `COMPANYNO26971R`.

## 2026-09-29 — run and verified

- `repair-label-prefixes` dry (job 1665) planned 150 rows, 52 landing on an identity that already stands. Wet (1666)
  applied 150, with 0 left standing as published. The 150 are more than the GB-only 121: the same English labels sit on
  45 non-GB rows, all company-register numbers under their own country and all pure digits:
  - IE: 16, CRO numbers;
  - DK: 13, CVR, including two `COMPANYNO…`/`COMPANYREGISTRATIONNO…` pairs of one CVR;
  - SE: 1, an organisation number;
  - NZ: 1.
- R2 dry (1667) planned 35 merges, all read by hand. 34 are one entity: a Danish name beside its English form, or case,
  punctuation and legal-form variants (Civica UK, Aareon UK, HS2, Novogene UK/Europe, Selwood Housing…). **One is not**:
  DK `39314878` would join Energinet Elsystemansvar A/S with Energinet Eltransmission A/S, which are sister companies.
  The second row carries the first one's CVR (published as `COMPANYREGNO39314878`). A `keep` verdict, cohort
  `447-label-strip-2026-09-29`, was recorded via `POST /admin/merge-verdicts`. Its rationale names Eltransmission's own
  CVR as 39314959, which is from memory and not checked against the register. The keep holds on the entity
  difference alone.
- The R2 re-plan (1668) had 34 groups and verdict-keep 193. The wet run (1669) merged 34 groups, removing 38 org rows
  and touching 61 tenders. Project job 1670 followed.
- Verify: the `COMPANYNO<digit>`/`COMPANYNUMBER<digit>` GB range reads **13** (was 109): exactly the composites and
  mutual-society numbers. `01628868` (Civica UK) is one organization.
