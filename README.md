# Abakus

## Čo je Abakus

Abakus je lokálna desktopová appka na analýzu PDF výpisov z Tatra banky. Rozdeľuje transakcie do kategórií a zobrazuje príjmy a výdavky v grafoch. Bankové údaje nikdy neopustia váš počítač. GitHub obsahuje iba zdrojový kód.

## Záruky súkromia

- [x] Appka neobsahuje sieťový kód okrem voliteľnej kontroly aktualizácií. Tá je predvolene vypnutá. Po zapnutí vykoná jeden `GET` na GitHub Releases a zapíše ho do denníka siete v appke.
- [x] CSP povoľuje pripojenie iba pre internú komunikáciu Tauri. Nepovoľuje externý `connect-src`.
- [x] Automatický test kontroluje závislosti. Zakazuje sieťové knižnice a sieťové pluginy Tauri. Jedinou povolenou výnimkou je klient pre voliteľnú kontrolu aktualizácií.
- [x] Kontrola pred commitom blokuje PDF súbory a skutočné slovenské IBAN-y. Povoľuje iba výslovne uvedené syntetické testovacie IBAN-y.
- [x] Dáta sa ukladajú do `%LOCALAPPDATA%\Abakus`.
- [x] Heslá k PDF sa ukladajú do Správcu poverení systému Windows. Nie sú v databáze.

