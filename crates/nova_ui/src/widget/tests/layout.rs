//! The layout kit, laid out by bevy_ui under both shipped themes, and its gate
//! and amount behaviour.

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use bevy::{
    asset::AssetPlugin,
    camera::{Camera, ComputedCameraValues, RenderTargetInfo},
    image::{Image, TextureAtlasLayout},
    math::UVec2,
    prelude::*,
    text::TextPlugin,
    ui::{ComputedNode, InteractionDisabled, UiGlobalTransform, UiPlugin},
    ui_widgets::Activate,
    window::{PrimaryWindow, WindowResolution},
};

use crate::{
    theme::{
        base::base_ui_themes, GameUiThemes, SelectedUiTheme, UiColorType, HARDWARE_THEME_ID,
        PHOSPHOR_THEME_ID,
    },
    widget::{
        amount_row, button, chip, fixtures::themed_app, metrics::CONTROL_H, tag, text_field,
        verb_row, Amount, AmountChanged, AmountField, AmountMax, AmountSlider, AmountSpec,
        AmountTop, ButtonSpec, GateTagOf, RowAlign, TextFieldSpec, TextFieldValue, Tooltip,
        VerbGate,
    },
    NovaUiPlugin,
};

const WINDOW: UVec2 = UVec2::new(800, 600);

/// A headless app that lays the UI out: bevy_ui, text and the kit over a
/// window camera, with `theme_id` selected.
fn laid_out_app(theme_id: &str) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        TransformPlugin,
        bevy::a11y::AccessibilityPlugin,
        bevy::camera::visibility::VisibilityPlugin,
        bevy::input::InputPlugin,
        bevy::picking::input::PointerInputPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        TextPlugin,
        UiPlugin,
        NovaUiPlugin,
    ));
    // The window backend and render plugins register these in the game.
    app.add_message::<bevy::window::WindowEvent>();
    app.init_asset::<Image>().init_asset::<TextureAtlasLayout>();
    app.init_asset::<Mesh>()
        .init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
    app.insert_resource(GameUiThemes(base_ui_themes()));
    app.insert_resource(SelectedUiTheme(theme_id.to_string()));
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(WINDOW.x, WINDOW.y),
            ..default()
        },
        PrimaryWindow,
    ));
    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: WINDOW,
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
        IsDefaultUiCamera,
    ));
    app
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// A laid-out node's rect in logical px.
fn rect(app: &App, entity: Entity) -> Rect {
    let node = app
        .world()
        .get::<ComputedNode>(entity)
        .expect("a laid-out node");
    let transform = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("a laid-out node");
    let scale = node.inverse_scale_factor();
    Rect::from_center_size(transform.translation * scale, node.size() * scale)
}

fn named(app: &mut App, name: &str) -> Entity {
    let world = app.world_mut();
    world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, n)| n.as_str() == name)
        .map(|(e, _)| e)
        .unwrap_or_else(|| panic!("a node named {name}"))
}

fn parent(app: &App, entity: Entity) -> Entity {
    app.world()
        .get::<ChildOf>(entity)
        .expect("a child")
        .parent()
}

