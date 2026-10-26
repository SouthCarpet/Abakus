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

Vetva pred prípravou vydania prešla nezávislou kontrolou kódu aj vzhľadu. Lokálne kontroly prešli: lint, frontendový build, 657 frontendových testov (11 vynechaných), 662 Rust testov a Clippy s varovaniami ako chybami. Repo nemá CI workflow. Výsledky a presný commit zostavy sú v správe k vydaniu.

## Inštalátor

- Súbor: `abakus-setup-0.3.0.exe`. Kontrolný súčet je v sprievodnom `SHA256SUMS.txt`.
- Zostavenie cez `packaging/build-installer.ps1` prešlo. Kontrola `packaging/check-payload.mjs` overila x64 PE, importy a pin PDFium pred balením.
- Inštalátor je pre aktuálneho používateľa a nepotrebuje práva správcu. Nie je podpísaný. Údaje a heslá zostávajú na počítači.
- Inštalátor ani zostavená aplikácia sa pri príprave tohto kandidáta nespúšťajú. Natívne prijatie zostáva na používateľovi.

Hash finálneho inštalátora je v `SHA256SUMS.txt`. Kontrolné súčty baleného `abakus.exe` a `pdfium.dll`, presný commit a veľkosť balíka sú v správe k vydaniu.
