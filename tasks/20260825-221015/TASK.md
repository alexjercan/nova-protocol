# Editor polish: make the node editor feel finished

- STATUS: IN_PROGRESS
- PRIORITY: 75
- TAGS: v0.12.0, editor, ui, polish

## Why

The node editor works. It does not feel finished. Two reviews of the same build
- one reading the code, one reading the screens - found 87 things, 10 of them
the same defect seen twice. None of them is a missing feature; all of them are
the difference between a tool you tolerate and a tool you reach for.

The catalogue lives with the task that commissioned the reviews:

- `tasks/20260714-081703/BACKLOG.md` - every item, merged and tagged
- `tasks/20260714-081703/REVIEW-INSIDE.md` - the code pass, 43 findings
- `tasks/20260714-081703/REVIEW-OUTSIDE.md` - the screen pass, 44 findings

That task keeps its own spine: save/reload, objectives, events surfacing. This
one takes the polish, and the two queued slices that are polish in all but name
(delete by key, Add obeying the context).

## The plan, in order

Each number is one commit. Ids in brackets are the catalogue's.

1. **The stage calms down.** The world grid is too thick and too bright: give
   the stage its own `GizmoConfigGroup` at a thinner `line_width` instead of the
   default 2px, drop the fine grid a shade below `PHOSPHOR_MUTED`, and let the
   decade lines carry what weight is left. The move and turn handles are too
   thick with them: `ARM_RADIUS` and `RING_THICKNESS` are both `0.035` and read
   as pipes. Thin them, and keep the grab target - picking is mesh picking, so a
   thinner arm is a thinner thing to hit; the fat body stays as an invisible
   sibling. a, b. S
2. **Keys move onto the things they act on, and the legend shrinks to what
   nothing else can show.** Every menu row that has a key shows it in the column
   it already has, drawn as the chips the gallery has; the placement verbs get
   Ship menu rows, greyed unless a part is in hand. What is left in the legend
   is what no surface can carry: WASD and Space/Shift, RMB to look, LMB to
   select, LMB twice to enter, Del, and Escape naming the rung the next press
   takes. Every hint gets the same width, so each one overflows inside its own
   cell and the line always fits between the rail and the Inspector. `F` gets
   one meaning or the legend names the mode; `Ship Skin reflows as you aim`
   moves onto the skin row; one grammar and one verb per gesture across the
   legend, the gallery footer and the hints.
   F1, F2, F3, F8, 1.5, 1.6, 7.4/G6. M
3. **Greyed rows stop lying.** Drop `Ctrl+S` from a row bound to nothing; mark
   the unbuilt rows `soon` in the column they already have; Play says why on
   hover instead of only in a log line. 1.3/F5, 2.8/F4. S
4. **Delete acts on the selection (slice 6).** Retire the delete brush. What is
   selected is what Delete deletes, at every depth: a world node, a ship, a
   section. The tree row carries a trash affordance on hover or on select, `Del`
   does the same from the keyboard, and Edit > Delete stops greying at a section
   and just works. This kills the mode that had to be armed, disarmed, named,
   and reported, so it takes 1.2, 2.4, 3.4, 5.1 and Delete Parts' share of 2.2
   with it. 1.1, 1.2, 2.4, 3.4, 5.1. S-M
5. **Add obeys the context (slice 7).** Grey the world palette and Add > Ship
   inside a ship - the greying machinery already exists in `sync_ship_menu`.
   Then the second half: inside a ship, Add offers that ship's parts.
   3.1/F7, 3.2. S then M
6. **One line the editor speaks through.** `set_status` becomes shared and every
   refusal calls it: Play inside a ship, a rebind conflict and which action
   holds the key, a refused checkbox or segment, a colour slider that will not
   write, Add with no document, an id clash, the hand emptied on the way out of
   a ship, the handles suppressed by an armed part.
   2.1, 2.3, 2.5, 2.6, 2.7, 2.9. M
7. **Nothing destroys a document without asking.** A confirm on File > New
   Scenario and on Back to Main Menu, in the window host that already exists.
   3.6. M
8. **A selection you can see.** A phosphor outline on the selected node in every
   context, including inside a ship where the handle rig is deliberately
   suppressed. Then cross-highlight both ways: hover a tree row and the thing
   lights on the stage, hover the stage and the row lights. D1, D2. M each
9. **Transform in three boxes (slice 5).** Position and rotation each become
   three axis-tinted boxes matching the handle colours; the row called `Heading`
   is renamed `Rotation`, which is what it is; degrees and yaw/pitch/roll stated
   on screen; a `TRANSFORM` heading over the pair. 4.1/C1, 4.2/C2, 4.7, C3. M
10. **Numbers that know what they are.** A unit per field and a floor on the
    ones that have one, so a negative radius is refused where it is typed rather
    than at run time. 4.6. M
11. **Names that agree.** A name row for ships; the tree shows the authored name
    with the id on hover; the Key row becomes the button that arms the rebind.
    4.3, 4.4, 4.5. M
