# UI themes for mods

A `UiTheme` is the look the whole interface paints in: the menus, Settings, the
pause screen, the shipyard, the HUD chrome and every widget on them. The base
game ships two - `base/phosphor`, the green terminal, and `base/hardware`, the
moulded casing - and a mod adds a third by declaring a new id.

A theme is DATA. It names colours, three metrics and the paint of every control
in every state it has. It runs no code, so a broken theme is a lint finding
rather than a crash.

This page is the field-by-field theme reference. For general RON spelling rules
such as double parentheses, `Some(...)`, and asset schemes, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## What a theme can and cannot change

A theme owns PAINT and two shapes:

- foreground, background and border colour;
- border width and corner radius;
- solid, linear and radial fills;
- glows and drop shadows;
- whether a slider meters in blocks or in one continuous fill;
- whether a badge is a `[TAG]` or a bordered chip.

A theme owns NO layout. It cannot set a position, a size, padding, a gap, a
flex or grid rule, a z order, visibility, pointer behaviour or a font size.
Those are the screen's, and a theme that could move them could make a screen
unusable rather than ugly.

A theme also cannot move the gameplay colours. A hostile reticle is red and an
ally green in every theme, so the threat, ally, nav and objective accents are
fixed in the game and are not theme roles. Everything a player would call
chrome goes through the theme.

## The UiTheme item

A theme is an item in any `*.content.ron` file, beside your campaigns and
ships:

```ron
[
    UiTheme((
        id: "my-mod/amber",
        name: "Amber CRT",
        inherit: Some("base/phosphor"),
        palette: (
            text: "#ffb84a",
            text_secondary: "#c98a28",
            text_body: "#ffe6bd",
        ),
        metrics: None,
        roles: (
            badge: Some((
                bracketed: false,
                padding_x: 8.0,
                border_width: 1.0,
                border_alpha: 0.4,
                fill_alpha: 0.06,
            )),
        ),
    )),
]
```

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | Stable key. The overlay key, the value Settings persists, and what another theme's `inherit` names. Prefix new ids with your mod id. |
| `name` | string | required | The label on the Settings row. |
| `inherit` | `Option` string | required | The theme this one derives from, or `None` for a root theme - see [Inheritance](#inheritance). |
| `palette` | palette | `()` | The fixed colour set, six-digit `#rrggbb` each - see [Palette](#palette). |
| `metrics` | `Option` metrics | required | Border width and the two radii, or `None` to take the parent's - see [Metrics](#metrics). |
| `roles` | roles | `()` | The per-control paint tables - see [Roles](#roles). |

Unknown fields are refused, not ignored. A misspelled field stops the content
file from parsing at all, and the message names what you wrote and every name
that would have been valid.

## Palette

Every theme sets the SAME colours, by the same names. The names say what a
colour is for, not what it looks like, so a role written for one theme reads
the same in another. The screens, the role tables and the editor's swatch grid
all use this list:

| colour | what it paints |
|---|---|
| `text` | The main ink: live text, titles, glyphs, and on a terminal look its lines too. |
| `text_body` | Readable body copy. |
| `text_secondary` | Secondary text and idle fills. |
| `text_muted` | Labels, section heads, tags and rules. |
| `text_hover` | A control's legend under the pointer. |
| `text_inverse` | Glyphs on a lit fill: a selected control, a checked box. |
| `surface` | The panel and screen face. |
| `surface_sunken` | A face set below the panel: a pressed control, an unchecked box. |
| `surface_raised` | A face raised above the panel: a selected row. |
| `surface_lit` | The lit top edge of a raised face, a rule, a scroll thumb. |
| `surface_hover` | A control's face under the pointer. |
| `surface_hover_light` | The lit top of `surface_hover`. |
| `surface_hover_dark` | The shaded foot of `surface_hover`. |
| `background` | The deepest field, behind everything. |
| `border` | The dark outline round a raised face. |
| `accent` | Selection, keycaps and warnings. |
| `accent_light` | The bright end of the accent family. |
| `accent_dark` | The deep end of the accent family. |
| `nominal` | Healthy, online, lit: a passed check, an enabled dependency. |
| `nominal_light` | The bright end of the nominal family. |
| `nominal_dark` | The deep end of the nominal family. |
| `danger` | Destructive actions and faults. |
| `danger_text` | A legend on or beside a danger face. |
| `danger_face` | A danger control's face. |
| `danger_face_dark` | The shaded end of `danger_face`. |
| `info` | Information. |
| `control_off` | An off switch's knob. |
| `highlight` | Light washes and the brightest legend. |
| `shadow` | Drop shadows and dark wells. |

A ROOT theme sets all of them. A DERIVED theme sets only the ones it changes;
each colour it leaves out comes from the nearest theme up its chain that sets
it. The example above recolours three and inherits the rest.

The list is closed. A name that is not on it, in the palette or in a
`Variable(...)`, stops the file from parsing, and the message names the
colours that would have been valid.

`nominal` is separate from `text` on purpose. On a phosphor screen the ink
IS green, so the two look like one colour and a screen that wanted "good" can
get away with asking for the ink. On a casing the ink is bone white and the
lamp is still green, and a theme that answers both questions with one colour
draws a white ONLINE badge.

Give the `text` family, `surface` and `background` your OWN tones. A theme that
copies another theme's ink and surfaces cannot differ from it anywhere a
screen paints by job, whatever its role table says.

## Metrics

Three numbers the whole interface is drawn with:

```ron
metrics: Some((
    border_width: 1.0,
    radius: 2.0,
    panel_radius: 2.0,
)),
```

| field | meaning |
|---|---|
| `border_width` | Hairline border width, in logical pixels. |
| `radius` | Control corner radius. Phosphor is 2.0 and square-ish, hardware 7.0 and moulded. |
| `panel_radius` | Panel corner radius. Phosphor is 2.0, the same hard corner as its controls; hardware 10.0, a moulded case. |

`metrics: None` takes the nearest ancestor's three whole. A root theme must
carry them.

## Colours

Every colour in a role table is a value plus an alpha:

```ron
border: (
    value: Variable(text),
    alpha: 0.4,
),
text: (
    value: Literal("#ffd9d5"),
),
```

| field | meaning |
|---|---|
| `value` | `Variable(name)` reads a palette colour by its name, such as `Variable(text_muted)`. `Literal("#rrggbb")` writes a one-off tone. |
| `alpha` | `0.0` to `1.0`. Omitted means opaque. It REPLACES the source colour's alpha rather than multiplying it, so the same colour at `0.12` reads the same in every theme. |

Transparent paint is alpha `0.0`, never a missing colour.

### Fills

A surface background is one of three:

```ron
fill: Solid((value: Variable(surface))),

fill: Linear(
    base: (value: Variable(surface)),
    degrees: 180.0,
    stops: [
        (color: (value: Variable(highlight), alpha: 0.06), percent: 0.0),
        (color: (value: Variable(shadow), alpha: 0.18), percent: 100.0),
    ],
),

fill: Radial(
    base: (value: Variable(surface)),
    anchor: Top,
    stops: [
        (color: (value: Variable(text), alpha: 0.05), percent: 0.0),
        (color: (value: Variable(shadow), alpha: 0.0), percent: 70.0),
    ],
),
```

`Linear` takes an angle in degrees, where 180 runs top to bottom - that is how
a moulded face gets its light. `Radial` takes an `anchor` of `Top` or `Center`
and always shapes to the farthest side; the phosphor panel blooms from its top
edge this way.

Both gradient forms carry an explicit `base`, the flat colour painted UNDER the
gradient. It is not derivable from the stops: the phosphor panel is a dark
screen face under a translucent glow, so its stops say nothing about the colour
a player sees. State it.

### Effects

A glow or a drop shadow, or `None` for a flat face:

```ron
effect: Some((
    color: (value: Variable(text), alpha: 0.25),
    offset_y: 0.0,
    blur: 12.0,
)),
```

One type serves both. A drop shadow is an offset dark blur, and a glow an
un-offset coloured one.

## Roles

Sixteen roles name every control the interface paints. Each is an `Option`:
`None` INHERITS that role whole from the parent, and a root theme must fill
every one.

| role | what it paints | states |
|---|---|---|
| `button` | The neutral button. | `normal`, `hovered`, `pressed`, `selected`, `selected_pressed`, `disabled` |
| `button_primary` | The call-to-action button. | the same six |
| `button_danger` | The destructive button. | the same six |
| `button_ghost` | The border-only button. | the same six |
| `panel` | A bordered panel surface. | one surface |
| `panel_head` | A panel's header band: `border`, `title`, `rule`, `tag`. | one band |
| `list_row` | A list row. | `normal`, `hovered`, `selected` |
| `segmented` | A segmented control's recessed container. | one surface |
| `slider_track` | A slider track - see [Shape](#shape). | one track |
| `text_field` | A text field. | `normal`, `hovered`, `focused`, `error` |
| `checkbox` | A checkbox, plus its own `radius`. | `off`, `on` |
| `toggle` | A pill toggle, plus `radius` and `knob_radius`. | `off`, `on` |
| `badge` | A status badge - see [Shape](#shape). | shape and alpha only |
| `scroll_bar` | A scrollbar: `track` and `thumb`. | two colours |
| `key_chip` | A keycap chip. | one state |
| `separator` | A rule between sections. | one colour |

A control state is a fill, a border, a text colour and an optional effect:

```ron
normal: (
    fill: Solid((value: Variable(text), alpha: 0.05)),
    border: (value: Variable(text), alpha: 0.4),
    text: (value: Variable(text)),
    effect: None,
),
```

Every state of a role a theme declares is explicit. A theme cannot leave one
out and inherit it from another state, because a button that painted five of
its six states would carry a dead state nobody noticed.

A `panel`, a `segmented` and a slider's well carry `radius: Option<f32>` -
`None` takes the metric, and a number overrides it for that surface alone.

A list row carries no text colour. What a row holds is arbitrary - icons,
badges, several spans - and each of those is already themed.

## Shape

Two roles carry shape as well as paint, because the shipped looks differ there
and the difference IS the look.

A slider meters in discrete blocks or in one continuous fill:

```ron
slider_track: Some((
    surface: (
        fill: Solid((value: Variable(shadow), alpha: 0.5)),
        border: (value: Variable(text), alpha: 0.32),
        effect: None,
        radius: None,
    ),
    height: 14.0,
    meter: Blocks(segments: 24, gap: 2.0),
    lit: (value: Variable(text)),
    unlit: (value: Variable(text), alpha: 0.16),
)),
```

`meter: Fill` draws one bar filled to the value instead, which is what the
hardware look does behind its moulded well. `height` is authored because the
two meters are not legible at one height.

A badge is the terminal's bracketed tag or a bordered chip:

```ron
badge: Some((
    bracketed: true,
    padding_x: 2.0,
    border_width: 0.0,
    border_alpha: 0.0,
    fill_alpha: 0.0,
)),
```

`bracketed: true` renders `[TAG]` as text with no chrome. `bracketed: false`
draws the chip, and then `padding_x`, `border_width`, `border_alpha` and
`fill_alpha` shape it. A badge's HUE comes from what it reports: a nominal
badge reads `text`, a warning `accent`, a fault `danger`. The theme sets
those colours in its palette, so the role itself carries alphas rather than
colours.

## Inheritance

`inherit: None` marks a ROOT theme. A root theme must define every one of the
sixteen roles, every palette colour, and its metrics. Nothing stands behind
it.

`inherit: Some("base/phosphor")` marks a DERIVED theme. Every role it omits
comes from its parent, and so does every palette colour it does not set. This
is the normal way to ship a look: recolour a few palette colours, override the
two or three roles that should differ, and inherit the rest.

A role a derived theme DOES declare replaces the inherited one COMPLETELY.
Individual states are never merged, so a theme can never leave a control
half-painted in two looks.

Chains can be any depth. The game refuses:

- an `inherit` that names a theme nothing declares;
- an inheritance cycle, by the names in it;
- a theme whose chain sets no value for a role, a palette colour or the metrics;
- a hex that does not parse.

Each is one lint finding naming the theme and what is missing.

A theme can only inherit from a theme it can SEE: one the base game ships, one
in the same bundle, or one in a bundle listed in `meta.dependencies`. `base` is
an implicit dependency and is never declared, so inheriting from
`base/phosphor` needs nothing.

## Restyling a base theme

Declaring `id: "base/phosphor"` REPLACES the shipped phosphor theme rather than
adding a theme beside it. The player keeps the theme they chose and it looks
different - which is what a restyle mod is.

The registration order is the Settings picker's order, and an overlay keeps a
theme where it was. A mod that restyles `base/phosphor` leaves the default in
the first position, where the player expects it.

Two mods that both restyle one id: the later-loaded one wins, as with every
other content kind.

## What the player sees

Settings lists every theme every ENABLED mod declares, by `name`, in
registration order. Picking one writes its `id`; the whole interface repaints
on the same frame, with no restart and no rebuild.

The setting persists as that id and nothing else. A theme id survives a
restart, which is the point of the format.

Enabling a mod never changes the player's selection. Disabling the mod whose
theme was selected falls back to `base/phosphor` and says why, beside the theme
row - so a player can tell a reverted theme from a forgotten setting.

## Check it

```sh
nix develop --command cargo run content lint --target path/to/your-mod
```

The lint RESOLVES every theme the bundle declares, against every theme visible
to it. It refuses an unknown parent, a cycle, a missing role, a palette colour
no theme in the chain sets, and an unparseable hex. An unknown colour name
fails earlier, when the file is parsed. A theme that does
not resolve is not a theme that looks wrong; it is one the game would refuse
and fall back from.

Then run the game and open Settings. Headless output cannot prove appearance.
