# Plán 091: obnova zo zálohy

Tento dokument opisuje backendový kontrakt bodu 10 pre pripravovanú verziu
0.2.0. Používateľské ovládanie je zapojené v `src/screens/Settings.tsx`,
hneď pod `BackupSection`: `src/components/RestoreSection.tsx` ponúka
tlačidlo **Vybrať zálohu**, náhľad v dialógu a potvrdzujúce tlačidlo
**Obnoviť databázu**.

## Tok náhľadu a obnovy

`Store::preview_backup(path)` otvorí `path` cez SQLite s príznakom
`SQLITE_OPEN_READ_ONLY`. Nikdy nezapíše ani bajt do vybraného súboru,
vrátane súboru, ktorý nie je databázou vôbec. Skontroluje, že v súbore
existujú všetky tabuľky, ktoré musí mať databáza Abakusu (`accounts`,
`statements`, `transactions`, `categories`, `settings`). Chýbajúca tabuľka
alebo súbor, ktorý sa nedá otvoriť ako SQLite, vráti `StoreError::BackupInvalid`
so slovenskou správou. Ak je uložená `schema_version` vyššia, než akú appka
podporuje (`store::migrate::CURRENT_SCHEMA_VERSION`), vráti
`StoreError::BackupTooNew { found, supported }`. Iba ak oba testy prejdú,
vráti počty riadkov (`accounts`, `statements`, `transactions`) a
`schema_version`.

`Store::restore_from(backup_path, db_path)` vykoná obnovu. Poradie krokov:

1. Zavolá `preview_backup(backup_path)` znova, tesne pred akoukoľvek zmenou.
   Súbor, ktorý medzičasom prestal byť platnou zálohou, obnovu zastaví skôr,
   než sa čokoľvek stane so živou databázou.
2. Vytvorí povinnú bezpečnostnú kópiu súčasnej živej databázy cez
   `backup_to` (rovnaký online-backup mechanizmus ako tlačidlo **Zálohovať
   databázu**), s časovou pečiatkou v názve, v priečinku živej databázy.
   Toto beží, kým je živé pripojenie ešte otvorené. Zlyhanie tohto kroku
   obnovu zastaví; živá databáza dovtedy nebola vôbec dotknutá.
3. Skopíruje vstupnú zálohu do dočasného súboru v tom istom priečinku ako
   živá databáza (`stage_backup_copy`), stále cez otvorené živé pripojenie.
   Zámena nižšie je tak premenovanie v rámci jedného disku, nie kopírovanie
   naprieč súborovými systémami, ktoré by mohlo zlyhať v polovici s už
   odsunutou živou databázou.
4. Zavrie živé pripojenie (Windows musí uvoľniť súborový uchyt pred
   premenovaním). Počas trvania volania to nie je pozorovateľné zvonka:
   volajúci (príkaz `restore_database`) drží zámok úložiska po celý čas.
5. Premenuje živý súbor na odložený súbor (`aside`).
6. Premenuje dočasný súbor so zálohou na meno živej databázy.
7. Znova otvorí pripojenie na tomto mene.

Pri zlyhaní ktoréhokoľvek z krokov 5 až 7 sa spustí rollback: odložený
súbor sa premenuje späť na pôvodné meno a znova otvorí. Volanie tak vždy
skončí s `self` pripojeným na skutočnú, otvoriteľnú databázu: obnovenú pri
úspechu, pôvodnú pri akomkoľvek zlyhaní. Ak zlyhá aj samotné vrátenie
(premenovanie späť alebo opätovné otvorenie), chybová správa menuje presnú
cestu k odloženému súboru aj k bezpečnostnej kópii, takže dáta nie sú
stratené, len appka ich sama nevie znova pripojiť.

Ani bezpečnostná kópia, ani odložený súbor sa týmto tokom nikdy nemažú, ani
pri úspešnej obnove. Ide o zámerne duplicitnú poistku: úspešná obnova
necháva na disku pôvodný súbor aj pod pôvodným, aj pod bezpečnostným
menom. Iba prechodný skopírovaný súbor zálohy (krok 3) sa po zámene
odstráni.

## Testovací hák pre zámeny

