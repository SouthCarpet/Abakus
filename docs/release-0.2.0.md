# Abakus 0.2.0

Rozsah: plán 091 (deväť pozorovaní Michala po dvoch dňoch s 0.1.6 plus dvanásť prijatých návrhov štyroch poskytovateľov). Body 1 až 5 a 7 až 21 sú v aplikácii; bod 6 (kontrola závislostí inštalátorom) má hotový zdroj a natívne prijatie na strane používateľa.

## Čo sa zmenilo

Abakus 0.2.0 mení dennú prácu s kategóriami a výpismi.

- Transakcie: potvrdenie odhadu ponúkne aj podobné platby rovnakého obchodníka, výber sa dá potvrdiť jedným klikom, posledné hromadné priradenie sa dá vrátiť tlačidlom Späť a tabuľka sa ovláda šípkami a klávesom Enter. Nezaradené platby sú zoskupené podľa obchodníka navrchu tabuľky. Vyhľadávanie spája obchodníka s miestom (`Penny Neuss` nenájde `Penny Berlin`).
- Kategórie: výber kategórie má vyhľadávanie podľa názvu (víťaz trojmodelového UI pilotu, xAI kandidát), podkategória sa dá vytvoriť priamo pri priraďovaní, archivácia je ikona, zmazanie je ikona a potvrdenie s presným počtom presunutých transakcií. Naučené pravidlá sa dajú zmeniť priamo v tabuľke a pred zmazaním pravidla appka ukáže počet dotknutých otvorených riadkov.
- Poplatky sú samostatný druh: filter Druh ich ponúka ako Poplatok, Prehľad má riadok Poplatky v súhrnnej tabuľke a vlastný stĺpec v grafe.
- Prehľad: pripomienka chýbajúceho výpisu s odkazom na Import (medzera v pokrytí výpismi, nie dôkaz chýbajúcich transakcií), klik na obchodníka otvorí filtrované Transakcie, karta porovnania s rovnakým obdobím vlani (mesiac alebo tri mesiace, oba rozsahy viditeľné).
- Import: úplný zoznam výpisov so súčtom, kontrolným súčtom, zobrazením a zmazaním; staršie výpisy sú zbalené. Stav kontroly výpisu ukáže otvorené body a nikdy netvrdí, že bankové dáta sú úplné.
- Nastavenia: voľba kontroly aktualizácií sa pamätá cez reštart a známa nová verzia sa ukáže ako tichý riadok v hlavičke appky; sieťový denník sa zbalí po desiatich riadkoch; obnova databázy zo zálohy s náhľadom obsahu, povinnou bezpečnostnou kópiou a dvojstupňovým rollbackom. Heslá zo Správcu poverení v zálohe nie sú.

Podrobnosti a poctivé limity sú v README (sekcie Transakcie, Kategórie, Prehľad, Import, Nastavenia, Obnova zo zálohy) a v KNOWN_ISSUES.md.

## Kaliber revízia 4 (plán 091)

