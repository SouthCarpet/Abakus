# Poznámky k vydaniu

Najnovšia verzia je hore. Text opisuje, čo vidí používateľ.

## 0.2.0 (nevydané)

- Výber kategórie (priradenie aj filter v Transakciách) má nové ovládanie s vyhľadávaním podľa názvu. Písanie ignoruje diakritiku aj veľkosť písmen. Prázdny výsledok ukáže text „Žiadna kategória sa nenašla.“. Šípky hore a dole a Enter vyberú kategóriu z klávesnice, Escape zavrie zoznam a vráti fokus na pôvodné tlačidlo.
- Potvrdenie odhadu ponúka aj potvrdenie ostatných nepotvrdených transakcií rovnakého obchodníka. Počet takých transakcií je vidieť pri tlačidle **Potvrdiť**.
- Nové tlačidlo **Potvrdiť vybrané** potvrdí odhady vo vybraných riadkoch naraz, rovnako ako doterajšie hromadné priradenie kategórie.
- Po hromadnom priradení kategórie appka na chvíľu ponúkne tlačidlo **Späť**, ktoré priradenie vráti a obnoví tabuľku. Späť zatiaľ nefunguje pre hromadné potvrdenie.
- Prehľad hore ukáže pripomienku, keď appke chýba výpis za predchádzajúci mesiac (sedem dní po konci mesiaca). Text hovorí o medzere v pokrytí, nie o chýbajúcich transakciách. Tlačidlo **Prejsť na import** pri pripomienke otvorí Import.
- Kliknutie na obchodníka v tabuľke **Najčastejší obchodníci** otvorí Transakcie s vyplneným hľadaním podľa jeho mena.
- Nová karta **Porovnanie s rovnakým obdobím vlani** vo vlastnom výbere mesiaca a dĺžky (jeden mesiac alebo tri mesiace) ukáže aktuálne aj vlaňajšie obdobie pri číslach príjmov, výdavkov a čistého súčtu.
- Novú kategóriu aj podkategóriu možno vytvoriť priamo pri priraďovaní transakcie výberom **Nová kategória...**, bez prechodu do Kategórií. Appka rovno priradí novú kategóriu danej transakcii.
- Archivácia kategórie v **Kategóriách** má nové kompaktné ikonové tlačidlo namiesto textového. Nové červené tlačidlo kategóriu zmaže. Pred zmazaním appka ukáže, koľko transakcií sa presunie do nezaradených, a po zmazaní výsledok potvrdí.
- Tabuľka **Pravidlá** v Kategóriách má priamo v riadku výber cieľovej kategórie na zmenu bez opustenia obrazovky. Tlačidlo **Zmazať** pri pravidle najprv ukáže, koľko otvorených riadkov sa naň odkazuje a koľko z nich po zmazaní zmení zaradenie.
- Filter **Druh** v Transakciách ponúka aj bankový poplatok (**Poplatok**). Prehľad má novú dlaždicu **Poplatky** a graf **Príjmy a výdavky** má vlastný stĺpec Poplatky.
- Nad tabuľkou Transakcií sa objavia skupiny nezaradených platieb podľa obchodníka, napríklad „Twitch, 15 platieb, nepriradené“. Kliknutie na skupinu vyberie jej riadky pre hromadné priradenie alebo potvrdenie.
- Tabuľka Transakcií sa dá ovládať klávesnicou: šípky hore a dole presúvajú zvýraznený riadok, Enter potvrdí jeho odhad. Fokus vo vyhľadávaní, poznámke alebo vo výbere kategórie tieto klávesy nepoužije.
- Malý odznak novej verzie je teraz v hornej lište appky na každej obrazovke, nielen v Nastaveniach. Ukáže sa, len keď kontrola aktualizácií (vypnutá, kým ju sami nezapnete) už predtým našla novšie vydanie. Kliknutím prejdete do Nastavení. Odznak sám nič nekontroluje.
- Zoznam **Sieťová aktivita** v Nastaveniach po 10 riadkoch zbalí zvyšok pod tlačidlo **Zobraziť všetky (N)**, s tlačidlom **Zbaliť** na návrat. Počet skutočne nahraných riadkov sa nemení.
- V **Nastaveniach**, hneď pod zálohou, tlačidlo **Vybrať zálohu** obnoví databázu zo súboru zálohy. Appka najprv ukáže náhľad (počet účtov, výpisov a transakcií) a v tom istom dialógu pripomenie, že najprv vytvorí bezpečnostnú kópiu súčasnej databázy, aj že heslá zo Správcu poverení v zálohe nie sú. Súbor, ktorý nie je zálohou Abakusu, appka odmietne so zrozumiteľnou správou; rovnako odmietne zálohu vytvorenú novšou appkou, než akú appka podporuje. Pred obnovou vždy vytvorí bezpečnostnú kópiu súčasnej databázy a ukáže jej cestu. Pri zlyhaní appka vráti pôvodnú databázu; ak sa nahradená databáza nedá znova otvoriť, odloží ju pod menom `abakus-obnova-zlyhala-*` a pôvodnú vráti späť. Jediný krok, ktorý sa nedá vrátiť, je zlyhanie tohto odloženia: vtedy ostanú pôvodné dáta netknuté v pomenovanom odloženom súbore aj v bezpečnostnej kópii a appka ich sama znova nepripojí.
- V **Importe** nahrádza úplný zoznam všetkých výpisov doterajších posledných osem. Každý riadok má sumu, kontrolný súčet, odznak kontroly výpisu s rozbaliteľným zoznamom otvorených položiek (nesediaci alebo neoveriteľný súčet, upozornenia parsera, počty nezaradených a odhadovaných riadkov), tlačidlo **Zobraziť transakcie** a **Zmazať**. Staršie výpisy sú zbalené pod tlačidlom **Staršie výpisy (N)**. Stav bez otvorených kontrol nepotvrdzuje úplnosť bankových dát.

## 0.1.6 (2026-09-09)

Opravuje import výpisov z nového generátora PDF Tatra banky a zjednodušuje nastavenie účtu.

- Výpisy, ktoré od júla 2026 končili hláškou „not a Tatra banka statement: no IBAN line“, sa importujú. Text z PDF sa skladá podľa základne písma a kroku znakov, nie podľa rámčekov glyfov, a hlavička sa číta až po riadok tabuľky.
- Konečný zostatok sa prečíta aj vtedy, keď je súhrn v tom istom bloku ako posledná transakcia, takže kontrolný súčet výpisu opäť vychádza.
- Detail platby kartou, ktorý pokračuje na ďalšej strane, sa spojí so svojou transakciou. Zmizli hlásenia „Blok sa nepodarilo spracovať“.
- Nové názvy z výpisu appka pozná: nákup E-COMM sa počíta ako platba kartou a `Poplatok za účet` aj `Poplatky za transakcie` sa uložia bez hlásenia o neznámom type. Rozpis poplatkov je detail transakcie.
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
