# Notes - editor parts gallery (first draft)

## Shape delivered

A full-screen gallery MODE, not a panel in the drawer. The owner's open
question ("separate tab for the parts, or keep them in the side menu list")
is answered as BOTH, so the two can be judged side by side:

- the rail keeps `Components`, which toggles the existing card drawer;
- the rail gains `Parts Gallery`, which opens the overlay.

Why a mode: a tile needs a real 3D preview, and a 280px drawer cannot hold one.
The overlay hides the editor's own rail + drawer while it is up
(`EditorChrome`), because the tile grid has to stay TRANSPARENT for the
previews behind it to show - UI always draws over 3D, so chrome behind the
gallery cannot be covered, only hidden.

## How the 3D works

- No second camera and no render layers. The gallery parks the editor's own
  free-fly camera (`ParkedPose` + removing `WASDCameraController`, the same
  move the scenario's cinematic camera makes) on a stage 2000 units above the
  build area, and restores it on close. The skybox is the backdrop.
- Layout owns the grid: each tile's preview is placed by unprojecting its UI
  cell's centre (`Camera::viewport_to_world`) at a fixed ray distance, so all
  tiles are the same apparent size and the grid stays responsive. Previews
  spawn hidden and appear once their cell has been measured.
- Previews are real preview sections through the shared
  `preview::insert_preview_section` (extracted from `placement.rs`), so a tile
  and a placed section can never render differently.

## Decisions to review

1. Two pickers (gallery + drawer) or one. Cheap to delete either.
2. Category ROW, not a dropdown. Six kinds fit across the header, and
   `nova_ui` has no dropdown widget; a row costs no new widget family. The
   ship family (racer / cargoa / cargob) is reachable through the text filter
   (it matches the catalog id) rather than as a second dropdown.
3. Click a tile = FOCUS it, not place it. The flow is browse -> filter ->
   focus -> place, which is the DoD's flow, but it costs mouse users one extra
   click per part. The drawer list is the one-click path.
4. The filter field is not `bevy_ui_widgets`' `EditableText`: the gallery owns
   the keyboard while it is up, so the box only has to SHOW what was typed.
   Adopting the real widget means pulling in `InputFocus` + IME plumbing.
5. Tiles fit to the part's authored COLLIDER extent, not its mesh bbox. Right
   for the semantic parts (tight primitive colliders); approximate for the
   standard sections, whose Kenney meshes are smaller than their unit-cube
   colliders.

## Coverage

- `nova_editor` unit tests: filtering (hidden prototypes, category, filter
  case/ID), selection clamping and resolution through the filter, and the
  filter field's typing rules.
- `examples/ui/editor.rs` walks the gallery with real gestures: open, browse
  (tiles are up), filter (typing narrows the grid), focus (the card names the
  part and reads its stats), place (arms the tool and closes), then a click on
  the ship builds that part. Run: `NOVA_AUTOPILOT=1 cargo run --example editor
  --features debug` (needs a display).
- New harness gesture `nova_autopilot::input::type_text`: `press_key` writes
  only the held-key state, so it cannot fill a text field.
- Figures: the walk shoots `editor-gallery.png` and
  `editor-gallery-focus.png` under `NOVA_CAPTURE`.

## Left for the land, not done here

- `feature-editor.png` (site index) now misses the new rail row. Refreshing
  the shipped web assets waits on the owner's verdict on the gallery's shape.
- Keybind wiki page: the gallery's keys (arrows / PgUp / PgDn / Enter / Esc /
  type) are on-screen in the hint line but not yet in `web/src/wiki/keybinds.md`,
  for the same reason.

# Notes - round 2 (owner playtest feedback, 2026-08-14)

The owner played the first draft. Verdict: "IT LOOKS GREAT", with a list. This
round answers it. The two open decisions from round 1 are closed by the owner:
the drawer is DELETED (the gallery is the picker), and the category ROW stays.

## The one that mattered: parts only fit the craft they were cut from

Reported as "there are A LOT of just PDCs, and I see WHY - parts do not match
really well with each other unless they are from the same ship; e.g. a normal
HULL and a torpedo bay from cargob has a weird rotation to it because of the
link point".

Two independent causes, both real:

1. `snap_placement` aligned the two NORMALS with `Quat::from_rotation_arc`.
   That is a shortest-arc rotation: its axis is whatever is perpendicular to
   both vectors, so the roll it leaves is an accident of the pair. For a socket
   facing exactly opposite the part's own, the arc has no axis at all and glam
   falls back to an arbitrary perpendicular - which is why the same turret
   mounted under a hull faced somewhere else than one mounted on top.

2. The semantic parts' normals were centre-to-centre directions. `link_points`
   in `ships/shared.rs` derives a socket from the ship's adjacency: midpoint
   between two part centres, normal pointing at the neighbour. Cut parts sit
   where the art put them, so the cargob pod's `to_fuselage` normal was
   (-0.806, 0.153, 0.573) - 36 degrees off -X. Anything mated onto it arrived
   tilted by exactly that much.

Fixes, in `nova_ship::sections::link_points`:

