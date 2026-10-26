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
Rozloženie, funkcie, klávesy a ukladanie dát sa nemenia.

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
