# Abakus 0.1.4

Verzia 0.1.4 pridáva overené sťahovanie aktualizácie priamo v appke a opravuje
inštaláciu na počítač bez modulu WebView2 Runtime.

## Čo sa zmenilo

- Keď je zapnutá **Kontrolovať aktualizácie (GitHub)** a existuje novšia verzia,
  Nastavenia teraz ukazujú poznámky k vydaniu ako čitateľný text, odkaz na
  stránku vydania a tlačidlo **Aktualizovať na `<verzia>`**.
- Kliknutie na tlačidlo appku nikdy neaktualizuje samo od seba. Až po kliknutí
  appka stiahne inštalátor, overí jeho SHA-256 súčet oproti súboru
  `SHA256SUMS.txt` z toho istého vydania, spustí inštalátor (bez tichých
  prepínačov, sprievodca sa pýta ako pri ručnom spustení) a appka sa ukončí.
  Pri nezhode súčtu appka stiahnutý súbor zmaže a zobrazí chybu.
- Odkaz na stránku vydania sa otvára v predvolenom prehliadači cez appku,
  nie priamym prekliknutím; appka pustí iba adresu na stránke tohto vydania
  (`github.com/SouthCarpet/Abakus/releases/...`).
- Inštalátor (`packaging/abakus.iss`) teraz pred inštaláciou skontroluje
  modul Microsoft Edge WebView2 Runtime. Ak chýba, opýta sa (po slovensky),
  či ho má stiahnuť a nainštalovať (asi 2 MB). Pri **Nie** alebo pri zlyhaní
  appku aj tak nainštaluje, iba upozorní, že sa appka bez modulu nespustí.
  Nevyžaduje práva správcu.
- Dialóg hesla k účtu (`SetPasswordDialog`) teraz obmedzuje dĺžku na 512
  znakov priamo v poli (`maxLength`) a pred uložením ukáže hlásenie
  **Limit je 512 znakov.**, tlačidlo **Uložiť** ostáva zablokované. Pole má
  aj miesto naviac vpravo, aby dlhý text neprekryl ikonu zobrazenia hesla
  (WebView2).

## Vývojárska poznámka: skúšobná adresa kontroly aktualizácie

V debug zostave appka rešpektuje premennú prostredia
`ABAKUS_RELEASES_URL_OVERRIDE` (čítanú pri kompilácii), ktorá nahradí
skutočnú adresu GitHub API vlastným testovacím serverom. V bežnej (release)
zostave sa táto premenná nikdy nečíta a appka vždy volá iba skutočné GitHub
API. Slúži výhradne na overenie celého toku sťahovania a inštalácie pred
tým, ako vydanie skutočne existuje na GitHube.

## Čo sa nezmenilo

- Kontrola aktualizácie ostáva jediné sieťové volanie mimo sťahovania
  inštalátora, obe len na tvoj pokyn a v predvolenom stave vypnuté.
- Import, kategórie, pravidelné platby, PDF report a zálohy sa nemenili.
