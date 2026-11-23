# Faction palettes for mods

<!-- Source: crates/nova_ship/src/sections/faction_palette.rs
     (`FactionPaletteConfig`, `FactionPaletteConfig::faults`,
     `SkinLook::worn_style`, GREEBLE_DENSITY_RANGE, MAGENTA_BAND_DEGREES,
     DECOR_KEY_COLOR). Pick per civilization:
     crates/nova_world_base/src/ship_parts.rs `ShipPartSnapshot::palette`.
     Key tint: crates/nova_ship/src/sections/skin_decor.rs `tint_decor_keys`.
     Base palettes: crates/nova_authoring/src/base_content/palettes.rs. -->

A `Palette` is the colours one civilization paints its hulls in, and its habit
with greebles. In a New Game world the generator gives every civilization one
palette from the catalog. Its civilians, industrials, scavengers, armored
ships, wrecks and stations all wear it, so a ship's colour says whose it is.

A palette changes the PLATES and the DECORATION a [style](../styles/) places.
The style still says which pieces a role's hull carries and where they go; the
palette says what colour the plates are, how busy the hull is, and which extra
pieces this civilization bolts on.

For RON spelling rules such as double parentheses and tagged colours, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## How the world picks a palette

Civilizations stand on a lattice. A civilization at node `(x, y, z)` takes

```text
order[(x + 2y + 4z) mod N]
```

where `N` is the number of palettes in the catalog and `order` is the catalog
shuffled by the world seed. For any neighbour node the step `dx + 2dy + 4dz` is
between -7 and 7 and never 0, so with 8 or more palettes no two neighbouring
civilizations share one. The catalog needs at least 8: with fewer the world
refuses to start, and lint names the catalog. The base game ships 15: maroon,
rust, crimson, ochre, bone, olive, moss, emerald, teal, sky, steel blue,
cobalt, indigo, plum and slate.

The shuffle can make any two palettes neighbours, so a new palette must read
apart from every other one, not only from the next hue. Two near hues can still
read apart by saturation or lightness: the base maroon is a dark red and
crimson a bright one.

The pick is a function of the seed and the node, so a save stores nothing new
for it. A generated hull writes its palette id into its design, so a hull you
buy keeps the colours of the station that sold it, and a wreck keeps its
builder's colours.

Ships that are not generated name no palette: start hulls, scenario ships and
editor builds wear their style's own palette.

## The Palette item

```ron
[
    Palette((
        id: "my_mod/rust_belt",
        name: "Rust Belt",
        roles: (
            civilian: (
                top: (color: Srgba((red: 0.62, green: 0.55, blue: 0.40, alpha: 1.0)), roughness: 0.45, metallic: 0.1),
                wall: (color: Srgba((red: 0.21, green: 0.18, blue: 0.13, alpha: 1.0)), roughness: 0.5, metallic: 0.1),
            ),
            industrial: (
                top: (color: Srgba((red: 0.48, green: 0.40, blue: 0.24, alpha: 1.0)), roughness: 0.9, metallic: 0.15),
                wall: (color: Srgba((red: 0.16, green: 0.13, blue: 0.08, alpha: 1.0)), roughness: 0.95, metallic: 0.15),
            ),
            scavenger: (
                top: (color: Srgba((red: 0.33, green: 0.30, blue: 0.24, alpha: 1.0)), roughness: 0.95, metallic: 0.2),
                wall: (color: Srgba((red: 0.11, green: 0.10, blue: 0.08, alpha: 1.0)), roughness: 1.0, metallic: 0.2),
            ),
            armored: (
                top: (color: Srgba((red: 0.28, green: 0.24, blue: 0.16, alpha: 1.0)), roughness: 0.6, metallic: 0.5),
                wall: (color: Srgba((red: 0.09, green: 0.08, blue: 0.05, alpha: 1.0)), roughness: 0.65, metallic: 0.5),
            ),
            station: (
                top: (color: Srgba((red: 0.50, green: 0.45, blue: 0.36, alpha: 1.0)), roughness: 0.7, metallic: 0.3),
                wall: (color: Srgba((red: 0.17, green: 0.15, blue: 0.12, alpha: 1.0)), roughness: 0.75, metallic: 0.3),
            ),
        ),
        greeble_density: 1.2,
        accents: [
            (style: "salvage", fixture: "salvage_drum", weight: 0.4, max_count: 2),
        ],
    )),
]
```

