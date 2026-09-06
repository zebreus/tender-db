# Rubric — one register number, names that share no core word (issue 362)

You are reviewing groups of `organizations` rows from a public-procurement database (EU TED +
national sources). Every row in a CASE carries the SAME validated register number under the
same country and scheme (`PL:nip`, `PL:regon`, `IT:piva`, `PT:nif`, `RO:cui`, `CZ:ico`…), yet
the rows' names share no core word — which is why an automatic fold refused them. Your job per
case: decide whether these rows are ONE organization that should be folded into one row, or
whether one of them carries a number that is not its own.

## What you see per member

- `name`: the organization's name as the database holds it (the most common published spelling).
- `identifier` / `kind`: the register number literally as stored; all members share the key.
- `country`, `provisional`: where the row stands; `provisional` rows were minted from a mention
  without a confirmed identity.
- `mentions`: how many notice mentions stand on the row. A row with 0–2 mentions beside one
  with hundreds is a stray; two rows with hundreds each are two established registrations.

## Verdicts

- `merge` — one legal entity under two spellings. The signatures: a RENAME or successor
  (Dimension Data Polska → NTT Poland; Tönsmeier → PreZero; Alibus International → Nomago
  Italia), an ACRONYM and its expansion (PGK / Przedsiębiorstwo Gospodarki Komunalnej; TAR
  Molise / Tribunale Amministrativo Regionale per il Molise; KOK / Konsorcjum Ochrony Kopalń),
  a TRANSLATION (Bureau for Forest Management and Geodesy / Biuro Urządzania Lasu i Geodezji
  Leśnej), a TYPO or SPACING (KrakTransRem / KrakTansRem; Lore star / Lorestar; Noma 2 / NOMA2),
  a person's business under the person's name and under its trade name (Waterworld Sylwia
  Galjan / Sylwia Galjan prowadząca działalność…), an office and its holder (Gmina Zator /
  Burmistrz Zatora — the same public body), a UNIT of a public body that publishes under the
  body's number (a Polish gmina and its school or sports centre after the 2017 VAT
  centralisation; a city and its district offices), a branch office of the same company.
- `keep` — two DIFFERENT organizations, one of which carries a number that is not its own. The
  signatures: a BUYER and its CONTRACTOR (a county / powiat / gmina / hospital / university /
  ministry beside a company — the buyer's number written into the winner's field), two
  unrelated companies (Sailovnia / Boat Base; Engave / TPM Services; Polbis Auto /
  Fiedorowicz), a company beside a person with a different surname, a consortium row
  ("Konsorcjum", "Lider:", "Członek konsorcjum", "RTI …/…", "mandante") beside one of its
  members or beside an unrelated firm, a parent and a SUBSIDIARY that has its own legal
  personality (Citonet-Kraków Sp. z o.o. beside Toruńskie Zakłady Materiałów Opatrunkowych
  S.A. — separate companies, separate NIPs; one row is wrong), an insurer's branch beside a
  hospital, a state research institute beside a private trader.
- `needs-more-evidence` — the names alone cannot tell (two plausible company names with no
  visible relation, no acronym or rename you can recognise). Say what would settle it.

## Confidence

`high` = you would execute the fold (or the permanent keep) yourself; `medium` = probably,
wants a second reader; `low` = a guess. Only HIGH merges are executed. A HIGH keep denies the
fold for good, so give it only where you are sure the rows are distinct organizations. When in
doubt between merge and keep, keep is the safe answer for the data (a fold cannot be undone by
rule), but say `needs-more-evidence` rather than a confident wrong keep.

Multi-member cases: the verdict covers the whole group — `merge` only if EVERY member is the
same entity; if one member is a stranger among agreeing rows, the verdict is `keep` and the
rationale names the stranger.

Output exactly one entry per case (the `case` string is the key), with a two-sentence
rationale naming the signature you used.
