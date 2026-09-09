# Known issues

- **Pravidelné platby sú odhady z importov.** Chýbajúce pokrytie nepreukazuje
  vynechanú platbu. Odhad ukončenia neznamená zrušenie zmluvy. Ručný výber
  konkrétnych platieb automaticky nepridáva budúce platby. Appka nevytvára
  bankové príkazy, upozornenia ani plánované úlohy.
- **PDF má výslovné limity.** Najviac 100 000 transakcií, 32 MiB zdrojového
  textu, 2 000 strán a 100 MiB výsledného súboru. Prekročenie ukončí export
  chybou, nie skráteným výpisom. Nepodporovaný znak má viditeľný zápis
  `[U+...]`. PDF používa vlastný výber účtov a obdobia, nie filtre tabuľky.
- **Detail pravidelnej platby s veľmi dlhým názvom obchodníka rozšíri stránku.**
  Tabuľka obchodníkov v detaile (Prehľad) nemá vlastné vodorovné posúvanie.
  Názov obchodníka s niekoľkými stovkami znakov (nameraný syntetický prípad:
  372 znakov) rozšíri stránku pri šírke 1280 px a treba ju posunúť vodorovne.
  Bežné názvy z výpisov sa zmestia. Testy rozloženia pripínajú názvy tried,
  nie geometriu; overenie geometrie robí kontrola snímok pri vydaní.
- **Ručný formulár na nové pravidlo nie je dostupný.** Pravidlá sa učia
  pri priradení alebo potvrdení transakcie s rozpoznaným obchodníkom.
  Samotné vytvorenie kategórie pravidlo nevytvorí. Cieľ vstavaného pravidla
  možno zmeniť v detaile ním zaradenej transakcie.

- **Záloha je nešifrovaný súbor SQLite.** Obsahuje bankové údaje a poznámky.
  Pôvodné PDF a heslá zo Správcu poverení do nej nepatria. Obnova nemá ovládanie
  v appke; zálohovanie nemá plánovač.
- **Pokrytie sa vzťahuje na obdobia výpisov.** Úplné pokrytie nie je dôkazom,
  že existujú všetky transakcie pre daný rozsah dátumov. Transakcia sa môže
  objaviť až vo výpise za neskorší mesiac. Historický zostatok je stav ku dňu
  výpisu, nie aktuálny stav účtu. Porovnanie pri voľbe Všetko nie je dostupné.

- **pdfium binary: trust on first use.** `scripts/fetch-pdfium.ps1` pins a release tag
  (`chromium/7469`) from `bblanchon/pdfium-binaries` and checks the archive against
  `scripts/pdfium.sha256`. The hash file was written on the first run, from that same
  download. This proves the binary stays the same across re-runs. It does not prove the
  binary was safe on day one. Treat the pinned hash as trust-on-first-use, not as an
  independent security check.
- **Uložené heslo potrebuje samostatnú kontrolu systému Windows.** Audit 078
  overil skutočný zamknutý PDF vytvorený zo syntetických údajov, zadanie hesla
  a následné spracovanie bez uloženia hesla. Automatické opätovné použitie
  skutočného hesla zo Správcu poverení nebolo súčasťou tejto kontroly.
  Chyby čistenia poverení pokrývajú testy s náhradnou službou. 0.1.3 pridala
  druhú cestu k tomu istému úložisku: nastavenie alebo zmenu hesla priamo v
  Nastaveniach, bez čerstvého PDF na overenie (`set_account_password`,
  testované náhradnou funkciou namiesto Správcu poverení). Skutočný zápis do
  Správcu poverení systému Windows nie je pokrytý žiadnym automatizovaným
  testom v tomto repozitári.
- **Net-audit connection sampler is a single snapshot, not continuous monitoring.**
  `sample_connections` reads the process's open TCP sockets once, at app start
  (`src-tauri/src/lib.rs`), and again whenever the settings screen re-runs it. A
  connection opened and closed between two samples leaves no trace in `net_log`. This
  catches a leak that stays open; it does not catch a brief one.
- **Closing-balance label set is fixed, not learned.** The parser recognizes four
  Slovak wordings for the closing-balance line (`parser::statement::CLOSING_LABELS`,
  the Tatra banka wording first, three fallbacks). A statement layout using a fifth
  wording reports `not_verifiable` rather than a wrong number, which is the safe
  failure mode, but it still means an unrecognized label needs a new fixture line and
  a parser fix rather than being handled automatically.