## Inštalácia pre vývoj

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
```

## Prvé použitie

1. V časti **Nastavenia** pridajte vlastné účty. Zadajte IBAN alebo zápis v tvare `kód banky/prefix-číslo účtu`.
2. V časti **Import** pridajte mesačné PDF výpisy. Pri zamknutom PDF zadajte heslo. Ak zvolíte **Zapamätať pre tento účet**, appka ho pri ďalšom importe použije automaticky. Heslo môžete nastaviť alebo zmeniť aj priamo v **Nastaveniach**, bez importu.
3. V časti **Transakcie** skontrolujte navrhnuté kategórie. Odhad potvrdíte jedným kliknutím.
4. V časti **Prehľad** si pozrite príjmy, výdavky, kategórie a vývoj v čase.

Graf **Podľa kategórií** zachováva znamienko súčtu. Záporné hodnoty vľavo znižujú výdavky, kladné vpravo ich zvyšujú. Kliknutie na kategóriu otvorí príslušné transakcie. Tabuľky v menšom okne umožňujú vodorovné posúvanie; suma a mena zostávajú spolu.

Každý importovaný výpis má odznak kontrolného súčtu. Appka overí, či počiatočný zostatok a všetky transakcie dávajú konečný zostatok. Text **Kontrolný súčet nesedí** znamená, že sa tieto hodnoty líšia o uvedenú sumu. Výpis sa importuje, ale zostane označený na kontrolu. Ak chýba potrebný zostatok, appka uvedie, že súčet nevie overiť.

## Ako sa appka učí

Vytvorenie kategórie ani podkategórie samo nevytvorí pravidlo. Po priradení kategórie transakcii alebo potvrdení návrhu si appka zapamätá rozpoznaného obchodníka a miesto. Presná zhoda sa priradí automaticky. Rovnakého obchodníka na inom mieste označí ako **odhad**, ktorý potvrdíte jedným kliknutím. Platby bez rozpoznaného obchodníka sa týmto spôsobom neučia ako jedna spoločná skupina.

Voľba **Použiť aj na podobné** navyše zaradí podobné už importované transakcie, ktoré ešte nie sú potvrdené. Potvrdené priradenia ponechá. V detaile transakcie možno zmeniť cieľ pôvodného vstavaného pravidla, ak ju zaradilo také pravidlo. Samostatný formulár na ručné vytváranie pravidiel zatiaľ nie je dostupný.

## Kontrola aktualizácií

Kontrola aktualizácií je v predvolenom stave vypnutá. Zapnete ju v časti **Nastavenia**. Appka potom vykoná jeden neautentifikovaný `GET` na najnovšie vydanie v GitHub Releases. Neposiela bankové údaje, prihlasovacie údaje ani telemetriu. Aktualizáciu nesťahuje, neinštaluje ani automaticky neotvára.

Každý pokus sa zobrazí v **Nastavenia > Stav > Sieťová aktivita**. Denník uvádza čas, adresu a výsledný stav.

## Kde sú dáta a ako zálohovať

Databáza je v súbore `%LOCALAPPDATA%\Abakus\abakus.db`. V **Nastaveniach** vytvorte **Zálohu databázy** a vyberte nový súbor. Appka uloží konzistentnú kópiu aj počas svojho behu. Existujúci cieľový súbor neprepíše.

Záloha obsahuje bankové údaje, účty, výpisy, kategórie, naučené pravidlá, poznámky, uložené voľby pravidelných platieb a nastavenia databázy. Nie je šifrovaná, preto ju uložte na bezpečné miesto. Neobsahuje heslá zo Správcu poverení ani pôvodné PDF. Obnova zo zálohy zatiaľ nemá ovládanie v appke.

## Čo appka nikdy nerobí

- Neukladá dáta do cloudu a nesynchronizuje ich.
- Nezbiera telemetriu.
- Nepoužíva LLM ani online obohacovanie údajov.
- Nesťahuje ani neinštaluje aktualizácie automaticky.

## Licencia a stav

Abakus je vo verzii 0.1.3 a používa licenciu MIT. Táto verzia pridáva priame nastavenie a zmenu hesla k výpisom účtu v Nastaveniach. Zmeny opisuje [prehľad vydania](docs/release-0.1.3.md), obmedzenia sú v súbore [KNOWN_ISSUES.md](KNOWN_ISSUES.md).

## Pravidelné platby a kategórie

V **Prehľade** nájdete pravidelné výdavky aj príjmy s mesačným, štvrťročným alebo ročným opakovaním. Odhady sú oddelené od platieb, ktoré potvrdíte. Detail ukáže konkrétne platby, ďalší očakávaný termín a prípadnú ustálenú zmenu ceny. Výpočty rozlišujú pôvodnú menu a prepočet do eur.

Platbu možno potvrdiť, ignorovať alebo obnoviť jej automatický odhad. V detaile transakcie možno určiť opakovanie aj ručne. Pri ručnom výbere konkrétnych platieb treba ďalšie platby priraďovať ručne. Chýbajúce výpisy vedú k neznámemu stavu. Odhad ukončenia neznamená potvrdené zrušenie zmluvy. Historické obdobie zobrazuje stav k jeho koncu, bez neskorších údajov.

Panel ukazuje mesačný alebo ročný prepočet a očakávané platby do konca mesiaca. Podiel na výdavkoch je dostupný iba pri dostatočnom overenom pokrytí všetkých vybraných účtov. Tieto hodnoty sú odhady z importovaných údajov, nie bankové príkazy ani aktuálny zostatok.

Kategóriu možno vytvoriť priamo pri priraďovaní transakcie. V **Kategóriách** ju možno presunúť pod inú nadradenú kategóriu alebo osamostatniť. Kategórie majú najviac dve úrovne. Podkategória preberá druh príjem alebo výdavok od rodiča. Zmena druhu s dopadom na staršie transakcie vyžaduje potvrdenie zobrazených počtov. Spotify má vlastnú podkategóriu; úprava starých údajov zachováva používateľské zmeny pravidiel.

## PDF s grafmi a výpisom

Na každej obrazovke je pri voľbe vzhľadu tlačidlo **Exportovať PDF**. Vyberte účty a obdobie:

- **Mesiac:** celý vybraný kalendárny mesiac.
- **Šesť mesiacov:** šesť po sebe idúcich mesiacov vrátane vybraného koncového mesiaca.
- **Rok:** celý vybraný kalendárny rok.
- **Celé obdobie:** všetky uložené transakcie vybraných účtov.

Náhľad ukáže rozsah a počet transakcií. PDF obsahuje súhrn, grafy, informáciu o pokrytí výpismi a úplný výpis vrátane poznámok. Použije všetky transakcie vo vybranom období a účtoch; filtre tabuľky sa nepoužijú. Aktuálne obdobie je označené ako neukončené. Údaje sa zachytia pri uložení, preto sa ich počet môže od náhľadu líšiť.

Report má svetlé strany A4, vložené písmo so slovenčinou a číslovanie strán. Export pracuje lokálne. Vyberte nový súbor PDF; existujúci súbor sa neprepíše. Zrušenie dialógu nič nevytvorí. Pri prekročení limitu reportu sa zobrazí chyba a výpis sa potichu neskráti.

## Pokrytie, porovnanie a historické zostatky

V **Prehľade** panel pokrytia uvedie medzery vo výpisoch pre jednotlivé účty. Pri vybranom období kontroluje aj jeho začiatok a koniec. Pri voľbe **Všetko** ukáže iba medzery medzi známymi výpismi. Prekrývajúce sa výpisy spočíta ako jedno pokryté obdobie. Ide o obdobia výpisov; dátumy jednotlivých transakcií môžu byť mimo nich.

Porovnanie kategórií ukáže výdavky vo vybranom období a v bezprostredne predchádzajúcom období s rovnakým počtom dní. Oba rozsahy sú uvedené v paneli. Refundácie znižujú výdavky, interné prevody sa nezapočítavajú. Pri nulovom základe percentuálna zmena nie je dostupná. Chýbajúce výpisy a budúce dni sú označené. Pri voľbe **Všetko** sa porovnanie nezobrazuje.

Historické konečné zostatky sú hodnoty z výpisov ku konkrétnemu dňu. Nie sú aktuálnym zostatkom bankového účtu. Chýbajúci zostatok zostáva neznámy a kontrolný súčet ukazuje stav overenia výpisu.

## Poznámky k transakciám

V detaile transakcie možno uložiť poznámku do 2 000 znakov vrátane nových riadkov. Prázdny text poznámku odstráni. Poznámky sa dajú vyhľadávať aj bez diakritiky a exportujú sa v stĺpci **poznamka** v CSV. Uloženie poznámky nemení kategóriu ani naučené pravidlo.

## Odstránenie účtu a vzhľad

V **Nastaveniach** kliknite v riadku účtu na **Zmazať účet**. Náhľad uvedie názov a počty výpisov, transakcií a pravidiel. Na potvrdenie napíšte presný názov účtu a kliknite na **Natrvalo zmazať účet**. Zrušenie nič neodstráni. Odstránenie importovaných údajov je nezvratné; pôvodné bankové PDF súbory zostanú na disku. Prípadné zlyhanie databázy alebo odstránenia hesla sa zobrazí v dialógu. Obnovenie zobrazenia po úspešnom odstránení neopakuje odstránenie.

V riadku účtu je aj tlačidlo **Nastaviť heslo** (alebo **Zmeniť heslo**, ak je už uložené). Otvorí dialóg s dvomi poľami: heslo a jeho zopakovanie. **Uložiť** je dostupné, až keď sa obe zhodujú a nie sú prázdne. Po úspechu appka zobrazí potvrdenie a heslo uloží do Správcu poverení systému Windows; riadok potom ukáže **heslo uložené**. Pri zlyhaní appka zachová rozpísané heslo a zobrazí dôvod.

Výber **Vzhľad** je dostupný na každej obrazovke: **Svetlý**, **Tmavý**, **Podľa systému**. Prvé dve možnosti ignorujú zmeny systému, tretia ich sleduje. Voľba sa uloží pre ďalšie spustenie. Pri poškodenom alebo nedostupnom úložisku sa použije systémový režim; pri zlyhaní zápisu platí nová voľba len v aktuálnom behu.

## Filtre a CSV

- **Exportovať filtrované CSV** v Transakciách exportuje aktuálne obdobie, účet, druh účtu, kategóriu, stav, hľadaný text aj obmedzenie na konkrétny výpis. Použije text viditeľný pri kliknutí, aj keď tabuľka ešte čaká na dokončenie hľadania. Filter sa zachytí pri kliknutí; CSV číta údaje z databázy pri exporte. Zrušenie výberu súboru nič neexportuje a zlyhanie zápisu sa zobrazí. Pôvodný export v Nastaveniach naďalej používa iba uložené obdobie.
- **Filter kategórie** umožňuje vybrať aj nadradenú kategóriu s podkategóriami a zobrazuje kategóriu prevzatú z grafu. Archivovaný alebo nedostupný výber zostáva označený; neznamená Všetky kategórie.
- **Vymazať všetky filtre** zobrazí všetky obdobia, účty, druhy, kategórie a stavy bez vyhľadávania a bez obmedzenia na výpis. Vymaže aj hromadný výber. Obdobie Všetko sa uloží pre ďalšie obrazovky.
- Panel **Súčty zobrazených transakcií** počíta presne načítané riadky tabuľky. Príjem, výdavky a čistá suma sa sčítajú v celých centoch podľa rovnakých pravidiel ako Prehľad, vrátane druhu priradenej kategórie a refundácií znižujúcich výdavky; interné prevody majú samostatný počet a do týchto súm nevstupujú. Pri načítavaní alebo chybe sa staré súčty nezobrazujú. Nejde o zostatok bankového účtu.

Vlastné obdobie potrebuje dva platné dátumy so začiatkom najneskôr v deň konca. Kým nie sú platné, dotaz a export sa nespustia. Prechod z Importu na konkrétny výpis zobrazí všetky dátumy tohto výpisu bez obmedzenia predtým uloženým obdobím. Počet výpisov v Nastaveniach má limit 100 000 načítaných výpisov. Tabuľka transakcií a jej súčty pracujú s celým výsledkom dotazu v pamäti.
