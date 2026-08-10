# Sub-cent published amounts stay quarantined: integer cents is confirmed

Money in the canonical layer is INTEGER cents plus a currency code (CONTEXT.md,
resolved 2026-07-19). The reclaim campaign's final diagnosis (issue 144) found
that decision's edge: a population of real notices whose published amounts carry
sub-cent precision — unit prices, fractional-cent totals — which the parser
correctly refuses as `unrepresentable-value` and ADR-0004 therefore holds whole.
Cause F is the diagnosis's only cross-pipeline cause (TED XML and DÖE zip alike),
point-estimated at 40–50% of the terminal residue: roughly 1.5–1.8k notices, the
dominant slice of the ~3.5k `unrepresentable-value` bucket, out of a 7.9M-tender
corpus.

**Decision: integer cents stands. A sub-cent published amount is unrepresentable
by policy, and a notice carrying one stays quarantined whole — as a *documented
keep*, disclosed on the dashboard (issue 184), never as an undifferentiated
actionable gap.**

Alternatives considered and rejected:

1. **A finer integer unit** (micro-units, or a value+scale pair). Representable,
   but it is a schema and API change over every amount in the corpus, and a
   representation change is an epoch bump: issue 179 measured what that costs —
   a full-corpus rewrite — and every API consumer pays a migration on top. That
   price for ~0.02% of notices is disproportionate. Revisit only if the value
   domain shifts (issue 171 is the watch): the quarantine keeps the raw bytes,
   so this decision is reversible by reclaim at any time.

2. **Rounding to the nearest cent.** Cheapest, and wrong: it introduces values
   the source did not publish, the exact line issue 131 established we do not
   cross. A corpus whose amounts may be silently off by half a cent can no
   longer claim that every canonical value traces to a Notice (ADR-0001), and
   that claim is worth more than the coverage of the notices it costs.

3. **Claim-and-store** (parse the notice, keep the offending amount as a lexical
   string in a side field). Recovers the notices' non-amount content, but splits
   the amounts surface: every consumer would have to know that the amount column
   is authoritative except when a shadow column overrides it. A documented
   absence is a better API than a sometimes-lie, at this scale. If cause F ever
   grows past a nuisance, this is the alternative to reopen first — it is
   cheaper than (1) and honest, just not worth the surface today.

Consequences: issue 184 runs the terminal relabel pass so every keep-class row
carries its true value-domain reason, then writes the dashboard disclosure so
the actionable headline stops counting this decision as a defect. The reclaim
campaign is declared complete in issue 144's sense: the remaining quarantine is
ADR-0004 doing its job, with cause F's share explicitly owned by this ADR.
