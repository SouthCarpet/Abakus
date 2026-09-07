# Návrh: predplatné a pravidelné platby v Abakuse

Stav: schválené na implementáciu v pláne 080, verzia 0.1.2. Dátum: 2026-09-07, revízia 2 po pripomienkach Michala. Autor návrhu: Claude (Fable 5.1).

Michal schválil celý návrh priamym pokynom po úlohách ledgera `080-navrh-predplatne` a `080-navrh-predplatne-r2`. Nasledujúci text zachováva pôvodný návrh a opis stavu pred implementáciou. Konečné správanie a obmedzenia uvádzajú poznámky k verzii 0.1.2.

## Cieľ v jednej vete

Prehľad má do piatich sekúnd odpovedať na štyri otázky: koľko odchádza a prichádza pravidelne, čo ešte príde tento mesiac, čo sa zmenilo, a čo chýba. Appka to zistí sama z výpisov a používateľ ju opraví jedným kliknutím, rovnako ako dnes potvrdzuje odhad kategórie.

## Východisko (čo appka dnes vie)

- Kategória **Predplatné** s podkategóriami Netflix, Max HBO, Voyo, YouTube, Apple (`crates/store/src/seed_categories.rs`).
- Seed pravidlá podľa obchodníka smerujú platby do týchto podkategórií (`crates/store/src/seed_rules.toml`, riadky 22 až 29).
- Prehľad ukazuje predplatné len ako stĺpec v grafe kategórií a v tabuľke Najčastejší obchodníci.
- Obrazovka Kategórie vie pridať hlavnú kategóriu, pridať podkategóriu, premenovať a archivovať. Nevie presunúť podkategóriu pod inú nadradenú a nevie vytvoriť kategóriu priamo pri zaraďovaní transakcie.
- Databáza má všetko, čo detekcia potrebuje: `merchant_norm`, `counterparty_iban`, `amount_cents`, `tx_date`, `kind` (Card, CardForeign, StandingOrder, TransferIn, ...), `orig_currency`.
- Vetva `release/0.1.2` pridáva poznámky, porovnanie období a pokrytie výpismi. Návrhy nižšie s nimi neprekážajú a pokrytie výpismi priamo využívajú.

## Princíp: navrhni, potvrď, nauč sa

Appka už dnes funguje tak, že navrhne kategóriu a používateľ ju potvrdí. Pravidelné platby idú tou istou cestou:

1. Detekcia nájde kandidáta (návrh 2).
2. Panel ho ukáže ako **Odhad** so zisteným intervalom.
3. Používateľ potvrdí, zmení interval, alebo povie **Toto nie je pravidelná platba**.
4. Rozhodnutie sa uloží (návrh 6) a appka ho už neprepíše.

Vďaka tomu nie je dôležité, či nájom a poistenie patria do panela. Detekcia ich ponúkne, používateľ rozhodne. Nič sa nehádže do zoznamu bez potvrdenia, okrem kategórie Predplatné, kde je odhad dostatočne istý.

## Návrh 1: Panel Pravidelné platby v Prehľade

**Čo:** Nová karta v Prehľade s dvoma časťami: **Výdavky** a **Príjmy**. Jeden riadok na službu alebo protistranu: názov, suma, interval, posledná platba, ďalšia očakávaná, stav. Nad tým sekcia **Odhady** s kandidátmi na potvrdenie.

**Prečo:** Dnes vidí používateľ len celkovú sumu kategórie. Nevidí, koľko služieb platí, kedy prídu ďalšie platby a či výplata alebo faktúra prišla načas. Príjmy sú tu preto, aby panel slúžil ako prehľad toku peňazí, nie len ako zoznam streamov.

**Ako:** Nový dotaz v `crates/store/src/summary.rs` po vzore `top_merchant_rows`. Zoskupí transakcie podľa `merchant_norm`, pri prevodoch podľa `counterparty_iban`. Frontend: nová komponenta vedľa `TopMerchantsTable` v `src/screens/Overview.tsx`. Kliknutie na riadok otvorí Transakcie s filtrom, ako to už robí graf kategórií. Rešpektuje filter obdobia a druhu účtu.

**Náročnosť:** nízka až stredná. Jeden dotaz, jedna karta, testy v `Overview.test.tsx`.

## Návrh 2: Detekcia pravidelných platieb a príjmov

**Čo:** Appka zistí, ktoré platby sa opakujú, v každej kategórii a na oboch stranách (výdavok aj príjem). Tri intervaly:

| Interval | Okno medzi platbami | Minimálny počet výskytov |
|---|---|---|
| mesačný | 28 až 33 dní | 3 |
| štvrťročný | 85 až 97 dní | 2 |
| ročný | 355 až 375 dní | 2 |

Suma sa môže líšiť do 10 % (kurz pri `CardForeign`, drobné zmeny cien). Interné prevody (`status = transfer`) sa nepočítajú.