| field | type | meaning |
|---|---|---|
| `id` | string | what a design names this palette by. A base id REPLACES that palette |
| `name` | string | display name |
| `roles` | roles | one plate finish per role and one for stations, all five required |
| `greeble_density` | float | how busy the hull is, 0.5 to 2.0. 1.0 keeps the style's own |
| `accents` | list | extra pieces this civilization adds, may be empty |

Every field is required. There is no default palette and no default finish.

### `roles`

`civilian`, `industrial`, `scavenger`, `armored` and `station`. Each is a
finish pair with the same `top` and `wall` fields as a style's `palette` - see
[Ship skin styles](../styles/#palette). A generated ship wears the finish of
its role; a generated station wears `station`.

The base palettes keep one tone per role in every hue, so the hue says whose
hull it is and the tone says what it does: civilian light and smooth,
industrial matte, scavenger dull, armored dark and metallic, station mid-grey
of the hue. A few base palettes (maroon, plum, bone, emerald, sky)
make every tone darker or lighter by one factor and keep the role order. A mod
palette may break that order, but a player reads the role by it.

### `greeble_density`

Every empty greeble of the wearing style has its weight divided by this
factor. Above 1.0 fewer plates stay bare; below 1.0 more do. The style's own
`max_count` and `near` rules still hold, so a dense palette cannot crowd a
piece the style limits.

### `accents`

| field | type | meaning |
|---|---|---|
| `style` | string | the style that authors the piece |
| `fixture` | string | its fixture id within that style |
| `weight` | float | its greeble weight against the wearing style's own weights, above 0 |
| `max_count` | integer | the most copies on one hull, at least 1 |

An accent is placed alone: it takes the link points of the wearing style's
first empty greeble, so it may stand where an empty plate may. A style with no
empty greeble places no accent. An accent may name a piece of any loaded
style, so a civilization can carry one kit's sensor on every hull it builds.

## Greeble tint: the key colour

A greeble model may leave part of itself for the hull to colour. Paint that
part of the `.glb` the KEY colour, pure magenta `(1, 0, 1)`, and give the
style's fixture `tint: Hull`. When the model spawns, every keyed material is
replaced by the hull's top finish: the palette's top for the ship's role, or
the style's own top when the ship names no palette. A stripe or a fin then
matches the plate it stands on in every palette.

A fixture with `tint: Baked` keeps every colour of its model.

## Lint

Lint refuses a palette, and the load drops it with an error, when:

- a `top` or `wall` colour has a hue within 20 degrees of the key magenta
  (hue 300): a piece that missed its tint must still show;
- `greeble_density` is outside 0.5 to 2.0;
- an accent names a style or fixture that no loaded style lists, names one
  fixture twice, or has a weight not above 0 or a `max_count` of 0;
- a style's greeble rules no longer compile with the palette's habit applied.

A grey colour, with a saturation under 0.15, has no hue and passes the hue
check. Reds are allowed. Keep a red or a purple well clear of magenta, so a
missed tint still stands out on the hull.

Lint also refuses:

- a ship design that names an unknown palette, or a palette with no style for
  it to dress;
- a fixture with `tint: Hull` whose model has no key colour, and a fixture
  with `tint: Baked` whose model has one: the first would never tint, and the
  second would show magenta;
- a catalog with fewer than 8 palettes.

A keyed piece that is spawned with no tint shows magenta. That is a bug to
report, not a look.

## Using a palette

A mod adds a palette by writing a new id, or recolours a base one by reusing
its id. Generated ships pick from the whole merged catalog, so a new palette
also changes which civilization wears which. A palette is part of the content
catalog a save is pinned to: a world saved before the change is refused, as
after any other content change.

A hand-made [ship](../ships/) may name a palette in its presentation:

```ron
presentation: (
    skin: true,
    style: Some("civilian"),
    palette: Some((id: "teal", role: Civilian)),
),
```

`role` is `Civilian`, `Industrial`, `Scavenger`, `Armored` or `Station`, and
picks the finish. A palette needs a style to dress: `style: None` with a
palette is refused.