- Kaliber r4 (Fira Sans): jadro (tokeny, `kaliber.css`, písmo) je prevzaté z r4. Prehľad má súčty ako knihu (Čisté vedúci riadok, poplatky a prevody pod ňou) a jeden riadok s platbami čakajúcimi na zaradenie. Hlavička každej obrazovky má názov a jeden popis. Počty v dialógoch Kategórií a Importu sa skloňujú (1 transakcia, 2 transakcie, 5 transakcií). V okne do 599 px je dolná navigácia. Tlačidlá v Nastaveniach sa neroztiahnu cez celú šírku. Transakcie majú popis s obdobím a účtom, nad tabuľkou len krátku nápovedu ku klávesom (podrobnosti a postup Zaradiť podľa obchodníka sú v Pomocníkovi), políčka v riadkoch s klikateľnou plochou 44 × 44 px na telefóne a výber kategórie so šípkou rozbalenia. Grafy majú jeden rámik, nie rámik v rámiku.
- Jazyk dizajnu Kaliber, revízia 4 (pravidlá R12 až R22 z r3 platia ďalej): `src/tokens.css` a `src/kaliber.css` sú bajtové kópie `design/web`, písmo Fira Sans, vlastné triedy appky s predponou `ab-` v `src/abakus.css`.
- Päť záložiek v hlavičke, Transakcie ako prvá obrazovka, hlavička obrazovky s jednou hlavnou akciou a menu **Ďalšie**, Pomocník v pätičke a na F1, hlásenia vpravo dole so **Späť**.
- Vzhľad v kľúči `kaliber-theme` (prenos z `abakus.theme`), prepínač slnko a mesiac v hlavičke a voľba v **Nastavenia > Vzhľad**.
- **O aplikácii** ako posledná sekcia Nastavení. Kontrola aktualizácií len so súhlasom: pri spustení (ticho), pri otvorení O aplikácii a na tlačidlo. `check_update_now` vracia chybu spojenia ako chybu, nie ako „nič novšie“.
- Klávesy a výber podľa `docs/plan-091-kaliber-r3.md`: Ctrl+O, Ctrl+čiarka, Ctrl+F, Ctrl+Z, Ctrl+Y a Ctrl+Shift+Z, Del s potvrdením, model výberu v tabuľke a vrstvy Esc.
- Syntetický akceptačný prípad Penny Neuss (`fixtures/synthetic/penny-neuss-2026-07.txt`, `crates/store/tests/penny_neuss.rs`).

## Overenie

- Každá dávka mala nezávislé cross-provider zdrojové overenie (Grok 4.6); obnova zo zálohy prešla dvoma adversariálnymi kolami zameranými na dátovú bezpečnosť.
- Spoločný integračný sweep celej vetvy (p3) a jeho dva nálezy (obnova stavu obrazoviek po restore, zastarané vety v docs) sú uzavreté v tejto vetve.
- Lokálne brány na vydávanom commite `188e67d` (2026-10-18, čistý worktree, logy `abakus-gate-cargo-test.log`, `abakus-gate-clippy.log` a `abakus-020-release.md` v ANIMUS artefaktoch plánu 127): `cargo test --workspace --jobs 4` 52 testových blokov, 662 testov, 0 zlyhaní; `cargo clippy --workspace --all-targets -- -D warnings` bez varovaní; `npx vitest run --exclude '**/.worktrees/**'` 67 súborov, 622 testov, všetky prešli; `tsc -b`, `eslint src` a `slop_lint.py src` bez nálezov. Skoršie kandidáty existovali; posledný pred Kaliber r4 bol `fb2d7c3` (2026-09-14). Repo nemá CI workflow; tieto lokálne brány sú bránou vydania.
- Inštalátor `abakus-setup-0.2.0.exe` (10 253 853 B) zostavený z commitu `188e67d` cez `packaginguild-installer.ps1` (Inno Setup, x64 PE a PDFium pin overené skriptom `check-payload.mjs`): SHA-256 `36976698e226fbd36746c295e130840f65bb1bb7534cb57815a34d12a56e8834`; balený `abakus.exe` `47dd76b33d021d4fd5a869e9bcffa98edb68278e3e37fc5901d0cd4da03f7341`, `pdfium.dll` `215c405d13963395b38872a656a18f161a294e80bbcaa00849b842f524e5474b`. Riadok `SHA256SUMS.txt` je publikovaný s vydaním na GitHub.
- Natívne prijatie inštalátora a kontrola obrazoviek prebehnú na strane používateľa po vydaní (vlastník to schválil 2026-10-14), prípadne cez aktualizáciu v appke.

## Doplnené po kandidátovi

Sumárne PDF exporty (plán 084) idú do 0.2.0. Backend (dáta, stránkovanie, príkazy so `scope`) je v `docs/plan-084-summary-pdf.md`, výber rozsahu (**Rozsah dokumentu**: Úplný výpis, Sumár na 1 stranu, Sumár do 3 strán) je v dialógu exportu. PDF v metadátach uvádza verziu aplikácie (`Abakus 0.2.0`).
