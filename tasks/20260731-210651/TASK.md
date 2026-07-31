# Make the local test suite runnable: cap link jobs and drop test-binary DWARF

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog
- KIND: TASK
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT

## Context

`cargo test` locally exhausts RAM. Root cause measured, not estimated:

- `rust-lld` (the nightly default linker on x86_64-unknown-linux-gnu; nothing
  in this repo selects it) peaks at **2.94 GB RSS for a single link** of
  `nova_core`'s test binary - one of the SMALLER ones at 465 MB. The 697 MB
  example binaries scale to roughly 4.4 GB each.
- The box is 24 cores / 31 GiB, and cargo's default `-j` equals nproc with no
  separate link-job throttle: 24 x 2.94 GB = 71 GB, or 106 GB on examples.
- CI survives only because `ubuntu-latest` is 2 cores. A 4- or 8-core runner
  would OOM identically.

**The existing mitigation does not work and says it does.** `Cargo.toml:180-186`
sets `split-debuginfo = "unpacked"` with `debug = "line-tables-only"`, and the
comment above claims this "drops the peak to roughly half of RAM". The two
settings cancel: split DWARF externalizes `.debug_info` (down to 1.9 MB, so the
split IS working) but cannot externalize line tables, which is exactly what
`line-tables-only` keeps. `readelf -S` on a test binary shows **274 MB of DWARF
still embedded** (`.debug_ranges` 117 MB, `.debug_line` 110 MB, `.debug_addr`
31 MB). The claim was written and never re-measured.

Aggravating: `flake.nix:65` sets `RUSTC_WRAPPER = "sccache"`, which cannot
cache link steps. On a warm cache the compiles collapse to near-zero and all
the links bunch into one window, so a warm `cargo test` is MORE likely to OOM
than a cold one.

`Cargo.toml:184` also points at
`../bevy-common-systems/docs/2026-07-03-test-memory.md`, which no longer
exists.

## Steps

- [ ] `.cargo/config.toml` - add `jobs = 4` to the existing empty `[build]`.
      4 x 4.4 GB (examples, the worst case) is ~18 GB against 31 GiB.
- [ ] `Cargo.toml` - add `[profile.test] debug = 0`, `split-debuginfo = "off"`
      (`off` is not worse than today: the `.dwo` files are dead weight for a
      target linked once and discarded). Use `debug = 1` instead if line
      numbers in test backtraces are wanted.
- [ ] Re-measure a single link with `/usr/bin/time -v` and write the NEW
      number into the Cargo.toml comment. Do not repeat the mistake of
      claiming an unverified win.
- [ ] Fix the Cargo.toml comment and repoint or delete the dead
      `bevy-common-systems` doc reference.
- [ ] `AGENTS.md:82` - sharpen `cargo test -p <crate>` to
      `cargo test --lib -p <crate>`. `-p nova_assets` alone still links 22
      integration binaries, enough to OOM on its own; `--lib` is the
      load-bearing flag.
- [ ] Record the WHY in AGENTS.md, not just the what. The current line says
      "do not run locally, CI owns both" with no reason and no warning that
      `-p` without `--lib` is also dangerous.

Rejected: `mold` (a link-SPEED tool; its peak RSS on DWARF-heavy links is
comparable or higher, and it is not in the dev shell). `codegen-units` and
`incremental` are non-levers - same bytes reach the linker, and
`CARGO_INCREMENTAL=0` is already set at `flake.nix:66`.

## Definition of Done

- `cmd: /usr/bin/time -v` on a single test-binary link - peak RSS recorded,
  and the Cargo.toml comment states that measured number.
- `cmd: nix develop --command env -u DISPLAY -u WAYLAND_DISPLAY cargo test
  --workspace --features debug` - completes without exhausting RAM on a
  24-core box.
- manual: `AGENTS.md` names `--lib` and gives the reason.