- `link_point_up(normal)` gives every socket a roll ZERO: ship up (+Y)
  projected onto the socket plane, or +Z on the sockets that face +-Y. DERIVED,
  not authored - which is the point. Two parts that never met agree on it with
  nobody writing a second vector, and no content file changes. The formula is
  even in `normal`, so a socket and the socket it mates resolve to the same up.
  `snap_placement` now mates the two socket FRAMES.
- `cardinal_axis(direction)` snaps a derived normal to the nearest axis.
  Antisymmetric by construction, so both ends of one authored edge stay exactly
  opposed and every shipped mate survives (`every_parts_ship_has_one_connected_
  mate_graph` and `every_built_in_parts_ship_has_a_valid_link_point_graph` both
  pass unchanged). Applied in `ships/shared.rs::link_points`, in SHIP space so
  the two ends read the same axis whatever each part is rotated by.
- `box_link_points(size)` generalises `unit_cube_link_points`: face sockets for
  a box of any size. A 0.3 mount mates a 1.0 hull face; sizes never had to
  agree, but nothing had said so.

Positions are NOT snapped. A socket sits at the midpoint of two part centres,
which both ends agree on by construction; moving it onto either part's own face
would break that agreement (the parts do not touch). Cross-family placement can
therefore leave a small gap where the two families' sockets sit at different
depths. Acceptable: the complaint was rotation, and the AABB overlap refusal
already allows the gap.

## The PDC glut

12 turret prototypes, 10 of them the same PDC on the same joint tree (port and
starboard, three craft, doubled by `light_turrets`). They carry no mesh of
their own, so they were never distinguishable. Now `hide_in_editor` - ships and
mods still name them - and the catalog gains ONE shared mount,
`pdc_turret_section` ("PDC Turret"): the better turret's gun on a 0.3 box with
`box_link_points` sockets. That is the "just re-use the same PDC turret for all
cases" the owner asked for, and it is the harness's proof that a part authored
at its own size mates a unit cube.

## The rest of the list

| Feedback | Answer |
|---|---|
| drop the Components list | `ui/card.rs`, `ui/drawer.rs`, `ui/tooltip.rs` deleted; rail row is `Parts` |
| keybind labels show over the gallery | `position_section_keybind_labels` was keyed on `With<WASDCameraController>`, which the gallery REMOVES when it parks the camera - the `Single` stopped matching, the system stopped running, and the chips froze mid-screen. Keyed on `EditorCamera` now, gated off while the gallery is up, and a hider takes them down |
| hard-to-guess names | `CargoB // Nose`, from a `family` threaded into `prototypes()` |
| bad lighting | the editor had ONE light pointing straight down. Key + rim + ambient, the same bearings `parts_viewer` uses |
| parts flash mid-screen on a tab switch | `spawn_tile` set `Visibility::Hidden` and then `insert_preview_section` overwrote it with the preview bundle's own. Hidden AFTER the insert |
| turret tiles four cells wide | tiles were fitted to the authored COLLIDER; a turret's collider is its mount box, its silhouette is a metre of barrel. `measure_gallery_items` measures the rendered AABBs as a rotation-invariant radius |
| ESC should be Back, not pause | new `EscapeOwner` resource in `nova_gameplay`: the editor claims ESC in `PreUpdate` while it has a back step (gallery up, part armed, rebind pending), `toggle_pause` reads it in `Update`. Cross-crate because neither crate can order against the other |
| TAB arms NOVA OS anywhere | `toggle_nova_os` was gated only on `GameStates::Playing`, which includes the editor. Opening now needs a `PlayerSpaceshipMarker`; closing stays ungated |
| F/R cycling overshoots | wheel rolls, Shift+wheel cycles the socket - both reversible. R and F still step forward |
| show the link points | `draw_link_points`: a ring plus a normal stub on every FREE socket, bright under the pointer, and the armed part's mating socket on the ghost |
| fewer clicks / pipette | `Q` arms whatever is under the cursor, read out of the build state |
| keybinds at the bottom left | contextual legend, driven by the armed tool |
| zoom + orbit in focus | wheel zooms, drag turns, the turntable resumes 2 s after you stop - from where you left it, not from a canned pose |
| show the ship's forward | `draw_ship_heading` reaches an arrow past the nose |

## Not done, and why

