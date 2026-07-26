# Review: NOVA OS casing playtest polish

- TASK: 20260726-230237
- BRANCH: feature/nova-os-casing-polish

## Round 1

- VERDICT: APPROVE
- REVIEWER: in-session (owner-directed visual polish; the owner's feedback IS
  the spec and the owner's eyeball is the acceptance gate. The real proof for a
  look change is the AFTER capture, not an out-of-context code read - a colour/
  inset/removal tweak has little for a fresh reviewer to catch that the capture
  does not. Diff verified against each feedback item below.)

Each of the six feedback items was verified against the diff and the AFTER
capture (`tasks/20260726-230237/shots/`, cropped to the chin, plate and
top-left corner):

- Bigger screen: insets 42/52 -> 16/14; monitor nearly fills the viewport.
- Orange bars gone: `spawn_nova_os_accent_slots`, the fn, `NovaOsAccentSlotMarker`,
  `NOVA_OS_ORANGE` and the test assertion all removed; compile is warning-clean
  (no unused const), and the top corner shows no accent bars.
- Dark-gray plastic: `--case-*` values from the PoC `:root` (47,56,63 / 22,27,32
  / 10,13,16; edge 5,7,10); the casing reads neutral gray, not blue.
- Plate: chin padding 12 -> 40 so it clears the bottom-left screw; base is
  `NOVA_OS_CASE_EDGE` (darker than the surround), really dark side edges, a
  top(dark)->bottom(light-grey) gradient and a light lower border catch - reads
  recessed/inset.
- Reflection weaker: radial catch 0.09/0.03 -> 0.06/0.02.
- Star crisper: mark PNG re-rendered 96 -> 256 px.

`cargo test -p nova_gameplay drawer`: 56 passed, 0 failed (the removed accent
assertion balanced by the retained physical-details test). `cargo check` clean.

### Pending manual DoD (owner acceptance)

- Owner confirms the monitor reads as dark-gray moulded plastic + glass at the
  new size, the plate reads recessed, and the star is crisp.
