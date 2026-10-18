# Plán 084: sumárne PDF exporty, kontrakt a backend

Tento dokument opisuje dátové rozhranie, stránkovanie a literálne oracle hodnoty
dvoch sumárnych PDF: **Sumár na 1 stranu** a **Sumár do 3 strán**. Sumáre idú do
vydania 0.2.0 spolu s plánom 091. Táto časť je len backend (Rust). Výber rozsahu
v dialógu exportu a náhľad strán sú úloha UI a v tomto dokumente sú len ako
dohoda o príkazoch.

Obsahová špecifikácia je v pláne 084, sekcie „Návrh obsahu“ a „Zásady“. Veta o
verzii 0.1.7 v pláne je zastaraná, cieľ je 0.2.0.

## Rozsah dokumentu

`ReportDocumentScope` (serde `snake_case`):

| Hodnota | Dokument |
| --- | --- |
| `full` (predvolená) | Dnešný úplný výpis. Správanie a testy sa nemenia. |
| `summary_one_page` | Sumár, vždy presne 1 strana A4. |
| `summary_three_pages` | Sumár, vždy presne 3 strany A4 (strop je 3). |

Príkazy `preview_pdf_report` a `export_pdf_report` dostanú voliteľný parameter
`scope`. Chýbajúci alebo `null` parameter znamená `full`, takže existujúce
volanie `{ request }` a `{ request, path }` funguje bez zmeny. Odpoveď
`ReportPreview` aj `PdfReportOutcome` má rovnaké kľúče ako doteraz.

Počet strán sumáru je daný rozsahom (1 alebo 3). Úplný výpis počet strán
nepozná vopred, vráti ho až export.

## Dátové rozhranie

Sumár potrebuje všetko, čo má úplný výpis (`ReportSnapshot`), a navyše
`SummaryExtras`. Oboje sa číta v **jednom** databázovom snímku:
`Store::report_summary_snapshot(request, clock)` otvorí jednu transakciu,
zachytí `ReportSnapshot`, potom dopočíta extra údaje a transakciu uzavrie.
Úplný výpis dopočítavanie nespúšťa (`Store::report_snapshot` je nezmenený).

`SummaryExtras`:

| Pole | Obsah |
| --- | --- |
| `days` | Jeden záznam na deň (`date`, `income_cents`, `expense_cents`), len pre obdobie typu mesiac. Inak prázdne. |
| `month_rows` | Riadok na mesiac obdobia: príjmy, výdavky, rozdiel a kumulatívny rozdiel od prvého mesiaca. |
| `account_totals` | Na každý účet výberu: príjmy, výdavky, rozdiel a počet transakcií. |
| `status_counts` | Počet transakcií podľa stavu: potvrdené, navrhnuté, nezaradené, prevod. |
| `largest` | Najviac 10 najväčších transakcií obdobia. |
| `comparison` | `AllTime`, `OutOfRange` alebo `Ready` s porovnaním. |
| `recurring` | Pravidelné platby výberu (odhady z importov). |

### Pravidlá výpočtu

- **Jeden vzorec na jedno číslo.** Príjem, výdavok a prevod sa počítajú funkciou
  `contribution` v `crates/store/src/report/totals.rs`, tou istou ako KPI
  úplného výpisu. Súčty po dňoch, mesiacoch, účtoch aj za predchádzajúce obdobie
  ju používajú priamo. Prevod nie je príjem ani výdavok. Vrátenie platby
  (`refund`) zníži výdavok kategórie.
- **Predchádzajúce obdobie** je bezprostredne predchádzajúci rozsah s rovnakým
  počtom dní, rovnako ako `previousEqualRange` v Prehľade. Nie je zarovnané na
  kalendárny mesiac: február s 28 dňami sa porovná s posledných 28 dňami
  januára (`2026-01-04` až `2026-01-31`). Rozsah sa počíta z dátumov
  `from` a `to` obdobia: `to_prev = from - 1 deň`, `from_prev = to_prev - (dni - 1)`.
  Ak by `from_prev` bol pred rokom 0001, porovnanie je `OutOfRange`.
- **Celé obdobie nemá porovnanie** (`comparison = AllTime`). Sumár to napíše
  vetou, nie prázdnou tabuľkou.
- **Rozdiel** je `teraz - predtým` v centoch. **Percento** je rozdiel delený
  `abs(predtým)`, v desatinách percenta (`458` je `45,8 %`), zaokrúhlené na
  najbližšie, polovica smerom od nuly. Ak je `predtým` nula, percento je
  `None` a sumár napíše „nedostupné“. Appka tak nikdy nezobrazí `0 %` ani
  nekonečno. Prehľad zobrazuje percento na jedno desatinné miesto, sumár tiež.
  Prehľad počíta percento v pohyblivej čiarke, sumár v celých číslach. Líšiť sa
  môžu len prípady presne na polovici desatiny.
