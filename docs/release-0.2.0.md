# Abakus 0.2.0

Rozsah: plán 091 (deväť pozorovaní Michala po dvoch dňoch s 0.1.6 plus dvanásť prijatých návrhov štyroch poskytovateľov). Body 1 až 5 a 7 až 21 sú v aplikácii; bod 6 (kontrola závislostí inštalátorom) má hotový zdroj a natívne prijatie na strane používateľa.

## Čo sa zmenilo

Abakus 0.2.0 mení dennú prácu s kategóriami a výpismi.

- Transakcie: potvrdenie odhadu ponúkne aj podobné platby rovnakého obchodníka, výber sa dá potvrdiť jedným klikom, posledné hromadné priradenie sa dá vrátiť tlačidlom Späť a tabuľka sa ovláda šípkami a klávesom Enter. Nezaradené platby sú zoskupené podľa obchodníka navrchu tabuľky. Vyhľadávanie spája obchodníka s miestom (`Penny Neuss` nenájde `Penny Berlin`).
- Kategórie: výber kategórie má vyhľadávanie podľa názvu (víťaz trojmodelového UI pilotu, xAI kandidát), podkategória sa dá vytvoriť priamo pri priraďovaní, archivácia je ikona, zmazanie je červená ikona s presným počtom presunutých transakcií. Naučené pravidlá sa dajú zmeniť priamo v tabuľke a pred zmazaním pravidla appka ukáže počet dotknutých otvorených riadkov.
- Poplatky sú samostatný druh: filter Druh ich ponúka ako Poplatok, Prehľad má dlaždicu Poplatky a vlastný stĺpec v grafe.
- Prehľad: pripomienka chýbajúceho výpisu s odkazom na Import (medzera v pokrytí výpismi, nie dôkaz chýbajúcich transakcií), klik na obchodníka otvorí filtrované Transakcie, karta porovnania s rovnakým obdobím vlani (mesiac alebo tri mesiace, oba rozsahy viditeľné).
- Import: úplný zoznam výpisov so súčtom, kontrolným súčtom, zobrazením a zmazaním; staršie výpisy sú zbalené. Stav kontroly výpisu ukáže otvorené body a nikdy netvrdí, že bankové dáta sú úplné.
- Nastavenia: voľba kontroly aktualizácií sa pamätá cez reštart a známa nová verzia sa ukáže globálne navrchu appky; sieťový denník sa zbalí po desiatich riadkoch; obnova databázy zo zálohy s náhľadom obsahu, povinnou bezpečnostnou kópiou a dvojstupňovým rollbackom. Heslá zo Správcu poverení v zálohe nie sú.

Podrobnosti a poctivé limity sú v README (sekcie Transakcie, Kategórie, Prehľad, Import, Nastavenia, Obnova zo zálohy) a v KNOWN_ISSUES.md.

## Overenie

- Každá dávka mala nezávislé cross-provider zdrojové overenie (Grok 4.6); obnova zo zálohy prešla dvoma adversariálnymi kolami zameranými na dátovú bezpečnosť.
- Spoločný integračný sweep celej vetvy (p3) a jeho dva nálezy (obnova stavu obrazoviek po restore, zastarané vety v docs) sú uzavreté v tejto vetve.
- Lokálne brány na vydávanom commite `fb2d7c3` (2026-09-14, log `p4-controller-gates-fb2d7c3.log` v ANIMUS artefaktoch plánu 091): `cargo test --workspace --jobs 4` 49 testových blokov, 0 zlyhaní; `cargo clippy --workspace --all-targets` bez varovaní; `npx vitest run --exclude '**/.worktrees/**'` 52 súborov, 475 testov, všetky prešli; `tsc --noEmit` a `eslint src` bez nálezov. Repo nemá CI workflow; tieto lokálne brány sú bránou vydania.
- Inštalátor `abakus-setup-0.2.0.exe` zostavený z tohto commitu cez `packaging\build-installer.ps1` (Inno Setup, x64 PE a PDFium pin overené skriptom `check-payload.mjs`): SHA-256 `13f48cb56f96669276c1bb919585688794b89dbff4b0ef779c0ff6239d3be4ca`; balený `abakus.exe` `da4b30f0deeeea3cdc0f158a10e3720f0c872b0b5fd82188437884e5e6bf6b2e`, `pdfium.dll` `215c405d13963395b38872a656a18f161a294e80bbcaa00849b842f524e5474b`. Riadok pre `SHA256SUMS.txt` vydania je v `packaging/output/SHA256SUMS.txt`.
- Natívne prijatie inštalátora a kontrola obrazoviek prebiehajú na strane používateľa pred tagom.
