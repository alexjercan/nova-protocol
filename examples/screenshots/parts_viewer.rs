//! parts_viewer: a browsable gallery of the generated part-candidate meshes
//! under `art/part-candidates/**` (task 20260812-100246), so a human can judge
//! silhouettes, colours and cut quality BEFORE any part is promoted into game
//! content. Nothing here touches `assets/` - the candidates load through a
//! dedicated `part-candidates://` asset source rooted in `art/`, which ships in
//! no build.
//!
//! Three views over the same catalog (every `manifest.json` the part cutter
//! wrote - see `scripts/cut-obj-into-parts.py`):
//! - GALLERY: a paged grid of every part with name + size labels.
//! - FOCUSED: one part, large, on a slow turntable.
//! - SHIP: a recipe ship (multi-part manifest) reassembled at its manifest
//!   origins, with an exploded toggle that pushes the parts outward.
//!
//! Controls (interactive run: `cargo run --example parts_viewer`):
//! - arrows / PageUp / PageDown: move the selection / flip pages
//! - Enter: focus the selected part; left/right cycles; Esc: back
//! - Tab: ship view (Tab / left / right cycles ships); X: explode; Esc: back
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example parts_viewer --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example parts_viewer --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```
//!
//! Like `widget_zoo`, this is a bare `App` (no game plugins): it carries
//! `GameStates` and steps into `Playing` at startup purely so the shared
//! harness can drive it - "reached Playing" here means "the catalog is up".

use std::{
    fs,
    path::{Path, PathBuf},
};

use bevy::{
    asset::{
        io::{file::FileAssetReader, AssetSourceBuilder},
        AssetApp,
    },
    prelude::*,
};
use clap::Parser;
use nova_protocol::prelude::GameStates;

#[derive(Parser)]
#[command(name = "parts_viewer")]
#[command(version = "1.0.0")]
#[command(about = "Browse the generated part-candidate meshes (art/part-candidates)", long_about = None)]
struct Cli;

/// The bevy asset source name the candidates load through. Rooted at
/// [`CANDIDATES_DIR`] relative to the crate root, NOT under `assets/`: the
/// assets directory is copied wholesale into every shipped build, and these
/// are candidates under judgement, not shipped content.
const CANDIDATES_SOURCE: &str = "part-candidates";
const CANDIDATES_DIR: &str = "art/part-candidates";

/// Gallery grid: 4 x 3 cells per page.
const COLS: usize = 4;
const ROWS: usize = 3;
const PAGE: usize = COLS * ROWS;
const CELL_W: f32 = 4.4;
const CELL_H: f32 = 3.6;

/// How far the exploded view pushes each part from the ship centre.
const EXPLODE_FACTOR: f32 = 2.0;

fn main() -> AppExit {
    let _ = Cli::parse();

    let catalog = scan_catalog(&candidates_root());
    assert!(
        !catalog.entries.is_empty(),
        "parts_viewer: no manifest.json under {CANDIDATES_DIR} - run the part \
         cutter first (scripts/cut-obj-into-parts.py)"
    );
    // println: the log subscriber only exists once DefaultPlugins is added.
    println!(
        "parts_viewer: {} parts, {} recipe ships",
        catalog.entries.len(),
        catalog.ships.len()
    );

    let mut app = App::new();
    // The candidate source must be registered BEFORE AssetPlugin (bevy builds
    // sources when the plugin is added, not lazily).
    app.register_asset_source(
        CANDIDATES_SOURCE,
        AssetSourceBuilder::new(|| Box::new(FileAssetReader::new(CANDIDATES_DIR))),
    );
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: (1600, 900).into(),
            title: "Nova Protocol - parts viewer".into(),
            ..default()
        }),
        ..default()
    }));

    app.insert_resource(catalog);
    app.insert_resource(ViewerState::default());
    app.insert_resource(ClearColor(Color::srgb(0.015, 0.022, 0.045)));

    // The state machine the shared harness drives; `reach_playing` is
    // unconditional so an interactive run and a harnessed one take the same
    // path (the widget_zoo idiom).
    app.init_state::<GameStates>();
    app.add_systems(Startup, (setup, reach_playing));
    app.add_systems(
        Update,
        (
            keyboard,
            rebuild_view,
            spin_focused,
            place_labels,
            draw_selection,
        )
            .chain(),
    );

    #[cfg(feature = "debug")]
    {
        // Probe wiring, matching the other screenshots/ examples: run timeline
        // + engine-bound invariants (each inert without its NOVA_PERF_* env).
        // No frame-time capture - a paged walk holds no steady-state window.
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        if nova_protocol::prelude::capturing() {
            app.add_systems(Startup, nova_protocol::prelude::force_capture_resolution);
        }
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // No DebugPlugin here (bare app), so the smoke sentinel is logged
            // by the example itself - same const, so the two cannot drift.
            app.add_systems(OnEnter(GameStates::Playing), || {
                info!("{}", nova_protocol::nova_debug::harness::REACHED_PLAYING)
            });
        }
        app.add_plugins(viewer_script());
    }

    app.run()
}

/// The one-shot transition that satisfies the harness: "Playing" here means
/// "the catalog is up", nothing more.
fn reach_playing(mut next: ResMut<NextState<GameStates>>) {
    next.set(GameStates::Playing);
}

// ------------------------------- catalog ------------------------------------

/// One part glb the cutter emitted, with the manifest data the views need.
struct PartEntry {
    /// Catalog id: manifest dir relative to the candidates root, plus the part
    /// name for multi-part manifests (`racer/nose`, `blocks/weapon_cannon`).
    label: String,
    /// Full asset path (`part-candidates://racer/nose.glb#Scene0`).
    asset_path: String,
    /// The part's anchor in ship space (recipe ships reassemble at these).
    origin: Vec3,
    /// Local bbox centre relative to the anchor, for centring a lone part.
    center: Vec3,
    /// Local bbox size.
    size: Vec3,
}

