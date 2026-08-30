# Abakus

Local-only bank-statement analyzer (Tatra banka PDF). Spec: `A:/projects-vault/docs/superpowers/specs/2026-08-30-abakus-bank-statement-analyzer-design.md`. Plan: `A:/projects-vault/docs/superpowers/plans/2026-08-30-abakus.md`.
Rules: no network code, ever; never read `fixtures/private/`; money in integer cents; labels compared through `parser::fold`; Kaliber tokens only; no em dashes.
Commands: `cargo test --jobs 4 --workspace`, `cargo clippy --workspace --all-targets`, `npm test`, `npm run tauri dev`.
