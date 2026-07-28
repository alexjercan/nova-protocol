# DECISION: ship-viewer app + `ship` CLI shape

- STATUS: ACCEPTED
- DATE: 2026-07-28

Re-scope of task `20260726-115339` after the NOVA OS grew a unified
`TerminalCommand` model (20260727-231546), a 3D `map` app (20260724-102320),
and a persistent header+main+footer layout (20260728-085741). These are the
load-bearing build-shape forks, confirmed with the owner before planning.

## Fork 1: command surface - `ship` opens an app, `ship view` prints

The builtin `ship` command (today a `CliOutput::Snapshot` that prints a status
summary) is REPLACED by an **app** command. Bare `ship` launches the ship
computer app (peer to `map`). The stdout summary moves to the `ship view`
subcommand. New subcommands: `ship section <id>` (per-section detail),
`ship reload <id>`, `ship repair <id>`.

ACCEPTED: `ship` = app, `ship view` = snapshot CLI, plus `section/reload/repair`
verbs taking a section id. This is exactly the `map` / `map view` precedent, so
the longest-prefix resolver and `&'static str` names carry over unchanged.

## Fork 2: render style - 3D orbit schematic (map-app pattern)

The app renders the player ship as a fake-3D schematic on the CRT using the
map app's render-to-texture + orbit-camera pipeline
(`crates/nova_gameplay/src/hud/nova_os_map.rs`):

- A dedicated `Camera3d` on an isolated `RenderLayers`, `RenderTarget::Image`,
  composited through the existing green CRT shader (no per-app shader work).
- Ship sections become **unlit green-phosphor proxy blocks** built from each
  section's `SectionCollider` + local `Transform` (a faithful "green-ified"
  schematic that still looks like the ship).
- Clicking a section uses **projected UI blips** (`Camera::world_to_viewport`
  -> absolutely-positioned `Button` nodes over the viewport), NOT 3D picking -
  this is how `map` makes RTT content clickable and directly answers the
  "hard to click things inside the ship" worry.
- Selection also works by keyboard (cycle `[`/`]`, number keys) so the mouse is
  never required mid-flight.
- Destroyed sections: leaf sections that despawn simply do not render; non-leaf
  `SectionInactiveMarker` (0 HP) sections render dim/dashed and are labelled
  inactive.

REJECTED: flat 2D blueprint (loses "looks like the ship" and orbit-around; the
map precedent removes the risk that justified it).

## Fork 3: section IDs - short kind+index codes

Real player ships use auto grid-coordinate `EntityId`s (`cube_i0_j0_k0`,
`cube_im1_j0_k2`) - unique but hostile to type and unreadable as labels. So the
viewer + CLI target sections by a **short kind+index code**: `HULL-1..N`,
`THR-1`, `CTL-1`, `PDC-1` (turret), `TRB-1` (torpedo bay), assigned stably per
session by a gameplay system and held in a new lightweight `SectionCode`
component. The grid `EntityId` stays as the underlying identity; `SectionCode`
is the human/CLI/label handle and the Tab-completion candidate set.

REJECTED: raw `EntityId` (unreadable), bare ordinals 1..N (carry no type/spatial
meaning and renumber under damage).

## Fork 4: action semantics - arcade now, designed for a queued/resource future

`reload`/`repair` are **instant and free** for v0.9.0 (reload refills a weapon
section's ammo to capacity; repair restores `Health` to max), so the whole
interactive computer ships now and can be play-tested.

BUT the mutation MUST be built as a **deferred command-effect**, not an inline
edit, so it can later become the owner's intended model without a rewrite:

- Later, an action becomes a **queued job** on the section that executes over
  time WHILE the player is outside the computer (drawer closed).
- Jobs **consume resources stored in hull sections** (giving hull sections a
  purpose beyond armor), surfaced through a future ship **inventory** panel in
  this same app.

Concretely: the CLI/app raises a structured `ShipSectionCommand { target,
action }` request; a gameplay handler system applies it. Today the handler
mutates instantly and reports; the same seam later enqueues a job component,
checks/decrements hull-stored resources, and ticks while the drawer is closed.
No `CliOutput`-inline mutation, no direct `Health` writes from `nova_os`.

FOLLOW-UP TASKS (not this one): queued/over-time execution, hull-stored resource
model + costs + combat lockout + "why disabled" notes, ship inventory panel.
