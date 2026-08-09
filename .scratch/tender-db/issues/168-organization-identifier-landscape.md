# 168 — organization identifier landscape across countries and eras

Status: open — research gap #2 (docs/research/research-gaps-2026-08.md), before launch if Organizations stay a headline feature
Role: run-driver

CONTEXT promises one profile per real-world entity; the only evidence in the
corpus is legacy NATIONALID fill rates (~50%, 16% junk). Unstudied: scheme
attributes and register formats per country (DE HRB/HRA/VR, FR SIREN/SIRET,
...), VAT formats, cross-country string collisions, and the measured
false-merge/false-split rates of the current exact-match auto-merge. Issue 86
(HRB prefix parsed as country code) is the first cost of format naivety.
Study is empirical via the bounded SQL surface (1-2 days) and feeds the B8
normalization design. Mentions are immutable, so recovery from a bad merge is
re-projection — bounded, but the feature's credibility rides on this.

2026-08-09 (orchestrator): first-pass study DONE —
docs/research/data-profile-2026-08.md §1. The merge key's failure classes
are now measured with specimens: fake-country register prefixes >=50,700
org rows lower bound (96.6% of stored HR-vat orgs are German HRB
companies); placeholder VATs merge strangers (DE123456789 = 138 distinct
names in one org); NULL-country national ids merge globally (code
contradicts its own comment); FI/RO/BE lexical variants false-split; the
country column speaks three vocabularies plus NULL. Validation-rule
catalog seed in §3 (rules 1-7 feed B8). Remaining for the full study
(false-merge/split RATES, full fake-country inventory): the snapshot
machine — folded into issue 167's rig authorization.
