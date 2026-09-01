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
3. Skompiluje `packaging\abakus.iss` cez ISCC.

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
(o balenie sa stará Inno Setup v kroku 3); `bundle.active` v
`tauri.conf.json` je aj tak `false`.

Ak už máte hotový release build, pridajte `-SkipBuild` a skript spustí iba
krok 3:

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

## WebView2

Abakus je Tauri appka a na Windows potrebuje modul WebView2 runtime. Windows
10 (verzia 2004 a novšie) a Windows 11 ho už majú predinštalovaný. Na
staršom Windows ho treba doinštalovať samostatne
(https://developer.microsoft.com/microsoft-edge/webview2/).

## Inštalátor nie je podpísaný

Windows SmartScreen môže pri spustení zobraziť varovanie, pretože súbor
nemá overeného vydavateľa. Ak inštalátor pochádza z dôveryhodného zdroja,
zvoľte **Ďalšie informácie** a potom **Spustiť napriek tomu**.
