# Abakus 0.1.5

Oprava inštalátora. Rozsah: len `packaging/abakus.iss`, žiadny Rust ani UI
kód appky sa nemenil.

## Čo sa zmenilo

- Sprievodca inštalátora teraz pri druhom spustení povie, že Abakus je už
  nainštalovaný, akú verziu má a kde. Podľa nájdenej verzie sa spýta na
  aktualizáciu, preinštalovanie alebo (pri novšej nainštalovanej verzii)
  na nahradenie staršou; downgrade je predvolene odmietnutý, aj v tichej
  inštalácii.
- Pri aktualizácii alebo preinštalovaní sprievodca vynechá stránku s
  odkazmi (Start/plocha), pretože predchádzajúcu voľbu si Inno Setup už
  pamätá, a na uvítacej stránke napíše, z akej verzie na akú aktualizuje.
- `AppId` má nový vývojársky prepínač `/DAppIdGuid=<guid>` (packaging/INSTALL.md,
  developer-only), ktorý testovaciu inštaláciu/odinštaláciu izoluje od
  skutočnej. Bežný release build ho nepoužíva; skutočný `AppId` je
  nezmenený.

## Čo sa nezmenilo

- Import, kategórie, pravidelné platby, PDF report, zálohy a kontrola
  aktualizácie v appke sa nemenili.
- Modul WebView2 Runtime sa kontroluje rovnako ako v 0.1.4.

## Overenie

Overenie: doplní kontrolér.
