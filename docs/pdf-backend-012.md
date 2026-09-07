# PDF backend in Abakus 0.1.2

The native backend captures and saves a complete transaction report for one
calendar month, six calendar months, one calendar year, or all stored time.
The request selects all accounts, every account of one kind, or one exact
account. Table search, category, status, statement, and page filters do not
enter the request.

## Data capture

`Store::report_snapshot` starts a deferred SQLite read transaction before its
first query. Account validation, transaction rows, current category names and
kinds, statement ranges, coverage, and totals all come from this one snapshot.
The method returns owned data and ends the transaction before rendering starts.
The renderer does not write to SQLite. Recurring decision and forecast tables
are outside its query set.

Finite periods retain their full calendar end. A current period says that it
has not ended. All-time range is null when no selected transaction exists.
Transactions use `tx_date` and keep descending `(tx_date, id)` order. Every
selected row is listed. `posted_date` appears only when it differs.

Money remains signed integer cents. Income, expense, refunds, category-kind
mismatches, and transfers follow the existing Summary rules. Incoming and
outgoing transfers are separate. Original foreign currency appears only on its
transaction. All aggregation uses `i128` intermediates and rejects an `i64`
overflow.

Statement coverage is calculated per account. Overlapping and adjacent valid
ranges merge. Finite reports show leading and trailing gaps. All-time reports
show gaps only inside known statement bounds. Invalid ranges add no coverage.
Unverified checksums remain a separate warning, so statement presence is not
presented as verified completeness.

## Document format

The report is A4 portrait with selectable vector text and vector charts. It has
the exact period, capture time and offset, account scope, coverage warnings,
integer KPI values, monthly or yearly chart keys, category keys, every
transaction, and `Strana X z Y` on every page. More than 24 all-time months use
named yearly buckets. More than 24 years continue in panels of at most 24.
Expense categories show ten largest absolute values and a reconciled
`Ostatné` entry for the remainder, including a zero remainder that hides
opposite signed values.

Rows wrap at word boundaries measured from glyph advances. A token wider than
its column uses measured character wrapping. A row can continue across pages
and says `Pokračovanie položky N`. Its ordinal and amount are counted once.
Table pages repeat the header and report context. The full note remains
present.

Noto Sans Regular and Bold 2.008 are bundled under SIL OFL 1.1. The PDF embeds
the full TrueType programs as `FontFile2`, uses Type0 Identity-H fonts with
CIDFontType2 descendants, an explicit CID-to-glyph map, and a bounded
`ToUnicode` map. Each Unicode scalar has its own CID. Text is normalized to NFC
for rendering. LF remains a line break and tabs become spaces. Unsupported and
control scalars become visible `[U+XXXX]` text, with a replacement count.
Arabic and Indic shaping, ligatures, and emoji glyph rendering are outside this
release. Unsupported emoji still remains visible as its scalar escape.

The paper colors adapt the light Kaliber tokens in `src/tokens.css`: dark ink,
muted text, a pale accent surface, green accent, blue comparison bars, and red
warnings. Exact RGB constants live in `src-tauri/src/report/layout.rs` because
the PDF does not run CSS.

## Bounds and file publication

Capture rejects more than 100,000 transactions or 32 MiB of source text used
by the report. Layout rejects more than 2,000 pages. Publication rejects a PDF
larger than 100 MiB. Each failure asks for a shorter period or narrower account
scope. No bound silently truncates output.

The destination must be an absolute local `.pdf` path. Device, UNC, alternate
data stream, directory, non-PDF, NUL-character, and existing targets are
rejected. Windows device leaves are also rejected with extensions:
CON/PRN/AUX/NUL/CLOCK$, COM1-9, LPT1-9, and their documented superscript 1-3
forms. Ordinary names such as COM10 remain valid. The backend writes an owned
temporary file in the selected directory, propagates serialization and write
errors, flushes it, calls `sync_all`, reads its final size, then uses
`persist_noclobber`. A race produces one complete winner and one target-exists
error. A failed operation drops only its owned temporary file. The success
result contains the final path, bytes, pages, and the actual fresh capture
preview. The app does not open the file automatically and does not claim
storage durability beyond write, flush, file sync, and no-clobber publication.

## Native commands and integration

`preview_pdf_report` captures under the store mutex and returns the preview.
`export_pdf_report` captures again after file selection, releases the mutex,
and renders in `tauri::async_runtime::spawn_blocking`. No SQLite transaction or
mutex crosses an await or the renderer.

Lane D added only these accepted integration lines to existing registries:

- `pub mod report;` in `crates/store/src/lib.rs`
- `pub mod report;` and `pub mod report_commands;` in `src-tauri/src/lib.rs`
- handlers `report_commands::preview_pdf_report` and
  `report_commands::export_pdf_report`

The final controller must preserve these lines while combining the v4
recurring and category work. Native dialog flows, final PDFium page inspection,
visual grading, combined production build, installation, and real-data
preservation remain controller acceptance work.
