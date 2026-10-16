# 552 — a head whose only defect is a line break is swapped for another spelling instead of being served collapsed

Status: needs-triage — filed 2026-10-16 from issue 524's v10 re-judge (plan 2543, wf_65a1fd84-adb). Next: decide
whether the head election should collapse a control-whitespace head in place (rule R0-like) before it competes with
the other spellings, then rebuild the `unfit` release class on that.
Kind: data quality (served organization names; issue 524's head election)
Relates to:
- 524 (rule v10's wet runs exclude the `unfit` junk releases because of this class: `exclude_released: ["unfit"]`);
- 548 (the head refresh's over-blocking classes);
- 520 / 526 (name cleanup precedents).

## Verify

    for o in 22289866 22661109 22737299; do curl -s https://tenders.zebreus.click/v1/organizations/$o | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['id'], repr(d['name']))"; done

- **done**: each serves its head's own words with the line break collapsed to one space, e.g. 22661109
  `Szkoła Podstawowa nr 13 z Oddziałami Dwujęzycznymi im. Jana III Sobieskiego w Tychach`. That is not a shorter
  sibling spelling, and not the head with a raw `\n`.
- **open (2026-10-16)**: each serves its head with a literal line break. 524 v10 excludes their junk release, so
  the planned changes, which would serve worse spellings, do not ship.

## What was found

Plan 2543 released 102 `unfit` heads from the recency holds. 81 of them are unfit only through control whitespace
(a `\n`, `\r` or `\t` inside the name). In every one, the change serves the elected cluster's spelling, never the
head collapsed (0 of 81). The re-judge read 101 unfit releases: 91 right, 5 neutral, 5 wrong. All five wrongs are
in the control-whitespace group:
- 22289866: `Vergabekammer Rheinland-Pfalz\nbei dem Ministerium …` → `Vergabekammer Ministerium für …`, losing the
  Land.
- 22545991: a ministry service `… (ΥΠΑΑΤ)\nΕιδική Υπηρεσία Διαχείρισης ΣΣ ΚΑΠ` → its bare acronym `ΕΥΔ ΣΣ ΚΑΠ`.
- 22661109: a school's full name → the name without its patron and town.
- 22718921: a company's name → an over-long legal-title spelling with quotes and a `διακριτικό τίτλο` tail.
- 22737299: an address-tailed head → a truncated `BPB Büro für Planung und Bauüberwac - im Verkehrswesen GmbH`.

The other 76 control-whitespace releases read right or neutral. Their elected spelling was as good as the head or
better, so the collapsed head would usually tie.

## Proposed units

1. **Measure (S).** On plan 2543's planned and held lists, for every control-whitespace head:
   - compare the collapsed head against the elected spelling;
   - count where they are equal up to case, punctuation and legal form;
   - count where the collapsed head is longer, with the elected spelling a prefix or subset of its words (the loss
     shape);
   - count where the head carries an address or extra block after the break (`… GmbH\nHohenzollernstr. 18\nD-56068
     KOBLENZ`).
2. **Decide (S).** Options:
   - (a) collapse a control-whitespace head in place (R0-style, always) and only then elect;
   - (b) keep the election, but serve the collapsed head when the elected spelling drops head words;
   - (c) split the head at the break and keep the first line when the rest is an address block.

   Each needs a rule for the address-tail case.
3. **Build** as a 524 epoch. Re-plan, then re-judge the control-whitespace releases (all 81) before lifting
   `exclude_released: ["unfit"]`.
