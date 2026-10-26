# Poznámky k vydaniu

Najnovšia verzia je hore. Text opisuje, čo vidí používateľ.

## Unreleased

- Základ vzhľadu Kaliber 5.0.1: aktívna záložka má zvýraznenú linku, malé stavové a navigačné značky sú kruhové a vypnutý prepínač nemá znak. Obnovené tokeny pridávajú prekrytie vypnutých ovládačov a rozmery ikon.
- Pripravený súbor ikon `public/kaliber-icons.svg`. Výmena ikon nasleduje samostatne.

## 0.2.0 (2026-10-18)

- Nový vzhľad podľa jazyka dizajnu Kaliber, revízia 4: písmo Fira Sans, plochý register s linkami, súhrnná tabuľka namiesto dlaždíc v Prehľade a päť záložiek v hlavičke. Appka sa otvorí na **Transakciách**.
- Transakcie ukazujú v popise obdobie a účet, register začína vyššie (len krátka nápoveda ku klávesom, podrobnosti a postup Zaradiť podľa obchodníka sú v **Pomocníkovi**), políčka v riadkoch majú na telefóne plochu 44 × 44 px a výber kategórie má šípku rozbalenia. Potvrdenia mazania skloňujú správne (1 pravidlo, 2 pravidlá, 5 pravidiel; Odstránia sa 2 transakcie). Grafy majú jeden rámik.
- Každá obrazovka má v hlavičke názov, jednu hlavnú akciu a menu **Ďalšie**. **Exportovať PDF** je v hlavičke Prehľadu a v menu Ďalšie v Transakciách, nie v lište nad každou obrazovkou.
- Vzhľad má prepínač so slnkom a mesiacom v hlavičke a voľbu **Svetlý**, **Tmavý**, **Podľa systému** v **Nastavenia > Vzhľad**. Doterajšia voľba sa prenesie.
- Nastavenia majú sekcie **Vzhľad**, **Sieťová aktivita** a poslednú **O aplikácii** (verzia, zostavenie, poznámky k vydaniu, nahlásenie problému, licencie, kontrola aktualizácií). Text „Priečinok s dátami“ je na obrazovke raz.
- Kontrola aktualizácií, keď ju zapnete, beží pri spustení (ticho), pri otvorení **O aplikácii** a na tlačidlo **Skontrolovať aktualizácie**. Vypnutá neposiela nič. Chyba spojenia sa ukáže inak ako „máte najnovšiu verziu“, s časom poslednej kontroly.
- **Pomocník** je v pätičke každej obrazovky a na F1: panel s hľadaním, témami a zoznamom klávesov.
- Klávesy: Ctrl+O (import), Ctrl+čiarka (Nastavenia), Ctrl+F (hľadanie), Ctrl+Z a Ctrl+Y alebo Ctrl+Shift+Z (vrátiť a znova vykonať hromadné priradenie), v tabuľke Home, End, PgUp, PgDn, medzerník, Shift+šípky, Ctrl+A a Esc. Del na účte, výpise alebo kategórii otvorí potvrdenie zmazania s fokusom na **Zrušiť**. Esc zatvára vždy len jednu vrstvu. F5 a Ctrl+R stránku neobnovia, aby sa nestratil rozpísaný text. Klávesy s AltGr nič nespustia.
- Výsledky akcií sú hlásenia vpravo dole; hromadné priradenie má v hlásení **Späť**. Riadok, ktorý zápis zmení, sa na chvíľu podfarbí.
- Export do PDF má okrem úplného výpisu dva sumáre: **Sumár na 1 stranu** (KPI, graf, najväčšie kategórie, pokrytie výpismi) a **Sumár do 3 strán** (navyše porovnanie s predchádzajúcim obdobím rovnakej dĺžky, súčty po účtoch, vývoj po mesiacoch, 10 najväčších transakcií a pravidelné platby ako odhady). Sumáre netvrdia úplnosť transakcií. Rozsah vyberiete v dialógu exportu (**Rozsah dokumentu**), náhľad ukáže počet strán a navrhnutý názov súboru má príponu `-sumar-1` alebo `-sumar-3`.
- Vyhľadávanie v Transakciách spája obchodníka s miestom: `Penny Neuss` nájde daného obchodníka v Neusse a vylúči `Penny Berlin`. Nerozlišuje veľkosť písmen ani diakritiku a znaky `%` a `_` berie doslovne.
- Výber kategórie (priradenie aj filter v Transakciách) má nové ovládanie s vyhľadávaním podľa názvu. Písanie ignoruje diakritiku aj veľkosť písmen. Prázdny výsledok ukáže text „Žiadna kategória sa nenašla.“. Šípky hore a dole a Enter vyberú kategóriu z klávesnice, Escape zavrie zoznam a vráti fokus na pôvodné tlačidlo.
- Potvrdenie odhadu ponúka aj potvrdenie ostatných nepotvrdených transakcií rovnakého obchodníka. Počet takých transakcií je vidieť pri tlačidle **Potvrdiť**.
- Nové tlačidlo **Potvrdiť vybrané** potvrdí odhady vo vybraných riadkoch naraz, rovnako ako doterajšie hromadné priradenie kategórie.
- Po hromadnom priradení kategórie appka na chvíľu ponúkne tlačidlo **Späť**, ktoré priradenie vráti a obnoví tabuľku. Späť zatiaľ nefunguje pre hromadné potvrdenie.
- Prehľad hore ukáže pripomienku, keď appke chýba výpis za predchádzajúci mesiac (sedem dní po konci mesiaca). Text hovorí o medzere v pokrytí, nie o chýbajúcich transakciách. Tlačidlo **Prejsť na import** pri pripomienke otvorí Import.
- Kliknutie na obchodníka v tabuľke **Najčastejší obchodníci** otvorí Transakcie s vyplneným hľadaním podľa jeho mena.
- Nová karta **Porovnanie s rovnakým obdobím vlani** vo vlastnom výbere mesiaca a dĺžky (jeden mesiac alebo tri mesiace) ukáže aktuálne aj vlaňajšie obdobie pri číslach príjmov, výdavkov a čistého súčtu.
- Novú kategóriu aj podkategóriu možno vytvoriť priamo pri priraďovaní transakcie výberom **Nová kategória...**, bez prechodu do Kategórií. Appka rovno priradí novú kategóriu danej transakcii.
- Archivácia kategórie v **Kategóriách** má nové kompaktné ikonové tlačidlo namiesto textového. Nové ikonové tlačidlo kategóriu zmaže; červené tlačidlo je až v potvrdení. Pred zmazaním appka ukáže, koľko transakcií sa presunie do nezaradených, a po zmazaní výsledok potvrdí.
- Tabuľka **Pravidlá** v Kategóriách má priamo v riadku výber cieľovej kategórie na zmenu bez opustenia obrazovky. Tiché ikonové tlačidlo zmazania pri pravidle (názov **Zmazať pravidlo** a kľúč) najprv ukáže, koľko otvorených riadkov sa naň odkazuje a koľko z nich po zmazaní zmení zaradenie.
- Filter **Druh** v Transakciách ponúka aj bankový poplatok (**Poplatok**). Prehľad má nový riadok **Poplatky** v súhrnnej tabuľke a graf **Príjmy a výdavky** má vlastný stĺpec Poplatky. Rady grafu sa líšia odtieňom aj vzorom obrysu (plný, bodkovaný, dlhé čiarky) a legenda ukazuje tú istú čiaru. Popisy osi sú skrátené (`1,5 tis. €`) a nezalamujú sa. Graf **Podľa kategórií a mesiacov** bez dát ukáže vetu namiesto prázdneho rámu.
- Nad tabuľkou Transakcií sa objavia skupiny nezaradených platieb podľa obchodníka, napríklad „Twitch, 15 platieb, nepriradené“. Kliknutie na skupinu vyberie jej riadky pre hromadné priradenie alebo potvrdenie.
- Tabuľka Transakcií sa dá ovládať klávesnicou: šípky, Home, End, PgUp a PgDn presúvajú zvýraznený riadok, Enter potvrdí jeho odhad. Fokus vo vyhľadávaní, poznámke alebo vo výbere kategórie tieto klávesy nepoužije.
- Tichý riadok **Nová verzia** je v hlavičke appky na každej obrazovke. Ukáže sa, len keď kontrola aktualizácií (vypnutá, kým ju sami nezapnete) našla novšie vydanie. Kliknutím prejdete do **Nastavenia > O aplikácii**. Riadok sám nič nekontroluje.
- Zoznam **Sieťová aktivita** v Nastaveniach po 10 riadkoch zbalí zvyšok pod tlačidlo **Zobraziť všetky (N)**, s tlačidlom **Zbaliť** na návrat. Počet skutočne nahraných riadkov sa nemení.
- V **Nastaveniach**, hneď pod zálohou, tlačidlo **Vybrať zálohu** obnoví databázu zo súboru zálohy. Appka najprv ukáže náhľad (počet účtov, výpisov a transakcií) a v tom istom dialógu pripomenie, že najprv vytvorí bezpečnostnú kópiu súčasnej databázy, aj že heslá zo Správcu poverení v zálohe nie sú. Súbor, ktorý nie je zálohou Abakusu, appka odmietne so zrozumiteľnou správou; rovnako odmietne zálohu vytvorenú novšou appkou, než akú appka podporuje. Pred obnovou vždy vytvorí bezpečnostnú kópiu súčasnej databázy a ukáže jej cestu. Pri zlyhaní appka vráti pôvodnú databázu; ak sa nahradená databáza nedá znova otvoriť, odloží ju pod menom `abakus-obnova-zlyhala-*` a pôvodnú vráti späť. Jediný krok, ktorý sa nedá vrátiť, je zlyhanie tohto odloženia: vtedy ostanú pôvodné dáta netknuté v pomenovanom odloženom súbore aj v bezpečnostnej kópii a appka ich sama znova nepripojí. Po obnove sa v Nastaveniach aj v Transakciách znova načítajú aktuálne dáta (účty, výpisy, sieťový log, kategórie); rozpracované Späť a výber riadkov z obdobia pred obnovou sa zrušia.
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
