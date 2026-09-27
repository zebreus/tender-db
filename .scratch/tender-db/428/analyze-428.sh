#!/bin/bash
# Issue 428: ANALYZE on a reflinked copy of the Sep 20 snapshot. Never the serving DB.
set -u
D=/data/db/analyze-428
P=/root/plan-probe-428
SNAP=/data/db/snapshots/tender-db-1789874587.db
echo "START $(date -u +%FT%TZ)"
for f in base stats; do
  [ -e "$D/$f.db" ] || cp --reflink=always "$SNAP" "$D/$f.db" || { echo "COPY-FAILED $f"; exit 1; }
done
ls -la "$D"
FIRST="tenders lots tender_versions organizations organization_names organization_mentions tender_version_dates tender_version_classifications tender_version_parties tender_version_texts tender_version_amounts tender_version_lots lot_results bids contracts tender_version_bids tender_version_bid_parties tender_version_contracts tender_version_lot_results tender_version_result_winners tender_version_result_stats tender_version_lot_group_members notice_withheld_fields currency_rates quarantine notices"
$P analyze "$D/stats.db" --resume $FIRST
echo "FIRST-DONE $(date -u +%FT%TZ) rc=$?"
$P analyze "$D/stats.db" --resume
echo "ALL-DONE $(date -u +%FT%TZ) rc=$?"
