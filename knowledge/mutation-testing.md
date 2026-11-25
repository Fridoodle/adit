# Mutation testing adit-core

Summary: how to run cargo-mutants here and why field checks look like equivalent mutants.

Concerns: `crates/adit-core/src/{engine,document}.rs`, `crates/adit-core/tests/`.

- Run: `cargo mutants -p adit-core --jobs 2 --timeout 120` (~25 min, 699 mutants). One line only:
  add `--file crates/adit-core/src/engine.rs --re "<regex>"`.
- `Document::apply` ends with `validate()` and maps a failure to `InvalidArgument`. So weakening a
  field check inside a command (`||` → `&&`) still yields the same error and the mutant survives. Kill
  it with a `Batch` whose later command repairs the bad value: only the command's own check can then
  fail the batch (C-BATCH-1).
- Known equivalent mutants (2026-11-25): `engine.rs` `clear` boundary comparisons (a trim by 0 changes
  nothing) and the `CloseGap` `>` vs `>=` (a clip starting at `time` is rejected either way).
- First run: 495 of 517 viable mutants caught; after boundary tests 18 more killed, 4 equivalent.

Verified: 2026-11-25
