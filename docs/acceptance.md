# Acceptance record — Task 16 mechanical gates (2026-08-31)

Every command below was run with:

```powershell
$env:ABAKUS_PDFIUM_DIR = "A:/projects-vault/apps/abakus/src-tauri/resources/pdfium"
```

## `cargo test --jobs 4 --workspace`

Exit code 0. 161 tests passed across all crates (`abakus_lib` 12, `commands_json` 20,
`import_flow` 6, `no_network` 4, `update` 2, `parser` unit 57, `fixtures_txt` 14,
`pdf_roundtrip` 3, `rules` unit 15, `store` unit 6, `assign_summary` 15,
`categories` 1, `import` 6, doc-tests 0), 0 failed.

```
   Compiling tauri-runtime-wry v2.11.4
   Compiling tauri-macros v2.6.3
   Compiling lopdf v0.44.0
   Compiling tempfile v3.27.0
   Compiling store v0.1.0 (A:\projects-vault\apps\abakus\crates\store)
   Compiling parser v0.1.0 (A:\projects-vault\apps\abakus\crates\parser)
   Compiling tauri v2.11.5
   Compiling rules v0.1.0 (A:\projects-vault\apps\abakus\crates\rules)
   Compiling tauri-plugin-fs v2.5.1
   Compiling tauri-plugin-dialog v2.7.2
   Compiling abakus v0.1.0 (A:\projects-vault\apps\abakus\src-tauri)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3m 50s
     Running unittests src\lib.rs (target\debug\deps\abakus_lib-b2466d76bc91c70e.exe)

running 12 tests
test update::tests::wants_check_follows_only_the_persisted_flag ... ok
test update::tests::older_tag_is_not_newer ... ok
test update::tests::malformed_tag_is_never_newer ... ok
test net::tests::is_localhost_classifies_loopback_only ... ok
test update::tests::equal_versions_are_not_newer ... ok
test update::tests::missing_tag_is_never_newer ... ok
test update::tests::newer_tag_is_newer ... ok
test net::tests::is_localhost_classifies_ipv4_mapped_loopback_too ... ok
test update::tests::prerelease_tag_is_never_reported_as_newer ... ok
test net::tests::fetch_and_log_records_a_success_row ... ok
test net::tests::fetch_and_log_rejects_a_body_over_the_cap_and_logs_it ... ok
test net::tests::fetch_and_log_records_a_failure_row_and_still_returns_err ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src\main.rs (target\debug\deps\abakus-bfb4012e0f3e7ee8.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\commands_json.rs (target\debug\deps\commands_json-f6874cfc3dfeb59c.exe)

running 20 tests
test assign_outcome_serializes_snake_case_incl_skipped_transfers ... ok
test bad_checksum_serializes_snake_case_all_four_fields ... ok
test category_serializes_snake_case ... ok
test account_serializes_snake_case ... ok
test assign_args_match_the_ui_call ... ok
test checksum_off_by_serializes_with_tag_and_content ... ok
test clear_password_args_match_the_ui_call ... ok
test net_log_row_serializes_snake_case ... ok
test import_statements_args_match_the_ui_call ... ok
test net_log_args_match_the_ui_call ... ok
test recent_statement_serializes_snake_case ... ok
test import_report_serializes_camel_case ... ok
test recent_statements_args_match_the_ui_call ... ok
test release_serializes_with_the_fields_the_ui_reads ... ok
test rule_view_serializes_snake_case ... ok
test save_category_args_match_the_ui_call ... ok
test summary_args_match_the_ui_call ... ok
test set_check_updates_args_match_the_ui_call ... ok
test tx_filter_deserializes_snake_case_including_statement_id ... ok
test tx_row_and_summary_serialize_snake_case ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\import_flow.rs (target\debug\deps\import_flow-6bdc906c234c37e6.exe)

running 6 tests
test encrypted_becomes_locked_and_other_errors_become_error ... ok
test unknown_account_fixture_reports_unknown_account ... ok
test remember_password_is_not_called_for_an_unknown_account ... ok
test unknown_account_report_carries_iban_and_kind_for_the_dialog ... ok
test remember_password_writes_and_flags_the_account_on_success ... ok
test imported_report_has_counts_and_checksum ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s

     Running tests\no_network.rs (target\debug\deps\no_network-f6326583ec39244f.exe)

running 4 tests
test lockfile_has_no_network_plugin ... ok
test reqwest_is_used_only_inside_net_rs ... ok
test only_the_app_crate_declares_reqwest ... ok
test no_manifest_declares_a_banned_network_crate ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests\update.rs (target\debug\deps\update-3ab391108c0bf82b.exe)

running 2 tests
test transport_is_never_called_when_check_updates_is_off ... ok
test transport_is_called_when_check_updates_is_on ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src\lib.rs (target\debug\deps\parser-3723071a35219cba.exe)

running 57 tests
[... 57 parser unit tests, all ok ...]

test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running unittests src\bin\abakus-cli.rs (target\debug\deps\abakus_cli-13a5ea770b354318.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\fixtures_txt.rs (target\debug\deps\fixtures_txt-52536e6a166ce27a.exe)

running 14 tests
test garbage_is_rejected ... ok
test empty_statement_has_no_records_and_an_ok_checksum ... ok
test unknown_account_fixture_parses_with_its_own_iban ... ok
test wrapped_closing_line_is_still_parsed ... ok
test card_block_split_by_a_page_break_stays_one_transaction ... ok
test missing_closing_balance_is_not_verifiable ... ok
test business_is_business_with_four_records_and_ok_checksum ... ok
test missing_opening_line_is_not_verifiable_even_if_sums_agree ... ok
test wrong_closing_balance_reports_the_difference ... ok
test personal_has_eight_records_in_order ... ok
test other_block_keeps_its_description_as_merchant ... ok
test personal_header_and_period ... ok
test personal_checksum_is_ok_and_the_only_warning_is_the_unknown_fee_kind ... ok
test standing_order_targets_the_business_account ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s

     Running tests\pdf_roundtrip.rs (target\debug\deps\pdf_roundtrip-1832649d44558532.exe)

running 3 tests
test missing_file_is_a_pdf_error ... ok
test a_non_statement_pdf_is_rejected ... ok
test synthetic_pdf_parses_like_its_text_source ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s

     Running unittests src\lib.rs (target\debug\deps\rules-693c3bd4decef4aa.exe)

running 15 tests
[... 15 rules unit tests, all ok ...]

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src\lib.rs (target\debug\deps\store-987cadb476d37bd8.exe)

running 6 tests
test net_log::tests::append_then_read_round_trips_newest_first ... ok
test settings::tests::set_then_get_round_trips ... ok
test settings::tests::default_is_off ... ok
test net_log::tests::limit_caps_the_returned_rows ... ok
test net_log::tests::empty_log_reads_back_empty ... ok
test net_log::tests::append_trims_the_table_to_the_newest_max_rows ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s

     Running tests\assign_summary.rs (target\debug\deps\assign_summary-426b879f3ec21194.exe)

running 15 tests
test statements_with_bad_checksum_reports_the_off_by_amount ... ok
test reclassify_after_account_change_flips_even_confirmed_rows ... ok
test reclassify_after_account_change_turns_a_now_own_transfer_and_moves_the_summary ... ok
test csv_has_header_and_one_line_per_row ... ok
test totals_exclude_transfers_and_net_refunds ... ok
test confirm_turns_a_suggestion_into_a_rule ... ok
test date_filter_uses_tx_date_not_posted_date ... ok
test confirm_never_touches_a_transfer_row ... ok
test deleting_a_rule_reopens_the_suggestion_it_made ... ok
test account_filter_limits_the_summary ... ok
test assign_rolls_back_the_whole_batch_on_any_failure ... ok
test combined_filter_matches_the_exact_surviving_row ... ok
test assign_skips_transfer_rows_and_reports_them ... ok
test assign_confirms_and_creates_exact_and_merchant_rules ... ok
test apply_to_matching_reclassifies_open_rows_of_that_merchant ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.68s

     Running tests\categories.rs (target\debug\deps\categories-c9fd42a507f218fb.exe)

running 1 test
test renaming_a_system_category_is_refused ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\import.rs (target\debug\deps\import-e4e7eff08db11441.exe)

running 6 tests
test seeds_every_category_and_every_seed_rule_resolves ... ok
test unknown_account_is_refused_with_iban_and_kind ... ok
test import_inserts_all_and_reports_checksum ... ok
test own_account_transfer_and_seed_suggestions_are_classified_on_import ... ok
test same_transactions_from_another_file_are_duplicates ... ok
test same_file_twice_is_already_imported ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s

   Doc-tests abakus_lib / parser / rules / store: 0 tests each, ok.
```

