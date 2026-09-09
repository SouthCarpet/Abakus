<div align="center">

# Abakus

Lokálna desktopová appka na PDF výpisy z Tatra banky.

[Releases](https://github.com/SouthCarpet/Abakus/releases) · [Changelog](./CHANGELOG.md) · [Bezpečnosť](./SECURITY.md) · [Známe obmedzenia](./KNOWN_ISSUES.md)

[![Latest release](https://img.shields.io/github/v/release/SouthCarpet/Abakus?label=release)](https://github.com/SouthCarpet/Abakus/releases/latest)
[![License: AGPL-3.0-or-later](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue.svg)](./LICENSE)

</div>

## About

Abakus is a local Windows desktop app for Tatra banka PDF statements. It imports statements, checks checksums, assigns categories with learned rules, tracks recurring payments, stores notes, and builds PDF reports with charts. Bank data stays on your computer. The only network use is an opt-in update check and a user-triggered installer download from GitHub Releases. Both are written to the in-app network log. There is no telemetry.

Version 0.1.4 is the first release with the in-app updater, the AGPL-3.0 licence and the public documentation. Source is at [https://github.com/SouthCarpet/Abakus](https://github.com/SouthCarpet/Abakus). The license is GNU AGPL-3.0-or-later. Install the per-user Windows installer from GitHub Releases. Administrator rights are not required.

## Čo to je

Abakus číta mesačné PDF výpisy z Tatra banky. Rozdelí transakcie do kategórií. Ukáže príjmy, výdavky, pravidelné platby a pokrytie výpismi. Údaje zostanú na vašom počítači. GitHub obsahuje zdrojový kód a inštalátory, nie vaše výpisy.

## Stav

**0.1.4.** Toto je prvé vydanie s aktualizáciou v aplikácii, licenciou AGPL-3.0 a verejnou dokumentáciou. Appka beží na Windows. Zostava je Tauri 2, Rust a React. Zoznam zmien je v [CHANGELOG.md](CHANGELOG.md). Obmedzenia sú v [KNOWN_ISSUES.md](KNOWN_ISSUES.md).

## Prečo

- Bankové PDF spracujete offline.
- Kontrolný súčet výpisu ukáže, či počiatočný zostatok a transakcie dávajú konečný zostatok.
- Kategórie sa učia z potvrdených priradení, nie z odoslania dát do cloudu.
- Pravidelné platby, poznámky, porovnanie kategórií a PDF report ostávajú lokálne.
- Sieť je predvolene vypnutá. Zapnete ju vy.

## Stiahnutie a inštalácia

1. Otvorte [stránku vydaní](https://github.com/SouthCarpet/Abakus/releases/latest).
2. Stiahnite `abakus-setup-<verzia>.exe`. Inštalátor je Inno Setup pre aktuálneho používateľa. Práva správcu nie sú potrebné.
3. Spustite inštalátor. Windows SmartScreen môže varovať, lebo súbor nie je podpísaný. Ak dôverujete zdroju, zvoľte **Ďalšie informácie** a **Spustiť napriek tomu**.
4. Appka potrebuje Microsoft Edge WebView2 Runtime. Inštalátor 0.1.6 ho vie skontrolovať a ponúkne stiahnutie, ak chýba. Windows 10 (2004 a novší) a Windows 11 ho často už majú.
5. Appka sa nainštaluje do `%LOCALAPPDATA%\Programs\Abakus`. Databáza je `%LOCALAPPDATA%\Abakus\abakus.db`.

Ak Abakus už máte nainštalovaný, druhé spustenie inštalátora ho aktualizuje na mieste. Sprievodca povie, akú verziu má nainštalovanú a na akú ju aktualizuje. Dáta a heslá zostanú.

Ďalšie podrobnosti o zostavení inštalátora sú v [packaging/INSTALL.md](packaging/INSTALL.md).

## Ako to funguje

1. V **Nastaveniach** pridajte účty. Zadajte IBAN alebo zápis `kód banky/prefix-číslo účtu`.
2. V **Import** pridajte mesačné PDF. Pri zamknutom PDF zadajte heslo. Voľba **Zapamätať pre tento účet** uloží heslo do Správcu poverení systému Windows. Heslo môžete nastaviť alebo zmeniť aj v Nastaveniach, bez importu. Ak výpis patrí účtu, ktorý appka ešte nepozná, otvorí sa hneď dialóg na jeho nastavenie s druhom účtu a prípadným heslom. Nastavenia zostávajú druhou cestou, ako účet pridať aj bez importu.
3. V **Transakciách** skontrolujte navrhnuté kategórie. Odhad potvrdíte jedným kliknutím.
4. V **Prehľade** uvidíte príjmy, výdavky, kategórie, pravidelné platby a vývoj v čase.

Graf **Podľa kategórií** zachováva znamienko súčtu. Záporné hodnoty vľavo znižujú výdavky. Kladné vpravo ich zvyšujú. Kliknutie na kategóriu otvorí príslušné transakcie. Tabuľky v menšom okne umožňujú vodorovné posúvanie. Suma a mena ostávajú spolu.

Každý importovaný výpis má odznak kontrolného súčtu. Appka overí, či počiatočný zostatok a všetky transakcie dávajú konečný zostatok. Text **Kontrolný súčet nesedí** znamená, že sa tieto hodnoty líšia o uvedenú sumu. Výpis sa importuje. Ostane označený na kontrolu. Ak chýba potrebný zostatok, appka uvedie, že súčet nevie overiť.

Databáza je `%LOCALAPPDATA%\Abakus\abakus.db`. V Nastaveniach vytvorte **Zálohu databázy** a vyberte nový súbor. Appka uloží konzistentnú kópiu aj počas behu. Existujúci cieľový súbor neprepíše. Záloha obsahuje účty, výpisy, kategórie, naučené pravidlá, poznámky, voľby pravidelných platieb a nastavenia databázy. Nie je šifrovaná. Neobsahuje heslá zo Správcu poverení ani pôvodné PDF. Obnova zo zálohy zatiaľ nemá ovládanie v appke.

## Ako sa appka učí

Vytvorenie kategórie ani podkategórie samo nevytvorí pravidlo. Po priradení kategórie transakcii alebo potvrdení návrhu si appka zapamätá rozpoznaného obchodníka a miesto. Presná zhoda sa priradí automaticky. Rovnakého obchodníka na inom mieste označí ako **odhad**. Ten potvrdíte jedným kliknutím. Platby bez rozpoznaného obchodníka sa takto neučia ako jedna spoločná skupina.

Voľba **Použiť aj na podobné** zaradí podobné už importované transakcie, ktoré ešte nie sú potvrdené. Potvrdené priradenia ponechá. V detaile transakcie možno zmeniť cieľ pôvodného vstavaného pravidla, ak ju zaradilo také pravidlo. Samostatný formulár na ručné vytváranie pravidiel zatiaľ nie je dostupný.

## Pravidelné platby a kategórie

V **Prehľade** nájdete pravidelné výdavky aj príjmy s mesačným, štvrťročným alebo ročným opakovaním. Odhady sú oddelené od platieb, ktoré potvrdíte. Detail ukáže konkrétne platby, ďalší očakávaný termín a prípadnú ustálenú zmenu ceny. Výpočty rozlišujú pôvodnú menu a prepočet do eur.

Platbu možno potvrdiť, ignorovať alebo obnoviť jej automatický odhad. V detaile transakcie možno určiť opakovanie aj ručne. Pri ručnom výbere konkrétnych platieb treba ďalšie platby priraďovať ručne. Chýbajúce výpisy vedú k neznámemu stavu. Odhad ukončenia neznamená potvrdené zrušenie zmluvy. Historické obdobie zobrazuje stav k jeho koncu, bez neskorších údajov.

Panel ukazuje mesačný alebo ročný prepočet a očakávané platby do konca mesiaca. Podiel na výdavkoch je dostupný iba pri dostatočnom overenom pokrytí všetkých vybraných účtov. Tieto hodnoty sú odhady z importovaných údajov. Nie sú bankové príkazy ani aktuálny zostatok.

Kategóriu možno vytvoriť priamo pri priraďovaní transakcie. V **Kategóriách** ju možno presunúť pod inú nadradenú kategóriu alebo osamostatniť. Kategórie majú najviac dve úrovne. Podkategória preberá druh príjem alebo výdavok od rodiča. Zmena druhu s dopadom na staršie transakcie vyžaduje potvrdenie zobrazených počtov. Spotify má vlastnú podkategóriu. Úprava starých údajov zachováva používateľské zmeny pravidiel.

## PDF s grafmi a výpisom

Na každej obrazovke je pri voľbe vzhľadu tlačidlo **Exportovať PDF**. Vyberte účty a obdobie:

- **Mesiac:** celý vybraný kalendárny mesiac.
- **Šesť mesiacov:** šesť po sebe idúcich mesiacov vrátane vybraného koncového mesiaca.
- **Rok:** celý vybraný kalendárny rok.
- **Celé obdobie:** všetky uložené transakcie vybraných účtov.

Náhľad ukáže rozsah a počet transakcií. PDF obsahuje súhrn, grafy, informáciu o pokrytí výpismi a úplný výpis vrátane poznámok. Použije všetky transakcie vo vybranom období a účtoch. Filtre tabuľky sa nepoužijú. Aktuálne obdobie je označené ako neukončené. Údaje sa zachytia pri uložení. Ich počet sa preto môže od náhľadu líšiť.

Report má svetlé strany A4, vložené písmo so slovenčinou a číslovanie strán. Export pracuje lokálne. Vyberte nový súbor PDF. Existujúci súbor sa neprepíše. Zrušenie dialógu nič nevytvorí. Pri prekročení limitu reportu sa zobrazí chyba. Výpis sa potichu neskráti.

## Pokrytie, porovnanie a historické zostatky

V **Prehľade** panel pokrytia uvedie medzery vo výpisoch pre jednotlivé účty. Pri vybranom období kontroluje aj jeho začiatok a koniec. Pri voľbe **Všetko** ukáže iba medzery medzi známymi výpismi. Prekrývajúce sa výpisy spočíta ako jedno pokryté obdobie. Ide o obdobia výpisov. Dátumy jednotlivých transakcií môžu byť mimo nich.

Porovnanie kategórií ukáže výdavky vo vybranom období a v bezprostredne predchádzajúcom období s rovnakým počtom dní. Oba rozsahy sú uvedené v paneli. Refundácie znižujú výdavky. Interné prevody sa nezapočítavajú. Pri nulovom základe percentuálna zmena nie je dostupná. Chýbajúce výpisy a budúce dni sú označené. Pri voľbe **Všetko** sa porovnanie nezobrazuje.

Historické konečné zostatky sú hodnoty z výpisov ku konkrétnemu dňu. Nie sú aktuálnym zostatkom bankového účtu. Chýbajúci zostatok zostáva neznámy. Kontrolný súčet ukazuje stav overenia výpisu.

## Poznámky k transakciám

V detaile transakcie možno uložiť poznámku do 2 000 znakov vrátane nových riadkov. Prázdny text poznámku odstráni. Poznámky sa dajú vyhľadávať aj bez diakritiky. Exportujú sa v stĺpci **poznamka** v CSV. Uloženie poznámky nemení kategóriu ani naučené pravidlo.

## Odstránenie účtu a vzhľad

V **Nastaveniach** kliknite v riadku účtu na **Zmazať účet**. Náhľad uvedie názov a počty výpisov, transakcií a pravidiel. Na potvrdenie napíšte presný názov účtu a kliknite na **Natrvalo zmazať účet**. Zrušenie nič neodstráni. Odstránenie importovaných údajov je nezvratné. Pôvodné bankové PDF súbory zostanú na disku. Prípadné zlyhanie databázy alebo odstránenia hesla sa zobrazí v dialógu. Obnovenie zobrazenia po úspešnom odstránení neopakuje odstránenie.

V riadku účtu je aj tlačidlo **Nastaviť heslo** (alebo **Zmeniť heslo**, ak je už uložené). Otvorí dialóg s dvomi poľami: heslo a jeho zopakovanie. Pri každom poli je tlačidlo **Zobraziť**, ktoré heslo odkryje a zostane funkčné aj po kliknutí mimo poľa. **Uložiť** je dostupné, až keď sa obe zhodujú a nie sú prázdne. Po úspechu appka zobrazí potvrdenie a heslo uloží do Správcu poverení systému Windows. Riadok potom ukáže **heslo uložené**. Pri zlyhaní appka zachová rozpísané heslo a zobrazí dôvod.

Výber **Vzhľad** je dostupný na každej obrazovke: **Svetlý**, **Tmavý**, **Podľa systému**. Prvé dve možnosti ignorujú zmeny systému. Tretia ich sleduje. Voľba sa uloží pre ďalšie spustenie. Pri poškodenom alebo nedostupnom úložisku sa použije systémový režim. Pri zlyhaní zápisu platí nová voľba len v aktuálnom behu.

## Filtre a CSV

- **Exportovať filtrované CSV** v Transakciách exportuje aktuálne obdobie, účet, druh účtu, kategóriu, stav, hľadaný text aj obmedzenie na konkrétny výpis. Použije text viditeľný pri kliknutí, aj keď tabuľka ešte čaká na dokončenie hľadania. Filter sa zachytí pri kliknutí. CSV číta údaje z databázy pri exporte. Zrušenie výberu súboru nič neexportuje. Zlyhanie zápisu sa zobrazí. Pôvodný export v Nastaveniach naďalej používa iba uložené obdobie.
- **Filter kategórie** umožňuje vybrať aj nadradenú kategóriu s podkategóriami a zobrazuje kategóriu prevzatú z grafu. Archivovaný alebo nedostupný výber zostáva označený. Neznamená Všetky kategórie.
- **Vymazať všetky filtre** zobrazí všetky obdobia, účty, druhy, kategórie a stavy bez vyhľadávania a bez obmedzenia na výpis. Vymaže aj hromadný výber. Obdobie Všetko sa uloží pre ďalšie obrazovky.
- Panel **Súčty zobrazených transakcií** počíta presne načítané riadky tabuľky. Príjem, výdavky a čistá suma sa sčítajú v celých centoch podľa rovnakých pravidiel ako Prehľad, vrátane druhu priradenej kategórie a refundácií znižujúcich výdavky. Interné prevody majú samostatný počet a do týchto súm nevstupujú. Pri načítavaní alebo chybe sa staré súčty nezobrazujú. Nejde o zostatok bankového účtu.

Vlastné obdobie potrebuje dva platné dátumy so začiatkom najneskôr v deň konca. Kým nie sú platné, dotaz a export sa nespustia. Prechod z Importu na konkrétny výpis zobrazí všetky dátumy tohto výpisu bez obmedzenia predtým uloženým obdobím. Počet výpisov v Nastaveniach má limit 100 000 načítaných výpisov. Tabuľka transakcií a jej súčty pracujú s celým výsledkom dotazu v pamäti.

## Aktualizácia

Kontrola aktualizácií je predvolene vypnutá. Zapnete ju v Nastaveniach voľbou **Kontrolovať aktualizácie (GitHub)**. Appka potom urobí jeden neautentifikovaný `GET` na najnovšie vydanie v GitHub Releases. Neposiela bankové údaje, heslá ani telemetriu.

Keď je na GitHub novšie vydanie, appka ukáže jeho poznámky k vydaniu. Ukáže aj tlačidlo `Aktualizovať na v<verzia>`. Až po kliknutí stiahne inštalátor. Sťahovanie overí voči `SHA256SUMS.txt` publikovanému s tým vydaním. Bez kliknutia sa nič nestiahne a nič sa nespustí.

Inštalátor, ktorý appka takto spustí, je ten istý sprievodca ako pri ručnom stiahnutí: aktualizuje existujúcu inštaláciu na mieste a povie to.

Každý pokus (kontrola aj sťahovanie) je v **Nastavenia > Sieťová aktivita**. Denník uvádza čas, adresu a výsledný stav.

## Súkromie a sieť

Appka nezbiera telemetriu. Nesynchronizuje dáta do cloudu. Nepoužíva LLM ani online obohacovanie údajov.

Sieťové volania sú len dve:

1. Voliteľná kontrola aktualizácií na GitHub Releases. Predvolene vypnutá.
2. Sťahovanie inštalátora z GitHub Releases. Len po kliknutí na `Aktualizovať na v<verzia>`.

Obidve sú v denníku **Sieťová aktivita**. CSP webového okna povoľuje pripojenie len pre internú komunikáciu Tauri. Nepovoľuje externý `connect-src`. Automatický test zakazuje sieťové knižnice a sieťové pluginy Tauri okrem klienta pre tieto volania.

Heslá k výpisom sú v Správcovi poverení systému Windows. Nie sú v databáze. Kontrola pred commitom blokuje PDF súbory a skutočné slovenské IBAN-y. Povoľuje len uvedené syntetické testovacie IBAN-y.

## Overenie vydania

Od vydania 0.1.4 obsahuje každé vydanie na GitHub súbor `SHA256SUMS.txt`. Vydanie v0.1.3 tento súbor nemá; jeho kontrolné súčty sú v tabuľke nižšie. Tag `v0.1.3` bol prvý verejný tag. Tagy podpisuje správca cez SSH.

Kontrola stiahnutého súboru v PowerShell:

```powershell
Get-FileHash -Algorithm SHA256 .\abakus-setup-0.1.6.exe
```

Porovnajte výstup s riadkom v `SHA256SUMS.txt` daného vydania. Kontrolné súčty pre 0.1.6 sú v `SHA256SUMS.txt` tohto vydania na GitHub, nie v tomto súbore (zostavujú sa až pri vydaní).

Známe kontrolné súčty SHA-256 pre **0.1.3**:

| Súbor | SHA-256 |
| --- | --- |
| inštalátor `abakus-setup-0.1.3.exe` | `4080b1869ba0187de226127aed4748b9a090b69516e49bd154fdbd1fb9a58601` |
| `abakus.exe` | `45f745e238a41dc476a4bfce37c4231ebf441f496929de84fec1cbc5af83be71` |

Kontrola podpisu tagu po `git fetch --tags`:

```powershell
git verify-tag v0.1.3
```

## Vývoj

Potrebujete Windows, PowerShell 7, Rust pre cieľ `stable-x86_64-pc-windows-msvc`, Node.js a npm.

Najprv raz stiahnite lokálnu knižnicu PDFium:

```powershell
pwsh -File .\scripts\fetch-pdfium.ps1
```

Potom nainštalujte balíky a spustite appku:

```powershell
npm install
npm run tauri dev
```

Pred testami nastavte cestu k PDFium. Potom spustite testy:

```powershell
$env:ABAKUS_PDFIUM_DIR = "$PWD\src-tauri\resources\pdfium"
cargo test --jobs 4 --workspace
npm test
cargo clippy --workspace --all-targets
```

Inštalátor (Inno Setup 6):

```powershell
.\packaging\build-installer.ps1
```

Skript zostaví frontend cez `tsc` a `vite`, potom `tauri build --no-bundle` a nakoniec Inno Setup. Podrobnosti sú v [packaging/INSTALL.md](packaging/INSTALL.md).

## Dokumentácia

- [CHANGELOG.md](CHANGELOG.md): poznámky k vydaniu, ktoré appka ukáže ako **Poznámky k vydaniu**.
- [KNOWN_ISSUES.md](KNOWN_ISSUES.md): známe obmedzenia.
- [docs/](docs/): interné poznámky k dráham a vydaniam, vrátane [docs/release-0.1.2.md](docs/release-0.1.2.md) a [docs/release-0.1.3.md](docs/release-0.1.3.md).
- [packaging/INSTALL.md](packaging/INSTALL.md): inštalátor, odinštalovanie a WebView2.
- [CONTRIBUTING.md](CONTRIBUTING.md): ako prispievať.
- [SECURITY.md](SECURITY.md): ako nahlásiť bezpečnostnú chybu.
- [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md): PDFium, písma a závislosti.

## Prispievanie

Pozrite [CONTRIBUTING.md](CONTRIBUTING.md). Pull requesty idú do `main`. Texty rozhrania sú po slovensky. Skutočné bankové dáta do fixture nepatria.

## Známe obmedzenia

Pozrite [KNOWN_ISSUES.md](KNOWN_ISSUES.md). Pravidelné platby sú odhady z importov. Záloha nie je šifrovaná. Obnova zo zálohy v appke nie je. PDF report má limity veľkosti.

## Licencia

Copyright (c) 2026 SouthCarpet (Michal).

Abakus je slobodný softvér pod [GNU Affero General Public License v3.0 alebo novšou](LICENSE) (AGPL-3.0-or-later). Úplný text je v [LICENSE](LICENSE).

Zabalené knižnice tretej strany (PDFium, Noto Sans, Rust crate-y, npm balíky) majú vlastné licencie. Zoznam je v [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
