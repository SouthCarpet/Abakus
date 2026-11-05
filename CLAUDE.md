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

### Documentation follows every change

For every change that alters behaviour, output, limits, host integration or
version-dependent facts, update README and every affected document in the
same task. Briefs name the documents. The additive-surface inventory has one
bullet for each new field, key or rule. An independent verifier checks
documentation currency with quoted facts and section attribution. Before a
version bump, scan all version-bearing sentences, including About, Stav,
install and checksum text. Rule:
`A:/projects-vault/vault/knowledge/rules/rule_docs_follow_every_change.md`.

### Owner decisions go through a choice page

When Michal must decide more than one thing, or choose between visual
proposals, build a local choice page and read his answer from the file the
page sends back. Do not ask a list of questions in chat. Rule:
`A:/projects-vault/vault/knowledge/rules/rule_owner_choice_pages.md`.

```powershell
py A:/projects-vault/scripts/choice_page.py serve <spec.json> --out-dir <dir> --port 0
```

The skill `choice-page` in this project (`.claude/skills/choice-page/SKILL.md`)
holds the steps. Every choice card carries a "Čo to je" block (what it does,
who uses it, what state it is in) above "Prečo sa pýtam". The tool is a plain
CLI plus a browser, so a Codex, Grok or Gemini session uses it the same way as
a Claude session: add `--open` when the session runs outside T3. One simple
question may stay in chat.
<!-- /multi-vault-block:v1 -->
