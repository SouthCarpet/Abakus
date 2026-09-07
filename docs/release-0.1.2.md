# Abakus 0.1.2

Verzia 0.1.2 spája opravy 0.1.1, päť funkcií pre kontrolu údajov, pravidelné platby, správu kategórií a PDF reporty do jedného lokálneho vydania.

## Nové funkcie

- Pokrytie výpismi ukáže chýbajúce obdobia po účtoch. Prekryté a opakované výpisy nevytvárajú falošné medzery. Dátumy transakcií sa môžu líšiť od obdobia výpisu.
- Lokálna záloha databázy v Nastaveniach uloží aj kategórie, naučené pravidlá a poznámky. Heslá a pôvodné PDF zostávajú mimo zálohy. Záloha nie je šifrovaná.
- Porovnanie výdavkov podľa kategórií ukáže rozdiel oproti predchádzajúcemu obdobiu rovnakej dĺžky. Uvádza oba rozsahy dátumov a chýbajúce pokrytie.
- Historické konečné zostatky zobrazia stav ku dňu výpisu a výsledok kontroly. Neznámy zostatok sa nenahrádza nulou.
- Poznámku k transakcii možno uložiť v detaile. Poznámky sú súčasťou vyhľadávania aj CSV.
- Pravidelné príjmy a výdavky: mesačné, štvrťročné a ročné opakovanie, potvrdenie alebo ignorovanie odhadu, ručné priradenie konkrétnych platieb, očakávaný termín a ustálená zmena ceny. Odhady a potvrdené položky majú oddelené súčty.
- Prehľad pravidelných platieb: mesačný a ročný prepočet, platby do konca mesiaca a podiel na výdavkoch pri dostatočnom overenom pokrytí. Chýbajúce údaje zostávajú neznáme. Historický pohľad nepoužíva neskoršie platby.
- Kategórie možno vytvoriť pri priraďovaní transakcie, presunúť alebo osamostatniť. Pri zmene druhu s dopadom na históriu sa zobrazia počty na potvrdenie. Vstavané pravidlo možno presmerovať na inú kategóriu. Spotify má samostatnú podkategóriu.
- Lokálny PDF report s grafmi a úplným výpisom vrátane poznámok: mesiac, šesť mesiacov, rok alebo celé obdobie, pre všetky účty, druh účtov alebo jeden účet. Report má vložené písmo, číslované strany a informáciu o pokrytí.

## Zahrnuté opravy

- Odstránenie účtu s náhľadom dôsledkov a potvrdením názvu. Ostatné účty a pôvodné PDF zostávajú zachované.
- Voľba Svetlý, Tmavý a Podľa systému je dostupná na každej obrazovke a uložená pre ďalšie spustenie.
- Export podľa aktuálnych filtrov, vymazanie všetkých filtrov a súčty zobrazených transakcií v celých centoch.
- Opravy spracovania chýb, súbežných požiadaviek, filtra nadradených kategórií, znamienok v grafoch a tabuliek v menšom okne.
- Priradenie nepomenovanej platby nevytvorí spoločné pravidlo pre ostatné nepomenované platby. Vysvetlenie učenia odlišuje vytvorenie kategórie od priradenia transakcie a od použitia na podobné platby.

## Prechod zo staršej verzie

Databáza sa pri prvom spustení aktualizuje na schému 4 pre poznámky a uložené voľby pravidelných platieb. Celá inicializácia a migrácia je jedna transakcia. Pri chybe sa zmeny vrátia. Staršie priradenia, poznámky a naučené pravidlá zostávajú zachované; úzka oprava pôvodného pravidla Spotify nemení používateľské úpravy. Pred aktualizáciou si vytvorte konzistentnú zálohu databázy.

PDF zachytáva údaje pri uložení a nepoužíva filtre tabuľky transakcií. Nezahŕňa predpovede pravidelných platieb. Existujúci cieľový súbor neprepíše a pri prekročení limitu reportu vráti chybu bez neúplného výsledku.

Inštalátor je lokálny a nepodpísaný. Obnova zo zálohy a publikovanie do GitHub Releases nie sú súčasťou tejto verzie.