**Prečo:** Nájom, poistenie, mobil, hosting, domény, výplata a pravidelné faktúry sú tiež pravidelné. Detekcia podľa dát je spoľahlivejšia než zoznam mien obchodníkov. Štvrťročný a ročný interval pokryjú poistenie, dane a ročné predplatné.

**Ako:** Čistá funkcia v store: vstup `merchant_norm` + zoznam dátumov a súm, výstup interval alebo `None`. Bez SQL, testovateľná na syntetických dátach. Pri nedostatku výskytov (napríklad druhé ročné predplatné ešte neprišlo) nevznikne odhad, používateľ ho môže zadať ručne (návrh 6).

**Náročnosť:** stredná. Hlavné riziko sú falošné nálezy, napríklad týždenný nákup potravín, ktorý náhodou sedí do mesačného okna. Preto minimálne počty, tolerancia sumy a stav Odhad, kým používateľ nepotvrdí.

## Návrh 3: Zmena ceny

**Čo:** Pri službe, ktorej suma sa zmenila, zobrazí riadok odznak **Zdraželo o 2,00 EUR od 2026-06** alebo **Zlacnelo**. Rovnaký odznak pri príjme: **Vyššia výplata od ...**.

**Prečo:** Streamovacie služby a poisťovne dvíhajú cenu bez výrazného oznámenia. Vo výpise sa to stratí. Najlacnejší návrh s najväčším praktickým prínosom.

**Ako:** Rozšíri dotaz z návrhu 1 o predposlednú sumu. Porovnanie v Rust vrstve. Pri `CardForeign` porovnáva `orig_amount_cents` v `orig_currency`, nie EUR, aby kurz nevyrábal falošné zmeny. Zmena sa ukáže až od dvoch rovnakých nových súm, jedna odchýlka je šum.

**Náročnosť:** nízka, ak existuje návrh 1.

## Návrh 4: Stav položky

**Čo:** Každý riadok má stav:

- **Aktívna**: posledná platba v rámci intervalu.
- **Príde do konca mesiaca**: očakávaný dátum je v aktuálnom mesiaci a ešte neprešiel.
- **Chýba platba**: očakávaný dátum prešiel o viac než 10 dní. Pri príjme to znamená meškajúcu výplatu alebo faktúru.
- **Ukončená**: žiadna platba dva intervaly po sebe.
- **Neznáme, chýba výpis**: očakávaný dátum padá do medzery v pokrytí výpismi.

**Prečo:** Ukáže zabudnuté zrušenie (platba stále chodí), nechcené ukončenie (platba prestala chodiť, napríklad po výmene karty) a meškajúci príjem. Všetky tri stoja peniaze alebo prístup.

**Ako:** Čistá funkcia stav(posledný dátum, interval, dnes, medzery pokrytia). Panel pokrytia z verzie 0.1.2 už medzery pozná, treba jeho výsledok len použiť.

**Náročnosť:** nízka. Závisí od návrhu 2 a pokrytia výpismi.

## Návrh 5: KPI toku peňazí

**Čo:** Tri kachle nad panelom:

- **Pravidelné výdavky za mesiac** (ročné a štvrťročné položky rozpočítané na mesiac) a podiel na priemerných mesačných výdavkoch.
- **Pravidelné príjmy za mesiac**.
- **Ešte príde tento mesiac**: súčet očakávaných, zatiaľ nezaplatených položiek do konca mesiaca, výdavky aj príjmy zvlášť.

Pod tým jeden riadok: **Voľné po pravidelných platbách** = pravidelné príjmy mínus pravidelné výdavky. Tlačidlo **Za rok** prepne rovnaké čísla na ročnú projekciu.

**Prečo:** Jedno číslo odpovie, koľko odchádza automaticky bez ohľadu na správanie, a koľko z toho ešte tento mesiac nepadlo. To je rýchly prehľad, ktorý Michal chce, bez nového grafu.

**Ako:** Súčet z riadkov panela so stavom Aktívna alebo Príde do konca mesiaca. Bez nového dotazu. Komponenta `Kpi` už existuje.

**Náročnosť:** nízka. Závisí od návrhov 1, 2 a 4.

## Návrh 6: Používateľ rozhoduje o pravidelnosti

**Čo:** Tri miesta, kde používateľ nastaví alebo opraví pravidelnú platbu:

1. **V paneli, riadok Odhad:** tlačidlá **Potvrdiť**, **Zmeniť interval** (mesačne, štvrťročne, ročne) a **Toto nie je pravidelná platba**.
2. **V detaile transakcie**, aj pri nezaradenej platbe: prepínač **Pravidelná platba** s výberom intervalu. Zaradí položku do panela hneď, bez čakania na tri výskyty. Hodí sa pri prvej platbe nového poistenia alebo domény.
3. **V paneli, riadok Aktívna:** **Upraviť** otvorí rovnaký dialóg. Tam sa dá interval zmeniť, keď sa napríklad mesačný Netflix zmení na ročný.

