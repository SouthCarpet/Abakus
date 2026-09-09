# Poznámky k vydaniu

Najnovšia verzia je hore. Text opisuje, čo vidí používateľ.

## 0.1.6 (2026-09-09)

Opravuje import výpisov z nového generátora PDF Tatra banky a zjednodušuje nastavenie účtu.

- Výpisy, ktoré od júla 2026 končili hláškou „not a Tatra banka statement: no IBAN line“, sa importujú. Text z PDF sa skladá podľa základne písma a kroku znakov, nie podľa rámčekov glyfov, a hlavička sa číta až po riadok tabuľky.
- Výpis z účtu, ktorý v aplikácii ešte nie je, otvorí dialóg `Účet nie je nastavený`: názov, druh (osobný alebo firemný, predvyplnený z výpisu) a voliteľné heslo k výpisom. Po potvrdení sa účet uloží a import zopakuje. `Neskôr` dialóg zavrie, `Pridať účet` na karte ho otvorí znova.
- Pri každom poli s heslom je tlačidlo `Zobraziť` a `Skryť`, ktoré zostane funkčné aj po kliknutí mimo poľa. Natívna ikona oka je vypnutá.
- Diagnostika pre vývoj: `abakus-cli lines` a `abakus-cli geometry` vypíšu text a geometriu strany s maskovanými číslicami.

## 0.1.5 (2026-09-09)

Opravuje inštalátor pri aktualizácii existujúcej inštalácie.

- Sprievodca inštalátora teraz povie, že Abakus je už nainštalovaný, akú verziu má a kde. Pri staršej nainštalovanej verzii ponúkne aktualizáciu, pri rovnakej verzii sa spýta na preinštalovanie, pri novšej nainštalovanej verzii ponúkne nahradenie staršou (predvolene odmietne downgrade).
- Pri aktualizácii alebo preinštalovaní sprievodca vynechá stránku s odkazmi (Start/plocha) a na stránke **Inštalácia je pripravená** napíše, z akej verzie na akú aktualizuje.
- Dáta v `%LOCALAPPDATA%\Abakus` a heslá v Správcovi poverení zostanú nedotknuté.

## 0.1.4 (2026-09-08)

Prvé vydanie s aktualizáciou v aplikácii, licenciou AGPL-3.0 a verejnou dokumentáciou. Zdrojový kód je na [GitHub](https://github.com/SouthCarpet/Abakus). Licencia je AGPL-3.0-or-later.

- V Nastaveniach zapnite **Kontrolovať aktualizácie (GitHub)**. Ak existuje novšie vydanie, appka ukáže jeho poznámky a tlačidlo `Aktualizovať na v<verzia>`.
- Tlačidlo stiahne inštalátor z GitHub Releases. Súbor sa overí voči `SHA256SUMS.txt` toho vydania. Nič sa nespustí, kým na tlačidlo nekliknete.
- Obe sieťové volania (kontrola aj sťahovanie) sú v **Nastavenia > Sieťová aktivita**.
- Inštalátor `abakus-setup-0.1.4.exe` je pre aktuálneho používateľa a nepotrebuje práva správcu. Skontroluje Microsoft Edge WebView2 Runtime a ponúkne jeho stiahnutie, ak chýba.
- Dialóg hesla k výpisu v Nastaveniach je upravený. Heslo sa naďalej ukladá len do Správcu poverení systému Windows.

## 0.1.3 (2026-09-08)

- V Nastaveniach má každý účet tlačidlo **Nastaviť heslo** alebo **Zmeniť heslo**. Dialóg žiada heslo a jeho zopakovanie. **Uložiť** je dostupné, až keď sa obe polia zhodujú a nie sú prázdne.
- Heslo sa uloží do Správcu poverení rovnako ako pri importe so **Zapamätať pre tento účet**. Nová cesta nepotrebuje čerstvý zamknutý PDF.

## 0.1.2 (2026-09-08)

- Prehľad ukáže pokrytie výpismi, porovnanie kategórií s predchádzajúcim obdobím a historické konečné zostatky z výpisov.
- V Nastaveniach vytvoríte lokálnu zálohu databázy SQLite. Záloha nie je šifrovaná. Heslá a pôvodné PDF do nej nepatria.
- K transakcii možno uložiť poznámku. Poznámky sú vo vyhľadávaní aj v CSV.
- Prehľad ukáže pravidelné príjmy a výdavky. Odhady sú oddelené od potvrdených položiek. Platbu možno potvrdiť, ignorovať alebo určiť ručne.
- Kategórie možno vytvoriť pri priradení, presunúť alebo osamostatniť. Najviac dve úrovne.
- Na každej obrazovke je **Exportovať PDF** pre mesiac, šesť mesiacov, rok alebo celé obdobie. Report má grafy, pokrytie a úplný výpis vrátane poznámok.
- Účet možno natrvalo zmazať po potvrdení názvu. Vzhľad Svetlý, Tmavý alebo Podľa systému je na každej obrazovke.
- Transakcie exportujú filtrované CSV, vedia vymazať všetky filtre a ukazujú súčty zobrazených riadkov.

## 0.1.1 (2026-09-06)

Opravy dátovej vrstvy pred spojením funkcií 0.1.2. Používateľ vidí spoľahlivejšie odstránenie účtu a výpisu a presnejšie súčty. Verejné poznámky k tejto verzii sú tenké.

## 0.1.0 (2026-08-30)

Prvé verzie. Appka už importovala PDF výpisy Tatra banky, radila transakcie do kategórií, ukazovala prehľad a mala voliteľnú kontrolu aktualizácií. Inštalátor bol lokálny. Podrobný zoznam zmien z týchto zostáv nie je k dispozícii.
