# Abakus 0.3.0

Rozsah: Kaliber 5.0.1 a úpravy rozhrania podľa plánu 125.

## Čo sa zmenilo

- Vzhľad používa Kaliber 5.0.1. Aktívna záložka má zvýraznenú linku. Malé stavové a navigačné značky sú kruhové. Vypnutý prepínač nemá znak.
- Ikony pochádzajú zo spoločnej sady Kaliber (Phosphor a vlastné kresby). Tlačidlá si zachovávajú názvy pre čítačku obrazovky.
- Chyby a problémové odznaky majú ikonu chyby. Platí to aj pre kontrolný súčet v Prehľade a v tabuľke Zostatky z výpisov, kontrolu importu, chýbajúcu pravidelnú platbu a chybu zápisu auditu. Správny súčet si ponecháva bodku.
- Všetky natívne výbery majú šípku rozbalenia. Šípku má aj sekcia O aplikácii.
- Pri minimálnej šírke okna 1024 px sa suma aj detail transakcie zmestia bez vodorovného posúvania registra. Text sa zalomí medzi slovami. Dlhý reťazec bez medzier sa môže zlomiť uprostred.
- V Importe zostáva tlačidlo zmazania výpisu dostupné aj pri nesediacom kontrolnom súčte a šírke 1024 px. Texty ostávajú vo svojich bunkách.
- Upozornenia v karte porovnania výdavkov podľa kategórií sú spojené do jedného hlásenia s krátkym zoznamom.

Podrobnosti sú v [poznámkach k prevzatiu Kaliber](kaliber-5-adoption.md). Obmedzenia sú v [KNOWN_ISSUES.md](../KNOWN_ISSUES.md).

## Overenie

Vetva pred prípravou vydania prešla nezávislou kontrolou kódu aj vzhľadu. Lokálne kontroly prešli: lint, frontendový build, 668 frontendových testov vrátane geometrických testov v prehliadači, 662 Rust testov a Clippy s varovaniami ako chybami. Repo nemá CI workflow.

## Inštalátor

- Súbor: `abakus-setup-0.3.0.exe`. Kontrolný súčet je v sprievodnom `SHA256SUMS.txt`.
- Zostavenie cez `packaging/build-installer.ps1` prešlo. Kontrola `packaging/check-payload.mjs` overila x64 PE, importy a pin PDFium pred balením.
- Inštalátor je pre aktuálneho používateľa a nepotrebuje práva správcu. Nie je podpísaný. Údaje a heslá zostávajú na počítači.
- Inštalátor ani zostavená aplikácia sa pri príprave tohto kandidáta nespúšťajú. Natívne prijatie zostáva na používateľovi.

- Inštalátor `abakus-setup-0.3.0.exe` (10 266 065 B) je zostavený z commitu `5bb400d`: SHA-256 `eb3cb1272c20023798c2a791f46b8cf35434d494bed610c8bec20b1da7fa2b08`. Balený `abakus.exe` má SHA-256 `f6d642d9112a15ec073867de0453f7cc9bd1af14d418c02302bc66d2e9ced271`, `pdfium.dll` má `215c405d13963395b38872a656a18f161a294e80bbcaa00849b842f524e5474b`. Riadok `SHA256SUMS.txt` je publikovaný s vydaním na GitHub.
- Commit po `5bb400d` mení len tieto poznámky. Kód zostavy sa nezmenil.
