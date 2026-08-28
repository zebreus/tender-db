# 307 — organization_names satellite: backfill the standing corpus

Status: CLOSED — RAN ON PROD 2026-08-28 (job 1321): 14,314,913 notices
walked, 40,241,608 mentions visited, **38,706,271 labelled variants
written** into organization_names — far above the eForms-only estimate,
because legacy r208/r209 AA_NAME/ML_TI_DOC buyer-name copies in the kept
languages (EN + original) are labelled and were harvested too. Sample
verified (POL/FIN/FRA/ITA rows on real orgs). Corpus-wide COUNT exceeds the
/v1/sql 10s cap — the satellite is ~38.7M rows; issue 300's matcher has its
multilingual input. Re-run after 304's campaign widens legacy languages.
Kind: one-time backfill walk
Relates to: ADR-0013 D4 (the satellite, built 2026-08-28), 259 (the
mention-idempotency lesson that makes this walk necessary), 300 (the
cross-language matcher this feeds), 304 (labelled legacy variants arrive with
its stage 1 — re-run or extend this walk after that campaign).

## Why a walk at all

The resolver writes satellite rows only on the NEWLY-RECORDED-mention path —
deliberately, because `resolve_mentions`' idempotency map short-circuits
re-folds, and 259 established that the mention layer is refold-invariant.
So from deploy day forward every new notice's labelled name variants land;
the STANDING corpus's eForms notices (multilingual BT-500 confirmed live:
14 multi-lang sections in the newest ~12k notices) stay satellite-less until
walked once.

## Shape

Windowed walk over `organization_mentions` by notice_id (or over
`notice_texts` kind-scoped like the D5 reveal recheck): for each recorded
mention, read the section's labelled `BT-500-Organization-Company` /
`ORG_NAME_FIELD(S)` text variants from `notice_texts`, normalize langs, and
`INSERT OR REPLACE` into `organization_names` keyed by the mention's
organization_id. Idempotent; bounded windows; checkpoint per batch; heavy
write — queue-gated like every backfill. Admin kind suggestion:
`backfill-org-name-variants`.

## Acceptance

- satellite row count > 0 and growing with the walk;
- a known multilingual org (Belgian buyer) shows FRA+NLD rows;
- re-run writes 0 (REPLACE idempotence measured via changes count).
