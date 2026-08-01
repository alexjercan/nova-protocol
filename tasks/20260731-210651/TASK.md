# Make the local test suite runnable: cap link jobs and drop test-binary DWARF

- STATUS: IN_PROGRESS
- PRIORITY: 0
- TAGS: backlog
- KIND: TASK
- FLOW STEP: REVIEWING
- PLAN STATUS: APPROVED

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

- [x] `.cargo/config.toml` - add `jobs = 4` to the existing empty `[build]`.
      4 x 4.4 GB (examples, the worst case) is ~18 GB against 31 GiB.
- [x] `Cargo.toml` - add `[profile.test] debug = 0`. KEEP
      `split-debuginfo = "unpacked"`: measurement reversed the first instinct
      to set it `"off"`, because the `.dwo` files hold ~12.9 GB of dependency
      DWARF that the linker never reads, and `"off"` pushes all of it back
      into rust-lld. Use `debug = 1` instead of `0` if line numbers in test
      backtraces are wanted.
- [x] Re-measure a single link with GNU `time -v` (this box is NixOS - the
      binary is `/run/current-system/sw/bin/time`, there is no `/usr/bin/time`)
      and write the NEW number into the Cargo.toml comment. Do not repeat the
      mistake of claiming an unverified win, and do not round a measured
      fraction up to "half".
- [x] Fix the Cargo.toml comment and repoint or delete the dead
      `bevy-common-systems` doc reference.
- [x] `AGENTS.md:82` - sharpen `cargo test -p <crate>` to
      `cargo test --lib -p <crate>`. `-p nova_assets` alone still links 22
      integration binaries, enough to OOM on its own; `--lib` is the
      load-bearing flag.
- [x] Record the WHY in AGENTS.md, not just the what. The current line says
      "do not run locally, CI owns both" with no reason and no warning that
      `-p` without `--lib` is also dangerous.

Rejected: `mold` (a link-SPEED tool; its peak RSS on DWARF-heavy links is
comparable or higher, and it is not in the dev shell). `codegen-units` and
`incremental` are non-levers - same bytes reach the linker, and
`CARGO_INCREMENTAL=0` is already set at `flake.nix:66`.

## Definition of Done

1. cmd: `nix develop --command /run/current-system/sw/bin/time -v cargo test --lib -p nova_core --no-run -j1` - peak RSS recorded, and the Cargo.toml
   comment states that measured number and no rounder claim.
2. cmd: `readelf -S target/debug/deps/nova_core-*` - no `.debug_*` section
   remains in the linked test binary.
3. cmd: `nix develop --command env -u DISPLAY -u WAYLAND_DISPLAY cargo test --workspace --features debug` - completes without exhausting RAM on a
   24-core box.
4. manual: `AGENTS.md` names `--lib` and gives the reason.

## Close-out

### What and why

Three changes, in descending order of effect:

1. `.cargo/config.toml` `[build] jobs = 4`. The load-bearing fix. Peak is
   `(links in flight) x (per-link cost)`, and cargo's default `-j` is nproc
   with no separate link throttle. Worth ~6x; the debuginfo knobs are worth
   ~30% each.
2. `Cargo.toml` `[profile.test] debug = 0`. Removes the DWARF residue that
   `split-debuginfo = "unpacked"` provably cannot externalize. Covers
   everything `cargo test` links, examples included.
3. `Cargo.toml` `[profile.dev.package."*"] debug = false`, adopted from the
   parallel `bevy-common-systems` investigation. Covers the dev profile, which
   `[profile.test]` never reaches: `cargo build`/`cargo run --example`.

Plus the AGENTS.md sharpening (`--lib` is load-bearing) and the CI-equivalent
headless full-suite form.

### Measurements

All `time -v` at `-j1` unless stated. Full-suite figures are summed toolchain
RSS sampled at 1s, so they are lower bounds (see Difficulties).

| Target / config | Peak RSS | Binary | Embedded DWARF |
| --- | --- | --- | --- |
| `nova_core` lib test, baseline | 2.94 GB | 444 MB | 274 MB |
| ... `package."*" debug=false` only | 1.84 GiB | 206 MB | 48.0 MB |
| ... `[profile.test] debug=0` only | 1.68 GiB | 158 MB | 0 |
| ... both | 1.66 GiB | 158 MB | 0 |
| `thruster_section` example, dev profile, baseline | 2.93 GiB | 582 MB | 262.2 MB |
| ... `package."*" debug=false` | 2.16 GiB | 369 MB | 49.0 MB |

