# Review: The Ledger campaign - collapsible header + hidden-chapter replay

- TASK: 20260724-220842
- BRANCH: feature/ledger-campaign

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context (findings), in-session (fix + re-verify)

The out-of-context reviewer ran every proof green: `webmods_validation` (2, incl.
the new `the_ledger_campaign_lists_its_chapters_in_order`), `ledger_ch5_raid` (11,
incl. the re-pinned finale-hidden + version-pin tests), `content -- lint` 0
findings, `cargo fmt --all --check` clean, and spot-ran `ledger_ch2_encounter`
(12) to confirm the name strip broke no name-dependent assertion. Cross-checks all
pass: the campaign lists exactly the 6 chapter ids in play order (verified against
each file's `id:`); the file is in the bundle; ch5 is `hidden: true`, ch1 visible;
version 1.13.0 with the pin test + guide-make-a-mod line updated; the resolution
test fails red on a missing/misordered member or un-hidden ch5; the ch5 test was
legitimately re-pinned (not weakened); news posts (dated history) correctly left
untouched; CHANGELOG entry accurate.

In-session re-verification: independently confirmed the campaign member ids match
the chapter files and re-ran the webmod tests + content lint (all green).

- [x] R1.1 (MINOR) webmods/the-ledger/README.md:8-9 - the name strip left the
  webmod's own README stale: it told the player to start "The Ledger 1: Dead
  Weight" flat from the picker, but that scenario is now "Dead Weight" under a
  collapsible campaign header. My close-out's "referenced nowhere outside the
  webmod" was narrowly true (the README is *inside* the webmod) - the grep was
  scoped too narrowly and missed this in-webmod hit.
  - Response: Fixed. README now tells the player to expand the "The Ledger"
    campaign header and start "Dead Weight", and notes the header lists the hidden
    later chapters for replay. Re-swept the whole `webmods/the-ledger/` tree:
    `grep -rnE 'The Ledger [0-9]|from the Scenarios picker'` returns nothing.

No BLOCKER/MAJOR findings. The one MINOR is fixed.

Pending user check (manual DoD, batched): enable The Ledger webmod, expand its
campaign header in the Scenarios tab, and replay a hidden chapter (e.g. The Raid)
from it.