/// A multi-part manifest: a recipe ship the SHIP view can reassemble.
struct RecipeShip {
    name: String,
    /// Indices into [`PartCatalog::entries`].
    parts: Vec<usize>,
}

#[derive(Resource)]
struct PartCatalog {
    entries: Vec<PartEntry>,
    ships: Vec<RecipeShip>,
}

/// Where the candidates live on disk for the startup SCAN (the asset source
/// resolves the same path itself). Under `cargo run` the manifest dir is the
/// crate root; a bare invocation falls back to the working directory.
fn candidates_root() -> PathBuf {
    let base = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    Path::new(&base).join(CANDIDATES_DIR)
}

/// Read every `manifest.json` under `root` into the catalog. Panics on a
/// malformed manifest - these are our own generated files, and a viewer
/// silently skipping parts would defeat its purpose.
fn scan_catalog(root: &Path) -> PartCatalog {
    let mut manifest_dirs = Vec::new();
    collect_manifest_dirs(root, &mut manifest_dirs);
    // Judged sets first (the recipe ships), then the packs, in a stable order.
    let rank = |rel: &str| {
        let top = rel.split('/').next().unwrap_or(rel);
        ["racer", "cargob", "blocks", "craft", "quaternius"]
            .iter()
            .position(|k| *k == top)
            .unwrap_or(usize::MAX)
    };
    manifest_dirs.sort_by(|a, b| (rank(a), a.clone()).cmp(&(rank(b), b.clone())));

    let mut entries = Vec::new();
    let mut ships = Vec::new();
    for rel in &manifest_dirs {
        let path = root.join(rel).join("manifest.json");
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("parts_viewer: read {path:?}: {e}"));
        let doc: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("parts_viewer: parse {path:?}: {e}"));
        let parts = doc["parts"]
            .as_array()
            .unwrap_or_else(|| panic!("parts_viewer: {path:?} has no parts array"));

        let mut indices = Vec::new();
        for part in parts {
            let name = part["name"].as_str().expect("part name");
            let file = part["file"].as_str().expect("part file");
            let lo = vec3_of(&part["bbox"]["min"]);
            let hi = vec3_of(&part["bbox"]["max"]);
            indices.push(entries.len());
            entries.push(PartEntry {
                label: if parts.len() > 1 {
                    format!("{rel}/{name}")
                } else {
                    rel.clone()
                },
                asset_path: format!("{CANDIDATES_SOURCE}://{rel}/{file}#Scene0"),
                origin: vec3_of(&part["origin"]),
                center: (lo + hi) / 2.0,
                size: hi - lo,
            });
        }
        if parts.len() > 1 {
            ships.push(RecipeShip {
                name: rel.clone(),
                parts: indices,
            });
        }
    }
    PartCatalog { entries, ships }
}

