# Abakus

## Čo je Abakus

Abakus je lokálna desktopová appka na analýzu PDF výpisov z Tatra banky. Rozdeľuje transakcie do kategórií a zobrazuje príjmy a výdavky v grafoch. Bankové údaje nikdy neopustia váš počítač. GitHub obsahuje iba zdrojový kód.

## Záruky súkromia

- [x] Appka neobsahuje sieťový kód okrem voliteľnej kontroly aktualizácií. Tá je predvolene vypnutá. Po zapnutí vykoná jeden `GET` na GitHub Releases a zapíše ho do denníka siete v appke.
- [x] CSP povoľuje pripojenie iba pre internú komunikáciu Tauri. Nepovoľuje externý `connect-src`.
- [x] Automatický test kontroluje závislosti. Zakazuje sieťové knižnice a sieťové pluginy Tauri. Jedinou povolenou výnimkou je klient pre voliteľnú kontrolu aktualizácií.
- [x] Kontrola pred commitom blokuje PDF súbory a skutočné slovenské IBAN-y. Povoľuje iba výslovne uvedené syntetické testovacie IBAN-y.
- [x] Dáta sa ukladajú do `%LOCALAPPDATA%\Abakus`.
- [x] Heslá k PDF sa ukladajú do Správcu poverení systému Windows. Nie sú v databáze.

## Inštalácia pre vývoj

Potrebujete Windows, PowerShell 7, Rust pre cieľ `stable-x86_64-pc-windows-msvc`, Node.js a npm.

Najprv raz stiahnite lokálnu knižnicu PDFium:

```powershell
pwsh -File .\scripts\fetch-pdfium.ps1
```

Potom nainštalujte balíky a spustite appku:

```powershell
npm install
npm run tauri dev
```

Pred testami nastavte cestu k PDFium. Potom spustite testy:

```powershell
$env:ABAKUS_PDFIUM_DIR = "$PWD\src-tauri\resources\pdfium"
cargo test --jobs 4 --workspace
```

## Prvé použitie

1. V časti **Nastavenia** pridajte vlastné účty. Zadajte IBAN alebo zápis v tvare `kód banky/prefix-číslo účtu`.
2. V časti **Import** pridajte mesačné PDF výpisy. Pri zamknutom PDF zadajte heslo. Ak zvolíte **Zapamätať pre tento účet**, appka ho pri ďalšom importe použije automaticky.
3. V časti **Transakcie** skontrolujte navrhnuté kategórie. Odhad potvrdíte jedným kliknutím.
4. V časti **Prehľad** si pozrite príjmy, výdavky, kategórie a vývoj v čase.

Každý importovaný výpis má odznak kontrolného súčtu. Appka overí, či počiatočný zostatok a všetky transakcie dávajú konečný zostatok. Text **Kontrolný súčet nesedí** znamená, že sa tieto hodnoty líšia o uvedenú sumu. Výpis sa importuje, ale zostane označený na kontrolu. Ak chýba potrebný zostatok, appka uvedie, že súčet nevie overiť.

## Ako sa appka učí

Po potvrdení si appka zapamätá presnú kombináciu obchodníka a miesta. Rovnakého obchodníka na inom mieste označí ako **odhad**, ktorý potvrdíte jedným kliknutím.

## Kontrola aktualizácií

Kontrola aktualizácií je v predvolenom stave vypnutá. Zapnete ju v časti **Nastavenia**. Appka potom vykoná jeden neautentifikovaný `GET` na najnovšie vydanie v GitHub Releases. Neposiela bankové údaje, prihlasovacie údaje ani telemetriu. Aktualizáciu nesťahuje, neinštaluje ani automaticky neotvára.

Každý pokus sa zobrazí v **Nastavenia > Stav > Sieťová aktivita**. Denník uvádza čas, adresu a výsledný stav.

## Kde sú dáta a ako zálohovať

Databáza je v súbore `%LOCALAPPDATA%\Abakus\abakus.db`. Pred zálohovaním appku zatvorte. Potom tento súbor skopírujte na bezpečné miesto.

## Čo appka nikdy nerobí

- Neukladá dáta do cloudu a nesynchronizuje ich.
- Nezbiera telemetriu.
- Nepoužíva LLM ani online obohacovanie údajov.
- Nesťahuje ani neinštaluje aktualizácie automaticky.

## Licencia a stav

Abakus je vo verzii 0.1 a používa licenciu MIT. Aktuálne obmedzenia a otvorené problémy sú v súbore [KNOWN_ISSUES.md](KNOWN_ISSUES.md).
