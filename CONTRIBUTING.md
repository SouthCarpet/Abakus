# Prispievanie

Ďakujeme za záujem o Abakus. Najprv si prečítajte [README.md](README.md) a [KNOWN_ISSUES.md](KNOWN_ISSUES.md).

## Ako nahlásiť problém

Otvorte issue v [SouthCarpet/Abakus](https://github.com/SouthCarpet/Abakus/issues).

Opíšte, čo ste čakali a čo sa stalo. Uveďte verziu appky. Do issue nedávajte skutočné bankové PDF, IBAN-y, heslá ani výpisy.

Bezpečnostné chyby hláste podľa [SECURITY.md](SECURITY.md).

## Pull requesty

Nová práca ide do `main` cez pull request.

1. Vychádzajte z aktuálneho `main`.
2. Držte zmenu v jednom téme.
3. Texty v rozhraní píšte po slovensky. Krátke vety. Jedna myšlienka na vetu. Nepoužívajte dlhú pomlčku.
4. Súkromné bankové dáta do `fixtures/` nepatria. Povolené sú len syntetické výpisy v `fixtures/synthetic/`. Priečinok `fixtures/private/` nikdy nečítajte a necommitujte.

## Testy pred odoslaním

Potrebujete Windows, Rust (`stable-x86_64-pc-windows-msvc`), Node.js a npm. PDFium raz stiahnite cez `scripts/fetch-pdfium.ps1`.

```powershell
npm install
npm test
$env:ABAKUS_PDFIUM_DIR = "$PWD\src-tauri\resources\pdfium"
cargo test --jobs 4 --workspace
cargo clippy --workspace --all-targets
```

Vývojová appka:

```powershell
npm run tauri dev
```

Inštalátor (Inno Setup 6):

```powershell
.\packaging\build-installer.ps1
```

Zmena správania má v tom istom pull requeste úpravu [CHANGELOG.md](CHANGELOG.md), ak ju používateľ uvidí.