- **Kategórie porovnania** sú zjednotenie kategórií oboch období podľa
  `category_id`. Kategória len v jednom období má v druhom nulu. Výdavky bez
  kategórie sú riadok `Nezaradené` s `category_id = None`, rovnako ako
  „Nezaradené výdavky“ v Prehľade. Poradie je podľa absolútneho rozdielu
  zostupne, potom podľa `category_id` (bez kategórie naposledy), potom podľa názvu.
- **Top kategórie.** Kategórie sú zoradené ako v úplnom výpise (absolútna suma
  zostupne). Sumár ukáže najviac 8 riadkov. Zvyšok je jeden riadok
  `Ostatné (N kategórií)`, jeho suma je súčet skrytých kategórií. Súčet
  zobrazených riadkov sa preto rovná KPI výdavkov. Podiel je suma delená KPI
  výdavkov v desatinách percenta. Ak je KPI výdavkov nula alebo záporné, podiel
  je „nedostupné“. Zaokrúhlené podiely sa nemusia sčítať na presne 100,0 %.
- **10 najväčších transakcií** sú transakcie obdobia okrem stavu `Prevod`,
  zoradené podľa absolútnej sumy zostupne, potom dátum zostupne, potom `id`
  zostupne. Prevody sa vynechávajú, lebo sú aj v KPI mimo príjmov a výdavkov.
  Je to rozhodnutie implementácie, špecifikácia prevody nespomína.
- **Kumulatívny stĺpec** je priebežný súčet rozdielu mesiacov od prvého mesiaca
  obdobia. Keď tabuľka skráti staršie mesiace, kumulatívna hodnota zobrazených
  riadkov ostáva pravdivá (zahŕňa skryté mesiace).
- **Pravidelné platby** sú výstup `recurring_overview` pre obdobie správy
  (rovnaké okno ako Prehľad), zúžený na účty výberu. Súčty `confirmed`,
  `estimates` a `excluded` sa dopočítajú z tých istých riadkov rovnakou
  funkciou. Zobrazia sa riadky v stave aktívna alebo blížiaca sa, ktoré nie
  sú ignorované. Všetko je označené ako odhad z importov.

### Neúplné pokrytie

- Sumár nikdy netvrdí úplnosť transakcií. Pevný riadok na prvej strane to
  hovorí: súčty vychádzajú zo zaznamenaných transakcií.
- Hlavička ukáže upozornenie, keď obdobie ešte neskončilo (`unfinished_period`)
  a keď niektorý účet nemá výpisy, má medzery, neoverený kontrolný súčet alebo
  neplatný rozsah výpisu.
- Porovnanie má vlastné upozornenia. Obdobie, ktoré neskončilo, dáva predbežné
  porovnanie. Predchádzajúce obdobie sa kontroluje rovnakou funkciou pokrytia
  ako aktuálne a sumár uvedie počet účtov bez výpisov a s medzerami v ňom.
- Chýbajúci výpis sa nepočíta ako nulová transakcia. Čísla ostávajú také, aké
  dávajú zaznamenané transakcie, a upozornenie ich opatrí.

## Stránkovanie

Sumáre nikdy nepokračujú na ďalšiu stranu. Každý blok má pevný strop riadkov a
nadbytok sa napíše vetou „a ďalších N“. Text používateľa (názvy účtov,
obchodníkov, kategórií) je v tabuľkách vždy na jednom riadku a pri nedostatku
miesta sa skráti s `…`. Znaky, ktoré font nepozná, ostávajú viditeľné ako
`[U+XXXX]` a ich počet je v poznámke dole na poslednej strane.

| Blok | Strop | Pri prekročení |
| --- | --- | --- |
| Zoznam účtov v hlavičke | 3 účty | `a ďalších N účtov` |
| Top kategórie (strana 1) | 8 + Ostatné | riadok Ostatné |
| Rozdiel po kategóriách (strana 2) | 12 riadkov | `a ďalších N kategórií` |
| Všetky kategórie s podielom (strana 2) | 22 riadkov | `a ďalších N kategórií` a ich súčet |
| Súčty po účtoch (strana 3) | 6 účtov | `a ďalších N účtov` |
| Vývoj po mesiacoch (strana 3) | 12 najnovších mesiacov | `a ďalších N skorších mesiacov` |
| 10 najväčších transakcií (strana 3) | 10 | pevný |
| Pravidelné platby (strana 3) | 6 riadkov | `a ďalších N` |

