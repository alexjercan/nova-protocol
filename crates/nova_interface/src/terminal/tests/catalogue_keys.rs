//! TAB and Esc in the Ship pane at a refit dock, where TAB is the
//! catalogue's key: it opens and closes the catalogue over the pane, puts a
//! part in hand away on the way in, and returns to the page the last visit
//! left. Esc steps back one layer per press.

use nova_ship::{
    gallery::gallery_keyboard,
    prelude::{
        BaseSectionConfig, GalleryCategory, GalleryLayout, GalleryState, GameSections, HeldPart,
        HullSectionConfig, SectionConfig, SectionKind, TurretSectionConfig,
    },
};
use nova_ui::prelude::InputMode;

use super::*;
use crate::{pane::InterfacePaneType, ship::ShipRuntime};

/// Thirty guns and a hull: three pages of Weapons at the default grid.
fn catalogue_sections() -> GameSections {
    let gun = |index: usize| SectionConfig {
        base: BaseSectionConfig {
            id: format!("gun_{index}"),
            name: format!("Gun {index}"),
            ..default()
        },
        kind: SectionKind::Turret(TurretSectionConfig::default()),
    };
    let hull = SectionConfig {
        base: BaseSectionConfig {
            id: "hull".to_string(),
            name: "Hull".to_string(),
            ..default()
        },
        kind: SectionKind::Hull(HullSectionConfig::default()),
    };
    GameSections((0..30).map(gun).chain([hull]).collect())
}

/// The gallery claims Browse while it is open, as `claim_gallery_keyboard`
/// does under a host, resolved before `Update` as the arbiter resolves it.
fn browse_while_open(gallery: Res<GalleryState>, mut mode: ResMut<InputMode>) {
    let wanted = if gallery.open {
        InputMode::Browse
    } else {
        InputMode::Normal
    };
    mode.set_if_neq(wanted);
}

/// The toggle rig with the back-out owner, the gallery keyboard and the
/// keyboard mode, in the order the plugins set: the back-out owner
/// (`CommandsSystems::Toggle`), then the toggle (`InterfaceSystems`), then
/// the gallery (`PartsGallerySystems`). The interface is open on `pane`, and
/// `refits` says whether the Ship pane shows the Catalogue button.
fn catalogue_rig(pane: InterfacePaneType, refits: bool) -> App {
    let mut app = toggle_app();
    app.add_message::<KeyboardInput>();
    app.init_resource::<InputMode>();
    app.init_resource::<GalleryState>();
    app.init_resource::<GalleryLayout>();
    app.init_resource::<HeldPart>();
    app.insert_resource(catalogue_sections());
    app.insert_resource(pane);
    app.add_systems(PreUpdate, browse_while_open);
    // `toggle_app` already runs the toggle; these stand either side of it.
    app.add_systems(
        Update,
        (
            close_surface_from_menu_keys.before(toggle_interface),
            gallery_keyboard.after(toggle_interface),
        ),
    );
    press_tab(&mut app);
    assert_eq!(pause_state(&app), PauseStates::Interface);
    app.insert_resource(ShipRuntime {
        refits,
        ..default()
    });
    app
}

/// One key press, released before the next frame.
fn tap(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(key);
    keys.clear();
    app.update();
}

fn gallery(app: &App) -> &GalleryState {
    app.world().resource::<GalleryState>()
}

fn held(app: &App) -> Option<&str> {
    app.world().resource::<HeldPart>().id()
}

#[test]
fn tab_opens_the_catalogue_in_the_ship_pane_at_a_refit_dock() {
    let mut app = catalogue_rig(InterfacePaneType::Ship, true);

    press_tab(&mut app);

    assert!(gallery(&app).open, "TAB opens the catalogue");
    assert_eq!(
        pause_state(&app),
        PauseStates::Interface,
        "the press that opens the catalogue leaves the interface open"
    );
}