**Prečo:** Žiadna detekcia nie je bez chýb. Bez ručnej opravy panel stratí dôveru pri prvom omyle. Ručné zadanie navyše pokryje položky, ktoré detekcia ešte nemá odkiaľ poznať.

**Ako:** Jediná nová tabuľka:

```sql
CREATE TABLE IF NOT EXISTS recurring (
  key TEXT PRIMARY KEY,                     -- merchant_norm alebo counterparty_iban
  mode TEXT NOT NULL CHECK (mode IN ('confirmed','ignored')),
  interval_days INTEGER,                    -- 30, 91, 365; NULL pri ignored
  set_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Detekcia ju číta pred výpočtom: `confirmed` prebije zistený interval, `ignored` položku skryje. Migrácia po vzore poznámok z 0.1.2. Záloha databázy ju zahrnie automaticky. Potvrdené rozhodnutie appka nikdy neprepíše, rovnako ako potvrdenú kategóriu.

**Náročnosť:** stredná. Jediný návrh so zmenou schémy, ale je jadrom princípu navrhni, potvrď, nauč sa, preto patrí do prvej vlny, nie na koniec.

## Návrh 7: Kategórie plne v rukách používateľa

**Vedľajší nález:** seed `spotify` mieri do `Predplatné/Apple` (`seed_rules.toml`, riadok 29). Spotify nie je Apple. Používateľ to dnes opraví len tak, že pravidlo zmaže a transakcie ručne zaradí inde. Opraviť v seede hneď: vlastná podkategória `Spotify`.

**Čo chýba, aby si používateľ vedel hierarchiu upraviť sám:**

1. **Presun kategórie pod inú nadradenú.** V obrazovke Kategórie výber **Nadradená kategória** pri úprave. Podkategória sa dá presunúť pod iného rodiča alebo povýšiť na hlavnú. Prevzatie druhu (výdavok alebo príjem) od nového rodiča, s potvrdením, ak sa druh mení a existujú zaradené transakcie.
2. **Vytvorenie kategórie priamo pri zaraďovaní.** V dialógu priradenia kategórie v Transakciách položka **Nová kategória...** s poľom názov a výberom nadradenej. Používateľ neopúšťa transakciu, ktorú práve triedi. Rovnaký dialóg použije návrh 6 pri potvrdení novej pravidelnej platby, ktorá zatiaľ nemá kategóriu.
3. **Oprava seed pravidla jedným krokom.** Pri transakcii zaradenej seed pravidlom ponúknuť **Presmerovať pravidlo**: zmení kategóriu pravidla a prezaradí nepotvrdené riadky, potvrdené riadky nechá tak, ako to už robí `apply_to_matching`.

**Ako:** `save_category` v `crates/store/src/categories.rs` už prijíma `parent_id`, frontend ho len nikdy nemení. Presun je preto najmä zmena v `src/screens/Categories.tsx` plus kontrola cyklu (kategória nemôže byť vlastný predok) a kontrola druhu v store. Vytvorenie z transakcie je nová komponenta zdieľaná Transakciami a panelom. Presmerovanie pravidla potrebuje `update_rule_category` v `rules_repo.rs`.

**Náročnosť:** nízka až stredná. Body 1 a 2 sú bez zmeny schémy.

## Odporúčané poradie

1. Oprava seedu Spotify (minúty).
2. Návrh 7 body 1 a 2 (presun kategórie, kategória z transakcie). Základ, na ktorý sa ostatné odkazujú.
3. Návrh 2 + návrh 6 (detekcia a potvrdenie). Jadro princípu navrhni, potvrď, nauč sa.
4. Návrh 1 + návrh 4 (panel a stav). Prvý viditeľný výsledok.
5. Návrh 3 + návrh 5 (zmena ceny, KPI toku peňazí). Lacné doplnky nad hotovými riadkami.
6. Návrh 7 bod 3 (presmerovanie pravidla) podľa potreby.

## Čo zámerne neriešiť

- Pripomienky pred platbou alebo notifikácie. Appka beží len keď je otvorená a nemá sieť ani plánovač.
- Odkazy na zrušenie služby alebo ceny z internetu. Porušilo by to záruku bez siete.
- Rozpoznávanie predplatných z názvu služby cez LLM. Dáta stačia, pravidlá ostávajú lokálne.
- Rozpočty a limity na kategóriu. Iná téma, iný návrh.
- Týždenný interval. Príliš veľa falošných nálezov (nákupy), prínos malý.

## Otvorené otázky pre Michala

1. Má sa detekcia počítať za všetky účty spolu, alebo osobný a firemný oddelene ako v Prehľade? Návrh: podľa aktívneho filtra druhu účtu, rovnako ako ostatné karty.
2. Má odhad v kategórii Predplatné prejsť do panela bez potvrdenia (návrh: áno), alebo aj tam čakať na kliknutie?
3. Má panel rozpočítať ročné položky na mesiac (návrh: áno, s malým odznakom **ročne**), alebo ich ukazovať len v mesiaci, keď padnú?
