# Plán 091: backend mazania kategórií

Tento dokument opisuje backendový kontrakt bodu 4 pre verziu
0.2.0. Používateľské ovládanie je zapojené v `src/screens/Categories.tsx`:
kompaktné ikonové tlačidlo na archiváciu, červené tlačidlo na zmazanie a
dialóg, ktorý pred zmazaním ukáže presný počet transakcií, ktoré appka
presunie do nezaradených.

## Tok preview a apply

`Store::category_delete_preview(category_id)` načíta celú vetvu kategórie.
Výsledok obsahuje aj archivované podkategórie. Systémovú kategóriu ani
nesystémovú kategóriu pod systémovým rodičom nemožno zmazať.

`Store::delete_category(CategoryDeleteRequest)` vyžaduje celý predchádzajúci
náhľad v poli `preview`. Po získaní databázového zámku vytvorí nový náhľad.
Zápis pokračuje iba vtedy, keď sa nový náhľad presne zhoduje s potvrdeným
náhľadom. Nová transakcia, podkategória, pravidlo, zdroj pravidla alebo členstvo
pravidelnej platby preto zneplatní starý náhľad.

Po úspešnom potvrdení sa všetky transakcie vo vetve, vrátane potvrdených,
presunú do existujúceho stavu nezaradené: `category_id = NULL`,
`status = 'unassigned'`, `rule_id = NULL` a `source = 'none'`. Pravidlá s
cieľom v zmazanej vetve sa zmažú. Ich riadky `rule_sources` sa zmažú cez
existujúcu cudziu väzbu `ON DELETE CASCADE`.

Rozhodnutia a členstvá pravidelných platieb nemajú cudziu väzbu na kategóriu.
Členstvo podľa fingerprintu ostáva uložené. Kategória pravidelnej platby sa
odvodzuje zo živých transakcií, preto po zmazaní už nie je priradená.

Interný prevod nemá mať kategóriu. Ak preview nájde transfer s odkazom na
odstraňovanú vetvu, operáciu zastaví bez zmeny transferu. Rovnako zastaví
pravidlo z odstraňovanej vetvy, ak naň odkazuje transakcia mimo vetvy. Backend
tak nevykoná skrytú zmenu mimo potvrdeného rozsahu.

Celé apply beží v jednej transakcii `BEGIN IMMEDIATE`. Chyba pri presune
transakcií, mazaní pravidiel, kaskáde záznamov pôvodu alebo mazaní kategórií vráti
všetky súvisiace riadky do pôvodného stavu. To isté pripojenie možno po
rollbacku znovu použiť.

## Pridané povrchy

### `CategoryDeleteItem`

- `id`: ID kategórie, ktorú apply zmaže.
- `parent_id`: rodič kategórie pred zmazaním alebo `null` pre koreň.
- `name`: názov kategórie z potvrdeného náhľadu.
- `archived`: informácia, či je kategória archivovaná.

### `CategoryDeletePreview`

- `category_id`: ID koreňa odstraňovanej vetvy.
- `affected_categories`: presný zoznam koreňa a všetkých potomkov ako
  `CategoryDeleteItem`.
- `transaction_count`: počet všetkých transakcií, ktoré sa presunú do
  nezaradených.
- `confirmed_count`: počet potvrdených transakcií zahrnutých v
  `transaction_count`.
- `rule_count`: počet pravidiel, ktoré sa zmažú.
- `rule_source_count`: počet záznamov pôvodu týchto pravidiel, ktoré zmaže
  existujúca kaskáda.
- `recurring_member_count`: počet uložených členstiev pravidelných platieb,
  ktorých transakcie stratia kategóriu. Členstvá sa nezmažú.

### `CategoryDeleteRequest`

- `preview`: úplný náhľad, ktorý používateľ výslovne potvrdil.

### Príkazy

- `category_delete_preview` prijíma Tauri argument `categoryId` a vracia
  `CategoryDeletePreview`.
- `delete_category` prijíma objekt `request` s `CategoryDeleteRequest` a vracia
  presný náhľad vykonaného dopadu.
- `categoryApi.previewDelete(categoryId)` volá read-only preview.
- `categoryApi.deleteCategory(preview)` odošle potvrdený náhľad na apply.

## Overenie

`crates/store/tests/category_delete.rs` používa iba dočasné syntetické
databázy. Testuje potomkov, potvrdené transakcie, pravidlá a ich pôvod,
zachovanie recurring členstva, archivovanú prázdnu kategóriu, neznáme ID,
zastaraný náhľad, ochranu systémovej vetvy, chybný transfer, chybný odkaz
pravidla mimo vetvy a rollback po SQL chybe s úspešným opakovaním.

`src-tauri/tests/category_json.rs` kontroluje názvy Tauri argumentov, povinný
úplný preview objekt, odmietnutie neznámych polí a serializáciu každého poľa.
