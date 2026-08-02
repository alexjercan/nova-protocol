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
| Nova-owned automation | `nova_autopilot` replaces the BCS harness and adds Nova-specific checkpoints. | `20260802-120019`, `20260802-120025` |
| Showcase content | A curated suite demonstrates flight, combat, gravity, editor, menu, and NOVA OS paths. | `20260802-120029` |
| Asset factory | One command runs evidence capture, profiling, screenshot staging, and packaging. | `20260802-120045`, `20260724-082856`, `20260715-220011` |
| Release confidence | Current tutorial media and warning-clean code ship with the automated evidence. | `20260730-111146`, `20260731-205553` |

## Done Means

- Nova automation no longer imports or activates the BCS debug harness; the
  in-repo crate owns the driver and completion protocol. (cmd: `test -f crates/nova_autopilot/Cargo.toml && ! rg -n "BCS_AUTOPILOT|debug::harness" crates examples scripts --glob '*.rs' --glob '*.py' --glob '*.sh'`)
- The curated showcase group completes through the real app, produces probe
  reports, and captures its declared assets from one command. (cmd: `nix develop --command cargo run -p nova_probe -- run showcase --capture --fps --release`)
- Every committed web screenshot and scenario thumbnail has a declared,
  reproducible producer; strict packaging reports no missing or stale output.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --check`)
- Workspace compiler and rustdoc warnings introduced or exposed by the release
  are zero. (cmd: `nix develop --command env RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --features debug`)
- The owner accepts the generated showcase report, scenario-picker thumbnails,
  and rendered web/tutorial pages as the v0.10.0 demonstration. (manual: inspect generated `report.html`, the Scenarios picker, and the locally rendered website)

## Child Tasks

| ID | Priority | Repo | Title | Landed result |
| --- | ---: | --- | --- | --- |
| `20260802-120019` | 100 | nova-protocol | Move the automation harness into `nova_autopilot` (9 children) | Pending |
| `20260802-120025` | 85 | nova-protocol | Add checkpoint-driven Nova automation scripts | Pending |
| `20260802-120029` | 80 | nova-protocol | Build the v0.10.0 showcase scenario suite | Pending |
| `20260802-120045` | 75 | nova-protocol | Generate showcase evidence and web assets with one command | Pending |
| `20260724-082856` | 70 | nova-protocol | Refresh frontend app images | Pending |
| `20260715-220011` | 68 | nova-protocol | Generate real per-scenario picker thumbnails | Pending |
| `20260730-111146` | 60 | nova-protocol | Refresh the tutorial against current UI and captures | Pending |
| `20260731-205553` | 50 | nova-protocol | Clear compiler and rustdoc warnings | Pending |

## Decisions

- `tasks/20260802-115955/DECISION.md` records Nova-first ownership: build and
  evolve `nova_autopilot` here; extract back to BCS only after real reuse.

## Frontier

Derive with `tatr frontier 20260802-115955`. Hard dependencies encode the
foundation -> scripts -> showcase -> asset-production path. Priority keeps
warning cleanup late without blocking it unnecessarily.

## Fog

- Exact showcase membership is selected in `20260802-120029` from existing
  production-path examples first; add content only where the suite lacks a
  required player-visible beat.
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

## Manual Acceptance

- [ ] Showcase selection tells a coherent v0.10.0 story, not a feature dump.
- [ ] Generated images are representative and legible at their shipped sizes.
- [ ] One-command output is useful as release evidence without hand repair.
