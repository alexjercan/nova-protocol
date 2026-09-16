# Training lessons for mods

A `Lesson` is one screen of the in-game handbook: a demonstration, a few lines,
the actions it names, a link into this manual, and sometimes a range to fly. The
handbook is the CONDENSED companion to these pages - a player reads a lesson in
the menu and comes here for the whole story.

Lessons are content like everything else. The base game ships its own, and an
enabled mod can add lessons to any category or replace a base lesson by
declaring its id.

This page is the field-by-field lesson reference. For general RON spelling rules
such as double parentheses, `Some(...)`, and asset schemes, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## The Lesson item

```ron
[
    Lesson((
        id: "my_mod_towing",
        category: Flight,
        order: 70,
        title: "Towing a dead hull",
        media: Loop(
            sheet: "self://training/towing.png",
            columns: 4,
            rows: 3,
            frames: 12,
            frames_per_second: 12,
            alt: "a tug closing on a drifting ship and matching its velocity",
        ),
        body: "Match the other ship's velocity before you close on it. \
               Starting a tow while you are still closing pushes both ships \
               off course.",
        actions: ["autopilot_stop"],
        wiki_path: "wiki/flight-autopilot#manual-flight",
        practice: Some("my_mod_drill_towing"),
        field_notes: [
            "Match the other ship's velocity before you start a tow.",
        ],
    )),
]
```

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | Stable key, the overlay key, and what the player's progress is recorded against. Prefix new ids with your mod id. |
| `category` | category | required | Which group the lesson is listed under - see [Categories](#categories). |
| `order` | integer | required | Position within the category. Ties break on `id`, so the list never depends on which mods are installed. |
| `title` | string | required | The row label and the heading over the lesson. |
| `media` | media | required | The demonstration - see [Media](#media). Required, so the layout never reflows between lessons. |
| `body` | string | required | The whole lesson, at most 45 words. A tip that needs more is two tips. |
| `actions` | list of strings | `[]` | Action names whose CURRENT bindings are drawn as chips. Never write a key. |
| `wiki_path` | string | required | Where this manual continues, without a leading slash: a page it ships, such as `wiki/flight-autopilot`, plus the heading anchor where the claim sits. |
| `practice` | `Option` string | `None` | A [`role: Lesson`](../scenarios/#roles) scenario the Practice button launches. |
| `field_notes` | list of strings | `[]` | Short facts drawn from this lesson - see [Field notes](#field-notes). |

## Categories

Six, fixed. A mod adds lessons to them; it does not add a seventh.

| category | what belongs in it |
|---|---|
| `StartHere` | What the game is and how to read the screen. |
| `Flight` | Flying the hull, by hand and by flight computer. |
| `Combat` | Finding, locking and shooting things. |
| `Shipbuilding` | The editor, and what a hull's parts cost. |
| `NovaOs` | The cockpit computer. |
| `Advanced` | Scenarios, mods, bindings - the game around the game. |

An empty category draws no header, so a mod that only writes Flight lessons does
not put five empty groups on a player's screen.

## Media

Every lesson carries a demonstration, in one of two forms.

A still:

```ron
media: Image(
    image: "self://training/towing.png",
    alt: "a tug closing on a drifting ship",
),
```

A loop, as a sprite sheet:

```ron
media: Loop(
    sheet: "self://training/towing.png",
    columns: 4,
    rows: 3,
    frames: 12,
    frames_per_second: 12,
    alt: "a tug closing on a drifting ship and matching its velocity",
),
```

| field | type | meaning |
|---|---|---|
| `image` / `sheet` | asset ref | A `self://` or `dep://` image in your bundle's `resources`. A plain 2D PNG - not a cubemap. |
| `columns`, `rows` | integer | How the sheet is cut into cells. Every cell is the same size, so the sheet's pixel size must divide evenly. |
| `frames` | integer | How many cells are played, read left to right, top to bottom. At most `columns * rows`. |
| `frames_per_second` | number | Playback rate. Above zero. |
| `alt` | string | What the demonstration shows. Drawn in the frame while the image loads, and if it cannot be drawn at all. |

A sprite sheet rather than a video file: it is one image through the same
pipeline every other texture uses, so it plays on the desktop build and the web
build without a second decoder. Nothing in a PNG says where its cells are, which
is why the grid is authored - a grid the sheet cannot be cut by is a content
error.

## Practice

`practice` names a scenario the Practice button launches, through the same
New Game hand-off the Scenarios menu uses. There is no second launch path.

The scenario must declare [`role: Lesson`](../scenarios/#roles). That is what
makes "somewhere focused to fly" checkable: a practice range is ONE thing to
try, with the briefing and the cast taken out, and a Practice button that
dropped the reader into an hour of campaign would not be practice. Naming a
chapter or a backdrop is a lint error.

The range is part of the lesson, so write it like one: hand the player only the
verbs this lesson names and withhold the others, put the reason on the comms
channel rather than on the objective board, light the chip for the verb in
hand, and END it in a banner the player can earn or retry. The
[practice-range contract](../scenarios/#roles) spells out each of the four.

Pressing Practice never marks the lesson completed. Reading a lesson marks it
read; only an outcome that asserts the skill can claim more.

A lesson with nothing focused to fly carries `practice: None`. Leave it out
rather than pointing at the nearest approximation.

## Field notes

A field note is the same fact the lesson teaches, cut to the two short lines the
menu card and the loading screen draw. The claim is written ONCE, on the lesson
it belongs to, and the note carries its lesson with it so the menu card can open
it.

Write a note as one sentence and let the slot break it: two lines of 64
characters. A note that needs a third line is a lesson, and the lint says so.

A note names an ACTION, never a key - the loading screen has no binding table to
correct it with.

## Overlay

A new `id` adds a lesson. Reusing one replaces that lesson whole - it is not a
field-level patch.

A lesson id denotes a stable learning outcome, because the player's progress is
recorded against it. A replacement may change every word and the picture, but
somebody who has already read that lesson has still read it, so replace a base
lesson when you have a better telling of the SAME thing - and add a new id when
you have something else to say.

## Check it

```sh
nix develop --command cargo run content lint --target path/to/your-mod
```

The lint refuses a blank required field, a body over the word cap, a loop grid
that cannot be cut, a field note that does not fit the slot, and a `practice`
that names an unknown scenario or one that is not a practice range. It warns
about a practice range no lesson sends the player to.