Graf na prvej strane má pevnú výšku a ukáže najviac 36 posledných stĺpcových dvojíc (inak to napíše pod grafom). Pre obdobie typu mesiac sú stĺpce po
dňoch (`days`), inak po mesiacoch zo `chart_buckets` (najviac 24 mesiacov,
inak ročné súčty, rovnako ako v úplnom výpise). Štítky osi sa vynechávajú, aby
sa nepretínali.

Obsah strán:

1. **Strana 1** (obe sumáre): hlavička s presným obdobím, výberom účtov, časom
   zachytenia a upozorneniami. Štyri KPI: príjmy, výdavky, rozdiel,
   prevody (odchádzajúce a prichádzajúce). Graf vývoja. Top kategórie
   výdavkov. Riadok pokrytia: počet výpisov, medzery, nesediace alebo
   neoveriteľné súčty a počet transakcií podľa stavu (potvrdené, navrhnuté, nezaradené a prevody, aby súčet sedel na počet transakcií).
2. **Strana 2** (len tri strany): porovnanie príjmov, výdavkov a rozdielu s
   predchádzajúcim obdobím, rozdiel po kategóriách v centoch a percentách,
   tabuľka všetkých kategórií s podielom.
3. **Strana 3** (len tri strany): súčty po účtoch, tabuľka vývoja po mesiacoch
   s kumulatívnym stĺpcom, 10 najväčších transakcií, stav pravidelných platieb.

Sumár do 3 strán má vždy 3 strany, aj keď je obsah krátky alebo prázdny.
Prázdny blok napíše vetu („Bez transakcií.“). Počet strán sa tak nemení podľa
dát a náhľad ho môže ukázať vopred.

Existujúce limity ostávajú: najviac 100 000 transakcií, 32 MiB textu, 2 000
strán, 100 MiB súboru, žiadna sieť, publikovanie bez prepísania existujúceho
súboru (`persist_noclobber`). Predchádzajúce obdobie sa číta s rovnakými
limitmi ako aktuálne.

## Syntetický profil a oracle hodnoty

Profil je v `crates/store/tests/report_summary.rs`. Dátum „dnes“ v teste je
`2026-09-07`. Účet 1 `Osobný` (osobný, výpis `2026-06-01` až `2026-08-31`),
účet 2 `Firemný` (firemný, výpis `2026-08-01` až `2026-08-31`). Kategórie:
výdavkové 901 Potraviny, 902 Bývanie, 903 Doprava, 904 Zábava, 905 Zdravie,
906 Oblečenie, 907 Cestovanie, 908 Vzdelanie, 909 Darčeky, 910 Poistenie a
príjmová 911 Plat.

### August 2026 (obdobie `2026-08`)

Súčty: príjmy `350 000`, výdavky `132 500`, rozdiel `217 500`, prevody von
`30 000` a dnu `30 000` (všetky sumy v tejto časti sú v centoch, PDF ukáže eurá, napr. `350 000` je `3 500,00 €`). Transakcií je 19: potvrdené 15, navrhnuté 1,
nezaradené 1, prevod 2.

Kategórie výdavkov (sumy v centoch, poradie ako v sumári):
Bývanie `80 000`, Potraviny `18 000` (po vrátení `2 000`), Cestovanie `9 000`,
Zábava `8 500`, Doprava `5 000`, Nezaradené `3 500`, Zdravie `3 000`,
Oblečenie `2 500`. Do `Ostatné (3 kategórií)` patria Vzdelanie `1 500`,
Darčeky `1 000` a Poistenie `500`, spolu `3 000`. Súčet top 8 `129 500` plus
`Ostatné` `3 000` sa rovná `132 500`. Podiely v desatinách percenta:
`604, 136, 68, 64, 38, 26, 23, 19` a `Ostatné 23` (súčet `1 001`, rozdiel je
zaokrúhlenie).

Predchádzajúce obdobie `2026-07-01` až `2026-07-31`: príjmy `240 000`, výdavky
`93 500`, rozdiel `146 500`.

| Ukazovateľ | Teraz | Predtým | Rozdiel | Percento |
| --- | ---: | ---: | ---: | ---: |
| Príjmy | 350 000 | 240 000 | +110 000 | 45,8 % |
| Výdavky | 132 500 | 93 500 | +39 000 | 41,7 % |
| Rozdiel | 217 500 | 146 500 | +71 000 | 48,5 % |

