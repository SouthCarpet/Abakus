# Abakus 0.1.6

Rozsah: parser (`crates/parser/src/lines.rs`, `header.rs`, `pdfium.rs`, `statement.rs`, `blocks.rs`, `card.rs`), diagnostický CLI (`abakus-cli lines`, `geometry`), UI (`PasswordInput.tsx`, `SetupAccountDialog.tsx`, `Import.tsx`, `SetPasswordDialog.tsx`, `kaliber.css`), súbory s číslom verzie, README, `crates/parser/README.md`, CHANGELOG, KNOWN_ISSUES.

## Čo sa zmenilo

- Nový generátor PDF Tatra banky (výpisy od júla 2026) hlási v PDFium rámčeky glyfov namiesto šírky bunky a vkladá syntetické medzery s nulovou šírkou. Riadky sa teraz skladajú podľa základne písma a medzery podľa mediánu kroku znakov; staré výpisy idú starou cestou bez zmeny. Hlavička sa číta až po prvý riadok tabuľky, nie len prvých 12 riadkov.
- Blok bez začiatku záznamu (detail platby cez zlom strany, rozpis poplatkov za kratším oddeľovačom) sa pripojí k predchádzajúcej transakcii. Blok, v ktorom je súhrn zliaty s poslednou transakciou, sa rozdelí pri prvom súhrnnom riadku, takže konečný zostatok a kontrolný súčet vychádzajú.
- `nákup E-COMM` je platba kartou; popis začínajúci na `poplat` je známy poplatok (druh `Other`, samostatný druh `Fee` je odložený) a už nevypíše varovanie o neznámom type.
- Dialóg `Účet nie je nastavený` sa otvorí sám pri výpise z neznámeho účtu: názov, druh, voliteľné heslo, uloženie a opakovaný import. Viac výpisov sa pýta po jednom.
- Vlastné tlačidlo `Zobraziť` a `Skryť` pri poliach s heslom; natívna ikona WebView2 je vypnutá.

## Overenie

Overenie: doplní kontrolér.