/// Every directory (relative to `root`) that holds a `manifest.json`, in
/// sorted traversal order.
fn collect_manifest_dirs(root: &Path, out: &mut Vec<String>) {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(read) = fs::read_dir(dir) else { return };
        let mut children: Vec<PathBuf> = read.filter_map(|e| e.ok().map(|e| e.path())).collect();
        children.sort();
        if dir.join("manifest.json").is_file() {
            let rel = dir
                .strip_prefix(root)
                .expect("walk stays under root")
                .to_string_lossy()
                .replace('\\', "/");
            out.push(rel);
        }
        for child in children {
            if child.is_dir() {
                walk(root, &child, out);
            }
        }
    }
    walk(root, root, out);
}

fn vec3_of(value: &serde_json::Value) -> Vec3 {
    let n = |i: usize| value[i].as_f64().expect("manifest vec3 component") as f32;
    Vec3::new(n(0), n(1), n(2))
}

// ------------------------------- state --------------------------------------

/// Which view is up. `rebuild_view` respawns the scene whenever this changes,
/// so the keyboard, the autopilot script and the capture walk all drive the
/// viewer through the same seam.
#[derive(Resource, Clone, PartialEq)]
enum ViewerState {
    Gallery { selected: usize },
    Focused { index: usize },
    Ship { ship: usize, exploded: bool },
}

impl Default for ViewerState {
    fn default() -> Self {
        Self::Gallery { selected: 0 }
    }
}

/// A spawned 3D view entity (scene instance root); cleared on every rebuild.
#[derive(Component)]
struct ViewItem;

/// A label following a world anchor (`None` keeps its authored screen spot).
#[derive(Component)]
struct ViewLabel {
    anchor: Option<Vec3>,
}

/// The focused part's turntable.
#[derive(Component)]
struct Spin;

// ------------------------------- setup --------------------------------------

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.6, 16.0).looking_at(Vec3::ZERO, Vec3::Y),
        // The parts are flat-Kd meshes; a touch of ambient keeps the shadow
        // side readable without washing out the key light's form.
        AmbientLight {
            color: Color::WHITE,
            brightness: 220.0,
            affects_lightmapped_meshes: true,
        },
    ));
    // Key + rim, echoing the screenshot rig's bearings so the parts read as
    // lit forms rather than flat fills.
    commands.spawn((
        DirectionalLight {
            illuminance: 9_000.0,
            ..default()
        },
        Transform::from_xyz(-6.0, 8.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 2_500.0,
            ..default()
        },
        Transform::from_xyz(5.0, -2.0, -7.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let font = asset_server.load("fonts/SGr-IosevkaTerm-Medium.ttf");
    commands.insert_resource(ViewerFont(font.clone()));
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: px(16.0),
            top: px(10.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(2.0),
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                Text::new("PARTS VIEWER // art/part-candidates"),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.95, 0.65)),
            ));
            header.spawn((
                Text::new(
                    "arrows: select   PgUp/PgDn: page   Enter: focus   Tab: ships   X: explode   Esc: back",
                ),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.45, 0.55, 0.6)),
            ));
        });
}

/// The label font, cached so rebuilds do not re-ask the asset server.
#[derive(Resource, Clone)]
struct ViewerFont(Handle<Font>);

// ------------------------------- input --------------------------------------

fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    catalog: Res<PartCatalog>,
    mut state: ResMut<ViewerState>,
) {
    let n = catalog.entries.len();
    let ships = catalog.ships.len();
    let pressed = |codes: &[KeyCode]| codes.iter().any(|c| keys.just_pressed(*c));

    let next = match *state {
        ViewerState::Gallery { selected } => {
            let mut sel = selected as isize;
            if pressed(&[KeyCode::ArrowLeft]) {
                sel -= 1;
            }
            if pressed(&[KeyCode::ArrowRight]) {
                sel += 1;
            }
            if pressed(&[KeyCode::ArrowUp]) {
                sel -= COLS as isize;
            }
            if pressed(&[KeyCode::ArrowDown]) {
                sel += COLS as isize;
            }
            if pressed(&[KeyCode::PageUp]) {
                sel -= PAGE as isize;
            }
            if pressed(&[KeyCode::PageDown]) {
                sel += PAGE as isize;
            }
            let sel = sel.clamp(0, n as isize - 1) as usize;
            if pressed(&[KeyCode::Enter]) {
                ViewerState::Focused { index: sel }
            } else if pressed(&[KeyCode::Tab]) && ships > 0 {
                ViewerState::Ship {
                    ship: 0,
                    exploded: false,
                }
            } else {
                ViewerState::Gallery { selected: sel }
            }
        }
        ViewerState::Focused { index } => {
            let mut idx = index as isize;
            if pressed(&[KeyCode::ArrowLeft]) {
                idx -= 1;
            }
            if pressed(&[KeyCode::ArrowRight]) {
                idx += 1;
            }
            let idx = idx.rem_euclid(n as isize) as usize;
            if pressed(&[KeyCode::Escape]) {
                ViewerState::Gallery { selected: idx }
            } else if pressed(&[KeyCode::Tab]) && ships > 0 {
                ViewerState::Ship {
                    ship: 0,
                    exploded: false,
                }
            } else {
                ViewerState::Focused { index: idx }
            }
        }
        ViewerState::Ship { ship, exploded } => {
            let mut s = ship as isize;
            if pressed(&[KeyCode::Tab, KeyCode::ArrowRight]) {
                s += 1;
            }
            if pressed(&[KeyCode::ArrowLeft]) {
                s -= 1;
            }
            let s = s.rem_euclid(ships as isize) as usize;
            let exploded = exploded ^ pressed(&[KeyCode::KeyX]);
            if pressed(&[KeyCode::Escape]) {
                ViewerState::Gallery { selected: 0 }
            } else {
                ViewerState::Ship { ship: s, exploded }
            }
        }
    };
    // Only a real change dirties the resource, so `rebuild_view` respawns
    // nothing on idle frames.
    if *state != next {
        *state = next;
    }
}

// ------------------------------- views --------------------------------------

/// Three-quarter presentation: yaw the nose (-Z) mostly toward the camera.
/// The camera sits slightly above the grid plane, so no per-part tilt needed.
fn present_rotation() -> Quat {
    Quat::from_rotation_y(2.5)
}

/// Ship presentation: more side-on than a lone part, plus a pitch that shows
/// the deck - a hauler read nose-on collapses into a lump.
fn ship_rotation() -> Quat {
    Quat::from_rotation_x(0.30) * Quat::from_rotation_y(2.05)
}

/// World position of a gallery slot (0..PAGE), centred under the header.
fn cell_pos(slot: usize) -> Vec3 {
    let col = (slot % COLS) as f32;
    let row = (slot / COLS) as f32;
    Vec3::new(
        (col - (COLS as f32 - 1.0) / 2.0) * CELL_W,
        ((ROWS as f32 - 1.0) / 2.0 - row) * CELL_H + 0.4,
        0.0,
    )
}

