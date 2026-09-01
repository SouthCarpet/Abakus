# Abakus

Local-only bank-statement analyzer (Tatra banka PDF). Spec: `A:/projects-vault/docs/superpowers/specs/2026-08-30-abakus-bank-statement-analyzer-design.md`. Plan: `A:/projects-vault/docs/superpowers/plans/2026-08-30-abakus.md`.
Rules: no network code outside the opt-in, audited update check (`src-tauri/src/net.rs`, off by default, every call logged); never read `fixtures/private/`; money in integer cents; labels compared through `parser::fold`; Kaliber tokens only; no em dashes.
Commands: `cargo test --jobs 4 --workspace`, `cargo clippy --workspace --all-targets`, `npm test`, `npm run tauri dev`.


<!-- multi-vault-block:v1 -->
### Multi-vault adapter (skills)

The central registry at `A:/projects-vault/vault/ops/registry.yaml` is the
single source of truth for which vaults exist on this machine. Any skill
that needs to enumerate or operate across vaults should call the central
adapter rather than re-parse the registry:

```powershell
py A:/projects-vault/scripts/multi_vault.py list --include-self --json
py A:/projects-vault/scripts/multi_vault.py paths --no-self
py A:/projects-vault/scripts/multi_vault.py current --json
py A:/projects-vault/scripts/multi_vault.py find <kw> --limit 10 --json
py A:/projects-vault/scripts/multi_vault.py for-each --cwd-mode project -- <cmd>
py A:/projects-vault/scripts/multi_vault.py doctor
```

The adapter auto-detects the caller's project from `$cwd`. No env var
required for the common case.

**Calling convention for upgraded skills:**

| Invocation                        | Scope                                         |
|-----------------------------------|-----------------------------------------------|
| `/<skill>`                        | THIS project's vault only (original behavior) |
| `/<skill> --all`                  | Every enabled vault in the registry           |
| `/<skill> --vault <name>`         | One named sibling vault                       |

No separate tag is required. Default behavior stays single-vault so
existing callers are unaffected. See
`A:/projects-vault/scripts/README.md` for the full list.
<!-- /multi-vault-block:v1 -->
