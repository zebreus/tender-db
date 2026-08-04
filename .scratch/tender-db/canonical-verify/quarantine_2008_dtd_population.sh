#!/usr/bin/env bash
# ============================================================================
# quarantine_2008_dtd_population.sh — issue 84's falsifier.
#
# THE QUESTION. 594,915 notices are held with reason `unparsable-xml`, detail
# `XML with DTD detected`; 593,017 of them are ted/monthly/2008. I filed that as
# the largest remaining reclaim opportunity (~4.8% of the corpus). I now believe
# it is the opposite — that those rows are the NON-ENGLISH duplicate siblings of
# the 26,948 English 2008 notices already reclaimed, i.e. a counting defect and
# not lost data. The mechanism is proven in code (see the issue's Correction, and
# `reclaim_accounts_for_held_members_a_dispatch_policy_skips`). The POPULATION is
# not: that rests on the ratio 593,017 / 26,948 = 22.0, and a ratio is not a
# population. This script is what turns it into one, or kills it.
#
# THE PREDICTION, stated before the run so this is a falsification and not a
# fishing trip. If the correction is right:
#   A. ~0 held rows have an `.en` member file; the rest spread over ~21 languages.
#   B. For held non-`.en` rows, the sibling notice `<doc>-<year>` EXISTS and is
#      `parsed` — the notice is already in the corpus, in English.
# If A shows a material `.en` share, or B shows the siblings are absent, then the
# correction is WRONG, those members really did fail to ingest, and issue 84
# stands as originally filed. Either outcome is a result; only running it is not.
#
# READ-ONLY, but it reads DATA PAGES (unlike hot_read_plans.sh, which is
# metadata-only). It must NOT be pointed at the serving database: a full scan of
# `quarantine` on the live file competes with the writer for the single disk.
# Point it at a checkpointed snapshot. A non-empty `-wal` is refused, because
# stock sqlite3 cannot read a turso-written WAL and would silently report a
# partial picture.
#
# USAGE: quarantine_2008_dtd_population.sh /data/db/snapshots/tender-db-<ts>.db
# ============================================================================
set -euo pipefail

DB="${1:-}"
if [[ -z "$DB" ]]; then
  echo "usage: $0 <snapshot.db>" >&2
  exit 2
fi
if [[ ! -f "$DB" ]]; then
  echo "FAIL: no such snapshot: $DB" >&2
  exit 2
fi
if [[ -s "$DB-wal" ]]; then
  echo "FAIL: $DB has a non-empty -wal. stock sqlite3 cannot read a turso WAL;" >&2
  echo "      the read would omit whatever lives only in WAL frames. Use a" >&2
  echo "      checkpointed snapshot." >&2
  exit 2
fi

# A snapshot is a photograph. Say what is being read and how old it is, so a
# reader never has to infer which corpus state these numbers describe.
echo "input:    $DB"
echo "taken:    $(date -r "$DB" '+%Y-%m-%d %H:%M:%S %Z')"
echo "age:      $(( ( $(date +%s) - $(date -r "$DB" +%s) ) / 3600 )) h"
echo "size:     $(du -h "$DB" | cut -f1)"
echo

sqlite3 -readonly "file:$DB?immutable=1" <<'SQL'
.mode box
.headers on

-- The bucket under test: still-held rows carrying the stale pre-issue-36 reason.
CREATE TEMP VIEW held AS
  SELECT q.member_path,
         -- the part after the last '.' — the language for this era's files
         lower(replace(q.member_path,
                       rtrim(q.member_path, replace(q.member_path, '.', '')), '')) AS lang
    FROM quarantine q
    JOIN fetches f ON f.id = q.fetch_id
   WHERE q.reason = 'unparsable-xml'
     AND q.detail = 'XML with DTD detected'
     AND q.reprocessed_at IS NULL
     AND f.source = 'ted' AND f.kind = 'monthly' AND f.period LIKE '2008%';

SELECT '--- A. held rows by member-file language (predict: ~0 en) ---' AS "";
SELECT lang, COUNT(*) AS held,
       ROUND(100.0 * COUNT(*) / (SELECT COUNT(*) FROM held), 2) AS pct
  FROM held GROUP BY lang ORDER BY held DESC;

SELECT '--- A2. the headline number, and the en share that decides it ---' AS "";
SELECT (SELECT COUNT(*) FROM held)                        AS held_total,
       (SELECT COUNT(*) FROM held WHERE lang = 'en')      AS held_en,
       (SELECT COUNT(DISTINCT lang) FROM held)            AS languages;

-- B. Does the English notice for each held sibling already exist, parsed?
-- `…/115165_2008.fr` -> publication id `115165-2008`.
CREATE TEMP VIEW held_sib AS
  SELECT lang,
         replace(
           substr(replace(member_path,
                          rtrim(member_path, replace(member_path, '/', '')), ''),
                  1,
                  length(replace(member_path,
                                 rtrim(member_path, replace(member_path, '/', '')), '')) - 3),
           '_', '-') AS publication_id
    FROM held WHERE lang <> 'en';

SELECT '--- B. sample of 5000 held non-en rows: is the notice already in? ---' AS "";
WITH s AS (SELECT * FROM held_sib LIMIT 5000)
SELECT CASE WHEN n.id IS NULL THEN 'NO notice row (would be a real gap)'
            WHEN n.parse_state = 'parsed' THEN 'notice present and parsed'
            ELSE 'notice present, state ' || n.parse_state END AS verdict,
       COUNT(*) AS rows
  FROM s LEFT JOIN notices n ON n.publication_id = s.publication_id
 GROUP BY verdict ORDER BY rows DESC;

SELECT '--- C. control: the reclaimed 2008 rows, same cut (predict: ~all en) ---' AS "";
SELECT lower(replace(q.member_path,
                     rtrim(q.member_path, replace(q.member_path, '.', '')), '')) AS lang,
       COUNT(*) AS reclaimed
  FROM quarantine q JOIN fetches f ON f.id = q.fetch_id
 WHERE q.reason = 'unparsable-xml' AND q.detail = 'XML with DTD detected'
   AND q.reprocessed_at IS NOT NULL
   AND f.source = 'ted' AND f.kind = 'monthly' AND f.period LIKE '2008%'
 GROUP BY lang ORDER BY reclaimed DESC;
SQL

echo
echo "READ: A ~0 en + B 'notice present and parsed' => the held rows are duplicate"
echo "      siblings; the outstanding count overstates the gap and no reclaim can"
echo "      move them. A material en share, or B 'NO notice row', => the correction"
echo "      is wrong and issue 84 stands as filed."
