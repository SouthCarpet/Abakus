# Bezpečnosť

Abakus je lokálna desktopová appka. Nemá server a nezbiera telemetriu.

## Ako nahlásiť chybu

Použite [súkromné hlásenie zraniteľnosti na GitHub](https://github.com/SouthCarpet/Abakus/security/advisories/new).

Do verejného issue nedávajte exploit, heslo, bankové PDF ani postup, ktorý by uľahčil únik údajov.

Ak súkromný formulár nie je dostupný, otvorte verejné issue len s prosbou o súkromný kontakt. Technické detaily tam nedávajte.

Do hlásenia uveďte:

- verziu appky (napríklad 0.1.4);
- verziu Windows;
- čo ste čakali a čo sa stalo;
- minimálny postup bez skutočných bankových dát.

## Čo je v rozsahu

Chyby v kóde tohto repozitára, ktoré porušia sľuby appky:

- odoslanie bankových dát, hesiel alebo telemetrie mimo počítača;
- sieťové volanie mimo voliteľnej kontroly aktualizácií a sťahovania inštalátora z GitHub Releases, ktoré spustí používateľ;
- únik hesla k výpisu zo Správcu poverení systému Windows do databázy, denníka alebo obrazovky;
- zápis hesla do `%LOCALAPPDATA%\Abakus\abakus.db`;
- obídenie denníka **Sieťová aktivita** pri týchto dvoch volaniach.

## Čo nie je v rozsahu

- samovoľná kompromitácia počítača, na ktorom appka beží;
- nepodpísaný inštalátor a upozornenie Windows SmartScreen;
- PDFium ako projekt tretej strany, okrem spôsobu, akým ho Abakus sťahuje a pripája;
- obnova zo zálohy, ktorú appka ešte neponúka;
- problémy, ktoré vyžadujú už sprístupnený Správca poverení iným programom.

## Heslá a dáta

Heslá k PDF výpisom sú v Správcovi poverení systému Windows. Nie sú v databáze. Appka ich nevypisuje do denníka.

Databáza je súbor `%LOCALAPPDATA%\Abakus\abakus.db`. Záloha z Nastavení je nešifrovaný súbor SQLite. Uložte ju na bezpečné miesto.

## Sieť

Sieť je predvolene vypnutá. Po zapnutí **Kontrolovať aktualizácie (GitHub)** appka urobí neautentifikovaný `GET` na najnovšie vydanie. Po kliknutí na `Aktualizovať na v<verzia>` stiahne inštalátor z GitHub Releases a overí ho voči `SHA256SUMS.txt`. Obe volania sú v Nastaveniach v **Sieťová aktivita**. Bankové údaje sa pri nich neposielajú.
