# Epic: Nova demonstrates itself automatically in v0.10.0

- PRIORITY: 100
- TAGS: v0.10.0, epic, tooling, content, automation
- KIND: EPIC
- ACTIVITY: PLANNING
- GATES: PLAN
- RESOLUTION: -

## Epic

v0.10.0 makes Nova capable of demonstrating itself automatically. Nova-owned
automation drives representative player paths, records correctness and frame
evidence, captures stable images, and publishes the same runs into the game and
website. The release favors one reusable pipeline over unrelated features.

## Delivery Tracks

| Track | Outcome | Child work |
| --- | --- | --- |
| Nova-owned automation | `nova_autopilot` replaces the BCS harness and becomes a generic predicate-driven state machine. | `20260802-120019`, `20260802-120025` |
| Example fleet | A spike sets the roster; every category has a contract, and the runs get deeper and assert what they claim on the shared driver. | `20260804-003244`, `20260802-120029`, `20260804-003301` |
| Asset refresh | The rebuilt `screenshots/` producers plus the python packaging script refresh the web imagery; scenario art gets generated placeholders until the owner draws it. | `20260724-082856`, `20260715-220011` |
| Release confidence | Current tutorial media and warning-clean code ship with the automated evidence. | `20260730-111146`, `20260731-205553` |

## Done Means

- Nova automation no longer imports or activates the BCS debug harness; the
  in-repo crate owns the driver and completion protocol. The `debug::harness`
  alternative needs the `(?<!nova_)` guard: without it the grep also matches the
  `nova_debug::harness::` adapter paths the migration requires, and fails on its
  own success. (cmd: `test -f crates/nova_autopilot/Cargo.toml && ! rg -n --pcre2 "BCS_AUTOPILOT|(?<!nova_)debug::harness" crates examples scripts --glob '*.rs' --glob '*.py' --glob '*.sh'`)
- Automation scripts advance on observed state, not wall-clock: a stalled step
  aborts naming itself, and no example carries a hand-rolled completion guard.
  (cmd: `! rg -n "run ended with the scripted run unfinished|playing_since" examples`)
- The example fleet runs per category contract through the real app and
  produces probe reports for correctness and frame time.
  (cmd: `nix develop --command cargo run -p nova_probe -- run --all --fps`)
- Every game-rendered web screenshot has a declared, reproducible producer,
  and every picker scenario has its own image (generated placeholder or owner
  art). The advisory coverage report runs in CI as a warning and lists only
  `manual` (hand-made art) or `historical` gaps outstanding.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- Workspace compiler and rustdoc warnings introduced or exposed by the release
  are zero. (cmd: `nix develop --command env RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --features debug`)
- The owner accepts the generated probe report, scenario-picker thumbnails,
  and rendered web/tutorial pages as the v0.10.0 demonstration. (manual: inspect generated `report.html`, the Scenarios picker, and the locally rendered website)

## Child Tasks

| ID | Priority | Repo | Title | Landed result |
| --- | ---: | --- | --- | --- |
| `20260802-120019` | 100 | nova-protocol | Move the automation harness into `nova_autopilot` (9 children) | CLOSED/DONE 2026-08-03 (`6a19ebf2`): all nine children landed. `nova_autopilot` is a `bevy`-only crate owning completion, autopilot, screenshot and reel; `BCS_* -> NOVA_*` renamed atomically with no aliases; `nova_debug::harness` is now the Nova adapter filling the caller hooks, and `nova_probe` names the crate directly. Follow-ups: `20260803-114158` (rustdoc nits), `20260803-094601` (per-test timeout), `20260803-143141` (pre-existing `hud_range` smoke failure, closed one criterion red). |
| `20260802-120025` | 85 | nova-protocol | Make `nova_autopilot` predicate-driven | Pending |
| `20260804-003244` | 82 | nova-protocol | Spike: decide the v0.10.0 example fleet roster | Pending |
| `20260802-120029` | 80 | nova-protocol | Rebuild the example fleet per category contract | Pending |
| `20260802-120045` | 75 | nova-protocol | Generate showcase evidence and web assets with one command | CLOSED/WONTDO 2026-08-03: owner call - screenshot capture and packaging stay in `scripts/gen-web-screenshots.py`; `nova_probe` is for profiling and correctness only, and probing a screenshot example yields no useful evidence. Capture refresh lives in `20260724-082856` / `20260715-220011`. |
| `20260724-082856` | 70 | nova-protocol | Refresh frontend app images | Pending |
| `20260715-220011` | 68 | nova-protocol | Generated placeholder thumbnails for the Scenarios picker | Pending |
| `20260730-111146` | 60 | nova-protocol | Refresh the tutorial against current UI and captures | Pending |
| `20260804-003301` | 55 | nova-protocol | Move the design PoC HTML pages into `web/design` | Pending |
| `20260731-205553` | 50 | nova-protocol | Clear compiler and rustdoc warnings | Pending |

## Decisions

- `tasks/20260802-115955/DECISION.md` records Nova-first ownership: build and
  evolve `nova_autopilot` here; extract back to BCS only after real reuse.

## Frontier

Derive with `tatr frontier 20260802-115955`. Hard dependencies encode the
driver -> example fleet -> asset refresh path. Priority keeps
warning cleanup late without blocking it unnecessarily.

## Fog

- The predicate vocabulary in `20260802-120025` is derived from the existing
  scripts' inventory, not designed up front; how much of Nova's state a
  generic predicate can observe without an adapter is still open.
- Which examples are kept, retired, rewritten or newly written - and which
  test-only scenarios to add - is decided by the spike `20260804-003244`
  before `20260802-120029` rewrites anything.
- Image comparison tolerances remain out of scope until captures prove stable
  enough; v0.10.0 gates producer coverage and scenario/probe invariants first.

## Out of Scope

- In-editor scenario authoring. v0.10.0 proves source/RON-authored content and
  automation first; the editor can consume the proven workflow later.
- Gamepad menu/editor navigation and mobile virtual controls. Valuable, but
  unrelated to the automated demonstration dependency chain.
- Dead NOVA OS objectives/flight-log row rendering cleanup. No production pane
  consumes it; keep as backlog maintenance unless automation exposes a defect.
- Further BCS extraction or unrelated crate architecture cleanup.
- Pixel-perfect screenshot golden tests. Renderer variance needs evidence
  before choosing thresholds or storage policy.
- Folding screenshot capture/packaging into `nova_probe`. The python script
  keeps that job (`20260802-120045` WONTDO).
- A serialized automation DSL. Scripts stay Rust closures with world access.
- In-game capture of scenario/mod art. Owner call 2026-08-04: scenario images
  are owner-drawn, not gameplay stills; `20260715-220011` ships generated
  placeholders and a specific in-game still needs its own briefed task.
- Gamepad and touch input synthesis. Keyboard and mouse cover the fleet.
- A hard-failing asset gate. Owner directive 2026-08-04: image coverage is an
  advisory worklist (CI warning, exit 0), because some shipped images are
  hand-made art no automation can produce.

## Manual Acceptance

- [ ] The example fleet reads as a curriculum, not a feature dump.
- [ ] Generated images are representative and legible at their shipped sizes.
- [ ] The generated scenario placeholders look deliberate, not broken.
- [ ] The probe report is useful as release evidence without hand repair.