Rozdiel po kategóriách (teraz, predtým, rozdiel, desatiny percenta):
Bývanie `80 000, 60 000, +20 000, 333`; Potraviny `18 000, 10 000, +8 000, 800`;
Zábava `8 500, 3 000, +5 500, 1 833`; Zdravie `3 000, 0, +3 000, nedostupné`;
Cestovanie `9 000, 12 000, -3 000, -250`; Oblečenie `2 500, 0, +2 500, nedostupné`;
Nezaradené `3 500, 1 000, +2 500, 2 500`; Doprava `5 000, 7 000, -2 000, -286`;
Vzdelanie `1 500, 0, +1 500, nedostupné`; Darčeky `1 000, 0, +1 000, nedostupné`;
Poistenie `500, 500, 0, 0`.

Pokrytie predchádzajúceho obdobia: účet 2 nemá v júli výpis, takže 1 účet bez
výpisov a 1 účet s medzerou.

Súčty po účtoch: Osobný príjmy `250 000`, výdavky `112 500`, rozdiel `137 500`,
16 transakcií. Firemný príjmy `100 000`, výdavky `20 000`, rozdiel `80 000`,
3 transakcie.

10 najväčších (id, centy): `1: +250 000`, `17: +100 000`, `4: -60 000`,
`18: -20 000`, `2: -12 000`, `9: -9 000`, `3: -8 000`, `5: -5 000`,
`16: -4 500`, `6: -4 000`. Prevod `-30 000` sa nezaradí.

Dni: 31 záznamov. `2026-08-01` výdavok `500`, `2026-08-02` príjem `250 000`,
`2026-08-26` (prevod) nula, `2026-08-27` (vrátenie) výdavok `-2 000`,
`2026-08-31` výdavok `4 500`. Súčet príjmov `350 000`, výdavkov `132 500`.

### Iné obdobia toho istého profilu

- **Šesť mesiacov do `2026-08`** (`2026-03-01` až `2026-08-31`, 184 dní):
  predchádzajúce obdobie `2025-08-29` až `2026-02-28`. Tabuľka mesiacov
  (príjmy, výdavky, rozdiel, kumulatívne): `2026-03` až `2026-05` samé nuly,
  `2026-06` `0, 999, -999, -999`, `2026-07` `240 000, 93 500, 146 500, 145 501`,
  `2026-08` `350 000, 132 500, 217 500, 363 001`. Príjmy za obdobie `590 000`
  proti nule, percento nedostupné. Pokrytie predchádzajúceho obdobia: 2 účty
  bez výpisov, 2 s medzerou.
- **Celé obdobie** má porovnanie `AllTime` a mesiace `2026-06` až `2026-09`
  s kumulatívnym rozdielom `-999, 145 501, 363 001, 362 890`.
- **Mesiac `2026-09`** (neskončil): predchádzajúce `2026-08-02` až `2026-08-31`
  (30 dní). Príjmy `0` proti `350 000` (`-350 000`, `-100,0 %`), výdavky `111`
  proti `132 000` (`-131 889`, `-99,9 %`), rozdiel `-111` proti `218 000`
  (`-218 111`, `-100,1 %`).
- **Február 2026** s riadkami `2026-01-03` (mimo), `01-04` a `01-31` (vnútri):
  predchádzajúce `2026-01-04` až `2026-01-31`, výdavky `100` proti `500`,
  `-400`, `-80,0 %`.
- **Rok 0001, január**: predchádzajúce obdobie by začínalo pred rokom 0001,
  porovnanie je `OutOfRange`.

### Počet strán

- Sumár na 1 stranu má presne 1 stranu pre mesiac, šesť mesiacov, rok aj celé
  obdobie na zaneprázdnenom profile (viac účtov, viac než 24 mesiacov, veľa
  kategórií, dlhé názvy, nepodporované znaky, neúplné pokrytie).
- Sumár do 3 strán má na tom istom profile 3 strany.

## Rozšírenie rozhrania (inventár)

- `store::report`: `ReportDay`, `ReportMonthRow`, `ReportAccountTotal`,
  `ReportStatusCounts`, `ReportDelta`, `ReportCategoryDelta`, `ReportPrevious`,
  `ReportComparison`, `ReportRecurring`, `SummaryExtras`, `SummarySnapshot`.
- `store::report`: `previous_range`, `ratio_tenths`, `LARGEST_TRANSACTION_COUNT`.
- `Store::report_summary_snapshot` a skrytý `report_summary_snapshot_with_hook`.
- `abakus_lib::report`: `ReportDocumentScope`, `SummaryLength`,
  `write_summary_pdf_report`, `render_summary_pdf_to`.
- Príkazy `preview_pdf_report` a `export_pdf_report`: voliteľný parameter `scope`.
- PDF metadáta `Title` sumárov: „Abakus · Sumár“. Úplný výpis ostáva
  „Abakus · Prehľad transakcií“.