- **Foreign-currency rate precision.** `card_foreign` transactions store the exchange
  rate as `rate_micros` (six decimal digits). Re-deriving the original-currency amount
  from `amount_cents` and `rate_micros` can be off by a cent from the bank's own
  printed original amount because of this rounding. The parser keeps both the printed
  original amount and the rate. Recurring price changes use the original currency;
  a projection in euros remains an estimate based on the last booked conversion.
- **Installer exists but is unsigned, and there is still no release.**
  `packaging/abakus.iss` and `packaging/build-installer.ps1` build a per-user
  Inno Setup installer (see `packaging/INSTALL.md`). `src-tauri/tauri.conf.json`
  keeps `bundle.active: false` on purpose: Inno is the bundler, not Tauri's
  built-in one. The installer is not signed, and there is no GitHub release or
  tag yet, so the in-app update check has nothing to find (see the next
  entry).
- **Update check needs a published release.** `check_update_now` reads GitHub's
  `releases/latest` endpoint (`src-tauri/src/update.rs`). Until the repo has a
  published release with a tag, that endpoint has nothing to return, so a fresh
  install reports "no update" even when the code on `main` is newer than the copy
  the user built.
- **The `Aktualizovať na <tag>` button only appears when a release has both
  an installer asset and a `SHA256SUMS.txt` asset.** `update::installer_assets`
  (spec 0.1.4) requires both to populate `installer_url`/`checksums_url`; a
  release published with just source archives, or with an installer but no
  checksums file, still shows the quiet informational line and the link, never
  a button with nothing to verify against.
- **"Použiť aj na podobné" never rewrites a confirmed row.** Applying a category to
  matching transactions (`Store::assign` with `apply_to_matching`, see
  `crates/store/src/assign.rs`) re-files only rows still `suggested` or
  `unassigned`. A row already `confirmed` keeps the category the user chose for it.
  This is intentional, not a gap: a confirmed choice is a decision Abakus never
  silently overwrites (spec-adjudicated 2026-08-31).
- **`apply_to_matching` now writes `rule_id` on matching rows.** A rule can
  therefore be used by transactions from several statements.
- **`hit_count` changes meaning after the first statement deletion.** It is
  then recomputed from rows whose `rule_id` points to each rule. The displayed
  count can drop.
- **`save_account` fails when an existing account's kind changes and it has
  imports.** The caller must use `update_account` and acknowledge the change.
- **Audit-write failures exist only in the current process.** Nastavenia shows
  them until the application closes.
- **Two chips share the label "Všetko".** On Prehľad the period cluster and the account-type cluster each end with a chip named "Všetko" (`src/screens/Overview.tsx`). Position tells them apart, wording does not. Reported by the closing visual verification (2026-09-01) as a SHOULD, left as a residual rather than reopening the round.
- **Poznámky k vydaniu v karte Stav vykresľujú iba nadpisy a odrážky.** Vnorený
  Markdown (`**tučné**`, odkazy, spätné apostrofy) sa zobrazí doslovne, nie
  naformátovaný.
- **Stav "Overujem podpis súboru…" sa nikdy nevykreslí.** Overenie beží v tom
  istom volaní ako sťahovanie, takže používateľ vidí "Sťahujem inštalátor…" a
  potom rovno "Spúšťam inštalátor…".
- **`SHA256SUMS.txt` riadok s názvom súboru obsahujúcim medzeru sa nenájde.**
  Hľadanie riadku delí text podľa bielych znakov. Dnešné názvy súborov medzeru
  nemajú.
- **Tabuľka Sieťová aktivita sa počas sťahovania neobnovuje automaticky.**
  Dáta v `net_log` sú úplné, iba obrazovka ich nenačíta znova sama od seba.
- **The monospace spacing path in `parser::lines` was tuned on one statement
  from a new Tatra banka PDF generator (dated 2026-06-30).** `group_lines`
  now handles a generator that reports identical loose and tight character
  boxes (see `crates/parser/README.md`), fit to that one statement's own
  `abakus-cli geometry` diagnostic. Other pages from the same generator are
  not covered by an automated fixture; they are verified only by Michal's
  own `abakus-cli check` runs on his real statements.

