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

`ISCC /DAppIdGuid=<guid> packaging\abakus.iss` skompiluje inštalátor s iným
(testovacím) `AppId`, takže testovacia inštalácia/odinštalácia nikdy
nezasiahne skutočnú inštaláciu Abakusu ani jej záznam v registri. Bez tohto
prepínača (bežný release build) sa použije skutočný pevný `AppId`,
nezmenený od predchádzajúcich vydaní.

## WebView2

Abakus je Tauri appka a na Windows potrebuje modul Microsoft Edge WebView2
Runtime. Windows 10 (verzia 2004 a novšie) a Windows 11 ho už majú
predinštalovaný, takže na väčšine počítačov sa nič nestane.

Od verzie 0.1.4 inštalátor kontroluje modul sám, ešte pred prvou obrazovkou
sprievodcu (`InitializeSetup` v `packaging\abakus.iss`). Kontrola číta
hodnotu `pv` z troch registrových kľúčov, v tomto poradí:

1. `HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`
2. `HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`
3. `HKCU\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`

Chýbajúca hodnota alebo `0.0.0.0` znamená, že modul nie je nainštalovaný.
Vtedy inštalátor po slovensky zobrazí:

> Abakus potrebuje Microsoft Edge WebView2 Runtime. Stiahnuť a nainštalovať
> teraz (asi 2 MB, vyžaduje internet)?

- **Áno** stiahne oficiálny Evergreen bootstrapper
  (`https://go.microsoft.com/fwlink/p/?LinkId=2124703`) do `{tmp}`, spustí
  ho s `/silent /install` a znova skontroluje registrové kľúče. Nevyžaduje
  práva správcu: bootstrapper sa spustí bez zvýšených práv, rovnako ako
  samotný inštalátor Abakusu (`PrivilegesRequired=lowest`).
- **Nie**, alebo zlyhanie sťahovania/inštalácie modulu, inštaláciu Abakusu
  nezastaví. Zobrazí sa iba upozornenie, že appka sa bez modulu nespustí a
  modul sa dá doinštalovať samostatne aj neskôr
  (https://developer.microsoft.com/microsoft-edge/webview2/).

Každý krok kontroly sa zapíše do inštalačného denníka (`Log(...)` v
`[Code]`), viditeľného pri behu s `/LOG="cesta.txt"`.

### Tiché prepínače (overené na tomto počítači)

Skúšobná inštalácia mimo skutočného `%LOCALAPPDATA%\Programs\Abakus`:

```powershell
abakus-setup-0.1.5.exe /VERYSILENT /DIR="C:\cesta\scratch" /MERGETASKS="!startmenuicon,!desktopicon"
```

`/NOICONS` nevypne vlastné úlohy `[Tasks]` `startmenuicon` a `desktopicon`
z `packaging/abakus.iss`; použite `/MERGETASKS` s `!` pred názvom úlohy.
`AppId` je pevný, takže druhá (skúšobná) inštalácia prepíše ten istý záznam
`HKCU\...\Uninstall\{AppId}_is1` aj odkazy existujúcej inštalácie. Jej
odinštalovanie potom zmaže tie isté odkazy a záznam.

Odinštalovanie tej istej skúšobnej inštalácie:

```powershell
"C:\cesta\scratch\unins000.exe" /VERYSILENT
```

### Vývojárska voľba: preskočenie kontroly

`ISCC /DWebView2Check=skip packaging\abakus.iss` skompiluje inštalátor, ktorý
kontrolu modulu úplne vynechá (iba do denníka zapíše, že bola preskočená).
Slúži pre zostavenie bez siete alebo bez možnosti overiť registre (napríklad
CI); bežný release build (bez tohto prepínača) kontrolu vždy spustí.

## Inštalátor nie je podpísaný

Windows SmartScreen môže pri spustení zobraziť varovanie, pretože súbor
nemá overeného vydavateľa. Ak inštalátor pochádza z dôveryhodného zdroja,
zvoľte **Ďalšie informácie** a potom **Spustiť napriek tomu**.