/// Respawn the 3D items + labels whenever the view changes. One code path for
/// every view, so the interactive keys and the capture script cannot drift.
fn rebuild_view(
    mut commands: Commands,
    state: Res<ViewerState>,
    catalog: Res<PartCatalog>,
    font: Option<Res<ViewerFont>>,
    asset_server: Res<AssetServer>,
    existing: Query<Entity, Or<(With<ViewItem>, With<ViewLabel>)>>,
    mut last: Local<Option<ViewerState>>,
) {
    let Some(font) = font else { return };
    if last.as_ref() == Some(&*state) {
        return;
    }
    *last = Some(state.clone());
    for entity in &existing {
        commands.entity(entity).despawn();
    }

    match &*state {
        ViewerState::Gallery { selected } => {
            let page = selected / PAGE;
            let start = page * PAGE;
            let end = (start + PAGE).min(catalog.entries.len());
            for (slot, index) in (start..end).enumerate() {
                let entry = &catalog.entries[index];
                let pos = cell_pos(slot);
                // True scale up to a cap, shrink-to-fit past it: relative part
                // sizes stay honest, whole craft still fit their cell.
                let fit = (2.7 / entry.size.max_element()).min(1.2);
                spawn_part(&mut commands, &asset_server, entry, pos, fit, false);
                let (cat, name) = split_label(&entry.label);
                spawn_label(
                    &mut commands,
                    &font,
                    Some(pos + Vec3::new(0.0, -CELL_H / 2.0 + 0.55, 0.0)),
                    &[
                        (name, 13.0, label_color(index == *selected)),
                        (cat, 10.0, Color::srgb(0.4, 0.5, 0.55)),
                    ],
                );
            }
            let pages = catalog.entries.len().div_ceil(PAGE);
            mode_line(
                &mut commands,
                &font,
                format!(
                    "gallery: parts {}-{} of {}  (page {}/{})",
                    start + 1,
                    end,
                    catalog.entries.len(),
                    page + 1,
                    pages
                ),
            );
        }
        ViewerState::Focused { index } => {
            let entry = &catalog.entries[*index];
            let fit = (4.6 / entry.size.max_element()).clamp(0.1, 9.0);
            spawn_part(
                &mut commands,
                &asset_server,
                entry,
                Vec3::new(0.0, 0.6, 4.0),
                fit,
                true,
            );
            mode_line(
                &mut commands,
                &font,
                format!(
                    "focused: {}   size {:.2} x {:.2} x {:.2}",
                    entry.label, entry.size.x, entry.size.y, entry.size.z
                ),
            );
        }
        ViewerState::Ship { ship, exploded } => {
            let ship = &catalog.ships[*ship];
            let factor = if *exploded { EXPLODE_FACTOR } else { 1.0 };
            // Assembled extent: union of origin-placed part boxes; its centre
            // goes to the view origin so both toggles frame the same ship.
            let mut lo = Vec3::MAX;
            let mut hi = Vec3::MIN;
            for &i in &ship.parts {
                let e = &catalog.entries[i];
                lo = lo.min(e.origin + e.center - e.size / 2.0);
                hi = hi.max(e.origin + e.center + e.size / 2.0);
            }
            let center = (lo + hi) / 2.0;
            let extent = (hi - lo).max_element() * factor;
            let fit = (8.0 / extent).min(1.6);
            commands
                .spawn((
                    ViewItem,
                    Transform::from_xyz(0.0, 0.6, 4.0)
                        .with_rotation(ship_rotation())
                        .with_scale(Vec3::splat(fit)),
                    Visibility::default(),
                ))
                .with_children(|parent| {
                    for &i in &ship.parts {
                        let e = &catalog.entries[i];
                        parent.spawn((
                            WorldAssetRoot(asset_server.load(&e.asset_path)),
                            Transform::from_translation((e.origin - center) * factor),
                        ));
                    }
                });
            mode_line(
                &mut commands,
                &font,
                format!(
                    "ship: {}  [{}]   {} parts",
                    ship.name,
                    if *exploded { "exploded" } else { "assembled" },
                    ship.parts.len()
                ),
            );
        }
    }
}

/// One part instance: parent carries the pose, the child recentres the mesh
/// on its bbox so oddly-anchored parts still sit in their cell.
fn spawn_part(
    commands: &mut Commands,
    asset_server: &AssetServer,
    entry: &PartEntry,
    pos: Vec3,
    scale: f32,
    spin: bool,
) {
    let mut item = commands.spawn((
        ViewItem,
        Transform::from_translation(pos)
            .with_rotation(present_rotation())
            .with_scale(Vec3::splat(scale)),
        Visibility::default(),
    ));
    if spin {
        item.insert(Spin);
    }
    item.with_children(|parent| {
        parent.spawn((
            WorldAssetRoot(asset_server.load(&entry.asset_path)),
            Transform::from_translation(-entry.center),
        ));
    });
}