- **Editing part values on select.** The owner floated it ("we might even
  allow editing values for parts if you click them"). It needs a real
  inspector: which fields are editable per kind, how an edit is stored
  (`SectionModification` already exists for scenario ships), and what happens
  when a mod overlay replaces the prototype underneath. Worth its own task.
- **Thruster exhaust alignment** ("some of the thrusters have a small issue
  with the shader"). Diagnosed, not fixed. `thruster_kind`
  (`ships/shared.rs`) anchors the plume at the COLLIDER's rear face centre and
  sizes it from the collider (`width: size.x*0.7, height: size.y*0.6`). For a
  cut part the collider bounds the whole part, pylon included, so the anchor
  sits off the nozzle axis and the rect is stretched: the racer's engine bbox
  runs y in [-0.3, 0.44], putting the anchor 0.07 above centre. The fix is to
  author the exhaust anchor per part rather than derive it from the box, which
  wants eyes on the render to place - it is a per-part art pass, not a formula.

## Coverage

- `nova_ship`: `one_part_mates_the_same_way_up_on_every_socket` (the frame
  mate's whole claim), `the_cardinal_axis_is_antisymmetric`,
  `the_socket_up_is_the_same_from_either_end`.
- `nova_authoring`: `every_semantic_part_mates_square_onto_a_plain_cube` -
  every socket of every shipped part, on every face of a cube, arrives
  axis-aligned. This is the owner's cargob-pod-on-a-hull case, generalised.
- `nova_editor`: `the_editor_claims_escape_only_when_it_has_a_back_step`,
  `escape_puts_the_armed_part_down`.
- `nova_menu`: `a_scene_surface_that_owns_escape_keeps_the_pause_overlay_down`.
- `nova_os_ui`: `tab_does_not_open_the_nova_os_without_a_ship`.
- `examples/ui/editor.rs`: the drawer beats are gone; the run arms its hull
  through the gallery, and the refusal beat mounts `pdc_turret_section` - which
  only works because a 0.3 mount mates a 1.0 hull face.

Live: `DISPLAY=:99 NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_SHOT_DIR=target/shots
cargo run --example editor --features debug` walks green and reshoots
`editor-gallery.png`, `editor-gallery-focus.png` and
`editor-placement-refused.png`.

## Round 2 follow-up: the PDC's mount, measured

The owner asked whether the shared PDC's 0.3 collider against a "1x1x1" render
was intended. Measured (glTF node transforms composed, not raw accessor bounds),
the turret art is not a unit cube at all - in section-local space the assembly
spans about 0.68 wide, 0.77 tall and 1.39 long, most of that length being barrel
reaching past the pivot. `better_turret_section` draws the SAME art inside a 1.0
collider, so a collider that disagrees with the silhouette is the norm here, not
something new: the collider is the mount FOOTPRINT (what physics hits, what
overlap refuses, what the sockets sit on), never the gun's reach - a barrel that
collided would refuse its own line of fire.

So the size is intended. Measuring it did turn up a real defect, though:
`turret_joint_tree` hardcoded its base joint at `(0, -0.5, 0)` - the bottom face
of a UNIT cube - so on the 0.3 mount the turntable was planted 0.35 BELOW the
section's own underside and the disc sat buried inside whatever the gun was
bolted to. Only the barrel showed. `mount` is a parameter now (the section's own
half-height); every shipped caller passes `UNIT_TURRET_MOUNT` and is byte-for-
byte unchanged, and the generated RON diff is one line.

The per-craft turret modules have the same mismatch (0.3 box, -0.5 base) and are
NOT corrected: their art was placed against that offset and the shipped craft
were framed with it there, so moving it moves the turret on every shipped ship.
That is an art call. `every_placeable_turret_stands_on_its_own_mount_face`
pins the rule for what the editor offers and names the exclusion rather than
skipping it quietly.

## Round 2 follow-up 2: the PDC really was unit-sized, and why

The owner's next read of the figure - "it feels like it has the same size" -
was right, and the earlier measurement had missed the reason. The GLB art is
NOT the whole turret: an unmeshed structural joint gets a DEFAULT primitive from
`insert_turret_joint_render`, `Cylinder::new(0.5, 0.1)`, and the turret's base
joint is unmeshed. That plate is a full unit across - exactly one hull face -
and it is the big dark disc under every turret. Nothing authored could resize
it, because the default branch ignored the joint's `render_mesh_transform`
entirely.

Measured against the shipped camera pose, the hull's top face projects 141.6 px
and the plate the same; the GLB turntable is only 96.9 px (0.683 units). So the
"1x1x1" the owner saw was the code-level plate, not the art.

Three changes, following the owner's call:

1. `RenderMeshTransform` gains `scale` (default ONE - hand-written `Default`,
   since a derived one would scale every unauthored mesh in the game to
   nothing; serde-skipped at unit, so no shipped RON moves and a file written
   before the field still reads as "as modelled").
2. The default plate obeys that transform, composed with its own small lift so
   the lift scales with it rather than surviving it.
3. `turret_joint_tree` takes a `scale` and applies it to every joint offset AND
   every joint's art. Both halves or neither: meshes alone leave the parts
   spaced for the old size, offsets alone leave full-size art in a smaller
   arrangement.

`PDC_TURRET_SIZE` is now 0.5 and is ONE number - collider, sockets and art
scale - so the mount agrees with itself. Half a section: it sits on a hull face
and leaves room to aim at the rest of it, which is what the compact mount was
for. The generated diff stays inside the PDC's own section (16 hunks, all
between its first and last line); every shipped turret still passes
`UNIT_TURRET_MOUNT` + `UNIT_TURRET_SCALE`.

Checked: the shorter barrel puts the muzzle 0.6 ahead instead of 1.2, which is
safe - `ProjectileHooks` filters contacts between a projectile and its owner, so
a muzzle over its own hull cannot self-hit.

Still not corrected, same reason as before: the per-craft turret modules pair a
0.3 box with unit-size art at the unit-cube offset. Making them agree would move
AND resize the turret on every shipped ship.
