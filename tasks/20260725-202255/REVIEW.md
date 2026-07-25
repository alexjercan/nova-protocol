# Review: Mirror OnNeutralized handlers in The Ledger webmod

- TASK: 20260725-202255
- BRANCH: master

## Round 1

- VERDICT: APPROVE
- REVIEWER: in-session; sub-agent delegation was not explicitly authorized, and
  the work was already in the shared checkout.

Findings: none.

Verified:

- `nix develop --command cargo fmt --check`
- `nix develop --command cargo run -p nova_assets --bin content -- lint --target the-ledger`
- `nix develop --command cargo test -p nova_assets --test ledger_ch2_encounter --test ledger_ch3_channel --test ledger_ch4_ending --test ledger_ch5_raid`

Notes:

- The audit found chapter one also needed the player neutralization retry.
- Ch2, ch2b, and ch5 counter paths now share per-target down flags between
  `OnDestroyed` and `OnNeutralized`, so neutralize-then-destroy cannot double
  count.
- Destroy-only paths were left alone for non-ship or unarmed targets such as
  ch2/ch2b `dray_mule` and ch1 cargo/blackbox objects.
