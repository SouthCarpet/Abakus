# Abakus 0.1.6

Rozsah: parser (`crates/parser/src/lines.rs`, `header.rs`, `pdfium.rs`), diagnostický CLI (`abakus-cli lines`, `geometry`), UI (`PasswordInput.tsx`, `SetupAccountDialog.tsx`, `Import.tsx`, `SetPasswordDialog.tsx`, `kaliber.css`), súbory s číslom verzie, README, CHANGELOG, KNOWN_ISSUES.

## Čo sa zmenilo

- Nový generátor PDF Tatra banky (výpisy od júla 2026) hlási v PDFium rámčeky glyfov namiesto šírky bunky a vkladá syntetické medzery s nulovou šírkou. Riadky sa teraz skladajú podľa základne písma a medzery podľa mediánu kroku znakov; staré výpisy idú starou cestou bez zmeny. Hlavička sa číta až po prvý riadok tabuľky, nie len prvých 12 riadkov.
- Dialóg `Účet nie je nastavený` sa otvorí sám pri výpise z neznámeho účtu: názov, druh, voliteľné heslo, uloženie a opakovaný import. Viac výpisov sa pýta po jednom.
- Vlastné tlačidlo `Zobraziť` a `Skryť` pri poliach s heslom; natívna ikona WebView2 je vypnutá.

## Overenie

Overenie: doplní kontrolér.
