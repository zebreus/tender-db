# 173 — takedown/redaction design note + restore the GDPR assessment record

Status: open — research gap #8 (docs/research/research-gaps-2026-08.md); record: NOW (1 hour, needs Lennart), mechanism: conditional
Role: team-lead (the record needs Lennart's lawyer's scope), then run-driver

(a) The lawyer's GDPR assessment is the factual basis of a standing rule
("never re-introduce privacy-driven design") but the research doc was deleted
— the corpus retains no record of what it covered, while the system
demonstrably stores natural-person data (UBOs, contact persons). Restore a
one-page dated scope record. (b) The architecture assumes sources never
retract published notices; D4 re-hash probes and D5 BT-198 reveal rechecks
were specified and are not confirmed running. If any takedown obligation ever
arrives (source-side redaction, court order — independent of GDPR), there is
no mechanism to remove one notice from an append-only versioned store + tar
archive + forever change log. Cheap insurance: a tombstone design note, not
machinery.

2026-08-09 (orchestrator): gap 8b half answered by the DR study
(dr-premise-2026-08.md §6): D4 (package re-hash probes) and D5 (BT-198
reveal recheck) are NOT implemented and NOT scheduled — no job kind, no
timer; D5's script exists only as a research artifact on the VPS outside
the repo (a D7 fragility). D4 is also a DR input (re-fetchability drift is
unmeasured and the original hashes die with the DB). Both are small
scheduled-job features; fold into the next ops batch. Gap 8a (the lawyer
record) remains with Lennart.