12. **The scenario node stops being an empty box.** Ships, objects, and which
    one the player flies. 3.3/C4. M
13. **One icon per kind, everywhere a mark is drawn.** Kill the shared `o` and
    `!`; line-art phosphor glyphs; the same glyph on the Add rows; a lead icon
    column in the menus, which lets the toggles use the checkbox glyph and frees
    the tail column for shortcuts alone; `@` and the driver mark stop competing
    for one column. 6.2/B1, 6.1, 6.4, 6.5, 2.2/6.3/C11, 5.5. M
14. **A tree you can read.** An ellipsis instead of a cut glyph; a stable
    ordinal order; a mark for a row that holds children; names rather than raw
    ids. B2, B3, B4, 5.3. M
15. **Placement that says what to do.** The refusal beside the ghost instead of
    at the window edge; every refusal names the key that resolves it; the
    neutral mate readout labelled and out of the error's slot. E1, E2, E3. S-M
16. **Sockets you can see.** Link points brighter and screen-space sized, with
    the one the ghost would take marked apart from the rest; keybind chips off
    the part they name, with a leader. E4, E5. M
17. **The gallery pass.** `1 part`; the filter visible when it is set;
    thumbnails framed to the part's bounds; sockets drawn on the focus preview;
    one case rule for stat keys; the focus card's empty half; plural chips
    against singular subtitles; and one look at the stray pixel.
    G1-G5, G7, G9, G10. M
18. **The wording pass.** One noun per thing: scenario (not SCENE / scenario /
    [SCENARIO]), skin (not look / style / cladding), Ship Settings against the
    Ship menu, `Add > New Ship`; Rebind Key takes its ellipsis; one treatment
    for the kind tag. B5, 5.4/F6, C8, 5.7, C5. S each
19. **The rest of the rail.** The attitude readout names its number and its fix
    and gains mass, thrust, hp and part count; an empty hull says "no parts yet";
    the look list stays visible and greyed with a swatch per row; `Placeholder`
    goes behind a debug feature; the colour swatch paints its hover. This step
    also absorbs the engineer readout, closed as `20260824-120535`: that file
    lists the rest of what the panel could hold - flip time, centre of mass,
    per-axis thrust, weapon totals - and what is ruled out. Start with the four
    numbers above and decide the rest once it can be seen; one panel, one
    display rule, no stat inventing its own.
    C6, C7, 5.6, 7.3, C9, C10, 7.1. M
20. **Navigation extras.** A clickable breadcrumb path; front / top / side / iso
    presets; an axis rose in the viewport corner, which is also the only spatial
    reference inside a ship; a double-click on a world object doing something.
    F9, D4, D3, 3.5. M
21. **Diegetic windows: a spike, then one of them.** The editor's chrome is
    docked panels on a CRT. Some of it wants to be a thing in the room instead:
    a floating readout that belongs to the ship it describes, a placement
    refusal on a panel beside the ghost, a gallery card that opens where the
    part is. Investigate what earns it, on paper and in one screen, before
    building a window system. Nothing here is settled. c. M spike


## Done when

The eight worst-per-effort items are gone, the numbered steps above are either
committed or consciously dropped with a line saying why, and a walk through the
editor with fresh eyes finds nothing on this list still standing.

## The follow-up plan, in order

Reopened 2026-08-26 on a walk through the built editor. Seven findings, six
steps; each number is one commit, as above.

22. **The stage's own text goes under the chrome.** The nameplates, the section
    keybind chips and their leaders all draw OVER the rail and the Inspector,
    because the panels sit at the default z and every stage-anchored layer
    claimed a positive one. One z ladder for the editor, written down once:
    stage-anchored surfaces below the docked panels, menus and windows above
    them. (1), (5). S
23. **Any key can be bound.** A section rebind refuses every key the flight rig
    drives, so Space cannot fire a thruster. The rig's claim becomes a WARNING
    on the status line, not a veto: the binding is taken and the line says what
    else that key does. Escape stays the cancel gesture. (2). S
24. **One click up the tree, and a key for it.** A single click on an ANCESTOR
    row leaves to it, instead of framing the thing you are already inside;
    Backspace steps out one level from anywhere. The double-click keeps its
    meaning going IN. (4). S
25. **Panels wide enough, with a scrollbar and a wheel that moves.** Both rails
    widen, each scrolling pane grows a real dragged scrollbar, and the wheel
    step stops being 20 px a notch. (6). S-M
26. **A vector is three boxes wherever it appears.** `RowValue::Axes` is used by
    the pose rows only; a `Vec3` inside a config - every offset, normal and
    extent - is still one text box holding `x, y, z`. Walk them as axes. (3). M
27. **An inspector that knows what it is looking at.** This is not a section
    editor: a turret's joint offsets, its render meshes and its sounds are not
    the builder's business, and burying the two numbers that are under eight
    headings of them is the reason the panel reads badly. Each kind gets a
    CURATED list - what to show, in what order, called what - filtered out of
    the same walk, flat. View > All Fields brings the raw walk back for the
    author who needs it. (7). M-L

