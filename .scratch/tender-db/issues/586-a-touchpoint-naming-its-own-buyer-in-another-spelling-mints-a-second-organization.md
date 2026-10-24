# 586 — a touchpoint naming its own buyer in another spelling mints a second organization, and contact labels mint hub organizations

Status: **DONE 2026-10-24** — U2 deployed (deploy #3 6cb4408); U3 drained (refold-sections 2768 + project 2769: 101,191 aliased — 4,866 by spelling, 28,891 by organ — 448,163 minted, 36,745 dropped; 9,160 Tenders corrected; 23,495 mentions retired; sweep 2772 swept 952 orphan orgs). Verify PASS: 540072 serves buyer and Lot-ReviewOrg as one org [22198704]; 31974991 "Dirección Principal" and the other unit-4 hub/label orgs and the SARGA duplicate 31975096 answer 404. Salted 30 aliased (586-u3-, 7 tier A incl. SARGA + 23 tier C): 30 R / 0 W. Was: ready-for-agent — **unit 3 DRAINED 2026-10-24 with 576 U5 (deploy #3 6cb4408; refold-sections 2768 + project 2769, 17:11→18:04Z)**: touchpoint roles 101,191 aliased (586: 4,866 by spelling, 28,891 by organ; was 67,434), 448,163 minted (was 491,287), 36,745 dropped (was 27,378); 9,160 Tenders corrected; 23,495 mentions retired; sweep-orphan-orgs 2772 queued. NEXT: read-back wf_0fa63db1-03e (Verify 540072 → [22198704], 31974991 0/404 after the sweep, salted 30 aliased with a 0-W bar), then DONE. Was: ready-for-agent — **unit 1 DECIDED 2026-10-24 (wf_e9baaa20-02f; `586-u1/decision.md`)**: D586-1 alias fold, compared only with the touchpoint's own parent ORG in the same notice — tier A (folded spelling: NFKD marks, punctuation, brackets with an allow-list on the touchpoint side, legal forms, articles) 182 of the 3,751 new orgs / 743 mentions; tier C (entity under its organ, town hall, bare town; gmina/miasto excluded; one-token only under a legal form; no cut to a whole government) 685 / 2,614; C4 a bare own organ/town-hall word. D586-2 touchpoint-only gate (whole labels, honorifics, prose, legal refs with a statute cue, platforms, bare organ/registry labels): 92 orgs / 3,491 mentions. Every salted 40 (designer + adversarial check, fresh salts) 0 W; R3 top 60 0 hits. NEXT: unit 2 build (new `touchpoint_names_586.rs`, project.rs, role_census `has_public_stem`, counter split aliased_a/aliased_c), ride deploy #3; unit 3 shares 576 U5's refold-sections Touchpoint run. Was: ready-for-agent — filed 2026-10-24 from the hourly AUDIT of 576 U3's fan-out (wf_b3035c50-bef: a measurer, two
opposed skeptics, a verdict; `.scratch/tender-db/audit-576-fanout/verdict.md`, `measure.md`). No flood (3,751 new orgs,
0 exact duplicates, 89.7% of minted mentions bound existing rows), but two defects no existing job will undo. Next:
unit 1, decide the alias fold tiers and the gate lists from the saved answers (no box reads). Unit 3's drain shares 576
U5's `refold-sections` Touchpoint run if it lands in the same deploy.
Kind: data quality / organizations — D1 duplicate provisional profiles of the buyer (N, permanent); D2 label, person and
prose organizations, three of which conflate unrelated buyers (W)
Relates to: 576 (D576-1 (i) alias and (iv) gate; U5 is the TenderNed half of D2), 300 §10, 351, 234, 432 (none folds these
rows), 456 (each party keeps its published name), 510/511/520/525 (drop classes), 585 (the ORG-path label rows).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/540072 | jq -c '[.parties[] | select(.role=="Procedure-Buyer" or .role=="Lot-ReviewOrg") | .organization_id] | unique'

- **open (2026-10-24 13:20Z)**: `[22198704,31975096]` — "Sociedad Aragonesa de Gestión Agroambiental, S.L.U. (SARGA)"
  as buyer and "Sociedad Aragonesa de Gestión Agroambiental S.L.U" (127 mentions) as its review touchpoint.
- **done**: `[22198704]`, and `curl -s …/v1/organizations/31974991 | jq .mentions` ("Dirección Principal", 420 while open)
  answers 0 or 404.

