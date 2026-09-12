# Plan 091: skupiny transakcií podľa obchodníka a miesta

## Kontrakt zdrojového helpera

`groupUnassignedTransactions(rows)` je čistý frontendový helper. Vstupom je iba pole práve viditeľných `TxRow`, ktoré mu odovzdá volajúci. Helper nič nedoťahuje a nemení vstup. Výstup zachová poradie prvého výskytu skupín aj riadkov a pre každú skupinu obsahuje:

- `merchant`: pôvodný názov obchodníka z prvého riadka, určený iba na zobrazenie,
- `place`: pôvodné miesto z prvého riadka alebo `null`, určené iba na zobrazenie,
- `count`: počet jednoznačných riadkov v skupine,
- `ids`: explicitné ID týchto riadkov v poradí vstupu.

## Pravidlá výberu a zoskupenia

| Hranica | Pravidlo |
| --- | --- |
| Viditeľnosť | Helper pracuje len s dodanými viditeľnými riadkami. Skrytý riadok musí volajúci vynechať. |
| Stav | Zaradí iba `unassigned`; `suggested`, `confirmed` a `transfer` vynechá. |
| ID | Ak sa rovnaké ID vo vstupe opakuje, všetky jeho výskyty vynechá ako nejednoznačné. |
| Obchodník | Neprázdny zložený kľúč tvorí z porovnaného obchodníka a miesta. |
| Neznámy obchodník | Prázdny obchodník po zložení ostáva samostatnou jednopoložkovou skupinou. Viac neznámych riadkov sa nikdy nezlúči. |
| Miesto | Chýbajúce miesto (`null`) sa nezhoduje so žiadnym pomenovaným miestom. Pomenované miesta sa porovnávajú po rovnakom zložení ako obchodník. |
| Poradie | Skupiny aj ID zachovávajú poradie prvého výskytu; helper nič netriedi. |

Zloženie štítkov cielene kopíruje pre podporované prípady zdrojové kroky `parser::fold`: kanonický rozklad NFD, odstránenie kombinujúcich značiek, malé písmená a zbalenie medzier. Testy pokrývajú slovenské akcenty, rozložené značky, veľkosť písmen a bežné biele znaky. Interpunkcia a identifikačné tokeny ostávajú významné. Implementácia netvrdí všeobecnú Unicode ekvivalenciu JavaScriptu s Rust crate `unicode-normalization` mimo týchto pokrytých prípadov.

Zobrazované `merchant` a `place` sa zámerne neprepisujú zloženou podobou. Preto môžu mať pôvodnú veľkosť písmen alebo medzery; prvý riadok skupiny je ich jediným zdrojom. Helper neportuje `rules::normalize`, nemá identitu naučeného pravidla a nenahrádza backendové `apply_to_matching`.

## Stav integrácie

UI napojenie je samostatná, zatiaľ neimplementovaná práca. Budúce UI môže backendu odovzdať iba explicitné `ids` z vybranej skupiny. Žiadne rozšírenie zhody na backende nie je týmto helperom implicitné.
