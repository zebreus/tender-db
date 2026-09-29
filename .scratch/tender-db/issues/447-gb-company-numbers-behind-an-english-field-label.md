# 447 — GB company numbers published behind an English field label (`COMPANYNO…`, `COMPANYNUMBER…`) split organizations from their bare twins

Status: in-progress — filed 2026-09-29 (hourly check-in audit, from issue 342's "filed as noticed" `COMPANY…` note).
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