/// The parts of one fixture: every control the kit builds.
struct Fixture {
    controls: Vec<(&'static str, Entity)>,
}

fn spawn_fixture(app: &mut App) -> Fixture {
    let world = app.world_mut();
    let root = world
        .spawn(Node {
            width: px(300),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            ..default()
        })
        .id();
    let verb = world
        .spawn((button(ButtonSpec::control("Repair")), ChildOf(root)))
        .id();
    let field = world
        .spawn((text_field(TextFieldSpec::new("12").tool()), ChildOf(root)))
        .id();
    let chip_e = world.spawn((chip("All", None), ChildOf(root))).id();
    let row = world.spawn((verb_row(RowAlign::End), ChildOf(root))).id();
    let tagged = world
        .spawn((
            button(ButtonSpec::control("Delete")),
            VerbGate::Tagged("PORT"),
            ChildOf(row),
        ))
        .id();
    let primary = world
        .spawn((
            button(ButtonSpec::control("Repair").primary()),
            ChildOf(row),
        ))
        .id();
    world.spawn((tag("REFIT", UiColorType::Nominal), ChildOf(root)));
    let amount = world
        .spawn((amount_row(AmountSpec { name: "Fixture" }), ChildOf(root)))
        .id();
    Fixture {
        controls: vec![
            ("button", verb),
            ("field", field),
            ("chip", chip_e),
            ("verb row", row),
            ("tagged verb", tagged),
            ("primary verb", primary),
            ("amount row", amount),
        ],
    }
}

/// Every control the kit builds is one control high, and both shipped themes
/// lay a block out to the same rects: a theme paints, it never moves a
/// control.
#[test]
fn every_control_is_one_control_high_in_every_theme() {
    let mut rects = Vec::new();
    for theme in [PHOSPHOR_THEME_ID, HARDWARE_THEME_ID] {
        let mut app = laid_out_app(theme);
        let fixture = spawn_fixture(&mut app);
        settle(&mut app);
        let mut parts: Vec<(String, Rect)> = Vec::new();
        for (what, entity) in &fixture.controls {
            let r = rect(&app, *entity);
            assert!(
                (r.height() - CONTROL_H).abs() < 0.01,
                "{theme}: the {what} is {} high",
                r.height()
            );
            parts.push(((*what).to_string(), r));
        }
        for name in ["FixtureField", "FixtureMax"] {
            let entity = named(&mut app, name);
            let r = rect(&app, entity);
            assert!(
                (r.height() - CONTROL_H).abs() < 0.01,
                "{theme}: {name} is {} high",
                r.height()
            );
            parts.push((name.to_string(), r));
        }
        let slider = named(&mut app, "FixtureSlider");
        let slot = parent(&app, slider);
        let r = rect(&app, slot);
        assert!(
            (r.height() - CONTROL_H).abs() < 0.01,
            "{theme}: the slider slot is {} high",
            r.height()
        );
        parts.push(("slider".to_string(), rect(&app, slider)));
        let gate_tag = {
            let world = app.world_mut();
            world
                .query_filtered::<Entity, With<GateTagOf>>()
                .single(world)
                .expect("the tagged verb shows one tag")
        };
        parts.push(("gate tag".to_string(), rect(&app, gate_tag)));
        rects.push(parts);
    }
    assert_eq!(
        rects[0], rects[1],
        "phosphor and hardware lay the same block out differently"
    );
}

/// The amount row's field, "/ max", slider and Max sit on one centre line.
#[test]
fn amount_row_parts_share_one_centre_line() {
    for theme in [PHOSPHOR_THEME_ID, HARDWARE_THEME_ID] {
        let mut app = laid_out_app(theme);
        app.world_mut()
            .spawn(amount_row(AmountSpec { name: "Line" }));
        settle(&mut app);
        let row_centre = {
            let world = app.world_mut();
            let row = world
                .query_filtered::<Entity, With<Amount>>()
                .single(world)
                .expect("one row");
            rect(&app, row).center().y
        };
        for name in ["LineField", "LineTop", "LineSlider", "LineMax"] {
            let entity = named(&mut app, name);
            let centre = rect(&app, entity).center().y;
            assert!(
                (centre - row_centre).abs() <= 0.5,
                "{theme}: {name} centres at {centre}, the row at {row_centre}"
            );
        }
    }
}

/// A headless amount row at `value` of `min..=max`, with a counter of the
/// `AmountChanged` events it fired and the value of the last one.
fn amount_fixture(value: u32, min: u32, max: u32) -> (App, Entity, Arc<AtomicU32>, Arc<AtomicU32>) {
    let mut app = themed_app(PHOSPHOR_THEME_ID);
    let fired = Arc::new(AtomicU32::new(0));
    let last = Arc::new(AtomicU32::new(0));
    let (f, l) = (fired.clone(), last.clone());
    let row = app
        .world_mut()
        .spawn(amount_row(AmountSpec { name: "Test" }))
        .observe(move |changed: On<AmountChanged>| {
            f.fetch_add(1, Ordering::SeqCst);
            l.store(changed.value, Ordering::SeqCst);
        })
        .id();
    app.update();
    *app.world_mut().get_mut::<Amount>(row).expect("a row") = Amount { value, min, max };
    app.update();
    app.update();
    (app, row, fired, last)
}

fn part<T: Component>(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<T>>()
        .single(world)
        .expect("one part")
}

fn field_text(app: &mut App) -> String {
    let field = part::<AmountField>(app);
    app.world().get::<TextFieldValue>(field).unwrap().0.clone()
}

fn top_text(app: &mut App) -> String {
    let top = part::<AmountTop>(app);
    app.world().get::<Text>(top).unwrap().0.clone()
}

/// Writing the amount moves every part and fires nothing; Max moves it to its
/// top and fires once.
#[test]
fn max_moves_the_amount_to_its_top() {
    let (mut app, row, fired, last) = amount_fixture(2, 1, 7);
    assert_eq!(field_text(&mut app), "2");
    assert_eq!(top_text(&mut app), "/ 7");
    let slider = part::<AmountSlider>(&mut app);
    assert_eq!(
        app.world()
            .get::<bevy::ui_widgets::SliderValue>(slider)
            .unwrap()
            .0,
        2.0
    );
    assert_eq!(
        fired.load(Ordering::SeqCst),
        0,
        "a caller's write fires nothing"
    );

    let max = part::<AmountMax>(&mut app);
    app.world_mut().trigger(Activate { entity: max });
    app.update();
    app.update();
    assert_eq!(app.world().get::<Amount>(row).unwrap().value, 7);
    assert_eq!(field_text(&mut app), "7");
    assert_eq!(fired.load(Ordering::SeqCst), 1, "one move, one event");
    assert_eq!(last.load(Ordering::SeqCst), 7);

    app.world_mut().trigger(Activate { entity: max });
    app.update();
    assert_eq!(
        fired.load(Ordering::SeqCst),
        1,
        "Max at the top moves nothing"
    );
}

/// A typed amount past the range lands on its edge, the field is rewritten to
/// it, and the row fires once.
#[test]
fn the_field_clamps_into_the_range() {
    let (mut app, row, fired, last) = amount_fixture(2, 1, 7);
    let field = part::<AmountField>(&mut app);
    app.world_mut().get_mut::<TextFieldValue>(field).unwrap().0 = "999".to_string();
    app.update();
    app.update();
    assert_eq!(app.world().get::<Amount>(row).unwrap().value, 7);
    assert_eq!(
        field_text(&mut app),
        "7",
        "the field shows the clamped amount"
    );
    assert_eq!(fired.load(Ordering::SeqCst), 1);
    assert_eq!(last.load(Ordering::SeqCst), 7);

    app.world_mut().get_mut::<TextFieldValue>(field).unwrap().0 = "0".to_string();
    app.update();
    app.update();
    assert_eq!(app.world().get::<Amount>(row).unwrap().value, 1);
    assert_eq!(fired.load(Ordering::SeqCst), 2);
}

/// Filling buttons that share a row split it evenly and stay inside it: the
/// Settings tab bar and its UI scale row overflowed their panel when a filled
/// button would not shrink.
#[test]
fn filled_buttons_share_their_row_inside_it() {
    let mut app = laid_out_app(PHOSPHOR_THEME_ID);
    let row = app
        .world_mut()
        .spawn(Node {
            width: px(400),
            column_gap: px(4),
            ..default()
        })
        .id();
    let buttons: Vec<Entity> = ["Audio", "Graphics", "Controls", "Interface"]
        .into_iter()
        .map(|label| {
            app.world_mut()
                .spawn((button(ButtonSpec::control(label).fill()), ChildOf(row)))
                .id()
        })
        .collect();
    settle(&mut app);
    let bar = rect(&app, row);
    let first = rect(&app, buttons[0]).width();
    for &entity in &buttons {
        let button = rect(&app, entity);
        assert!(
            button.max.x <= bar.max.x + 0.5,
            "a button ends at {} past the row's {}",
            button.max.x,
            bar.max.x
        );
        assert!(
            (button.width() - first).abs() < 0.5,
            "the row splits evenly"
        );
    }
}

/// A hidden verb takes no space: the verb after it moves into its place.
#[test]
fn a_hidden_verb_takes_no_space() {
    let mut app = laid_out_app(PHOSPHOR_THEME_ID);
    let row = app.world_mut().spawn(verb_row(RowAlign::Start)).id();
    let hidden = app
        .world_mut()
        .spawn((
            button(ButtonSpec::control("Salvage")),
            VerbGate::Ready,
            ChildOf(row),
        ))
        .id();
    let next = app
        .world_mut()
        .spawn((button(ButtonSpec::control("Repair")), ChildOf(row)))
        .id();
    settle(&mut app);
    let row_left = rect(&app, row).min.x;
    assert!(rect(&app, next).min.x > row_left, "Salvage stands first");

    *app.world_mut().get_mut::<VerbGate>(hidden).unwrap() = VerbGate::Hidden;
    settle(&mut app);
    assert_eq!(
        app.world().get::<ComputedNode>(hidden).unwrap().size(),
        Vec2::ZERO
    );
    assert!(
        (rect(&app, next).min.x - row_left).abs() < 0.01,
        "Repair took the hidden verb's place"
    );
}

/// A tagged verb is disabled and shows its word as a tag before it, with no
/// tooltip; back to Ready, the tag goes and the verb works again.
#[test]
fn a_tagged_verb_is_disabled_and_shows_its_word() {
    let mut app = themed_app(PHOSPHOR_THEME_ID);
    let row = app.world_mut().spawn(verb_row(RowAlign::End)).id();
    let verb = app
        .world_mut()
        .spawn((
            button(ButtonSpec::control("Delete")),
            VerbGate::Tagged("PORT"),
            ChildOf(row),
        ))
        .id();
    app.update();
    app.update();
    assert!(app.world().entity(verb).contains::<InteractionDisabled>());
    assert!(!app.world().entity(verb).contains::<Tooltip>());
    let children: Vec<Entity> = app.world().get::<Children>(row).unwrap().to_vec();
    assert_eq!(children.len(), 2, "the tag and the verb");
    assert_eq!(children[1], verb, "the tag stands before the verb");
    let words: Vec<String> = {
        let world = app.world_mut();
        let mut q = world.query::<&Text>();
        world
            .get::<Children>(children[0])
            .unwrap()
            .iter()
            .filter_map(|c| q.get(world, c).ok().map(|t| t.0.clone()))
            .collect()
    };
    assert_eq!(words, vec!["PORT".to_string()]);

    *app.world_mut().get_mut::<VerbGate>(verb).unwrap() = VerbGate::Ready;
    app.update();
    app.update();
    assert!(!app.world().entity(verb).contains::<InteractionDisabled>());
    assert_eq!(app.world().get::<Children>(row).unwrap().len(), 1);
}