`Store::restore_from_with(backup_path, db_path, rename_out, rename_in)` je
verejný hák za `restore_from`. `rename_out` premenuje živý súbor na odložený,
`rename_in` publikuje pripravenú zálohu na miesto živého súboru. Produkčný
kód posiela `std::fs::rename` pre oba parametre. Testy vkladajú zlyhávajúcu
uzáverovú funkciu pre ktorýkoľvek z nich, aby dokázali rollback bez
spoliehania sa na skutočné, ťažko reprodukovateľné zamykanie súborov vo
Windows.

## Odmietnutie počas importu

`AppState.import_in_progress` (`AtomicBool`) je nastavený na `true` po celý
čas behu `import_statements` aj `import_with_password`, cez RAII strážcu
(`ImportGuard`), ktorý ho vždy vypne pri návrate aj pri paniku. Príkaz
`restore_database` tento príznak skontroluje PRED pokusom o zámok úložiska;
ak je import v behu, obnovu rovno odmietne zrozumiteľnou správou namiesto
ticheho čakania na zámok, čo by tlačidlo nechalo vyzerať zaseknuté bez
vysvetlenia.

## Pridané povrchy

### `BackupPreview`

- `accounts`, `statements`, `transactions`: počty riadkov v zálohe.
- `schema_version`: verzia schémy uložená v zálohe.

### `RestoreOutcome`

- `safety_copy_path`: absolútna cesta k bezpečnostnej kópii databázy, ktorá
  bola živá bezprostredne pred touto obnovou.

### `StoreError`

- `BackupInvalid(String)`: súbor nie je čitateľný ako SQLite, alebo mu
  chýba niektorá z povinných tabuliek Abakusu.
- `BackupTooNew { found, supported }`: záloha má vyššiu `schema_version`,
  než akú appka podporuje.

### Príkazy

- `restore_preview` prijíma Tauri argument `backupPath` a vracia
  `BackupPreview`. Nikdy nedrží zámok úložiska; iba raz, read-only, otvorí
  `backupPath`.
- `restore_database` prijíma `backupPath` a vracia `RestoreOutcome`.
  Odmietne počas behu importu (pozri vyššie), inak zamkne úložisko a
  zavolá `Store::restore_from`.
- `api.restorePreview(backupPath)` a `api.restoreDatabase(backupPath)` v
  `src/api.ts`.

## Overenie

`crates/store/tests/restore.rs` používa iba dočasné, súborovo podložené
databázy (rovnako ako `crates/store/tests/backup.rs`). Testuje presné počty
v platnom náhľade, odmietnutie súboru, ktorý nie je databázou, odmietnutie
súboru bez povinných tabuliek, odmietnutie zálohy s novšou schémou, úspešnú
obnovu s dokázateľne obnoviteľným pôvodným stavom z bezpečnostnej kópie,
odmietnutie neplatnej zálohy bez akéhokoľvek dotyku živej databázy,
zlyhanie premenovania na oboch stranách zámeny s rollbackom, a že úspešná
obnova nemaže bezpečnostnú kópiu ani odložený súbor, ale odstráni prechodný
kopírovaný súbor.

`src-tauri/src/commands.rs` testuje `refuse_if_importing` (odmietnutie iba
počas nastaveného príznaku) a `ImportGuard` (príznak sa vypne pri bežnom aj
panikou ukončenom behu).

`src-tauri/tests/commands_json.rs` kontroluje názov Tauri argumentu
`backupPath` pre oba príkazy a serializáciu `BackupPreview` aj
`RestoreOutcome`.

`src/components/RestoreSection.test.tsx` kontroluje: text o
Správcovi poverení je viditeľný pred akýmkoľvek kliknutím aj v dialógu
náhľadu; zrušenie dialógu výberu súboru nič nevolá; platná záloha ukáže
presné počty a nezavolá `restoreDatabase` sama od seba; neplatná záloha sa
odmietne pred otvorením dialógu; úspešná obnova zavrie dialóg a ukáže cestu
k bezpečnostnej kópii; zlyhaná obnova nechá dialóg otvorený a opakovanie
funguje; tlačidlá sú počas vlastného behu vypnuté.
