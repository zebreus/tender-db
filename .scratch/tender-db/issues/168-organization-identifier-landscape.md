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
