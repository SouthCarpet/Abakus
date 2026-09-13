# Plán 091: backend kontroly výpisu

`Store::statement_review(statement_id)`, Tauri príkaz `statement_review` a
`api.statementReview(statementId)` vracajú samostatný `StatementReview`.
Používateľské rozhranie je zapojené v zozname výpisov v Importe (pozri nižšie).
História výpisov má rovnaký DTO, filtre aj poradie ako pred týmto krokom.

## Uložené dôkazy a migrácia

Schéma 6 pridáva `statements.parser_warnings_json TEXT`. SQL `NULL` znamená,
že výsledok parsera nebol uložený. JSON `[]` znamená známy výsledok bez
upozornení. Existujúce importy zostávajú `NULL`. Migrácia nespúšťa parser
znova a nedopĺňa historické upozornenia.

Prvý úspešný import nového hashu súboru uloží pole upozornení v rovnakej
transakcii ako výpis, jeho riadky a klasifikáciu. Text, poradie, opakovania,
Unicode a nové riadky sa zachovajú presne. Opakovaný import rovnakého hashu
uložené dôkazy nikdy nemení, ani keď sú neznáme alebo známe prázdne.
`ImportOutcome` a `ImportReport` naďalej vracajú výsledok aktuálneho pokusu.

Nová aj staršia databáza získajú stĺpec cez verziovanú migráciu. Otvorenie
schémy 6 neopakuje `ALTER`. Inicializácia, migrácie, prípadné počiatočné dáta
a konečný zápis verzie sú v jednej transakcii. Neskorá chyba vráti pôvodnú
schému aj dáta. Vyššia verzia schémy je odmietnutá bez zmien.

## Stav kontroly

Kontrola číta uložený checksum a upozornenia. Počty číta z aktuálnych
zachovaných transakcií, ktorých `statement_id` patrí požadovanému výpisu.
Všetky polia číta jeden SQL dotaz. Stav sa neukladá.

Podmienky sa vyhodnocujú v tomto poradí:

| Stav | Podmienka |
| --- | --- |
| `needs_attention` | Checksum `OffBy`, neprázdne známe upozornenia alebo aspoň jeden riadok `unassigned` či `suggested`. |
| `evidence_incomplete` | Bez predošlej podmienky, ale checksum je `NotVerifiable` alebo upozornenia sú neznáme. |
| `no_open_checks` | Checksum `Ok`, známe prázdne upozornenia a oba počty sú nula. |

Aj pri `needs_attention` sa vracajú všetky polia vrátane neznámych
upozornení a neoveriteľného checksumu. Riadky `confirmed` a `transfer` sa
do otvorených počtov nezapočítajú. Priradenie, potvrdenie alebo nová
klasifikácia sa prejavia pri ďalšom čítaní.

`no_open_checks` nepotvrdzuje úplnosť bankových dát, počet riadkov pôvodného
PDF ani dokončenie ľudskej kontroly. Nový export s iným hashom má vlastné
upozornenia, ale môže mať nulové počty. Duplicitné transakcie naďalej patria
pôvodnému výpisu. Kontrola ich neobnovuje ani nepresúva. Ručné obídenie
kontroly ani zahodenie upozornení nemá API.

## Chyby a mazanie

Neexistujúce ID vracia `StoreError::UnknownStatement`. Chyba databázy,
neplatný JSON, iný JSON typ ako pole reťazcov alebo neplatný checksum vracia
chybu. SQL `NULL` sa líši od JSON textu `null`, ktorý je neplatný.
Checksum uzná len `ok` a `not_verifiable` bez rozdielu alebo `off` s
celočíselným rozdielom. Neznámy stav, chýbajúci rozdiel pri `off` či
neočakávaný rozdiel pri ostatných stavoch je chyba. Existujúca tolerantná
konverzia checksumu v histórii zostáva bez zmeny.

Mazanie výpisu odstráni upozornenia s jeho riadkom. Mazanie účtu odstráni
upozornenia všetkých jeho výpisov. Iné výpisy si ponechajú svoje dôkazy.

## Pridaný povrch

- `statements.parser_warnings_json`: nullable TEXT, `NULL` neznáme, inak presné JSON pole reťazcov z prvého úspešného importu.
- Verzia schémy `6`: atómové pridanie stĺpca bez historického dopĺňania.
- `StatementReview.statement_id`: existujúce požadované ID, Rust `i64`, JSON a TypeScript `number`.
- `StatementReview.checksum`: existujúci tagged JSON typ `Checksum`, rozdiel v celočíselných centoch.
- `StatementReview.parser_warnings`: Rust `Option<Vec<String>>`, JSON pole alebo `null`, TypeScript `string[] | null`.
- `StatementReview.unassigned_count`: aktuálny počet vlastnených riadkov `unassigned`, Rust `i64`, JSON a TypeScript `number`.
- `StatementReview.suggested_count`: aktuálny počet vlastnených riadkov `suggested`, Rust `i64`, JSON a TypeScript `number`.
- `StatementReview.status`: odvodený `StatementReviewStatus` s troma hodnotami z tabuľky.
- `Store::statement_review(statement_id)`: čítanie jedného výpisu alebo typovaná chyba.
- Tauri `statement_review`: argument `statementId`, výsledok DTO alebo reťazec chyby.
- TypeScript `StatementReview`, `StatementReviewStatus`, `api.statementReview(statementId)`: typovaný prístup; UI je zapojené v Import obrazovke (sekcia UI (bod 21) nižšie).

## Overenie

Syntetické Store testy overujú presné upozornenia po otvorení databázy,
precedenciu stavov, živé počty, priradenie a klasifikáciu, oddelenie výpisov
a účtov, opakovaný import a deduplikáciu, chyby, atómový import a mazanie.
Zmrazená SQL schéma v5 overuje zachovanie dát, `NULL`, opakované otvorenie,
neskorý rollback po `ALTER`, úspešný opakovaný pokus a odmietnutie verzie 99.
Tauri JSON testy overujú presné polia a rozdiel medzi `null` a `[]`.
Natívny beh aplikácie zostáva mimo tohto overenia.

## UI (bod 21)

Každý riadok zoznamu výpisov v Importe (`src/screens/Import.tsx`) má vlastný
odznak kontroly, ktorý si sám načíta `api.statementReview(statementId)` len
pri vykreslení (zbalené staršie výpisy teda nič nevolajú, kým sa nerozbalia).
Kliknutím sa rozbalí zoznam otvorených položiek: nesediaci alebo neoveriteľný
kontrolný súčet, upozornenia parsera (alebo veta, že nie sú známe) a počty
nezaradených a odhadovaných riadkov. Text pri stave `no_open_checks` výslovne
hovorí "Nepotvrdzuje úplnosť bankových dát." a nikde v odznaku sa neobjaví
tvrdenie, že bankové dáta sú úplné.

Súvisiace: [história výpisov](plan-091-statement-history.md),
[backend histórie, poznámok a zálohy](backend-012.md).