28. **A picket is a ship, and the tree should say so.** The seeded spacecraft
    are `ObjectNode`s carrying `ScenarioObjectKind::Spaceship`, so the rail
    files them with the rocks and the beacons: a generic object glyph, an
    object's kind tag, an object's inspector. They are hulls. Give them the
    ship's mark and the ship's reading, without making them editable ships -
    they are seeded, not built. (8). S-M
29. **Ids you can read off the screen.** Events and filters name nodes by id,
    and the id is currently something the tree hides behind a hover. Put it
    where it can be read and copied while wiring an event up. (9). S
30. **F5 reloads, and so does leaving the editor.** A scenario saved in the
    editor does not appear in the sandbox's Scenarios list until the game is
    restarted, because content is merged once at startup. Two halves, the way
    Wesnoth does it: `F5` reloads everything from disk on demand, and leaving
    the editor or the mod portal reloads on the way out, so what you just
    saved is there when you go looking for it. (10). M-L

## Resolution

CLOSED 2026-08-26. The nine follow-up steps landed, one commit each, on master:

22. `682f4a02` one z ladder - the stage's own text sits under the chrome
23. `054bd986` any key can be bound; the flight rig's claim is a note
24. `1b5c0fd7` one click up the tree, and Backspace does the same
25. `9497f31f` wider panels, a dragged scrollbar, three lines to a notch
26. `a0084dec` every vector is three boxes, at any depth of any config
27. `3b583c6c` a curated first screen per kind, with View > All Fields past it
28. `f99dd504` a picket wears the ship marks, stands with the ships, reads
    as one - which took the `Reflect` derive `SpaceshipConfig` never had
29. `2d369e2d` the id under the Inspector title, and View > Ids on the tree
30. `ec31e8ca` F5 re-reads the content; so does leaving the editor or mods

Step 29 stops at READING the id. There is no clipboard in the tree, and
pulling one in for a copy button would mean a native dependency the web build
cannot follow; the id is now on screen beside the thing it names, which is what
wiring an event actually needs.

Step 30 also switches the editor's own save ON. Installed is not enabled
anywhere else - a portal install deliberately waits for the player - but a save
is the builder's own document, and asking them to go and enable it before their
range appears in Scenarios is the friction the step was written to remove.

Verified live: the driven walk gained beats for the curated turret, the id
tree, and the one that answers the complaint directly - after File > Save the
walk waits for `editor_save` to appear in `GameScenarios`, with no restart.

REOPENED 2026-08-26 for the follow-up plan above; the 21 steps below stand.

CLOSED 2026-08-26. All 21 steps landed, one commit each, on master:

1. `0f5188f6` stage calms down - own gizmo group, thin grid, thin handles
2. `2b51a53c` keys on the things they act on; the legend cut to gestures
3. `d1f7d376` greyed rows read `soon`, and Play says why on hover
4. `8482cd9a` Delete acts on the selection at any depth; the brush is gone
5. `17f48566` Add obeys the context, and offers the edited ship's parts
6. `ad38daf5` one status line, and every refusal answers on it
7. `5269def0` a confirm before New Scenario and Back to Main Menu
8. `34a92938` + `6655198d` selection outline, then cross-highlight both ways
9. `64cf476e` transform in three axis-tinted boxes per row
10. `089a0e8e` units and floors on the fields that have them
11. `32257f35` authored names, id on hover, Key row arms the rebind
12. `309e37b2` the scenario node reports what the document holds
13. `9932b7e8` one glyph per kind, everywhere a mark is drawn
14. `63788d34` a readable tree - ellipsis, ordinal order, names
15. `e7b509ac` the verdict beside the ghost, naming the key that fixes it
16. `1d949c3e` sockets that read at any distance, chips off the part
17. `75941004` the gallery pass
18. `048c3ba5` the wording pass - scenario, ship, skin, each said once
19. `de4b732e` the rest of the rail - the readout, the greyed style list
20. `69850152` a clickable path, four view presets, an axis rose
21. `bf1e9744` the diegetic spike and its one screen: hull nameplates

Step 21 decided against a window system. `diegetic-surfaces.html` beside this
file grades every editor surface against one rule - one thing, eyes on it, read
not operated - and finds nothing that is both about a single object in the room
and operated. Two of the three candidates the step named stay docked panels
(the floating readout, the gallery card), one was already anchored (the
placement callout), and the surface that earned an anchor was the one nobody
listed: the document's names, said on the stage. Hulls always carry a plate,
everything else takes one while it is marked or hovered.

Nothing was dropped. Step 19's "decide the rest once it can be seen" resolved
to the four numbers plus part count and the limit note; flip time, centre of
mass, per-axis thrust and weapon totals stay unbuilt, and the reasoning for
that is in the absorbed `20260824-120535`.

Verified live under Xvfb rather than by check alone: the driven walk in
`examples/systems/system_ship_editor.rs` grew beats for the view presets, and
`screenshot_editor` learned the double-click gesture it needed to leave a ship.
