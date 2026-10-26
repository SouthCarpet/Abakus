# Kaliber 5.0.1: základ vzhľadu

Pracovná vetva preberá Kaliber 5.0.1 (plán 125, kroky A1 a A2).
Číslo vydanej appky zostáva 0.2.0. Historický plán 091 a poznámky
k vydaniu 0.2.0 opisujú vtedajší Kaliber r4.

- `src/tokens.css` je bajtová kópia výstupu `design-tokens css --app abakus`.
- `src/kaliber.css` je bajtová kópia `design/web/kaliber.css` verzie 5.0.1.
- `.gitattributes` zachováva bajty `src/kaliber.css` a súboru ikon aj v Gite.
- `public/kaliber-icons.svg` je bajtová kópia spoločného súboru ikon.
  Vite ho sprístupní na `/kaliber-icons.svg`. Všetky ikony ho používajú
  cez `Icon` v `src/kaliber/icons.tsx`. Vedľa textu majú 16 px, samostatné
  ikony a spodná navigácia 20 px. Dekoratívne SVG majú `aria-hidden`;
  samostatné tlačidlá si zachovávajú prístupný názov.
- Aktívna záložka má vybraný odtieň, tučný text a 2 px zvýraznenú linku.
  Značka `is-dirty` je určená len pre neuložené zmeny. Terajšie sekcie
  taký stav nemajú, preto ju nedostávajú. Bodka sa nehýbe.
- Stavové a navigačné značky sú kruhové. Stav prevodu si zachováva čiaru.
- Vypnutý prepínač nemá znak. Zapnutý má čiaru. Vypnuté ovládače
  používajú spoločné pravidlá Kaliber. Natívny select a tlačidlá si podľa
  Kaliber 5 zachovávajú sivú výplň. Vlastné pravidlo sivej výplne výberu
  kategórie je odstránené; jeho vzhľad riadi spoločný select.

Nové tokeny:

- `--color-disabled-overlay`: prekrytie vypnutého ovládača.
- `--icon-sm`: ikona 16 px.
- `--icon-md`: ikona 20 px.
- `--icon-lg`: ikona 24 px.
- `--icon-stroke-units`: hrúbka ťahu v súradniciach ikony.
- `--icon-grid-units`: veľkosť súradnicovej mriežky ikony.

Appka už neprepisuje selektory `.k-*` v `src/abakus.css`. Tieto vlastné
triedy zachovávajú doterajšie rozmery a rozloženie:

- `.ab-bottom-nav-item`: rozmery a výplň tlačidla dolnej navigácie.
- `.ab-theme-segmented`: najväčšia šírka voľby vzhľadu.
- `.ab-page-head`: okraj názvu obrazovky v hlavičke.
- `.ab-filter-select-wrap`: mriežka obalu filtra.
- `.ab-filter-input`: šírka poľa hľadania.
- `.ab-filter-select`: šírka výberu vo filtroch vrátane kategórie.

Pravidlo šírky priamych tlačidiel sekcie používa existujúcu `.ab-btn`.
Funkcie, klávesy a ukladanie dát sa nemenia. Opravy úzkeho registra sú uvedené nižšie.

## Rozhranie ikon (A2)

- `Icon.name`: povinný názov z typu `IconName`. Odkazuje na `k-icon-<name>`.
- `Icon.small`: voliteľná voľba 16 px (`k-icon--sm`), inak 20 px.
- `Icon.className`: voliteľné triedy pre umiestnenie alebo existujúci prechod.
  Komponent vždy zachováva `k-icon`, `aria-hidden="true"` a `focusable="false"`.
- `InlineAlert.children`: text alebo obsah hlásenia.
- `InlineAlert.announce`: predvolene `true`. Statické správy importu a
  pokrytia používajú `false`, aby sa z nich nestali nové živé hlásenia.
- `InlineAlert.kind`: voliteľný typ hlásenia, predvolený je `error`.

## Stavy a priebeh (A2)

- `InlineAlert` používa `k-alert`, `k-alert-body` a malú `k-alert-icon`.
  Typy `info`, `success`, `warning`, `error` majú ikony rovnakého významu;
  `error` používa `danger`. Chyba a varovanie majú rolu `alert`, ostatné `status`.
- `OperationStatus` zobrazuje chybu ako `k-state is-error` s ikonou `danger`
  v `k-state-icon`. Text chyby zostáva prístupný.
- `ToastItem.kind` rozlišuje `success` a `error`. Tretí voliteľný parameter
  `toast` vyberá typ, predvolený je `success`. Neúspešné vrátenie priradenia
  používa `k-toast--error` a `danger` v `k-toast-dot`. Potvrdenia majú bodku.
- Import, záloha, obnova, export CSV/PDF, načítanie a aktualizácia neposkytujú
  percento priebehu. Existujúca spoločná čiara aktivity a texty zostávajú.
  Počet vybraných súborov ani číslo fázy nie sú percentom spracovania.
- Čiary legendy grafu a stĺpce kategórií zostávajú kresbami grafu.
- Licencia Phosphor (MIT) je v `assets/icons/PHOSPHOR-LICENSE.txt` a
  `THIRD_PARTY_NOTICES.md`. Generátor licencií zachováva aj tieto poznámky.

## Opravy po vizuálnej kontrole (fix1)

