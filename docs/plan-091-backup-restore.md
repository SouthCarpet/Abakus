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
súbor sa premenuje späť na pôvodné meno a znova otvorí cez
`Store::open_connection` (rovnaký `PRAGMA foreign_keys = ON` a migračná
sekvencia ako `Store::open`, nie holé `Connection::open`). Vďaka tomu sa
záloha so staršou, ale ešte podporovanou schémou zmigruje na aktuálnu
verziu hneď v tomto behu appky, nielen po reštarte procesu. Volanie tak
vždy skončí s `self` pripojeným na skutočnú, otvoriteľnú databázu: obnovenú
pri úspechu, pôvodnú pri akomkoľvek zlyhaní, ktoré sa dá vrátiť vôbec (pozri
nižšie ten jeden prípad, kde sa nedá). Ak zlyhá aj samotné vrátenie
(premenovanie späť alebo opätovné otvorenie), chybová správa menuje presnú
cestu k odloženému súboru aj k bezpečnostnej kópii, takže dáta nie sú
stratené, len appka ich sama nevie znova pripojiť.

**Windows: obsadený cieľ po kroku 6.** `std::fs::rename` na tomto systéme
nikdy neprepíše existujúci cieľ. Ak sa krok 6 (`rename_in`) podarí, ale krok
7 (opätovné otvorenie) zlyhá, `db_path` už obsahuje uverejnený, ale
neotvoriteľný súbor zálohy: jednoduché "premenuj odložený súbor späť" by na
tomto mieste vždy zlyhalo, lebo cieľ je obsadený. Pre presne tento prípad
`rollback_after_publish` najprv premenuje obsadený, neotvoriteľný súbor
nabok pod meno `abakus-obnova-zlyhala-<časová pečiatka>-<pid>.db` (nikdy sa
nemaže, viditeľný súbor v priečinku s dátami appky, nie skrytý dočasný
súbor), čím uvoľní `db_path` pre bežný rollback, ktorý potom vráti pôvodnú
databázu z odloženého súboru presne ako v ostatných prípadoch.

Ani bezpečnostná kópia, ani odložený súbor, ani prípadný súbor
`abakus-obnova-zlyhala-*` sa týmto tokom nikdy nemažú, ani pri úspešnej
obnove. Ide o zámerne duplicitnú poistku: úspešná obnova necháva pôvodné
bajty na disku pod odloženým (`.abakus-restore-aside-*`) aj pod bezpečnostným
menom; na ceste `abakus.db` je už obnovená záloha. Iba prechodný
skopírovaný súbor zálohy (krok 3) sa po zámene odstráni. `backup_to`
(bezpečnostná kópia aj bežné zálohovanie) navyše po úspešnom uverejnení
zavolá `File::sync_all` na výsledný súbor (na POSIX aj na priečinok, kde to
platforma umožňuje; na Windows stačí súbor samotný), čo je mierne zlepšenie
trvanlivosti nad už kompletným a viditeľným súborom, nie podmienka
správnosti.

## Testovací hák pre zámeny

`Store::restore_from_with(backup_path, db_path, rename_out, rename_in)` je
verejný hák za `restore_from`. `rename_out` premenuje živý súbor na odložený,
`rename_in` publikuje pripravenú zálohu na miesto živého súboru. Produkčný
kód posiela `std::fs::rename` pre oba parametre. Testy vkladajú zlyhávajúcu
uzáverovú funkciu pre ktorýkoľvek z nich, aby dokázali rollback bez
spoliehania sa na skutočné, ťažko reprodukovateľné zamykanie súborov vo
Windows.

`Store::restore_from_with_seams(backup_path, db_path, rename_out, rename_in,
open_conn)` je širší hák, ktorý `restore_from_with` interne používa
(s `Store::open_connection` ako produkčným `open_conn`). Testy ním vedia
vložiť zlyhávajúce opätovné otvorenie presne raz, aby dokázali práve
scenár obsadeného cieľa vyššie bez skutočného zamykania súboru vo Windows.

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

### `Store::schema_version` (teraz verejná)

Predtým `pub(crate)`. Vracia `schema_version` živého úložiska; testy a
prípadná budúca diagnostika ňou vedia priamo overiť, že obnova zálohy
zmigrovala schému na `CURRENT_SCHEMA_VERSION` hneď v tom istom behu.

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
kopírovaný súbor. Ďalej testuje presne scenár obsadeného cieľa (krok 6
uspeje, krok 7 zlyhá cez `restore_from_with_seams`): pôvodná databáza je po
rollbacku živá v tom istom behu aj po opätovnom otvorení, bezpečnostná
kópia aj súbor `abakus-obnova-zlyhala-*` oba prežijú, a dočasné mená
`restore-aside`/`restore-staged` po dobehnutí zmiznú. Samostatný test
overuje, že záloha so staršou, ale podporovanou schémou dosiahne
`CURRENT_SCHEMA_VERSION` hneď v tom istom behu appky (nie až po reštarte).
Jednotkový test priamo v `crates/store/src/restore.rs` (`rollback_after_publish_moves_the_occupied_destination_aside_before_restoring_the_original`)
overuje samotný mechanizmus obsadeného cieľa v izolácii: súbor vopred
umiestnený na `db_path` sa musí odsunúť skôr, než sa naň premenuje odložený
originál.

`src-tauri/src/commands.rs` testuje `refuse_if_importing` (odmietnutie iba
počas nastaveného príznaku) a `ImportGuard` (príznak sa vypne pri bežnom aj
panikou ukončenom behu).

`src-tauri/tests/commands_json.rs` kontroluje názov Tauri argumentu
`backupPath` pre oba príkazy a serializáciu `BackupPreview` aj
`RestoreOutcome`.

`src/components/RestoreSection.test.tsx` kontroluje: text o
Správcovi poverení je viditeľný pred akýmkoľvek kliknutím aj v dialógu
náhľadu; veta o bezpečnostnej kópii je viditeľná priamo v potvrdzujúcom
dialógu, nie iba na karte pod ním; zrušenie dialógu výberu súboru nič
nevolá; platná záloha ukáže presné počty a nezavolá `restoreDatabase` sama
od seba; neplatná záloha sa odmietne pred otvorením dialógu; úspešná
obnova zavrie dialóg a ukáže cestu k bezpečnostnej kópii; zlyhaná obnova
nechá dialóg otvorený a opakovanie funguje; tlačidlá sú počas vlastného
behu vypnuté.
