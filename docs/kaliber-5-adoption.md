# Kaliber 5.0.1: základ vzhľadu

Pracovná vetva preberá základ Kaliber 5.0.1 (plán 125, krok A1).
Číslo vydanej appky zostáva 0.2.0. Historický plán 091 a poznámky
k vydaniu 0.2.0 opisujú vtedajší Kaliber r4.

- `src/tokens.css` je bajtová kópia výstupu `design-tokens css --app abakus`.
- `src/kaliber.css` je bajtová kópia `design/web/kaliber.css` verzie 5.0.1.
- `.gitattributes` zachováva bajty `src/kaliber.css` a súboru ikon aj v Gite.
- `public/kaliber-icons.svg` je bajtová kópia spoločného súboru ikon.
  Vite ho sprístupní na `/kaliber-icons.svg`. Výmena existujúcich ikon
  a doplnenie ikon do chybových stavov patrí do kroku A2.
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
Rozloženie, funkcie, klávesy a ukladanie dát sa nemenia.