- `DangerBadge.children`: text problémového odznaku. Malá skrytá ikona `danger` nahrádza bodku; `.ab-badge-danger` používa farbu chyby a odstráni pôvodnú pseudo bodku.
- `Card.badgeDanger`: voliteľná hodnota `boolean`, predvolene `false`. Import ju zapne len pre stav `error` (Chyba).
- Odznaky kontroly výpisu zobrazia `danger` pri chybe načítania a pri stave `needs_attention`. Odznak kontrolného súčtu ju zobrazí pri `off_by`. Správny súčet si ponechá bodku.
- Ďalší problémový odznak je `RecurringTable`, stav `missing` (Chýba platba). Stav `active` ostáva s bodkou. Zmeny ceny, odhad ukončenia a neznáme údaje sú informácie, nie potvrdené chyby.
- Karta nesediaceho súčtu v Prehľade má statický `InlineAlert` typu `error` bez novej živej roly. Názov chýb zápisu auditu ostáva odsekom a dostane malú skrytú ikonu `danger` cez `.ab-notice-title`.
- Dialóg `CategoryDialog` používa pre oba výbery `k-select-wrap` a `k-select-chevron`. O aplikácii používa priamu ikonu `k-chevron` v súhrne, ktorú spoločný Kaliber pri otvorení otočí.
- Pri šírke 600 až 1100 px má `.ab-register` pevnú tabuľkovú mriežku. Text a akcie sa môžu zalomiť; suma, výber kategórie a poznámka ostávajú v regióne. Detailný zdrojový text používa zalomenie. Pri 1280 px zostáva automatická šírka stĺpcov.
- Varovania v porovnaní kategórií tvoria jeden statický `InlineAlert` so zoznamom `.ab-warning-list`. Pri úplných dátach a uzavretom období sa nezobrazí.
- Regresné testy `fix1` overujú odznaky, obe chybové správy, šípky a zoznam varovaní. `layout-probe.mjs` meria pri 1024 × 680 a 1280 × 820 sumu, otvorený detail, posun regiónu aj presah stránky.

## Opravy po druhej vizuálnej kontrole (fix2)

- Všetky natívne výbery v appke používajú existujúci vzor `k-select-wrap` a malú skrytú ikonu `k-select-chevron`. Zahŕňa aj samostatné formuláre vytvorenia a úpravy na obrazovke Kategórie, vytvorenie a úpravu účtu v Nastaveniach, nastavenie neznámeho účtu po importe, oba výbery PDF exportu a interval pravidelnej platby. Výber kategórie `CategoryPicker` je vlastné tlačidlo s existujúcou šípkou, nie natívny select.
- Import pri šírke 600 až 1100 px používa pre riadky výpisov mriežku s pevným miestom pre zmazanie. Texty účtu, kontrolného súčtu a akcie sa zalomia. Tlačidlo zmazania ostáva v karte aj pri `off_by`. Pri 1280 px zostáva pôvodný riadok.
- Tabuľka Zostatky z výpisov v Prehľade používa pri `off_by` existujúci `DangerBadge`. Ostatné stavy ostávajú textom.
- Nepribudol žiadny token, verejný parameter ani nová trieda. Tri kópie Kaliber ostávajú nezmenené.
- Testy `fix2` otvárajú formuláre z Kategórií, Nastavení, Importu, Prehľadu a panelu pravidelných platieb. Spoločná kontrola overí všetky natívne výbery na otvorenej obrazovke alebo vo formulári vrátane budúcich doplnení. Geometrická sonda otvorí Import pri 1024 × 680 a 1280 × 820 s jedným `off_by` a dvoma správnymi súčtami a overí polohu všetkých tlačidiel zmazania.

## Opravy po tretej vizuálnej kontrole (fix3)

- Riadky výpisov v Importe majú pri šírke 600 až 1100 px pre každý textový stĺpec spodnú hranicu v `ch`, ktorá pojme najdlhšie slovo. Slová sa zalomia len medzi slovami. Pravidlo `overflow-wrap: anywhere` zmizlo z buniek riadku. Len bunka účtu má `overflow-wrap: break-word`, lebo označenie účtu môže byť dlhý reťazec bez medzier (IBAN). Tlačidlo „Zobraziť transakcie“ sa pri úzkom stĺpci zalomí a rastie do výšky.
- Register transakcií pri šírke 600 až 1100 px už nemá `overflow-wrap: anywhere` na každej bunke. Platí len pre bunku obchodníka a bunku účtu, kde sa môže vyskytnúť dlhý reťazec bez medzier.
- Dialóg Exportovať PDF sa nemenil. `.ab-dialog-wide` má `overflow: auto` a `max-height`, takže sa pri 1024 × 680 posúva ako celok a „Uložiť PDF“ je po posunutí v okne. Dokazuje to geometrická kontrola.
- Testovacia pomôcka `expectSelectChevrons` kontroluje každý natívny `select`, nielen `select.k-select`.
- Geometrická sonda meria text každej bunky Importu a registra pri 1024 × 680 a 1280 × 820: každý riadok textu leží v bunke aj v riadku, bunka sa neposúva sama v sebe a žiadne slovo nie je rozdelené medzi riadky (dva susedné znaky jedného slova na rôznych riadkoch). Dlhý reťazec od 20 znakov bez medzier sa smie zlomiť. Takto by sa zachytilo „Vyžad / uje“.
- Nepribudol žiadny token, verejný parameter ani nová trieda. Tri kópie Kaliber ostávajú nezmenené.
