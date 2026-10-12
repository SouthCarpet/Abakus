# Plán 091: Abakus 0.2.0 preberá Kaliber revíziu 3

Zdroj: brief `animus/data/agent-runs/091_abakus-020-ux/r3/brief-a1-kaliber-r3.md`.
Pravidlá: špecifikácia Kaliber r3 (R12 až R22), revízia 2 (R1 až R11),
`rule_design_language.md` body 1 až 12, wiki `adoption.md` ("App shell
checklist", "Adoption backlog by app"), výskum `ux-research-returned.md`
oddiel 7 a `keyboard-research-returned.md` oddiel 6. Katalóg AI slop má
prednosť pred všetkým.

Stav pred zmenou (vetva `feature/084-summary-pdf`, `8f9d001`): Abakus nesie
Kaliber spred smeru A. `src/tokens.css` má písmo Inter, polomery 4/6/8 px a
tieň, `src/kaliber.css` je vlastný port so starými triedami (`.k-rail`,
`.k-kpi-*`, `.k-shell` ako mriežka appky).

## Identita (R16)

| Páka | Voľba | Dôvod |
|---|---|---|
| Akcent | odtieň 145 (bez zmeny) | `design/tokens/apps/abakus.json` aj `consumers.json` majú 145. `check_identity.py` hlási len pdf-editor (264), Abakus prejde. Prekryv so stavom success (150) je prijatý Michalom 2026-09-20 (`accepted_hue_overlaps`). |
| Neutrálny tón | odtieň 260, chroma 0,012 (bez zmeny) | Súbor tokenov patrí ekosystému, nie tejto vetve. Návrh pre Michala: ponechať, alebo prejsť na neutrály smeru A (200). |
| Navigácia | záložky v hlavičke (`.k-tabs`) | Abakus má 5 sekcií: Transakcie, Prehľad, Import, Kategórie, Nastavenia. R12: 5 a menej znamená záložky. `consumers.json` dnes uvádza `navigation-pane`; navrhujem zmenu na `header-tabs`. |
| Hlavná obrazovka | register, obrazovka Transakcie | Appka sa otvorí na Transakciách (R16 bod 3, príklad "the Abakus register"). Prehľad stratí dlaždice KPI (SLOP-LAYOUT-05) a dostane súhrnnú tabuľku `.k-summary`. |
| Glyfy | Lucide, 1,5 px, vložené SVG | Slnko, mesiac, šípka menu, krížik panela. Bez novej závislosti. Licencia ISC v `THIRD_PARTY_NOTICES.md`. |
| Dôraz pohybu | F2 atrament na riadkoch registra | Riadok, ktorý zmení kategóriu alebo stav (potvrdenie, priradenie, hromadné priradenie, Späť), dostane tón `accent-soft` a po `motion.ink_hold` (1600 ms) vyschne. Strop 6 zmien za sekundu; nad ním bez atramentu a pätička povie, čo sa zmenilo. Import: indeterminovaná čiara v hlavičke počas behu a datovaná poznámka v pätičke na konci (koncová forma P1; backend nedáva priebeh, preto bez determinovanej výplne). Téma: S1 prelínanie len po kliknutí. |

## Klávesy (R22, katalóg ledger:9299)

| Kláves | Akcia v Abakuse | Kde |
|---|---|---|
| Enter | Prijať odhad na zameranom riadku; uložiť jednoriadkové pole | Tabuľka Transakcie; dialógy |
| Šípky hore a dole | Posun zamerania po riadkoch | Tabuľka Transakcie |
| Home, End | Prvý, posledný riadok | Tabuľka Transakcie |
| PgUp, PgDn | O 10 riadkov | Tabuľka Transakcie |
| Shift+šípky | Rozšíriť výber od kotvy | Tabuľka Transakcie |
| Medzerník | Prepnúť výber zameraného riadku | Tabuľka Transakcie |
| Ctrl+A | Vybrať všetky viditeľné riadky (bez prevodov); druhé Ctrl+A výber zruší | Tabuľka Transakcie |
| Esc | Jedna vrstva na stlačenie: toast, menu alebo zoznam kategórií, text v hľadaní, výber, dialóg alebo panel Pomocník | Celá appka |
| Ctrl+Z | Vrátiť posledné hromadné priradenie | Transakcie, keď je Späť dostupné |
| Ctrl+Y, Ctrl+Shift+Z | Znova vykonať hromadné priradenie, ktoré Ctrl+Z vrátilo; iný zápis túto možnosť zruší | Transakcie |
| Del | Otvorí existujúci pomenovaný dialóg mazania s počtom, zameranie na Zrušiť; po zmazaní zameranie na ďalší riadok (alebo predchádzajúci pri poslednom). Nikdy nemaže bez dialógu. Del v textovom poli upravuje text. | Účty (Nastavenia), výpisy (Import), kategórie |
| Ctrl+F | Zamerať pole Hľadať | Transakcie |
| Ctrl+O | Otvoriť Import a výber PDF | Celá appka |
| F1 | Panel Pomocník | Celá appka |
| Ctrl+čiarka | Nastavenia | Celá appka |
| F5, Ctrl+R | Zablokovať obnovenie stránky (neuložený text v dialógoch) | Celá appka |

Rozhodnutie kontrolóra 2026-10-12: Del sa viaže na zoznamy s mazaním v režime `confirm` (katalóg: "standard where a delete exists") a redo k Ctrl+Z.

Nepriradené a prečo:

- **Ctrl+K**: Abakus nemá paletu príkazov; každý príkaz je na obrazovke alebo
  v menu Ďalšie. Katalóg ju neukladá.
- **Ctrl+P, Ctrl+S**: Abakus nič netlačí ani neukladá dokument. Export PDF je
  report, nie tlač.
- **Jednoznakové skratky** (`?`, `/`): žiadne. Preto Nastavenia nepotrebujú
  prepínač jednoznakových skratiek (WCAG 2.1.4).
- **F2**: premenovanie kategórie je inline pole, ktoré už má zameranie;
  samostatný kláves nepridáva hodnotu.

Pravidlá zhody: primárny modifikátor je Ctrl (Command na macOS), nikdy
`ctrlKey || metaKey`. Písmená podľa `key`, číslice podľa `code`, AltGr a
Ctrl+Alt sa ignorujú, kompozícia a mŕtve klávesy tiež (port
`kaliber-keys.js` do `src/kaliber/keys.ts`).

## App shell checklist (adoption.md) a medzery z výskumu

| # | Pravidlo | Abakus dnes | Zmena | Test |
|---|---|---|---|---|
| 1 | Jedna primárna akcia v riadku hlavičky obrazovky (R12) | `App.tsx:69` globálny panel s Exportom PDF; obrazovky majú `h2` bez akcie | Riadok `.k-pagehead` na každej obrazovke: Prehľad "Exportovať PDF", Import "Vybrať PDF", Kategórie "Nová kategória" | `App.test.tsx`: názov a primárne tlačidlo v hlavičke |
| 2 | Menu "Ďalšie" na konci riadku (R12, R13) | Transakcie majú voľný rad tlačidiel (`Transactions.tsx:648-652`) | Transakcie: menu "Ďalšie" s Obnoviť, Exportovať filtrované CSV, Exportovať PDF | test menu: položky, Esc vráti zameranie na spúšťač |
| 3 | Navigácia: záložky pre 5 sekcií (R12) | `Rail.tsx` ľavý pás | `.k-tabs` v hlavičke, `aria-current="page"` | test poradia a výberu záložky |
| 4 | Nastavenia posledné, Ctrl+čiarka | posledné v páse, bez klávesu | posledná záložka, Ctrl+čiarka | test klávesu |
| 5 | Téma: Svetlá, Tmavá, Podľa systému v Nastavenia > Vzhľad, predvolene systém, rýchly prepínač slnko a mesiac v hlavičke, kľúč `kaliber-theme` (R20) | select v globálnom paneli (`App.tsx:69`, `ThemePicker.tsx`), kľúč `abakus.theme` (`lib/theme.ts:5`) | segmentový prepínač v sekcii Vzhľad, `.k-switch--theme` v hlavičke, jeden stav, migrácia `abakus.theme` bez straty voľby, prvé vykreslenie skriptom `public/kaliber-theme-boot.js` (CSP zakazuje inline skript) | test migrácie, oboch smerov a znovunačítania |
| 6 | O aplikácii ako posledná sekcia Nastavení (R12) | chýba; verzia nikde (`Settings.tsx:449-462`) | sekcia O aplikácii: názov, značka, verzia, zostavenie, poznámky k vydaniu, Nahlásiť problém, licencie | test obsahu a poradia sekcií |
| 7 | Kontrola aktualizácií je voliteľná, s časom a výsledkom poslednej kontroly; nájdená verzia je tichý riadok (R20) | prepínač v karte Stav, výsledok bez času; indikátor ako tlačidlo | prepínač automatickej kontroly (predvolene vypnutý), tlačidlo Skontrolovať aktualizácie v O aplikácii, čas a výsledok poslednej kontroly, `.k-update-line` v hlavičke vedie do O aplikácii | test: bez súhlasu žiadne volanie; čas a výsledok |
| 8 | Pomocník v pätičke a F1, panel s hľadaním (R12, R15) | chýba (`Transactions.tsx:526` bez nápovedy) | odkaz Pomocník v pätičke každej obrazovky, F1, `.k-drawer` s poľom hľadať, témy podľa obrazovky, zoznam klávesov | test F1, hľadania, Esc a návratu zamerania |
| 9 | Hľadanie v nástrojoch hlavičky, Ctrl+F, Ctrl+K | pole Hľadať vo filtri Transakcií | Ctrl+F zameria toto pole; bez globálneho hľadania a palety (pozri Klávesy) | test Ctrl+F |
| 10 | Stav a činnosť: čiara v hlavičke, stav v pätičke, jedna zdvorilá živá oblasť | texty "Prebieha..." v obrazovkách | čiara činnosti počas importu a operácií, pätička so stavom, `aria-live="polite"` | test triedy `is-busy` |
| 11 | Deštruktívne akcie posledné a oddelené, nikdy primárne | dialógy s počtom už sú (`Import.tsx:360-367`) | ponechať; menu Ďalšie nemá deštruktívnu položku | existujúce testy |
| 12 | Toast vpravo dole so Späť (R12, R14) | riadok so Späť v obrazovke (`Transactions.tsx:659-664`), toast v toku stránky | `.k-toast-region` vpravo dole; hromadné priradenie: toast so Späť a Ctrl+Z | test toastu, Späť a Ctrl+Z |
| 13 | Jednoznakové skratky majú prepínač | žiadne nie sú | bez zmeny, zdôvodnené | test, že `?` nič nerobí |
| 14 | Esc vrstvy (R22) | Dialog a CategoryPicker zatvárajú, nič iné | zásobník vrstiev `src/kaliber/escape.ts`: toast, popup, text hľadania, výber, dialóg, panel | test každej vrstvy |
| 15 | Výber podľa modelu (R22) | zaškrtávacie políčka, bez klávesov | Shift+šípky, Shift+klik, medzerník, Ctrl+A, Esc | testy výberu |
| 16 | Nápovedy klávesov v tooltipoch a menu | žiadne | `title` a `aria-keyshortcuts` z jednej tabuľky väzieb (`src/kaliber/bindings.ts`) | test textu tooltipu |
| 17 | Identita v `consumers.json`, `check_identity.py` | riadok existuje, páky `null` | hodnoty v správe; register upravuje kontrolór | `py design/scripts/check_identity.py` |
| 18 | "Priečinok s dátami" dvakrát (SLOP-COPY-07) | `Settings.tsx:440`, `:450` | raz, v sekcii Údaje; počty účtov a výpisov tiež tam | test počtu výskytov |
| 19 | Export PDF v globálnom paneli (E11) | `App.tsx:69` | primárna akcia Prehľadu, položka menu Ďalšie v Transakciách | test umiestnenia |
| 20 | Dlaždice KPI (SLOP-LAYOUT-05) | `Overview.tsx:128-135`, `RecurringKpis.tsx:40` | `.k-summary` (dl, riadky so stupnicou) | testy Prehľadu |
| 21 | Písmo Atkinson Hyperlegible Next, tokeny smeru A | Inter, staré tokeny | `src/tokens.css` = `design/web/abakus.tokens.css`, `src/kaliber.css` = `design/web/kaliber.css` bajt po bajte, `@font-face` v `src/abakus.css`, triedy appky s predponou `ab-` | test zhody kópií, `tokens-only` |
| 22 | Zachovať: náhľady mazania s počtami, Späť pri hromadnom priradení | `Import.tsx:360-367`, `Transactions.tsx:420`, `:662` | ponechať | existujúce testy |
| 23 | Kontrola aktualizácií (Michal 2026-10-12): so zapnutou kontrolou pri spustení, pri otvorení sekcie O aplikácii a na tlačidlo; s vypnutou žiadna požiadavka na žiadnej z troch ciest. Kontrola pri spustení je tichá, len označí nájdenú verziu. | `Settings.tsx:307`, `:319` kontrola pri otvorení Nastavení a pri zapnutí; `commands.rs:386` zlúči chybu s "nič novšie" | `src/lib/updateCheck.ts` (`checkAtStart`, `checkNow`), App volá `checkAtStart`, Nastavenia kontrolujú pri otvorení O aplikácii a na tlačidlo; zapnutie samo nekontroluje; súhlas overuje aj backend (`commands.rs:382`) | `updateCheck.test.ts`, `App.shell.test.tsx`, `Settings.test.tsx` |
| 24 | Chyba spojenia nie je "máte najnovšiu verziu" | `commands.rs:386` `.ok().flatten()` | `check_update_now` vráti `Err` pri zlyhaní požiadavky, HTTP chybe a nečitateľnej odpovedi; riadok poslednej kontroly ich pomenuje | `src-tauri/tests/update.rs` (2 nové testy), `updateCheck.test.ts` |

## Doplnok 091 p3 (ledger seq 5433)

Syntetický akceptačný prípad Penny Neuss: výpis vo `fixtures/synthetic/`,
test nad trvalou databázou, ktorý overí presnú zhodu hľadania, náhľad
podobných platieb, hromadné potvrdenie a negatívny prípad obchodníka
(`Penny Berlin` a iný obchodník sa nezmenia).

## Kroky

1. Tento plán.
2. Tokeny, `kaliber.css`, písmo, triedy `ab-`.
3. Shell: hlavička, záložky, pätička, téma, Nastavenia (Vzhľad,
   Aktualizácie, O aplikácii), Pomocník, toast, menu Ďalšie.
4. Klávesy a výber.
5. Atrament F2 a čiara činnosti.
6. Prípad Penny Neuss.
7. Dokumentácia: README, CHANGELOG 0.2.0, `docs/release-0.2.0.md`,
   KNOWN_ISSUES.

Brány pred každým commitom: vitest, tsc, eslint, `slop_lint.py src`;
cargo test a clippy pri zmene Rustu.
