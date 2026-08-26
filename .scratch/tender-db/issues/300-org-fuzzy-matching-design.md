# 300 — smarter organization matching (the layer CONTEXT.md said "may be layered on later")

Status: BACKLOG (filed 2026-08-26; CONTEXT.md:60-61; spec non-goal "fuzzy org
matching"; prerequisite study DONE — issue 168)
Kind: capability (organization layer quality)
Relates to: 168 (identifier landscape + false-merge bound — the measured input),
234 (exact-identifier + name+country merge, done), 291 (multilingual org names
interact with any matcher).

Exact-identifier merge + the 234 provisional collapse are done; the 168 study
measured the landscape (17.4% ≥2-name upper bound, actionable placeholder class
≤1.8%). The matcher itself — normalization beyond lowercasing, legal-form
stripping, identifier-scheme cross-walks — is unbuilt. Design AFTER the
multilingual org-name satellite (291 step 3): a matcher built on single-language
name_norm would be redone. Bar from CONTEXT.md stands: never merge on name
similarity alone (false merges corrupt; missed merges are recoverable).
