# Plán 091, bod 5: uložená voľba aktualizácií

## Správanie

Voľba kontroly aktualizácií ostáva v SQLite nastavení `check_updates`.
Chýbajúci záznam znamená vypnutú kontrolu. Chyba čítania databázy sa vráti
ako chyba, nie ako úspešne načítaná vypnutá voľba.

Nastavenia povolia prepínač až po načítaní uloženej hodnoty. Pri chybe
čítania ostane neaktívny a obrazovka zobrazí chybu. Opätovné otvorenie
Nastavení načíta hodnotu znova. Neúspešný zápis nezmení zobrazenú uloženú
hodnotu. Existujúci `useAction` blokuje ďalší zápis počas operácie.

Ak sa používateľ vráti do Nastavení počas zápisu, nové načítanie počká
na jeho dokončenie a potom číta databázu. Čaká aj na neúspešný zápis,
po ktorom načíta poslednú uloženú hodnotu. Pamäť obsahuje iba prísľub
dokončenia zápisu. Neobsahuje ďalšiu uloženú kópiu voľby.

Výsledok alebo chyba starej kontroly neprepíše stav po zmene voľby.
Zápis dokončený po odchode z Nastavení už nespustí kontrolu aktualizácie.
Nové Nastavenia s uloženou zapnutou voľbou spustia vlastnú kontrolu.
Backend pred kontrolou sám načíta uloženú voľbu. Vypnutá voľba transport
nespustí. Už odoslaná požiadavka sa touto úpravou neruší.

## Nový povrch

- `updatePreferenceLoaded`: lokálny stav dostupnosti načítanej voľby.
- `pendingWrite`: spoločný prísľub dokončenia zápisu, bez hodnoty voľby.
- `readUpdatePreference()`: počká na zápis a načíta hodnotu cez existujúce API.
- `saveUpdatePreference(on)`: uloží voľbu cez existujúce API a sprístupní dokončenie ďalšiemu načítaniu.
- Pravidlo čítania: iba chýbajúci záznam je predvolená vypnutá voľba; chyby SQL sa vracajú volajúcemu.
- Pravidlo životného cyklu: neaktuálne načítanie nesmie zobraziť chybu ani výsledok a neaktuálny zápis nesmie spustiť kontrolu.

## Overenie a limity

Testy používajú syntetickú SQLite databázu a falošné API. Overujú zatvorenie
a otvorenie Store, návrat do Nastavení, neúspešné čítanie a zápis, rýchle
kliknutia a oneskorené dokončenie. Test databázy prešiel aj pred opravou.
Strata hodnoty pri skutočnom reštarte alebo inštalácii novej verzie sa
nepotvrdila. Natívna aplikácia ani inštalátor sa pri tomto overení nespustili.

Globálny indikátor aktualizácie je tichý riadok v nástrojoch hlavičky
(`src/components/UpdateIndicator.tsx`, `src/lib/updateStatus.ts`). Ukáže sa,
len keď kontrola našla novšie vydanie; indikátor sám nič nekontroluje a
nevolá sieť. Kliknutie otvorí **Nastavenia > O aplikácii**.

Od Kaliber r3 (Michal 2026-10-12) kontrola beží len so súhlasom a len na
troch cestách: pri spustení (ticho, `checkAtStart`), pri otvorení sekcie
O aplikácii a na tlačidlo (`checkNow`), všetko v `src/lib/updateCheck.ts`.
Zapnutie prepínača samo nekontroluje. `check_update_now` vracia pri zlyhaní
požiadavky, HTTP chybe alebo nečitateľnej odpovedi `Err`; čas a výsledok
poslednej kontroly sa ukladajú v `localStorage["abakus.update.lastCheck"]`. Podrobné dôkazy sú
v artefaktoch plánu 091 s predponou `update5-`.
