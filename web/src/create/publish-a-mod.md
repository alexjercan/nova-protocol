# Publish a mod

This guide starts after your mod already works locally. It prepares the release,
runs the publishing checks, and generates the static portal files that players
install.

If you still need the folder or manifest, start with [Mod files](../mod-files/).
For content authoring, use [Create your first scenario](../author-a-scenario/),
[Campaign files](../campaigns/), or [Ship sections for mods](../sections/).

## 1. Prepare the release folder

Portal source mods live at `webmods/<id>/`. The directory name is the public mod
id and URL segment.

Create this directory and file layout manually:

```text
webmods/my-mod/
|- my-mod.bundle.ron
|- my-mod.content.ron
|- CHANGELOG.md
`- thumbnails/
   `- mission.png
```

Use only lowercase ASCII letters, digits, and `-` for the directory id. Do not
reuse a shipped mod id.

Create `webmods/my-mod/my-mod.bundle.ron` and write:

```ron
(
    content: ["my-mod.content.ron"],
    resources: ["thumbnails/mission.png"],
    meta: (
        name: "My Mod",
        description: "One complete mission.",
        author: "Your Name",
        version: "1.0.0",
        dependencies: ["base"],
    ),
)
```

Create `webmods/my-mod/my-mod.content.ron`. Put the tested `Scenario((...))`
item from your local mod inside its content list:

```ron
[
    Scenario((
        id: "my_mod_first_mission",
        name: "My First Mission",
        description: "One complete mission.",
        cubemap: "dep://base/textures/cubemap.png",
        events: [
            // Use the handlers you already tested.
        ],
    )),
]
```

Copy any images, models, or sounds that the scenario uses into this directory
and list each path under `resources`. Keep your own ids unique; prefix them with
`my_mod_`. The bundle must contain exactly one root `*.bundle.ron` file, and
every listed content file, resource, and dependency must exist.

`base` is always available, so declaring it is optional. The portal mods declare
it for clarity.

## 2. Set the version

The version is an opaque release string. The game detects an update when the
installed version differs from the catalog version.

Use a clear release sequence such as `1.0.0`, `1.1.0`, and `1.1.1`. Change the
version before every republish. The portal stores files under
`<id>/<version>/`; publishing changed files under an unchanged version makes the
update indistinguishable from the previous release.

Keep a mod-local `CHANGELOG.md` when the mod has more than one release:

```md
# Changelog

## 1.1.0

- Added a second mission.
- Rebalanced the escort ship.
```

## 3. Lint the mod

From the repository root:

```sh
nix develop --command cargo run content lint \
    --target webmods/my-mod \
    --report /tmp/my-mod-report.html
```

Open `/tmp/my-mod-report.html`. Fix every error and review every warning. The
report checks references, duplicate ids, asset membership, ship geometry,
scenario flow, and other content rules.

A fairness warning you INTEND belongs in the mod's own
[`balance_acks.ron`](../mod-files/#balance-acknowledgments). The linter reads
that file from the bundle it lints, so the reason travels with the mod.

## 4. Generate the portal

Generate a local portal tree:

```sh
python3 scripts/gen-portal.py \
    --source webmods \
    --shipped assets/mods.catalog.ron \
    --out /tmp/nova-mod-portal
```

The command validates every portal mod and exits non-zero on a publishing
error. It writes:

```text
/tmp/nova-mod-portal/
|- catalog.json
`- my-mod/
   `- 1.0.0/
      |- my-mod.bundle.ron
      |- my-mod.content.ron
      `- thumbnails/mission.png
```

Each `catalog.json` file entry includes its size and SHA-256 hash. The game
checks both while downloading.

Open `/tmp/nova-mod-portal/catalog.json` and confirm your id, name, version,
dependencies, and file list.

## 5. Publish

Add the complete `webmods/my-mod/` folder to the repository and land it on the
published branch. The next web deployment regenerates the portal catalog and
copies the versioned files.

Players then:

1. Open **Mods**.
2. Browse the online catalog.
3. Install the mod.
4. Enable it.
5. Open **Scenarios** to play its visible scenarios.

Downloaded files are verified before installation. The mod loads through the
same bundle and content pipeline used by shipped mods.

## Publish an update

For every update:

1. Change the content or resources.
2. Bump `meta.version`.
3. Add a matching changelog entry.
4. Run content lint and open its report.
5. Regenerate `/tmp/nova-mod-portal`.
6. Confirm the new version in `catalog.json`.
7. Publish the changed mod folder.

Changing dependencies is a content change. Use a new version and mention the
change because enabling the update can add or remove another mod's overlays.

## Preview the repository portal

The deployed portal is static. The site serves the game at `/play/`, the
catalog at `/mods/catalog.json`, and versioned mod files under
`/mods/<id>/<version>/`. The browser requires the game and portal to use the
same origin.

For a live repository preview that regenerates the portal when `webmods/`
changes:

```sh
nix develop --command scripts/serve-web.sh
```

Open the URL printed by the script. Launch the web game, open **Mods**, and use
**Explore online**. The site, game, and portal use separate helper servers but
are proxied onto one origin.

For the closest check to deployment, build and serve the complete static site:

```sh
nix develop --command scripts/preview-web.sh
```

This creates the production `/play/` and `/mods/` sibling layout. It does not
watch files; rerun it after a change.

Native builds do not enforce browser CORS. To test the generated portal against
a native game:

```sh
nix develop --command scripts/serve-mods.sh
# Use the printed port in a second shell:
NOVA_MODDING_PORTAL_URL=http://localhost:<port>/mods \
    nix develop --command cargo run
```

Do not place the portal under `/play/mods/`. Web builds always look for the
`/mods/` sibling.

## What the portal verifies

The generator is a package gate. It checks bundle metadata, file membership,
ids, dependencies, resource references, sizes, and hashes. It does not prove
that scenario and section data can load or play correctly. This is why a
release requires both checks:

1. `content lint` for loading and content rules.
2. `gen-portal.py` for package and catalog rules.

The repository's CI runs the deeper loader integration coverage. Mod authors do
not need a separate local load-test command after lint succeeds.

The generated catalog is deterministic. Each entry contains the mod id,
version, manifest path, metadata, total size, and every file's size and SHA-256
hash. The game verifies downloaded files before committing them to its local
mod cache.

## Publishing failures

- **No mod found** - `--source` points at the wrong directory.
- **Invalid id** - rename the folder to lowercase letters, digits, and `-`.
- **Wrong bundle count** - keep exactly one root `*.bundle.ron` file.
- **Missing member** - a `content` or `resources` path does not exist inside the
  mod folder.
- **Unknown dependency** - publish the dependency too or use a shipped id.
- **Shipped id collision** - choose a new portal id.
- **No visible scenario** - leave `hidden` unset on at least one scenario that
  players should launch directly.
