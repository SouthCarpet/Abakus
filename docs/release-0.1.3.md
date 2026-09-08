# Abakus 0.1.3

Verzia 0.1.3 pridáva priame nastavenie a zmenu hesla k výpisom účtu.

## Čo sa zmenilo

- V Nastaveniach pribudlo pri každom účte tlačidlo **Nastaviť heslo** (alebo **Zmeniť heslo**, ak je už uložené). Dialóg žiada heslo a jeho zopakovanie; **Uložiť** je dostupné, až keď sa zhodujú a nie sú prázdne.
- Heslo sa ukladá do Správcu poverení systému Windows rovnakým spôsobom ako doteraz pri importe so **Zapamätať pre tento účet**. Táto nová cesta na to nepotrebuje čerstvý zamknutý PDF výpis.
- Po úspešnom uložení appka zobrazí potvrdenie a riadok účtu ukáže **heslo uložené**. Pri zlyhaní appka zachová rozpísané heslo a zobrazí dôvod.

## Čo sa nezmenilo

- Import zamknutého PDF so **Zapamätať pre tento účet** funguje ako predtým.
- **Zabudnúť heslo** ostáva na svojom mieste vedľa nového tlačidla.
- Heslo naďalej nie je nikde v databáze, iba v Správcovi poverení. Appka ho nikde nevypisuje ani nezaznamenáva do denníka.
- Žiadna iná časť appky (import, kategórie, pravidelné platby, PDF report, zálohy) sa nemenila.
