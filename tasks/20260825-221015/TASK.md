# Editor polish: make the node editor feel finished

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.12.0,editor,ui,polish

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
