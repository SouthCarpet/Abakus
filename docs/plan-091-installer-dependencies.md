# Plán 091, bod 6: závislosti inštalátora

## Rozsah

Kontrola sa týka baleného EXE, PDFium a registrácie WebView2. Nevytvára
správcu systémových závislostí. SQLite je zabudovaný. Overené importy
existujúceho EXE a PDFium nevyžadujú samostatný VC++ Redistributable.
Zoznam importov je obmedzený na overené Windows DLL. Nový import alebo
delay import zastaví balenie a vyžaduje nové posúdenie.

Windows 10+ vychádza z [podpory cieľa Rust](https://doc.rust-lang.org/rustc/platform-support.html),
riadku `x86_64-pc-windows-msvc`. Existujúca politika `x64compatible`
zostáva. Natívne správanie na ARM64 nie je overené.

## Pôvod PDFium

Pin v `packaging/pdfium-pin.json` odkazuje na presný existujúci release
`chromium/7469`, nie na latest. Archív bol 12. 9. 2026 overený proti
udržiavanému `scripts/pdfium.sha256` pred rozbalením. Hash jeho člena
`bin/pdfium.dll` sa zhodoval s dodávanou DLL. Žiadna DLL sa nenahradila
ani nespustila. Tým vznikol samostatný dôveryhodný oracle pre balenie.

Samotný hash budúceho EXE nie je oracle správnosti zostavenia. Kontrola PE
zachytáva hlavičky, architektúru, skrátené sekcie a importy. Zostavenie,
verzia zdrojov, dynamické načítanie DLL a runtime test sú samostatné dôkazy.

## Pravidlá správania

- `MinVersion=10.0`: Windows 10 alebo novší, s existujúcou politikou x64 kompatibility.
- Povinná kontrola payloadu: rovnaká aj pri `-SkipBuild`.
- `pdfium-pin.json.source`: presná URL pôvodného archívu.
- `pdfium-pin.json.archiveSha256`: dôveryhodný hash archívu, musí súhlasiť so `scripts/pdfium.sha256`.
- `pdfium-pin.json.member`: člen archívu, z ktorého sa odvodil pin DLL.
- `pdfium-pin.json.dllSha256`: overená identita rozbalenej DLL.
- `abakus-payload-<verzia>.json`: výsledok statickej kontroly pre konkrétny balík.
- Výstup `exe` a `pdfium`: samostatné záznamy pre oba súbory.
- Výstup `machine`: overená architektúra x64.
- Výstup `imports`: názvy priamych Windows importov, bez tvrdenia o dynamických závislostiach.
- Výstup `sha256`: identita súboru; pri PDFium sa porovnáva s dôveryhodným pinom.
- `ExeSha256`: povinný údaj pre ISCC, zachytí zmenu EXE po validácii a po kopírovaní.
- `PdfiumSha256`: povinný údaj pre ISCC z validátora, kontroluje PDFium pred balením aj po kopírovaní.
- `pv`: štyri číselné časti, platná verzia väčšia než nula; explicitné registry HKLM32/64 a HKCU32/64.
- `DependenciesReady`: jediný výsledok, ktorý povoľuje ponuku spustenia; hash payloadu a registrácia runtime musia prejsť.
- `PayloadFailed`: zlyhanie záverečnej integrity vypne spustenie a dá návratový kód 4.
- Interaktívne odmietnutie alebo chyba WebView2: upozornenie a odložená inštalácia bez spustenia.
- Tichý chýbajúci runtime: ukončenie pred kopírovaním, kód 1, bez dialógu a sťahovania v našich kontrolách.
- Tichá aktualizácia/preinštalovanie: pokračuje; tichý downgrade sa odmietne.
- `WebView2Check=skip`: existujúca testovacia voľba teraz vždy vypne ponuku spustenia.
- Kontrola registrácie nie je test funkčnosti WebView2.
- Platná inštalovaná PDFium sa zachová, chýbajúca alebo odlišná sa nahradí z balíka.
- Zrušená aktualizácia nesmie spustiť závislosti; súhlas sa vyhodnotí prvý.

## Matica testov

| Vstup alebo scenár | Očakávaný výsledok | Stav |
|---|---|---|
| Syntetický PE x64 | Rozpoznaná architektúra a importy | Node test |
| Chýbajúci EXE, chybný DOS/PE, x86/ARM64, PE32, skrátená sekcia | Balenie sa zastaví s chybou vstupu | Node test |
| Cudzia platná x64 DLL namiesto PDFium | Nezhoda dôveryhodného pinu | Node test |
| Nový VC import, chybný import RVA, delay import | Balenie sa zastaví pred ISCC | Node test |
| Existujúci EXE a PDFium | Zhodné importy a hash PDFium s inventárom | Staticky overené, starší EXE |
| Všetky závislosti na čistom stroji | Žiadne sťahovanie, UI a syntetické PDF fungujú | Neoverené |
| Prázdne, nulové, nečíselné, pretečené pv | Nesplnená kontrola, žiadne pripravené spustenie | Natívne neoverené |
| Platné pv, poškodený WebView2 | Registry sa nesmú vydávať za runtime health test | Natívne neoverené |
| WebView2 chýba, používateľ súhlasí | Existujúci bootstrapper, návrat 0 a nová kontrola registra | Neoverené |
| Odmietnutie, offline, chyba bootstrappera | Varovanie, bez ponuky spustenia | Neoverené |
| SILENT a VERYSILENT, oba aj so SUPPRESSMSGBOXES | Bez dialógu našich kontrol, bez sťahovania, kód podľa pravidiel | Neoverené |
| Poškodené/chýbajúce inštalované PDFium | Oprava z balíka, potom kontrola hashov | Neoverené |
| Záverečný hash nesúhlasí | Jasná chyba, kód 4, bez spustenia | Neoverené |
| Upgrade/reinstall/downgrade zrušený | Bez akcie WebView2 | Neoverené |
| ARM64, jednotlivé Windows buildy | Reálny test x64 emulácie a systémových API | Neoverené |

Syntetické Node testy nevolajú inštalátor ani DLL. Pre čistý stroj nestačí
vyčistiť PATH. Musí chýbať `ABAKUS_PDFIUM_DIR` aj compile-time cesta do
vývojárskeho repozitára, pretože parser ich zatiaľ preferuje. Táto úloha
poradie načítania parsera nemení. Prijatie bodu 6 preto vyžaduje ďalší
izolovaný natívny test a inventár konečného release EXE.

Príkazy a používateľské výsledky sú v [INSTALL.md](../packaging/INSTALL.md).
