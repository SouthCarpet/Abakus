# Inštalácia Abakusu (Windows)

## Ako zostaviť inštalátor

Potrebujete Rust toolchain, Node.js, npm a Inno Setup 6
(`winget install JRSoftware.InnoSetup`).

```powershell
.\packaging\build-installer.ps1
```

Skript spraví toto:

1. Zostaví frontend (`tsc -b`, potom `vite build` do `dist\`).
2. Zostaví appku v režime release cez tauri-cli (`tauri build --no-bundle`,
   jobs 4).
3. Overí EXE, PDFium a ich importy. Zapíše výsledok kontroly.
4. Skompiluje `packaging\abakus.iss` cez ISCC.

Frontend aj tauri-cli sa spúšťajú priamo cez `node`, nie cez `npm run
build` alebo `npm run tauri build`. Príkazové `.cmd` obaly týchto nástrojov
(`tsc.cmd`, `vite.cmd`, `tauri.cmd`) na tomto počítači hlásia "Access is
denied". Je to zámok antivírusu ESET na súbore obalu, nie skutočná chyba
zostavenia. `node node_modules\@tauri-apps\cli\tauri.js` obalu nepoužíva.

Krok 2 volá `tauri build`, nie priamy `cargo build`. Dôvod: Tauri rozlišuje
vývojovú a produkčnú verziu podľa cargo príznaku (feature) `custom-protocol`
zapnutého v čase kompilácie. Bez neho appka pri spustení otvorí vývojový
server (`http://localhost:1420`) namiesto zabudovaného frontendu. `tauri
build` zapne tento príznak sám. `cargo build` by ho musel niekto pridať
ručne, a taká zostava by sa pri budúcej zmene ľahko rozišla so skutočným
tauri-cli krokom.

`tauri build` normálne pred kompiláciou sám spustí `build.beforeBuildCommand`
z `tauri.conf.json`, čo je tu nastavené na `npm run build`. To by narazilo na
rovnaký zámok `.cmd` obalu. Skript preto do `tauri build` pridáva
`--config packaging\tauri.build-override.json`, čo tento príkaz nahradí
prázdnym reťazcom. Frontend v `dist\` je v tom momente už hotový z kroku 1,
takže sa nič nestratí. `--no-bundle` iba vypína vlastné balenie tauri-cli
(o balenie sa stará Inno Setup v kroku 4); `bundle.active` v
`tauri.conf.json` je aj tak `false`.

Ak už máte hotový release build, pridajte `-SkipBuild` a skript spustí iba
kroky 3 a 4:

```powershell
.\packaging\build-installer.ps1 -SkipBuild
```

Skript hľadá `ISCC.exe` v tomto poradí: na `PATH`, na
`C:\Users\Asus\AppData\Local\Programs\Inno Setup 6\ISCC.exe`, na
`%LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe`, na
`C:\Program Files (x86)\Inno Setup 6\ISCC.exe`.

Antivírus ESET niekedy krátko zamkne práve zapísaný súbor inštalátora.
Skript to rieši opakovaním kompilácie (až 6 pokusov po 15 sekundách).

## Výstup

Inštalátor je `packaging\output\abakus-setup-<verzia>.exe`. Verzia sa číta
z `src-tauri\tauri.conf.json`.

## Čo inštalátor robí

- Inštaluje sa iba pre aktuálneho používateľa, do
  `%LOCALAPPDATA%\Programs\Abakus`. Nevyžaduje práva správcu.
- Ponúkne dve voliteľné zaškrtávacie polia: odkaz v ponuke Štart a odkaz na
  ploche. Obe sú pri prvej inštalácii zaškrtnuté.
- Inštaluje `abakus.exe` a knižnicu `pdfium.dll` (appka ju potrebuje na
  čítanie PDF výpisov). Appka nájde `pdfium.dll` automaticky, bez potreby
  nastavovať premenné prostredia.

## Čo odinštalovanie robí

Počas odinštalovania, nad oknom s priebehom, sa zobrazia dve samostatné otázky.
Obe majú predvolenú odpoveď **Nie**, takže bez potvrdenia sa nič nezmaže.

1. **Vymazať používateľské nastavenia?** Áno vymaže iba heslá k výpisom
   uložené v Správcovi poverení systému Windows (položky appky Abakus).
   Ostatné nastavenia appky sú uložené v dátovom súbore, takže táto otázka
   ich nezmaže.
2. **Vymazať používateľské údaje?** Áno vymaže priečinok
   `%LOCALAPPDATA%\Abakus` s databázou. Databáza obsahuje aj nastavenia
   uložené v appke a sieťový denník, takže táto voľba ich vymaže tiež.

## Aktualizácia existujúcej inštalácie

`AppId` v `packaging\abakus.iss` je pevný, takže Inno Setup druhý spustený
inštalátor automaticky aktualizuje tú istú inštaláciu (rovnaký priečinok,
rovnaký záznam `HKCU\...\Uninstall\{AppId}_is1`), nevytvorí druhú kópiu.

Od verzie 0.1.5 sprievodca navyše sám prečíta, čo je už nainštalované
(`DisplayVersion`, `InstallLocation` z uvedeného záznamu) a povie to skôr,
než čokoľvek zmení:

- **Staršia nainštalovaná verzia.** Hlásenie s tlačidlami OK/Zrušiť
  (predvolené OK): pokračovanie aktualizuje na novú verziu, dáta v
  `%LOCALAPPDATA%\Abakus` a heslá v Správcovi poverení zostanú. Zrušenie
  inštaláciu ukončí bez zmeny.
- **Rovnaká verzia.** Hlásenie Áno/Nie (predvolené Áno), pýta sa, či
  preinštalovať. Nie inštaláciu ukončí.
- **Novšia nainštalovaná verzia ako inštalátor.** Hlásenie Áno/Nie
  (predvolené **Nie**), pýta sa, či nahradiť novšiu verziu staršou.
  Predvolené Nie znamená, že **tichá inštalácia (`/VERYSILENT`) downgrade
  odmietne**: sprievodca skončí s nenulovým návratovým kódom a nič
  nezmení.

Po hlásení (Zrušenie/Nie ho zastaví, inak pokračuje) sprievodca pri
aktualizácii alebo preinštalovaní vynechá stránku s odkazmi (Start/plocha),
pretože Inno si predchádzajúcu voľbu už pamätá (`UsePreviousTasks`). Text o
aktualizácii (z akej verzie na akú a do akého priečinka) sa zapíše na
stránku **Inštalácia je pripravená**, poslednú stránku pred kliknutím na
Inštalovať. Uvítacia stránka sa v tomto prípade nezobrazuje (overené
spustením sprievodcu), takže na nej text o aktualizácii nemá zmysel
zapisovať. Pri prvej (čistej) inštalácii sa nič z tohto nezobrazí, stránka
**Inštalácia je pripravená** má pôvodný text.

### Vývojárska voľba: testovací `AppId`

Pridanie `/DAppIdGuid=<guid>` k overenej kompilácii nižšie vytvorí inštalátor s iným
(testovacím) `AppId`, takže testovacia inštalácia/odinštalácia nikdy
nezasiahne skutočnú inštaláciu Abakusu ani jej záznam v registri. Bez tohto
prepínača (bežný release build) sa použije skutočný pevný `AppId`,
nezmenený od predchádzajúcich vydaní.

## Závislosti a kontrola balíka

Podporovaná platforma je Windows 10 alebo novší a prostredie kompatibilné
s x64. Inno používa `MinVersion=10.0` a existujúcu politiku
`x64compatible`. Emulácia x64 na ARM64 tým nie je runtime testovaná.
Rust uvádza Windows 10+ pre použitý cieľ
[x86_64-pc-windows-msvc](https://doc.rust-lang.org/rustc/platform-support.html).

Koncový používateľ potrebuje WebView2 Runtime. PDFium je súčasť balíka.
SQLite je zabudovaný. Pre skontrolovaný EXE a PDFium nie je doložená potreba
samostatného VC++ Redistributable, .NET, Javy alebo OpenSSL. Build nástroje
sa používateľovi neinštalujú.

Pred ISCC sa vždy spustí `packaging/check-payload.mjs`, aj s `-SkipBuild`.
Kontroluje x64 PE hlavičky, rozsahy sekcií a importy proti overenému zoznamu
Windows DLL. Nový import alebo delay import vyžaduje nové posúdenie.
PDFium musí mať hash z `packaging/pdfium-pin.json`. Tento pin vznikol
overením archívu proti `scripts/pdfium.sha256` a až potom rozbalením
`bin/pdfium.dll`. Nový pin nesmie vzniknúť iba zahashovaním miestnej DLL.

Výsledok je `packaging/output/abakus-payload-<verzia>.json`.
Pre EXE aj PDFium obsahuje `machine`, `imports` a `sha256`.
Hash EXE identifikuje súbor. Nedokazuje správne zostavenie, pôvod zdrojov
ani neporušenosť ľubovoľných zmien jeho bajtov. Kontrola PE zachytí
štrukturálne poškodenie. Kontrola zabudovaného frontendu zostáva samostatná.
Inno pred balením znova porovná oba hashe, aby zachytil zmenu po kontrole.

Po kopírovaní Inno overí hashe nainštalovaného EXE a PDFium.
Zhodné PDFium nekopíruje znova. Chýbajúcu alebo odlišnú DLL nahradí
overeným obsahom balíka. Ak záverečná kontrola zlyhá, vypne spustenie,
zapíše chybu a skončí s kódom 4. Nespustený alebo nefunkčný runtime
táto kontrola neopravuje.

## WebView2

Kontrola sa spustí až po súhlase s aktualizáciou, preinštalovaním alebo
downgrade. Číta `pv` z kľúča
`SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`.
Použije explicitné pohľady `HKLM32`, `HKLM64`, `HKCU32`, `HKCU64`.
Prijme štyri číselné časti a verziu väčšiu než `0.0.0.0`.
Chýbajúci, nulový alebo neplatný údaj znamená nesplnenú kontrolu.
Platný záznam nepotvrdzuje funkčnosť WebView2. Existujúci platný modul
sa neinštaluje znova. WebView2 je na mnohých počítačoch už prítomný;
inštalátor to vždy kontroluje.

- **Áno** pri interaktívnej otázke stiahne existujúci oficiálny
  [Evergreen bootstrapper](https://go.microsoft.com/fwlink/p/?LinkId=2124703),
  spustí ho s `/silent /install` a pri návratovom kóde 0 znova overí register.
  Malý bootstrapper sťahuje aj samotný runtime. Práva sa nezvyšujú.
- **Nie** je predvolená odpoveď. Odmietnutie alebo chyba ponechá odloženú
  inštaláciu Abakusu s upozornením. Ponuka spustenia nebude dostupná.
  Používateľ musí doplniť WebView2 a zopakovať inštaláciu.
- Spustenie sa ponúkne iba po úspešnej kontrole záznamu WebView2 a hashov
  nainštalovaných súborov. Je to kontrola predpokladov, nie test funkčnosti.

### Tichá inštalácia

`/SILENT` aj `/VERYSILENT`, samostatne aj s `/SUPPRESSMSGBOXES`,
použijú rovnakú politiku našich kontrol. Chýbajúci alebo neplatný WebView2
ukončí setup pred kopírovaním s kódom 1 (`InitializeSetup=False`).
Kontrola nezobrazí dialóg a nestiahne runtime. Platný záznam pokračuje.
Aktualizácia a preinštalovanie pokračujú, downgrade sa odmietne.
Tichý režim appku nikdy nespúšťa.

Pridajte `/LOG="cesta.txt"`, aby sa zachovali dôvody a výsledky kontroly.
Zlyhanie záverečnej integrity má kód 4. Interaktívne odloženie WebView2
môže skončiť kódom 0, ale bez pripraveného spustenia. Natívne scenáre
nových kontrol sú zatiaľ neoverené; pozrite
[plán overenia](../docs/plan-091-installer-dependencies.md).

`/NOICONS` nevypne naše úlohy odkazov. Použite
`/MERGETASKS="!startmenuicon,!desktopicon"`. Skúšobný priečinok sám
neizoluje registry: testovací inštalátor musí mať odlišný `AppIdGuid`.

### Vývojárska kompilácia

Bežný build spúšťajte cez `build-installer.ps1`. Pri priamom ISCC najprv
spustite rovnakú kontrolu a použite jej výstup:

```powershell
$payload = node packaging/check-payload.mjs target/release/abakus.exe src-tauri/resources/pdfium/pdfium.dll
if ($LASTEXITCODE -ne 0) { throw 'Payload validation failed' }
$checked = ($payload -join "`n") | ConvertFrom-Json
ISCC /DAppVersion=0.1.6 "/DExeSha256=$($checked.exe.sha256)" "/DPdfiumSha256=$($checked.pdfium.sha256)" packaging/abakus.iss
```

`/DExeSha256` a `/DPdfiumSha256` sú povinné údaje z validátora.
`/DAppIdGuid=<guid>` možno pridať pre izolovanú testovaciu identitu.
`/DWebView2Check=skip` vynechá register a sťahovanie iba v testovacom
builde. Taký build nikdy neponúkne spustenie a nesmie sa vydať používateľom.

## Inštalátor nie je podpísaný

Windows SmartScreen môže pri spustení zobraziť varovanie, pretože súbor
nemá overeného vydavateľa. Ak inštalátor pochádza z dôveryhodného zdroja,
zvoľte **Ďalšie informácie** a potom **Spustiť napriek tomu**.
