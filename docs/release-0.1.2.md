# Abakus 0.1.2

Verzia 0.1.2 spája päť nových funkcií s opravami 0.1.1.

## Nové funkcie

- Pokrytie výpismi ukáže chýbajúce obdobia po účtoch. Prekryté a opakované výpisy nevytvárajú falošné medzery. Dátumy transakcií sa môžu líšiť od obdobia výpisu.
- Lokálna záloha databázy v Nastaveniach uloží aj kategórie, naučené pravidlá a poznámky. Heslá a pôvodné PDF zostávajú mimo zálohy. Záloha nie je šifrovaná.
- Porovnanie výdavkov podľa kategórií ukáže rozdiel oproti predchádzajúcemu obdobiu rovnakej dĺžky. Uvádza oba rozsahy dátumov a chýbajúce pokrytie.
- Historické konečné zostatky zobrazia stav ku dňu výpisu a výsledok kontroly. Neznámy zostatok sa nenahrádza nulou.
- Poznámku k transakcii možno uložiť v detaile. Poznámky sú súčasťou vyhľadávania aj CSV.

## Zahrnuté opravy

- Odstránenie účtu s náhľadom dôsledkov a potvrdením názvu. Ostatné účty a pôvodné PDF zostávajú zachované.
- Voľba Svetlý, Tmavý a Podľa systému je dostupná na každej obrazovke a uložená pre ďalšie spustenie.
- Export podľa aktuálnych filtrov, vymazanie všetkých filtrov a súčty zobrazených transakcií v celých centoch.
- Opravy spracovania chýb, súbežných požiadaviek, filtra nadradených kategórií, znamienok v grafoch a tabuliek v menšom okne.

## Prechod zo staršej verzie

Databáza sa pri prvom spustení aktualizuje pre poznámky. Pred aktualizáciou si vytvorte zálohu databázy. Staršie priradenia kategórií a naučené pravidlá sa zachovajú.

Inštalátor je lokálny a nepodpísaný. Obnova zo zálohy a publikovanie do GitHub Releases nie sú súčasťou tejto verzie.