Full `cargo test --workspace --features debug --no-fail-fast` at `jobs = 4`,
warm dependency graph both times: **8.07 GiB** with `[profile.test] debug = 0`
alone, **8.19 GiB** with both knobs, largest sampled rust-lld 2.15 GiB. The
dependency knob is worth nothing on this run and that is expected - every
target in it goes through the test profile.

Supporting: 65,556 `.dwo` files totalling 9.6 GB, of which first-party is
6.96 GB and dependencies 2.64 GB (`nova_gameplay` alone 3.95 GB). 7 doctests.
Edition 2021.

### Alternatives rejected

- `split-debuginfo = "off"` - the first instinct, reversed by measurement.
  `.dwo` holds 2.64 GB of dependency DWARF the linker never reads; `"off"`
  pushes it all back into rust-lld.
- `mold` - a link-speed tool; peak RSS on DWARF-heavy links is comparable or
  worse, and it does not touch concurrency.
- `RUST_TEST_THREADS` - the dominant term in `bevy-common-systems` (60
  doctests) and a rounding error here (7). Setting it would be speculative
  config for something unmeasurable above the noise.
- `-Wl,--no-keep-memory` / `--reduce-memory-overheads` - GNU bfd flags, not
  implemented by lld.
- `codegen-units` / `incremental` - same bytes reach the linker.

### Difficulties and diagnosis

Four measurement attempts failed before one worked. `cargo test --workspace`
fail-fasted after 4 of 64 binaries on an unrelated failure (fixed with
`--no-fail-fast`); the rerun finished in 60s reusing linked binaries and proved
nothing; `touch crates/nova_core/src/lib.rs` recompiled only 3 crates because
nova_core is not the dependency hub it looked like. The working form is
`touch crates/*/src/lib.rs src/*.rs`, confirmed by
`grep -c '^   Compiling'` = 16 and 64 test binaries.

A 1s-interval RSS sampler **under-reports per-process link peaks by ~27%**:
`time -v` (which uses `ru_maxrss` and cannot miss) measured 2.93 GiB on an
isolated link where the sampler's best observation across a whole storm of
comparable links was 2.14 GiB. RSS ramps to a narrow peak near the end of a
link, so a 1s grid samples the ramp even on multi-second links. Whole-run
summed peaks are smoothed by N-in-flight and are more trustworthy. Rule
adopted: sampler for whole-run bounds, `time -v` at `-j1` for per-link.

### Corrections to this task's own Context section

The Context above says examples are 697 MB and scale to ~4.4 GB per link.
**That was an estimate inherited from an earlier agent, never measured.** The
measured figure is 582 MB / 2.93 GiB. The `jobs = 4` arithmetic in the Steps
("4 x 4.4 GB is ~18 GB") is therefore conservative rather than wrong - the real
whole-run peak at that cap is 8.19 GiB. Left in place rather than rewritten so
the correction is visible.

An earlier revision of the `[profile.test]` comment also claimed examples are
built with the dev profile under `cargo test`. Measured refutation: the same
example is 582 MB / 262.2 MB DWARF under `cargo build --example` and
352 MB / 0 under `cargo test`. `[profile.test]` does cover example targets.

### Evidence

1. DoD 1: peak RSS 2.94 GB -> 1.66 GiB, stated in the Cargo.toml comment with
   the per-knob breakdown rather than a single rounded claim.
2. DoD 2: `readelf -S` on the linked test binary reports 0 `.debug_*` sections
   (10 before).
3. DoD 3: `cargo test --workspace --features debug --no-fail-fast` completes at
   8.19 GiB peak on the 24-core / 31 GiB box. Exit 101 from one **pre-existing**
   failure, `scenario::shakedown::tests::an_early_derelict_kill_skips_to_the_fight`
   (1466 passed / 1 failed across 64 binaries). Verified deterministic at
   `--test-threads=1`, at default threads, and in isolation, so it is not a
   concurrency artifact of this change. Filed as `20260731-215407`.
4. DoD 4: AGENTS.md names `--lib` with the reason (bare `-p nova_assets` links
   22 integration binaries) and forbids raising `-j` past the cap.

### Reflection

The instinct that a comment describing a fix means the fix works was the whole
bug: `split-debuginfo = "unpacked"` + `debug = "line-tables-only"` had been
sitting there for months under a comment claiming it halved peak RAM, and the
two settings cancel exactly. Every number in the replacement comments is
measured and says how.

Cross-repo comparison with `bevy-common-systems` was worth more than either
investigation alone. Same two knobs, ~100% of the DWARF mass there and ~27%
here, and the difference is not the config - it is which crates own the
debuginfo. "Measure your `.dwo` ownership, then pick the knob" transfers;
"here is the profile block that fixed it" does not. Exchange recorded in
`/tmp/claude-xchange/`.
