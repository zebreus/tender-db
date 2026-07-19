# Survey: German state & commercial procurement portals as tender-db Sources

Sub-survey feeding docs/research/german-portals.md. All hands-on checks done
2026-07-19 via curl/WebFetch. Baseline for comparison verified first:
**oeffentlichevergabe.de** bulk export works unauthenticated —
`https://oeffentlichevergabe.de/api/notice-exports?pubDay=2026-07-17&format=ocds.zip`
returned HTTP 200, ~2.0 MB; historical days work back to at least
`pubDay=2023-11-02` (200, ~956 KB). Formats eForms-DE/OCDS/CSV, license CC Zero
per BeschA
(https://www.bescha.bund.de/DE/ElektronischerEinkauf/Datenservice_Oeffentlicher_Einkauf/Bekanntmachungsservice/OpenData-Schnittstelle_im_Bekanntmachungsservice/OpenData-Schnittstelle_im_Bekanntmachungsservice_node.html,
swagger at
https://www.oeffentlichevergabe.de/documentation/swagger-ui/opendata/index.html).

## Cosinex ecosystem — DTVP, VMP NRW, Metropole Ruhr, Brandenburg, Niedersachsen, RLP

One software family (cosinex "Vergabemarktplatz"), so one verdict covers all.

1. **Origin, not mirror.** Notices originate here; since eForms-DE cosinex
   pushes to the Bekanntmachungsservice/Datenservice Öffentlicher Einkauf
   (https://blog.cosinex.de/2023/02/03/datenservice-oeffentlicher-einkauf-buendelt-vergabeverfahren-von-bund-laendern-und-kommunen/),
   i.e. their content lands in the federal feed.
2. **Public search without account: yes** (verified HTML 200): DTVP
   `https://www.dtvp.de/Center/company/announcements/categoryOverview.do?method=show`,
   NRW
   `https://www.evergabe.nrw.de/VMPCenter/company/announcements/categoryOverview.do?method=show`
   (200, 63 KB), Brandenburg
   `https://vergabemarktplatz.brandenburg.de/VMPCenter/...` (200), RLP
   `https://www.vergabe.rlp.de/VMPCenter/`, Niedersachsen
   `https://vergabe.niedersachsen.de/Satellite/`, Metropole Ruhr at
   `https://www.vergabe.metropoleruhr.de/VMPSatellite` (bare hostname fails
   TLS SNI). **Machine-readable: weak.** The only JSON interface is the
   "TIS-Schnittstelle" (JSON via URL), but it must be activated **per
   contracting authority** and is capped/configurable — not a platform-wide
   feed
   (https://blog.cosinex.de/2024/03/14/tis-schnittstelle-bekanntmachungen-auf-der-eigenen-homepage/).
   An "Open Data" interface since VMP 7.5 covers only "currently published"
   notices and has no public docs
   (https://blog.cosinex.de/2018/09/26/die-neue-vergabemarktplatz-version-7-5/).
   Search is a stateful ISO-8859-1 JSP app (sort/pagination via POST).
3. **History: effectively none.** Search offers only running notices + recent
   awards; no archive UI, no date-range into the past. An ascending-sort
   attempt only surfaced same-day notices. Confidence that expired notices
   vanish: medium-high (consistent with the "currently published" open-data
   wording); exact retention unverified.
4. **Terms/robots hostile-ish:** DTVP app pages carry `meta robots noindex`;
   Brandenburg robots.txt is `Disallow: /` for all agents. No reuse license
   found.

**Verdict: not a serious Source.** Origin data, but no bulk access, no
history, and their notices reach oeffentlichevergabe.de anyway.

## Sachsen — evergabe.sachsen.de

Origin platform ("NetServer" app). Public search without login verified:
`https://www.evergabe.sachsen.de/NetServer/PublicationSearchControllerServlet?function=SearchPublications&Gesetzesgrundlage=All&Category=InvitationToTender`
(200, 209 KB HTML). Award listing (`Category=ContractAward`) shows oldest
entries from **06.05.2026** — i.e. roughly a 10-week retention window, no
archive. No API/RSS found; robots.txt is 404. **Verdict: no** — same pattern
as cosinex, worse tooling.

## Bayern — auftraege.bayern.de / vergabe.bayern.de

`https://www.auftraege.bayern.de/` is now a static Bavaria-branded landing
page for **Deutsche eVergabe (Healy Hudson)** — "BayVeBe"
(https://service-vst.deutsche-evergabe.de/kb/a123/bayvebe-bayerische-vergabe-und-bekanntmachungsplattform.aspx);
it currently even links to `test.deutsche-evergabe.de` (mid-migration
sloppiness). robots.txt 404; the dashboard path 404s and
`www.deutsche-evergabe.de` fails TLS verification from here.
`https://www.vergabe.bayern.de/` is a separate RIB platform for construction.
**Verdict: no** — unstable, couldn't even verify a working public search (low
confidence on Deutsche eVergabe's search; unverified).

## Berlin — berlin.de/vergabeplattform / "Meine Vergabeplattform"

`https://www.berlin.de/vergabeplattform/` is an info CMS site; the actual
platform `https://my.vergabeplattform.berlin.de/` is a **RIB eVergabe login
page** (verified title "RIB eVergabe Login"). The site's RSS
(`https://www.berlin.de/vergabeplattform/index.php/rss`) contains only nav
items, not tenders. Found no anonymous notice search. **Verdict: no** (fails
the no-sign-in criterion for anything useful).

## Hamburg / NRW portal / others

`https://fbhh-evergabe.web.hamburg.de/` responds 200 but is a JS app with no
crawlable content (not investigated further — low confidence).
`https://www.vergabe.nrw.de/` is just a Drupal info portal in front of the
cosinex VMP above. `https://www.evergabe.nrw.de/robots.txt` oddly serves a
maintenance page.

## evergabe-online.de (e-Vergabe des Bundes, Beschaffungsamt BMI)

Origin platform for federal buyers; pushes to TED/service.bund.de. Public
search **works without login** (verified:
`https://www.evergabe-online.de/search.html` → 200, 202 KB, "Ausschreibungen
suchen - e-Vergabe, die Vergabeplattform des Bundes") but requires a cookie
handshake; it's an Apache-Wicket session app; no RSS at any obvious path
(`/rss`, `/rss.xml`, etc. all 404). **Verdict: redundant** — its notices are a
subset of what oeffentlichevergabe.de/service.bund.de already carry, with
worse access.

## Commercial platforms (classification)

- **subreport ELViS** (`https://www.subreport-elvis.de/` → 302 to
  `/login.html`): **origin** platform, but search requires sign-in. Fails
  hard criteria.
- **evergabe.de** (evergabe.de GmbH, Dresden): **hybrid origin + aggregator**;
  open SEO listing pages (`https://www.evergabe.de/auftraege` 200 without
  login), even an `llms.txt`, but documents/details behind account;
  commercial terms. Not a Source; TED/BKMS covers its origin content.
- **Vergabe24** (`https://www.vergabe24.de/`, Staatsanzeiger BW):
  **aggregator**, paywalled subscription; marketing site on WordPress. No.
- **ausschreibungen-deutschland.de**: **aggregator/TED mirror** — notice pages
  are verbatim TED content ("See the notice on TED website", "© Europäische
  Union, ted.europa.eu" in footer; verified on notice 2517049). Free and
  ad-financed, BUT history is not durable: a 2012 notice URL from their own
  robots.txt now 404s, and pagination beyond the recent window returns empty
  pages. Zero advantage over TED itself.
- **ibau / bi-medien**: **aggregators**, fully paywalled B2B subscription
  services. Classify-only; no.

## Ranked shortlist

1. **oeffentlichevergabe.de (Bekanntmachungsservice / Datenservice
   Öffentlicher Einkauf)** — the only German source with verified no-auth bulk
   machine-readable access (eForms-DE + OCDS + CSV per publication day), CC
   Zero, and durable retention back to the eForms-DE start (verified to Nov
   2023). Since all the state platforms above push into it, it *subsumes*
   them for post-2023 data. This is the German Source to build; TED covers
   the pre-2023 EU-threshold history.
2. **service.bund.de** — complement for below-threshold national notices not
   in TED; not re-verified in this sub-survey (medium confidence).
3. **Nothing else qualifies.** Every state portal fails the long-history
   criterion (notices vanish at deadline; awards retained ~2-3 months,
   verified for Sachsen), and none offers a platform-wide machine-readable
   feed (cosinex TIS is per-authority opt-in). Commercial portals are either
   login-walled (subreport, Vergabe24, ibau/bi) or TED mirrors without
   durable history (ausschreibungen-deutschland.de).

**Bottom line:** no German state or commercial portal beats or meaningfully
complements the federal options as a Source. Recommended German Source #2
after TED: the oeffentlichevergabe.de OpenData export (day-wise OCDS/eForms
zips, CC Zero, no auth). Main unverified points: exact cosinex retention
policy, Deutsche eVergabe/Bayern public search (TLS-blocked), and Hamburg's
JS portal.
