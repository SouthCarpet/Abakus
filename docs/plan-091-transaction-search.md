# Vyhľadávanie transakcií podľa obchodníka a miesta

Bod 9 plánu 091 rozširuje existujúci filter `TxFilter.text`. Fráza sa teraz môže tiahnuť cez názov obchodníka a miesto transakcie. Napríklad `Penny Neuss` nájde riadok s obchodníkom `PENNY` a miestom `Neuss`, ale nenájde `PENNY` v Berlíne.

Vyhľadávanie zostáva doslovným podreťazcom po spracovaní funkciou `parser::fold`. Nerozlišuje veľkosť písmen ani diakritiku a viacnásobné medzery zjednotí. Znaky `%` a `_` nemajú význam SQL zástupných znakov. Nepridáva tokenové `AND`, voľné poradie slov ani regulárne výrazy.

Filter skúša tieto textové plochy:

- obchodník a miesto spojené jednou medzerou,
- poznámka,
- názov protistrany.

Samostatné hľadanie obchodníka, miesta, poznámky a protistrany ostáva funkčné. Chýbajúce miesto sa správa ako prázdny text. Dátum, účet, druh účtu, kategória, stav a výpis sa naďalej vyhodnotia spolu s textom. Export CSV používa tú istú inštanciu filtra cez `list_transactions`, preto obsahuje rovnaké vyfiltrované transakcie ako zoznam.

Zmena nepridáva endpoint, schému databázy ani dátový objekt. Obrazovka Transakcie už odosiela hodnotu vyhľadávania v `TxFilter.text`, preto nepotrebovala zmenu rozhrania.

## Overenie

Verejné integračné testy nad trvalou SQLite databázou a syntetickými výpismi overujú:

- hlásený prípad `Penny Neuss` a vylúčenie `Penny Berlin`,
- veľkosť písmen, diakritiku a nadbytočné medzery,
- obchodníka bez miesta a pôvodné hľadanie poznámky a protistrany,
- doslovné znaky `%` a `_`,
- prienik s filtrom účtu a dátumu,
- zhodný filter zoznamu a CSV exportu.