#[test]
fn tab_closes_the_catalogue_back_to_the_ship_view_with_the_interface_open() {
    let mut app = catalogue_rig(InterfacePaneType::Ship, true);
    press_tab(&mut app);
    assert!(gallery(&app).open);

    press_tab(&mut app);

    assert!(!gallery(&app).open, "TAB closes the catalogue it opened");
    assert_eq!(
        (
            pause_state(&app),
            *app.world().resource::<InterfacePaneType>()
        ),
        (PauseStates::Interface, InterfacePaneType::Ship),
        "and the Ship pane is still up"
    );
}

#[test]
fn tab_with_a_part_in_hand_puts_it_away_and_opens_the_catalogue() {
    let mut app = catalogue_rig(InterfacePaneType::Ship, true);
    app.insert_resource(HeldPart(Some("gun_0".to_string())));

    press_tab(&mut app);

    assert_eq!(held(&app), None, "the part goes back");
    assert!(gallery(&app).open, "and the catalogue opens");
    assert_eq!(pause_state(&app), PauseStates::Interface);
}

/// Away from the Ship pane, and in the Ship pane with no Catalogue button,
/// TAB is the interface's key and closes it, catalogue untouched.
#[test]
fn tab_closes_the_interface_in_another_pane_and_in_the_ship_pane_without_a_refit() {
    for (pane, refits) in [
        (InterfacePaneType::Map, true),
        (InterfacePaneType::Inventory, true),
        (InterfacePaneType::Ship, false),
    ] {
        let mut app = catalogue_rig(pane, refits);
        if pane != InterfacePaneType::Ship {
            // The cached flag follows the Ship pane; a stale one must not
            // keep TAB on the catalogue in another pane.
            app.world_mut().resource_mut::<ShipRuntime>().refits = false;
        }

        press_tab(&mut app);

        assert_eq!(
            pause_state(&app),
            PauseStates::Unpaused,
            "TAB closes the interface in {pane:?} (refits {refits})"
        );
        assert!(!gallery(&app).open, "and opens no catalogue in {pane:?}");
    }
}

/// Every layer the Ship pane can stack, from the innermost out: the filter
/// caret, the focus card, the catalogue, the part in hand, the interface.
/// Each Esc takes exactly one.
#[test]
fn escape_steps_back_one_layer_per_press_from_the_filter_caret_to_the_interface() {
    let mut app = catalogue_rig(InterfacePaneType::Ship, true);
    app.insert_resource(HeldPart(Some("gun_0".to_string())));
    *app.world_mut().resource_mut::<GalleryState>() = GalleryState {
        open: true,
        filter_focused: true,
        focused: true,
        ..default()
    };
    app.update();

    let layers = |app: &App| {
        let state = gallery(app);
        (
            state.filter_focused,
            state.focused,
            state.open,
            held(app).is_some(),
            pause_state(app) == PauseStates::Interface,
        )
    };
    for (press, expected) in [
        ("the caret", (false, true, true, true, true)),
        ("the focus card", (false, false, true, true, true)),
        ("the catalogue", (false, false, false, true, true)),
        ("the part in hand", (false, false, false, false, true)),
        ("the interface", (false, false, false, false, false)),
    ] {
        tap(&mut app, KeyCode::Escape);
        assert_eq!(layers(&app), expected, "one Esc leaves {press} alone");
    }
}

/// Place closes the catalogue with the part in hand; TAB puts the part away
/// and returns to the same category, filter and page, on the grid.
#[test]
fn reopening_the_catalogue_keeps_its_category_filter_and_page() {
    let mut app = catalogue_rig(InterfacePaneType::Ship, true);
    press_tab(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<GalleryState>();
        state.category = GalleryCategory::Weapons;
        state.filter = "gun".to_string();
        state.selected = 14;
        state.focused = true;
    }
    tap(&mut app, KeyCode::Enter);
    assert_eq!(held(&app), Some("gun_14"), "Place puts the part in hand");
    assert!(!gallery(&app).open, "and hides the catalogue");

    press_tab(&mut app);

    let state = gallery(&app);
    assert!(state.open, "TAB returns to the catalogue");
    assert_eq!(
        (state.category, state.filter.as_str(), state.selected),
        (GalleryCategory::Weapons, "gun", 14),
        "on the category, filter and page it left"
    );
    assert!(!state.focused, "on the grid");
    assert_eq!(held(&app), None, "with the part put away");
}