## What was found (verdict.md, numbers that survived both skeptics)

- **Size of the mint.** 3,751 new orgs (ids 31,974,972–31,978,722; 3,745 from 2759), all provisional, identifier-less,
  with a country; every mention on them is a touchpoint mention (24,664). ~233k mentions minted, 10.3% to new orgs.
- **D1, self-duplicates (N).** D576-1 (i) aliases only when the touchpoint equals its parent under `org_name_norm`
  (case, whitespace, trailing `.,;`). Tier A (equal after folding accents, punctuation, brackets, legal forms): 74–107
  new orgs, 414–529 mentions — certain. Tier C (the entity under its organ, a bare town): 622–646. Self-shaped in all:
  1,381 (36.8%) by classifier; a salted hand read of 40 says 60% SELF. Plus ~25k mentions bound to older identifier-less
  rows naming the parent (e.g. 1217875 "Ayuntamiento de Alcobendas" while the buyer is identified 22179642). One
  Unicode miss: Gmina Panki ×2 differing by U+037E vs ';'.
- **D2, gate gap (W pocket).** 9 label orgs hold 2,926 mentions — hubs: 31975008 "Principal" (715, ~4 bodies), 31974991
  "Dirección Principal" (420, 4), 31975059 "Palacio Provincial" (139, 2); single-publisher: "Sede Tibidabo" 621, "Info
  Adicional" 294, "RECURSOS" 268, "Dirección Postal" 221, "Dirección Recepción de Recursos" 143, "Dirección Recurso" 105.
  1689043 "Ayuntamiento" gained 125 mentions from 26 parents. People ("Monsieur Anthony LINTILHAC", "DOTT. ANGELO
  GAMBARDELLA", Michaela Cechova…), prose ("Procede recurso especial…", "Ver apartado 17…" ×6), platforms ("TUTTOGARE
  RACCONIGI", "Plataforma Electrónica de Contratación de la Autoridad Portuaria de Barcelona").
- **Not defects:** wrong countries (6 orgs, 7 mentions, all the publisher's own BT-514); higher mention counts on the
  tribunal rows (intended); spelling variants of one tribunal (300 §10 accepts them).
- **Permanent:** 351's fold needs a NULL country; 432 finds name_norm correct; 234 uses the same key; `match_norm` folds
  only Greek tonos and an N2 match never merges.

## Proposed units

1. **Measure and decide (S)** over the saved answers (`audit-576-fanout/refute/raw/04`, `second-look/raw/02`,
   `second-look/raw/t2_classified.json`), no box reads:
   - (a) an alias fold that compares a touchpoint only with its own parent ORG in the same notice — tier A (NFKC + marks,
     case, all punctuation incl. U+037E, brackets incl. "(Punto de contacto)", legal forms, articles) and tier C (parent
     minus a leading organ phrase; "Ayuntamiento/Ajuntament/Concello de <place>"; the bare place). `org_name_norm`
     stays at spelling (canonical.rs:7432: the cross-notice reuse arm has no wall). Salted 40 per tier, 0-W bar: no
     review organ ("TARCAS. AYUNTAMIENTO DE SEVILLA") and no distinct department may alias.
   - (b) gate stems: the 9 labels, "Ayuntamiento" alone, "Procurement Department"; honorifics (Monsieur, Madame, Dott.,
     Herr, Frau, Mr, Mrs, Ing. …); prose ("procede recurso", "ver apartado", "en los lugares establecidos"); platforms
     (TUTTOGARE, "Plataforma Electrónica de Contratación"). Bar: drops no class-X tribunal and no body in R3's top 60.
2. **Build (M)** in project.rs `touchpoint_decision` / `touchpoint_gate`, tests: SARGA aliases; AVRA ×5 alias; Gmina
   Panki (U+037E) aliases; TARCAS still mints; "Stadt Fürth - Zentrale Vergabestelle" still mints; "Dirección
   Principal" and "Monsieur Anthony LINTILHAC" drop; the unit-3 re-mint drain test holds. Split the aliased counter by
   tier.
3. **Drain (M):** `refold-sections` Touchpoint (198,857 carriers, ~45 min), shared with 576 U5's when deployed together.
   Confirm orphaned new orgs are retired by `retire_mentions`, else sweep. Then Verify and a salted read.
