# Rubric — does this company number belong to this organization? (issue 452)

Each CASE is one organization in a UK public-procurement database, identified by a Companies House company
number (`number`), which is the org's identity in the database. You see:
- `head`: the org's displayed name, a first-seen election;
- `mention_names`: what procurement notices call the org, with how many mentions each;
- `register`: what Companies House says about `number`. Either the live snapshot (current name, status, previous
  names), the register page (current name, status, previous names), or `http: 404`, meaning no company has ever had
  that number.

`kind` tells you why the case is here:
- `live-mismatch`: a live company whose names match none of the org's;
- `never-issued`: a 404;
- `absent-mismatch`: a dissolved company whose names match none of the org's.

## Verdicts

- `right-number`: the number IS this organization's. The org's names are a trading name, brand, product line,
  rename, acronym, or spelling variant of a register name, or the company has since changed its name. Public bodies
  with a registered company (an NHS trust's company, a council's trading company) count only if the notices name
  THAT company.
- `related-company`: the number belongs to a related legal entity, not to the named organization. A parent group for a
  subsidiary or site (Ramsay Health Care for "Pinehill Hospital"), a subsidiary for its parent, a sister company, or a
  council's arms-length company for the council itself.
- `wrong-number`: the number belongs to someone unrelated, or to no one. The signatures:
  - a 404;
  - a transposed or mistyped number, where you can often name the right one (Harvey Nash 02202746 vs 02202476);
  - an unrelated live or dissolved company (Cheltenham Borough Council under DIGIMUNE LTD);
  - a statutory body (council, NHS trust, university, authority) under a random company's number;
  - a non-company identifier in the field (a charity number, a Dutch KvK, an NHS code).
- `unclear`: the evidence cannot tell. Say what would settle it.

When you can, give `correct_number`: the company number the organization really has. Find it with the search
`https://find-and-update.company-information.service.gov.uk/search/companies?q=<name>`, or `null` for a body with no
company number. You MAY fetch Companies House pages with curl or WebFetch for unclear cases. Keep it to those.

## Confidence

- `high`: plain from the evidence.
- `medium`: probable.
- `low`: a guess.

Give one entry per case (`case` string exactly), each with a two-sentence rationale naming the evidence.