/// A label block: stacked text lines, centred on a world anchor by
/// [`place_labels`] (or fixed on screen when `anchor` is `None`).
fn spawn_label(
    commands: &mut Commands,
    font: &ViewerFont,
    anchor: Option<Vec3>,
    lines: &[(String, f32, Color)],
) {
    let mut label = commands.spawn((
        ViewLabel { anchor },
        Node {
            position_type: PositionType::Absolute,
            left: px(-1000.0),
            top: px(-1000.0),
            width: px(320.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
    ));
    label.with_children(|parent| {
        for (text, size, color) in lines {
            parent.spawn((
                Text::new(text.clone()),
                TextFont {
                    font: font.0.clone().into(),
                    font_size: FontSize::Px(*size),
                    ..default()
                },
                TextColor(*color),
            ));
        }
    });
}

/// The view's status line, bottom left.
fn mode_line(commands: &mut Commands, font: &ViewerFont, text: String) {
    let mut label = commands.spawn((
        ViewLabel { anchor: None },
        Node {
            position_type: PositionType::Absolute,
            left: px(16.0),
            bottom: px(12.0),
            ..default()
        },
    ));
    label.with_children(|parent| {
        parent.spawn((
            Text::new(text),
            TextFont {
                font: font.0.clone().into(),
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::srgb(0.75, 0.85, 0.9)),
        ));
    });
}

fn label_color(selected: bool) -> Color {
    if selected {
        Color::srgb(1.0, 0.85, 0.3)
    } else {
        Color::srgb(0.85, 0.9, 0.95)
    }
}

/// `racer/nose` -> ("racer", "nose"); `blocks/structure_wing_double` ->
/// ("blocks structure", "wing double") - the pack prefix moves into the
/// category line so the name line stays short enough for its cell.
fn split_label(label: &str) -> (String, String) {
    let (top, rest) = label.split_once('/').unwrap_or(("", label));
    match top {
        "blocks" => {
            let (cat, name) = rest.split_once('_').unwrap_or(("", rest));
            (format!("blocks {cat}"), name.replace('_', " "))
        }
        _ => (top.to_string(), rest.replace('_', " ")),
    }
}

/// Turntable for the focused part.
fn spin_focused(time: Res<Time>, mut spinners: Query<&mut Transform, With<Spin>>) {
    for mut transform in &mut spinners {
        transform.rotation = Quat::from_rotation_y(2.5 + time.elapsed_secs() * 0.5);
    }
}

/// Project each anchored label under its part. Screen-fixed labels keep their
/// authored spot.
fn place_labels(
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut labels: Query<(&ViewLabel, &mut Node)>,
) {
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    for (label, mut node) in &mut labels {
        let Some(anchor) = label.anchor else { continue };
        match camera.world_to_viewport(camera_transform, anchor) {
            Ok(pos) => {
                node.left = px(pos.x - 160.0);
                node.top = px(pos.y);
            }
            Err(_) => {
                node.left = px(-1000.0);
            }
        }
    }
}

/// A wire box around the selected gallery cell.
fn draw_selection(mut gizmos: Gizmos, state: Res<ViewerState>) {
    let ViewerState::Gallery { selected } = *state else {
        return;
    };
    let pos = cell_pos(selected % PAGE);
    // Nearly flat: a deep box seen off-axis skews across half the screen.
    gizmos.cube(
        Transform::from_translation(pos).with_scale(Vec3::new(CELL_W - 0.9, CELL_H - 0.7, 0.05)),
        Color::srgb(0.4, 0.9, 0.5),
    );
}

// ------------------------------- harness ------------------------------------
//
// The capture walk: page through the whole gallery, focus one blocks piece,
// then show each recipe ship assembled and exploded - shooting a PNG at every
// stop when NOVA_CAPTURE is armed (the shared `shoot` idiom; the unarmed smoke
// run walks identical steps and writes nothing).

#[cfg(feature = "debug")]
fn viewer_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    use nova_protocol::prelude::{
        frames, shoot, shot_written, state_is, SETTLE_FRAMES, SHOT_DEADLINE_SECS,
    };

    // The catalog is scanned before the app is built, so the script can size
    // itself to the real page count and the real recipe-ship list.
    let catalog = scan_catalog(&candidates_root());
    let pages = catalog.entries.len().div_ceil(PAGE);

    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("viewer: reach Playing")
        .until(state_is(GameStates::Playing))
        .deadline(30.0)
        .add()
        // The scenes stream in async; the settle covers load + first layout.
        .step("viewer: settle the gallery")
        .until(frames(SETTLE_FRAMES * 2))
        .add();

    for page in 0..pages {
        let shot = format!("parts-viewer-gallery-{}.png", page + 1);
        if page > 0 {
            script = script
                .step(format!("viewer: flip to gallery page {}", page + 1))
                .on_enter(move |world: &mut World| {
                    *world.resource_mut::<ViewerState>() = ViewerState::Gallery {
                        selected: page * PAGE,
                    };
                })
                .until(frames(SETTLE_FRAMES))
                .add();
        }
        let ack = shot.clone();
        script = script
            .step(format!("viewer: shoot {shot}"))
            .on_enter(move |world: &mut World| shoot(world, &shot))
            .until(shot_written(ack))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }

    script = script
        .step("viewer: focus a blocks piece")
        .on_enter(|world: &mut World| {
            let index = {
                let catalog = world.resource::<PartCatalog>();
                catalog
                    .entries
                    .iter()
                    .position(|e| e.label == "blocks/structure_cockpit_model_a")
                    .unwrap_or(0)
            };
            *world.resource_mut::<ViewerState>() = ViewerState::Focused { index };
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("viewer: shoot the focused part")
        .on_enter(|world: &mut World| shoot(world, "parts-viewer-focused.png"))
        .until(shot_written("parts-viewer-focused.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add();

    // Cut-face close-ups: pose recipe parts so the camera looks INTO the
    // former cut openings - the shots that prove the caps read solid and the
    // recipe planes clip no feature geometry. The turntable is stopped (Spin
    // removed) so the pose is deterministic; faces whose posed normal gains a
    // world +Z component face the camera, so yaw -pi/4 shows a part's +X and
    // +Z sides, +pi/4 its -X and +Z sides, and +-3pi/4 the -Z pairings.
    for (label, yaw, shot) in [
        (
            "racer/wing_port",
            -std::f32::consts::FRAC_PI_4,
            "parts-viewer-racer-wing-cut.png",
        ),
        (
            "racer/fuselage",
            -3.0 * std::f32::consts::FRAC_PI_4,
            "parts-viewer-racer-fuselage-cut.png",
        ),
        // Inboard cut (-X) plus the nozzle rear (+Z): the full nacelle width.
        (
            "racer/engine_starboard",
            std::f32::consts::FRAC_PI_4,
            "parts-viewer-racer-engine-cut.png",
        ),
        // Inboard cut (+X) plus the thruster rear (+Z).
        (
            "cargob/engine_port",
            -std::f32::consts::FRAC_PI_4,
            "parts-viewer-cargob-engine-cut.png",
        ),
        // Torpedo bay mouths at the pod front (-Z) plus the inboard cut (-X).
        (
            "cargob/pod_starboard",
            3.0 * std::f32::consts::FRAC_PI_4,
            "parts-viewer-cargob-pod-bay.png",
        ),
    ] {
        script = script
            .step(format!("viewer: focus {label}"))
            .on_enter(move |world: &mut World| {
                let index = {
                    let catalog = world.resource::<PartCatalog>();
                    catalog
                        .entries
                        .iter()
                        .position(|e| e.label == label)
                        .unwrap_or(0)
                };
                *world.resource_mut::<ViewerState>() = ViewerState::Focused { index };
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("viewer: pose {label} cut faces toward the camera"))
            .on_enter(move |world: &mut World| {
                let mut spinners = world.query_filtered::<Entity, With<Spin>>();
                let entities: Vec<Entity> = spinners.iter(world).collect();
                for entity in entities {
                    let mut item = world.entity_mut(entity);
                    item.remove::<Spin>();
                    if let Some(mut transform) = item.get_mut::<Transform>() {
                        transform.rotation = Quat::from_rotation_y(yaw);
                    }
                }
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("viewer: shoot {shot}"))
            .on_enter(move |world: &mut World| shoot(world, shot))
            .until(shot_written(shot))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }

    // Every recipe ship, assembled + exploded. The list comes from the same
    // startup scan as the page count; the index is re-resolved by name at
    // runtime so catalog-order changes cannot skew a shot onto another ship.
    for ship_name in catalog.ships.iter().map(|s| s.name.clone()) {
        for exploded in [false, true] {
            let shot = format!(
                "parts-viewer-{}-{}.png",
                ship_name.replace('/', "-"),
                if exploded { "exploded" } else { "assembled" }
            );
            let ack = shot.clone();
            let target = ship_name.clone();
            script = script
                .step(format!("viewer: show {shot}"))
                .on_enter(move |world: &mut World| {
                    let ship = {
                        let catalog = world.resource::<PartCatalog>();
                        catalog
                            .ships
                            .iter()
                            .position(|s| s.name == target)
                            .unwrap_or(0)
                    };
                    *world.resource_mut::<ViewerState>() = ViewerState::Ship { ship, exploded };
                })
                .until(frames(SETTLE_FRAMES))
                .add()
                .step(format!("viewer: shoot {shot}"))
                .on_enter(move |world: &mut World| shoot(world, &shot))
                .until(shot_written(ack))
                .deadline(SHOT_DEADLINE_SECS)
                .add();
        }
    }
    script
}