## `cargo clippy --workspace --all-targets -- -D warnings`

Exit code 0, no warnings.

```
    Checking store v0.1.0 (A:\projects-vault\apps\abakus\crates\store)
    Checking parser v0.1.0 (A:\projects-vault\apps\abakus\crates\parser)
    Checking abakus v0.1.0 (A:\projects-vault\apps\abakus\src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 13.17s
```

## `node node_modules/vitest/vitest.mjs run`

Exit code 0.

```
 Test Files  13 passed (13)
      Tests  46 passed (46)
   Start at  15:36:53
   Duration  8.28s (transform 3.64s, setup 12.05s, import 10.72s, tests 1.45s, environment 48.19s)
```

## `node node_modules/typescript/bin/tsc -b`

Exit code 0, no output (no type errors).

## `node node_modules/eslint/bin/eslint.js src`

Exit code 0, no output (no lint findings).

## `git config core.hooksPath`

```
scripts/hooks
```

## `fc.exe CLAUDE.md AGENTS.md`

```
Comparing files CLAUDE.md and AGENTS.MD
FC: no differences encountered
```

## IBAN history audit (A12)

`git log --all -p | grep -oE 'SK[0-9]{2} ?[0-9]{4} ?[0-9]{4} ?[0-9]{4} ?[0-9]{4} ?[0-9]{4}'`,
normalized (spaces stripped, uppercased) and deduped, over the full committed history up
to and including commit `956e2ff`:

```
SK0281800000007000000001
SK0902000000005230000001
SK3112000000198742637541
SK3212000000198742637541
SK3711000000000098765432
SK4411000000000012345678
```

All six are on the pre-commit allowlist that existed before this task
(`SK4411000000000012345678`, `SK3711000000000098765432`, `SK0281800000007000000001`,
`SK0902000000005230000001`, `SK3112000000198742637541` the Wikipedia example,
`SK3212000000198742637541` the same example with one checksum digit changed on purpose
for the `is_valid` rejection test). No unlisted IBAN is present in history. This
commit adds a seventh fixture, IBAN `SK8911000000000055555555` for the unknown-account
edge case (check digits verified: `is_valid("SK8911000000000055555555")` is `true`,
computed independently), and the same seventh entry to `scripts/hooks/pre-commit`'s
`ALLOW` list, so it will be on the allowlist for the commit that first introduces it.

A second, differently-shaped check across every tracked and untracked-but-not-ignored
file in the working tree (not just committed history) confirms the same seven IBANs are
the only ones present, all seven on the (updated) allowlist.

## Icon decision

`src-tauri/icons/icon.ico` was a 134-byte placeholder, unreferenced by
`src-tauri/tauri.conf.json` (no `bundle.icon` key; `bundle.active` is `false`). Pillow
(12.3.0) is installed but has no SVG parser, so `scripts/generate-icon.py` draws the
same primitives as `assets/icon.svg` (rounded background, stroked rounded square, two
bars, three dots, same colours and coordinates) with `PIL.ImageDraw` at 4x supersample
and downsamples to a real multi-size `.ico` (16, 32, 48, 64, 128, 256 px), replacing the
placeholder. New size: 21,350 bytes.
