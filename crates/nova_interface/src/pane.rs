//! The TAB interface: one themed full-screen surface over the frozen world,
//! with a Map pane, a Ship pane, an Inventory pane, a Jobs pane and, docked,
//! a Station pane. The Inventory pane's Craft button opens the Craft view
//! over it (`crate::craft`).
//!
//! TAB opens it from flight (`Unpaused -> Interface`) and closes it again.
//! Keyboard M and gamepad Y step Map -> Ship -> Inventory -> Jobs -> Station
//! -> Map, Station only while docked, and the buttons in the card's title
//! row pick a pane. The first open shows Map; after that TAB reopens the
//! last pane. The `;` command modal opens over the pane without tearing it
//! down, so closing the modal returns to the same pane with its scene, camera
//! and selection.
//!
//! Escape is not read here: the one back-out owner is
//! [`close_surface_from_menu_keys`](crate::terminal), so a single press never
//! closes both the modal and the pane.
//!
//! Touch this module when changing how the interface opens, which pane shows,
//! or the layout around the panes.

use bevy::{prelude::*, ui_widgets::Activate, window::PrimaryWindow};
use nova_gameplay::{
    prelude::{
        cr_digits, play_menu_select, play_time_text, Chart, Clocks, DerelictShipMarker,
        FreezeOwner, GameItems, ItemDesignId, PlayerSpaceshipMarker, ShipCredits,
        SpaceshipRootMarker, StationMarker, StationServiceType, StationServices, WorldClock,
    },
    GameStates, PauseStates,
};
use nova_input::prelude::{
    ActionContext, ActiveContexts, InputBindings, InputDeviceType, InputSources,
};
use nova_ship::prelude::{DockedShip, DockingConnection, GalleryAction, GalleryState, HeldPart};
use nova_sound::prelude::{PlaySound, SoundBank, SoundCategoryType, UiSfx};
use nova_ui::{prelude::*, theme::UiColorType};

use crate::{
    icons::{icon_node, InterfaceIcons, SectionIconType},
    inventory::{category_color, inventory_body},
    jobs::jobs_body,
    map::{
        map_chart_button, on_map_chart_button, on_map_reframe_button, spawn_map_panel,
        MapLegendMarker, MapRuntime, MapViewportMarker,
    },
    ship::{
        on_ship_fit_button, on_ship_reset_button, on_ship_viewport_click, on_ship_viewport_enter,
        on_ship_viewport_leave, on_ship_viewport_move, spawn_ship_panel,
        spawn_ship_rebind_click_guard, spawn_ship_rebind_prompt, ShipCatalogueButton, ShipRuntime,
        ShipSplitMarker, ShipViewportMarker,
    },
    station::{station_body, StationDock},
    terminal::NovaOsCloseTransition,
};

/// Which pane the TAB interface shows.
///
/// A resource, not a state: the pane is remembered while the interface is
/// closed, and it must survive the `;` modal opening over it.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum InterfacePaneType {
    /// The local-space map: contacts, range and bearing, and GOTO.
    #[default]
    Map,
    /// The player ship: sections, repair, rebind and mates.
    Ship,
    /// What the player ship carries, and what a docked ship carries.
    Inventory,
    /// The jobs the player holds, always, and the docked station's board.
    Jobs,
    /// What the docked station sells: ships, escorts and the station itself.
    /// Its tab shows only while the player ship is docked at a living station.
    Station,
}

impl InterfacePaneType {
    /// Every pane, in title-row order, with its button label.
    const ALL: [(Self, &'static str); 5] = [
        (Self::Map, "Map"),
        (Self::Ship, "Ship"),
        (Self::Inventory, "Inventory"),
        (Self::Jobs, "Jobs"),
        (Self::Station, "Station"),
    ];

    /// The per-pane action context id its verbs are bound under.
    pub(crate) fn context_id(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Ship => "ship",
            Self::Inventory => "inventory",
            Self::Jobs => "jobs",
            Self::Station => "station",
        }
    }

    /// The pane `interface_next_tab` steps to: Map -> Ship -> Inventory ->
    /// Jobs -> Station -> Map, skipping Station while `docked` is false.
    fn next(self, docked: bool) -> Self {
        match self {
            Self::Map => Self::Ship,
            Self::Ship => Self::Inventory,
            Self::Inventory => Self::Jobs,
            Self::Jobs if docked => Self::Station,
            Self::Jobs | Self::Station => Self::Map,
        }
    }

    /// The pane's title and button label.
    fn label(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(pane, _)| *pane == self)
            .map(|(_, label)| *label)
            .expect("InterfacePaneType::ALL lists every pane")
    }
}

/// The full-screen interface root. [`rebuild_interface_body`] builds the card
/// under it once and replaces only the pane body on a switch.
#[derive(Component)]
pub(crate) struct InterfaceRootMarker;

/// `interface_toggle` opens the interface from flight and closes it again.
///
/// Opening needs a ship: `Playing` also covers the editor's build mode, and an
/// interface over a scene with no ship would have nothing to show. Closing is
/// ungated by the ship, but an armed section rebind owns the next key, so it
/// refuses the toggle rather than lose the capture. Under the command modal the
/// desk key is the prompt's completion, so `Commands` is left alone.
pub(crate) fn toggle_interface(
    sources: InputSources,
    bindings: Res<InputBindings>,
    player: Query<(), With<PlayerSpaceshipMarker>>,
    current: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
    ship: Option<Res<ShipRuntime>>,
) {
    let Some(action) = bindings.get("interface_toggle") else {
        return;
    };
    if !sources.just_pressed(action) {
        return;
    }
    toggle_interface_state(
        *current.get(),
        !player.is_empty(),
        ship.is_some_and(|ship| ship.rebind_armed()),
        &mut next,
    );
}

/// The one open-or-close step the toggle key and the card's Close button
/// share: flight opens the interface when a player ship exists, the interface
/// closes to flight, and an armed section rebind refuses both so the capture
/// is not lost.
fn toggle_interface_state(
    current: PauseStates,
    player_present: bool,
    rebind_armed: bool,
    next: &mut NextState<PauseStates>,
) {
    if rebind_armed {
        return;
    }
    match current {
        PauseStates::Unpaused if player_present => next.set(PauseStates::Interface),
        PauseStates::Interface => next.set(PauseStates::Unpaused),
        PauseStates::Unpaused
        | PauseStates::Paused
        | PauseStates::Commands
        | PauseStates::Debug => {}
    }
}

/// Play the pause overlay's toggle blip as the interface opens from flight or
/// closes back to it.
///
/// Registered on those two transitions, not fired by the toggles: TAB and the
/// Escape back-out can both close the interface in one frame, and one
/// transition plays one cue. The command modal's transitions keep its own
/// power sweeps. The bank is absent on headless rigs, where the cue is a no-op.
pub(crate) fn play_interface_toggle(mut commands: Commands, bank: Option<Res<SoundBank<UiSfx>>>) {
    let Some(bank) = bank else {
        return;
    };
    commands.trigger(PlaySound::ui(
        bank.get(UiSfx::UiToggle),
        SoundCategoryType::UiClick,
    ));
}

/// `interface_next_tab` steps to the next pane while the interface owns the
/// screen. An armed section rebind takes the key instead: leaving the Ship pane
/// would drop the capture. The plugin runs it in [`InputMode::Normal`] only, so
/// a focused text field, such as the Inventory quantity, types the key instead.
///
/// [`InputMode::Normal`]: nova_ui::prelude::InputMode::Normal
pub(crate) fn next_interface_pane(
    sources: InputSources,
    bindings: Res<InputBindings>,
    current: Res<State<PauseStates>>,
    mut pane: ResMut<InterfacePaneType>,
    ship: Option<Res<ShipRuntime>>,
    dock: StationDock,
) {
    if *current.get() != PauseStates::Interface {
        return;
    }
    if ship.is_some_and(|ship| ship.rebind_armed()) {
        return;
    }
    let Some(action) = bindings.get("interface_next_tab") else {
        return;
    };
    if !sources.just_pressed(action) {
        return;
    }
    *pane = pane.next(dock.docked());
}

/// Leave the Station pane for the Ship pane while the player ship is not
/// docked at a living station. Runs before the body is built, so the card
/// never opens on a Station pane it cannot fill.
pub(crate) fn leave_station_pane(dock: StationDock, mut pane: ResMut<InterfacePaneType>) {
    if *pane == InterfacePaneType::Station && !dock.docked() {
        *pane = InterfacePaneType::Ship;
    }
}

/// Show the Station tab only while the player ship is docked at a living
/// station. Runs after the card is built, so a new card's tab is right on its
/// first frame.
pub(crate) fn show_station_tab(
    dock: StationDock,
    mut q_tab: Query<(&ButtonValue<InterfacePaneType>, &mut Node)>,
) {
    let display = if dock.docked() {
        Display::Flex
    } else {
        Display::None
    };
    for (value, mut node) in &mut q_tab {
        if value.0 == InterfacePaneType::Station && node.display != display {
            node.display = display;
        }
    }
}

/// Whether the interface is on screen: open, or covered by the command modal
/// that will return to it.
pub(crate) fn interface_shown(pause: &State<PauseStates>, close: &NovaOsCloseTransition) -> bool {
    match pause.get() {
        PauseStates::Interface => true,
        PauseStates::Commands => close.return_to == PauseStates::Interface,
        PauseStates::Unpaused | PauseStates::Paused | PauseStates::Debug => false,
    }
}

/// Spawn the interface root when the interface comes on screen, and despawn it
/// when it goes. The root outlives the command modal over it, so the pane's
/// scene is not rebuilt on the way back.
pub(crate) fn spawn_interface_root(
    mut commands: Commands,
    pause: Res<State<PauseStates>>,
    close: Res<NovaOsCloseTransition>,
    q_root: Query<Entity, With<InterfaceRootMarker>>,
) {
    let shown = interface_shown(&pause, &close);
    let roots: Vec<Entity> = q_root.iter().collect();
    assert!(
        roots.len() <= 1,
        "the TAB interface has {} roots; spawn_interface_root owns the only one",
        roots.len()
    );
    match (shown, roots.first()) {
        (true, None) => {
            commands
                .spawn((
                    InterfaceRootMarker,
                    Name::new("InterfaceRoot"),
                    Node {
                        position_type: PositionType::Absolute,
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(px(16)),
                        row_gap: px(12),
                        ..default()
                    },
                    // Above the flight HUD, below the command modal. No fill: the
                    // card is the surface, and the frozen world shows around it.
                    GlobalZIndex(MENU_PANEL_Z),
                ))
                .with_children(spawn_ship_rebind_click_guard);
        }
        (false, Some(&root)) => commands.entity(root).despawn(),
        _ => {}
    }
}

#[cfg(test)]
mod tests;

/// Widest the interface grows, in logical px.
const INTERFACE_MAX_PX: f32 = 1520.0;

/// The card's title row: the pane title, the pane tabs, the mode tags, the
/// player balance, the gain it counts up to, the play clock, the next-pane
/// hint and Close. Everything right of the mode tags sits in a fixed slot, so
/// the tabs never move; a narrow card takes its room from the rule, then from
/// the mode tags.
#[derive(Component)]
pub(crate) struct InterfacePaneHead;

/// The node the shown pane's body is built in. Only its children are replaced
/// on a pane switch.
#[derive(Component)]
pub(crate) struct InterfacePaneBody;

/// The play clock at the end of the title row, `h:mm:ss` of armed play.
/// Empty until the world arms, which is when the [`Chart`] first holds the
/// start cell; a scenario with no streamed world never shows it.
#[derive(Component)]
pub(crate) struct InterfacePaneClock;

/// The Close button at the right end of the title row.
#[derive(Component)]
pub(crate) struct InterfaceCloseButton;

/// The title row's next-pane hint: a key chip and the word `Next`. Hidden
/// when the card is narrower than [`PANE_FOOTER_SPLIT_PX`].
#[derive(Component)]
pub(crate) struct InterfaceNextHint;

/// The key chip of [`InterfaceNextHint`], whose text follows the live
/// `interface_next_tab` binding.
#[derive(Component)]
pub(crate) struct InterfaceNextKey;

/// Width of the Close button's slot in the title row, in logical px: room for
/// the word, a key chip of a short key name and the button's padding.
const CLOSE_SLOT_PX: f32 = 120.0;

/// Width of the pane title's slot in the title row, in logical px: room for
/// `INVENTORY`, the longest pane title, so the tabs keep their x on a switch.
const TITLE_SLOT_PX: f32 = 80.0;

/// Width of the balance slot in the title row, in logical px: the credit mask
/// and eleven digits with their separators.
const BALANCE_SLOT_PX: f32 = 120.0;

/// Width of the gain slot in the title row, in logical px: `+999,999` in the
/// UI font. A wider gain overhangs the slot's right side, toward the clock;
/// it never moves the balance. Every px here comes off the mode tags of a
/// 1024 px window.
const GAIN_SLOT_PX: f32 = 52.0;

/// Width of the play clock slot in the title row, in logical px: `hhh:mm:ss`.
const CLOCK_SLOT_PX: f32 = 64.0;

/// A fixed-width title-row slot that holds its content at its right edge.
fn fixed_slot(width: f32) -> Node {
    Node {
        width: px(width),
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::FlexEnd,
        ..default()
    }
}

/// A [`fixed_slot`] that holds its content at its left edge, against the
/// slot before it.
fn fixed_slot_leading(width: f32) -> Node {
    Node {
        justify_content: JustifyContent::FlexStart,
        ..fixed_slot(width)
    }
}

/// The title row's mode tags: the docked station's name and the services it
/// offers, or nothing in flight. Holds the words it shows, so the tags respawn
/// only when they change.
#[derive(Component, Default)]
pub(crate) struct InterfaceModeTags(Vec<String>);

/// The `Name` of the title row's mode tag slot, for tests and captures.
pub const INTERFACE_MODE_TAGS: &str = "InterfaceModeTags";

/// The services a mode tag names, in tag order, and the word each shows.
/// Rebuild is a refit service to the player, so both read `REFIT`.
const MODE_TAG_SERVICES: [(StationServiceType, &str); 5] = [
    (StationServiceType::Refit, "REFIT"),
    (StationServiceType::Rebuild, "REFIT"),
    (StationServiceType::Craft, "CRAFT"),
    (StationServiceType::Trade, "TRADE"),
    (StationServiceType::Jobs, "JOBS"),
];

/// The words of the mode tags for a station named `name` that offers
/// `services`: its name, then one word per service the head names, once each.
pub(crate) fn mode_tag_words(name: &str, services: &StationServices) -> Vec<String> {
    let mut words = vec![name.to_uppercase()];
    for (service, word) in MODE_TAG_SERVICES {
        if services.offers(service) && !words.iter().any(|shown| shown == word) {
            words.push(word.to_string());
        }
    }
    words
}

/// Write the docked station's name and services into the title row's mode
/// tags, and clear them while the player ship is not docked at a living
/// station. Runs with the pane shut too, as [`show_pane_clock`] does.
pub(crate) fn show_pane_mode_tags(
    mut commands: Commands,
    q_player: Query<
        (Entity, &DockedShip),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_connection: Query<&DockingConnection>,
    q_station: Query<
        (Option<&Name>, Option<&StationServices>),
        (With<StationMarker>, Without<DerelictShipMarker>),
    >,
    mut q_tags: Query<(Entity, &mut InterfaceModeTags)>,
) {
    let Ok((slot, mut shown)) = q_tags.single_mut() else {
        return;
    };
    let words = q_player
        .single()
        .ok()
        .and_then(|(player, docked)| {
            let connection = q_connection.get(docked.connection).ok()?;
            let station = if connection.first_ship == player {
                connection.second_ship
            } else {
                connection.first_ship
            };
            let (name, services) = q_station.get(station).ok()?;
            Some(mode_tag_words(
                name.map_or("Station", Name::as_str),
                services.unwrap_or(&StationServices::default()),
            ))
        })
        .unwrap_or_default();
    if shown.0 == words {
        return;
    }
    commands.entity(slot).despawn_children();
    commands.entity(slot).with_children(|slot| {
        for word in &words {
            slot.spawn(panel_head_tag(word));
        }
    });
    shown.0 = words;
}

/// Close the interface from its Close button, by the path the toggle key
/// takes.
fn on_interface_close_click(
    _activate: On<Activate>,
    player: Query<(), With<PlayerSpaceshipMarker>>,
    current: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
    ship: Option<Res<ShipRuntime>>,
) {
    toggle_interface_state(
        *current.get(),
        !player.is_empty(),
        ship.is_some_and(|ship| ship.rebind_armed()),
        &mut next,
    );
}

/// Write the live key names into the title row's chips: the Close chip reads
/// `interface_toggle`, the Next chip `interface_next_tab`, both for the device
/// the player pressed last. Repaints when the bindings or the device change
/// and on a new card.
///
/// # Panics
///
/// When either action is not registered in [`InputBindings`].
pub(crate) fn refresh_card_head_keys(
    bindings: Res<InputBindings>,
    device: Res<InputDeviceType>,
    q_new: Query<(), Or<(Added<InterfaceCloseButton>, Added<InterfaceNextKey>)>>,
    q_close: Query<Entity, With<InterfaceCloseButton>>,
    q_next: Query<Entity, With<InterfaceNextKey>>,
    q_children: Query<&Children>,
    mut q_label: Query<&mut Text, With<KeyChipLabel>>,
) {
    if !bindings.is_changed() && !device.is_changed() && q_new.is_empty() {
        return;
    }
    let shown = |action: &str| {
        bindings
            .get(action)
            .unwrap_or_else(|| panic!("the card head names `{action}`, which is not registered"))
            .hint_display(*device)
    };
    for (chips, action) in [
        (q_close.iter().collect::<Vec<_>>(), "interface_toggle"),
        (q_next.iter().collect::<Vec<_>>(), "interface_next_tab"),
    ] {
        let key = shown(action);
        for chip in chips {
            for node in q_children.iter_descendants(chip) {
                if let Ok(mut text) = q_label.get_mut(node) {
                    if text.0 != key {
                        text.0.clone_from(&key);
                    }
                }
            }
        }
    }
}

/// Hide the next-pane hint when the card is narrower than
/// [`PANE_FOOTER_SPLIT_PX`] logical px, so the tabs and the Close button keep
/// their room.
pub(crate) fn fit_card_head(
    q_window: Query<&Window, With<PrimaryWindow>>,
    scale: Res<UiScale>,
    mut q_hint: Query<&mut Node, With<InterfaceNextHint>>,
) {
    let Ok(window) = q_window.single() else {
        return;
    };
    let display = if ui_width(window, &scale) < PANE_FOOTER_SPLIT_PX {
        Display::None
    } else {
        Display::Flex
    };
    for mut node in &mut q_hint {
        if node.display != display {
            node.display = display;
        }
    }
}

/// The width the UI lays out in, in logical px: the window's logical width
/// over the [`UiScale`]. `Window::width` is before the scale, so at 150% on a
/// 1280 px window the UI has 853 px. Every layout breakpoint reads this.
pub(crate) fn ui_width(window: &Window, scale: &UiScale) -> f32 {
    window.width() / scale.0
}

/// The `Name` of the title row's play clock, for tests and captures.
pub const INTERFACE_PANE_CLOCK: &str = "InterfacePaneClock";

/// Write the play clock into the title row while the world is armed, and
/// empty it otherwise; its slot keeps its width either way. Runs with the pane
/// shut too, at the cost of one text compare per frame, so the clock is right
/// on the frame the card opens.
pub(crate) fn show_pane_clock(
    clock: Option<Res<WorldClock>>,
    chart: Option<Res<Chart>>,
    mut q_clock: Query<&mut Text, With<InterfacePaneClock>>,
) {
    let Ok(mut text) = q_clock.single_mut() else {
        return;
    };
    let armed = chart.is_some_and(|chart| !chart.is_empty());
    let shown = clock
        .filter(|_| armed)
        .map(|clock| play_time_text(clock.play_seconds))
        .unwrap_or_default();
    if text.0 != shown {
        text.0 = shown;
    }
}

/// The `Name` of a credit figure's digits, for tests and captures.
pub const CREDIT_FIGURE: &str = "CreditFigure";

/// The share of a credit figure's `size` that the mask rises, so its centre
/// meets the digits' centre. Measured on Iosevka Term at 12, 13 and 15 px:
/// the centred mask sat 0 to 2 px under the digits, by the row's rounding,
/// and a larger lift put the 15 px Station price mask 2 px high.
const CREDIT_MASK_LIFT: f32 = 0.10;

/// The credit mask, then `credits` as [`cr_digits`], both at `role`'s size in `tone`.
/// `marker` goes on the digits node for callers that rewrite it in place; a
/// `Name` in it replaces [`CREDIT_FIGURE`]. The figure ignores the pointer, so
/// a row under it keeps its clicks.
pub(crate) fn spawn_credit_figure(
    parent: &mut ChildSpawnerCommands,
    icons: &InterfaceIcons,
    credits: u64,
    role: TextRole,
    tone: UiColorType,
    marker: impl Bundle,
) {
    let size = role.size();
    parent
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: px(size * 0.3),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|figure| {
            figure
                .spawn((
                    Node {
                        top: px(-size * CREDIT_MASK_LIFT),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|lift| {
                    lift.spawn(icon_node(icons.credit(), tone, size));
                });
            figure
                .spawn((
                    text_toned(cr_digits(credits), role, tone),
                    TextLayout::new(Justify::Left, LineBreak::NoWrap),
                    Name::new(CREDIT_FIGURE),
                    Pickable::IGNORE,
                ))
                .insert(marker);
        });
}

/// The digits of the player balance in the title row, left of the play clock.
#[derive(Component)]
pub(crate) struct InterfacePaneBalance;

/// The `Name` of the title row's player balance, for tests and captures.
pub const INTERFACE_PANE_BALANCE: &str = "InterfacePaneBalance";

/// The gain the title row's balance counts up to, `+1,460`, right of the
/// balance. Empty while no count runs; its slot keeps its width either way.
#[derive(Component)]
pub(crate) struct InterfacePaneGain;

/// The `Name` of the title row's gain figure, for tests and captures.
pub const INTERFACE_PANE_GAIN: &str = "InterfacePaneGain";

/// Real seconds the title row's balance takes to count up to a gain.
const BALANCE_COUNT_SECS: f32 = 0.8;

/// Real seconds the gain figure shows, from the start of its count.
const BALANCE_GAIN_SECS: f32 = 2.5;

/// The most real time one frame steps a count by, in seconds. The frame
/// after a hand-in rebuilds the Jobs pane body; a frame as long as the count
/// would otherwise show its end only.
const BALANCE_COUNT_STEP_MAX_SECS: f32 = 0.1;

/// One count of the title row's balance: the figure rises from `from_cr` to
/// `to_cr`, and `gain_cr` shows beside it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct BalanceCount {
    /// The figure the head showed when the count began.
    from_cr: u64,
    /// The true balance the count ends on.
    to_cr: u64,
    /// Real seconds since the count began, stepped by at most
    /// [`BALANCE_COUNT_STEP_MAX_SECS`] a frame.
    age_secs: f32,
}

impl BalanceCount {
    /// The figure to show now: an ease-out from `from_cr` that reaches
    /// `to_cr` at [`BALANCE_COUNT_SECS`] and never exceeds it.
    fn shown_cr(&self) -> u64 {
        let left = 1.0 - (self.age_secs / BALANCE_COUNT_SECS).clamp(0.0, 1.0);
        if left <= 0.0 {
            return self.to_cr;
        }
        let span = (self.to_cr - self.from_cr) as f64;
        let risen = span * (1.0 - f64::from(left).powi(3));
        (self.from_cr + risen as u64).min(self.to_cr)
    }

    /// The gain shown beside the balance: all the head counts up by, so the
    /// two figures agree whatever else paid the player in the frame the count
    /// began.
    fn gain_cr(&self) -> u64 {
        self.to_cr - self.from_cr
    }

    /// Whether the count and the gain's dwell are both over.
    fn is_over(&self) -> bool {
        self.age_secs >= BALANCE_GAIN_SECS
    }
}

/// What the title row's balance slot and gain slot show this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BalanceFigures {
    /// The balance figure.
    balance_cr: u64,
    /// The gain beside it, while a count or its dwell runs.
    gain_cr: Option<u64>,
}

/// The title row's balance count-up: a gain the player was just paid shows
/// as the balance rising to the true figure over [`BALANCE_COUNT_SECS`], with
/// `+<gain>` beside it for [`BALANCE_GAIN_SECS`] in all.
///
/// A caller pays the player ship first and calls [`raise`](Self::raise) in
/// the same frame; [`show_pane_balance`] does the rest. The count runs on
/// real time: the interface holds virtual time. The gain is the true balance
/// when the count starts less the figure the head showed, so it holds every
/// credit that reached the player in that frame (a scenario grant beside a
/// job reward) and always equals what the head counts up by. The head always
/// ends on the true balance:
///
/// - A balance change with no count raised (a purchase, a fee) ends a running
///   count at once and shows the true balance.
/// - A count raised during a count starts a new one from the figure shown;
///   its gain holds what the first count had left.
/// - The count ends with the title row: a closed interface runs none, and a
///   count raised while it is closed shows nothing when it opens.
#[derive(Resource, Default, Debug, PartialEq)]
pub struct BalanceCountUp {
    /// Whether a count was raised since [`show_pane_balance`] last ran.
    raised: bool,
    /// The running count, kept through the gain's dwell.
    count: Option<BalanceCount>,
    /// The balance figure the title row shows, or `None` while it shows none.
    shown_cr: Option<u64>,
}

impl BalanceCountUp {
    /// Count the title row's balance up to the player's balance and show
    /// the rise beside it. Only a job reward raises one: any other credit
    /// change alone shows the true balance at once.
    pub fn raise(&mut self) {
        self.raised = true;
    }

    /// The gain the title row shows beside the balance, while a count or its
    /// dwell runs.
    pub fn gain_cr(&self) -> Option<u64> {
        self.count.map(|count| count.gain_cr())
    }

    /// Whether the balance figure is still short of the true balance.
    pub fn is_counting(&self) -> bool {
        self.count
            .is_some_and(|count| count.shown_cr() < count.to_cr)
    }

    /// Advance one frame of `delta_secs` real seconds against the true
    /// balance `true_cr`, and give what the title row shows.
    fn step(&mut self, true_cr: u64, delta_secs: f32) -> BalanceFigures {
        if core::mem::take(&mut self.raised) {
            // A figure at or above the true balance has nothing to count
            // up to: the gain was spent in the frame that paid it.
            let from_cr = self.shown_cr.unwrap_or(true_cr);
            self.count = (from_cr < true_cr).then_some(BalanceCount {
                from_cr,
                to_cr: true_cr,
                age_secs: 0.0,
            });
        } else if let Some(count) = &mut self.count {
            count.age_secs += delta_secs.min(BALANCE_COUNT_STEP_MAX_SECS);
            if count.to_cr != true_cr || count.is_over() {
                self.count = None;
            }
        }
        let balance_cr = self.count.map_or(true_cr, |count| count.shown_cr());
        self.shown_cr = Some(balance_cr);
        BalanceFigures {
            balance_cr,
            gain_cr: self.gain_cr(),
        }
    }
}

/// Write the player ship's [`ShipCredits`] into the title row, or the figure
/// a [`BalanceCountUp`] count stands at with its gain beside it, and hide the
/// balance figure while there is not exactly one player ship root. Runs with
/// the pane shut too, as [`show_pane_clock`] does.
pub(crate) fn show_pane_balance(
    time: Res<Time<Real>>,
    mut count_up: ResMut<BalanceCountUp>,
    q_player: Query<&ShipCredits, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    mut q_balance: Query<(&mut Text, &ChildOf), With<InterfacePaneBalance>>,
    mut q_gain: Query<&mut Text, (With<InterfacePaneGain>, Without<InterfacePaneBalance>)>,
    mut q_row: Query<&mut Node>,
) {
    let Ok((mut text, child_of)) = q_balance.single_mut() else {
        // The title row is gone: its count ends with it, and a gain raised
        // with the interface shut starts none.
        count_up.set_if_neq(BalanceCountUp::default());
        return;
    };
    let mut row = q_row
        .get_mut(child_of.parent())
        .expect("the balance digits sit in their credit figure row");
    let credits = q_player.single().ok();
    let display = if credits.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    if row.display != display {
        row.display = display;
    }
    let figures = credits.map(|credits| count_up.step(credits.0.into(), time.delta_secs()));
    if figures.is_none() {
        count_up.set_if_neq(BalanceCountUp::default());
    }
    if let Some(figures) = figures {
        let shown = cr_digits(figures.balance_cr);
        if text.0 != shown {
            text.0 = shown;
        }
    }
    let gain = figures
        .and_then(|figures| figures.gain_cr)
        .map(|gain_cr| format!("+{}", cr_digits(gain_cr)))
        .unwrap_or_default();
    let mut gain_text = q_gain
        .single_mut()
        .expect("the title row with a balance has its gain slot");
    if gain_text.0 != gain {
        gain_text.0 = gain;
    }
}

/// Build the card on a fresh root, and on a pane change replace only the
/// body's children.
///
/// The card, its title row and the pane tabs are spawned once per root. A
/// switch rewrites the title text and moves [`Selected`] to the shown pane's
/// tab in place, writing only a difference: a tab click has already moved it
/// through `button_on_setting`, a keyboard or pad switch has not.
pub(crate) fn rebuild_interface_body(
    mut commands: Commands,
    pane: Res<InterfacePaneType>,
    icons: Res<InterfaceIcons>,
    q_root: Query<(Entity, Ref<InterfaceRootMarker>)>,
    q_body: Query<Entity, With<InterfacePaneBody>>,
    q_head: Query<&Children, With<InterfacePaneHead>>,
    mut q_title: Query<&mut Text, With<PanelHeadTitle>>,
    q_tab: Query<(Entity, &ButtonValue<InterfacePaneType>, Has<Selected>)>,
) {
    let Ok((root, marker)) = q_root.single() else {
        return;
    };
    if marker.is_added() {
        spawn_interface_card(&mut commands, root, *pane, &icons);
        return;
    }
    if !pane.is_changed() {
        return;
    }
    let pane = *pane;
    let body = q_body
        .single()
        .expect("an interface root that is not new has its card and pane body");
    commands.entity(body).despawn_children();
    commands
        .entity(body)
        .with_children(|body| pane_body(body, pane, &icons));

    let title = pane.label().to_uppercase();
    for child in q_head
        .single()
        .expect("an interface root that is not new has its title row")
    {
        if let Ok(mut text) = q_title.get_mut(*child) {
            if text.0 != title {
                text.0.clone_from(&title);
            }
        }
    }
    for (tab, value, selected) in &q_tab {
        match (value.0 == pane, selected) {
            (true, false) => {
                commands.entity(tab).insert(Selected);
            }
            (false, true) => {
                commands.entity(tab).remove::<Selected>();
            }
            _ => {}
        }
    }
}

/// The card under `root`: the title row with the pane tabs, and the body of
/// `pane`.
fn spawn_interface_card(
    commands: &mut Commands,
    root: Entity,
    pane: InterfacePaneType,
    icons: &InterfaceIcons,
) {
    commands.entity(root).with_children(|root| {
        root.spawn((
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                width: percent(100),
                max_width: px(INTERFACE_MAX_PX),
                ..panel_node()
            },
            panel(),
        ))
        .with_children(|card| {
            // TODO: bevy_ui 0.19.0 `clip_check_recursive` clips a node only by
            // its direct parent, so a scrolled side panel's nested rows stay
            // pickable past the panel's top edge and take tab clicks. The
            // `ZIndex` stacks the head above the body for picking. Remove it
            // when bevy_ui is at 0.19.1 or later, which walks every ancestor.
            let mut tabs = Entity::PLACEHOLDER;
            card.spawn((
                InterfacePaneHead,
                ZIndex(1),
                panel_head_fixed(pane.label(), TITLE_SLOT_PX, &[]),
            ))
            .with_children(|head| {
                tabs = head
                    .spawn(segmented_container())
                    .with_children(|seg| {
                        for (value, label) in InterfacePaneType::ALL {
                            let mut option = seg.spawn((
                                segmented_option_fit(label),
                                ButtonValue(value),
                                ButtonCueType::Owned,
                                Name::new(format!("InterfaceTab{label}")),
                            ));
                            option.observe(on_interface_tab_click);
                            if value == pane {
                                option.insert(Selected);
                            }
                        }
                    })
                    .id();
                // The one slot that gives way after the rule: it shrinks to
                // zero, and a tag that does not fit wraps to a second line its
                // one-tag height clips, so the last services drop first.
                head.spawn((
                    InterfaceModeTags::default(),
                    Name::new(INTERFACE_MODE_TAGS),
                    Node {
                        height: px(metrics::TAG_H),
                        min_width: px(0),
                        flex_shrink: 1.0,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::FlexEnd,
                        align_content: AlignContent::FlexStart,
                        align_items: AlignItems::Center,
                        column_gap: px(metrics::GAP_S),
                        row_gap: px(metrics::GAP_S),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ));
                // Fixed slots from here to the end: a growing balance or
                // a key name never moves what stands left of it.
                head.spawn(fixed_slot(BALANCE_SLOT_PX))
                    .with_children(|slot| {
                        spawn_credit_figure(
                            slot,
                            icons,
                            0,
                            TextRole::Value,
                            UiColorType::Text,
                            (InterfacePaneBalance, Name::new(INTERFACE_PANE_BALANCE)),
                        );
                    });
                head.spawn(fixed_slot_leading(GAIN_SLOT_PX))
                    .with_children(|slot| {
                        slot.spawn((
                            InterfacePaneGain,
                            Name::new(INTERFACE_PANE_GAIN),
                            text_toned("", TextRole::Value, UiColorType::Nominal),
                            TextLayout::new(Justify::Left, LineBreak::NoWrap),
                            Pickable::IGNORE,
                        ));
                    });
                head.spawn(fixed_slot(CLOCK_SLOT_PX)).with_children(|slot| {
                    slot.spawn((
                        InterfacePaneClock,
                        Name::new(INTERFACE_PANE_CLOCK),
                        text("", TextRole::Label),
                        TextLayout::new(Justify::Right, LineBreak::NoWrap),
                    ));
                });
                head.spawn((
                    InterfaceNextHint,
                    Name::new("InterfaceNextHint"),
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(metrics::GAP_S),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ))
                .with_children(|hint| {
                    hint.spawn((InterfaceNextKey, key_chip("")));
                    hint.spawn((
                        text("Next", TextRole::Label),
                        TextLayout::new(Justify::Left, LineBreak::NoWrap),
                    ));
                });
                head.spawn((Name::new("InterfaceCloseSlot"), fixed_slot(CLOSE_SLOT_PX)))
                    .with_children(|slot| {
                        slot.spawn((
                            InterfaceCloseButton,
                            Name::new("InterfaceClose"),
                            button(ButtonSpec::control("Close").ghost().key("")),
                            // Closing plays the interface toggle's blip.
                            ButtonCueType::Owned,
                        ))
                        .observe(on_interface_close_click);
                    });
            })
            // The tabs stand right of the title, left-anchored; the head's
            // rule then fills the room between them and the mode tags.
            .insert_children(1, &[tabs]);
            card.spawn((
                InterfacePaneBody,
                Node {
                    flex_grow: 1.0,
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    padding: UiRect::all(px(12)),
                    ..default()
                },
            ))
            .with_children(|body| pane_body(body, pane, icons));
        });
    });
}

/// The body of `pane`.
fn pane_body(body: &mut ChildSpawnerCommands, pane: InterfacePaneType, icons: &InterfaceIcons) {
    match pane {
        InterfacePaneType::Map => map_body(body, icons),
        InterfacePaneType::Ship => ship_body(body, icons),
        InterfacePaneType::Inventory => inventory_body(body, icons),
        InterfacePaneType::Jobs => jobs_body(body),
        InterfacePaneType::Station => station_body(body),
    }
}

/// Click once when a tab that is not the shown pane is activated. The
/// shown pane's tab carries [`Selected`] until the switch's commands apply, so
/// a click on it changes nothing and stays silent.
fn on_interface_tab_click(
    activate: On<Activate>,
    q_tab: Query<Has<Selected>, With<ButtonValue<InterfacePaneType>>>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if q_tab.get(activate.entity) == Ok(false) {
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// How a pane action ended: which cue it plays.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActionOutcomeType {
    /// The action went through.
    Done,
    /// The action went through and paid the player a job reward.
    Paid,
    /// The action changed nothing.
    Refused,
}

/// Play the one cue an action outcome gets, if the sound bank has loaded:
/// the interface click for Done, the payout cue for Paid, the deny buzz for
/// Refused. A button whose outcome plays this does not click on press, so one
/// action is one sound.
pub(crate) fn play_outcome_cue(
    commands: &mut Commands,
    bank: Option<&SoundBank<UiSfx>>,
    outcome: ActionOutcomeType,
) {
    let Some(bank) = bank else {
        return;
    };
    let (cue, category) = match outcome {
        ActionOutcomeType::Done => (UiSfx::MenuSelect, SoundCategoryType::UiClick),
        ActionOutcomeType::Paid => (UiSfx::Payout, SoundCategoryType::UiConfirm),
        ActionOutcomeType::Refused => (UiSfx::EditorDeny, SoundCategoryType::UiError),
    };
    commands.trigger(PlaySound::ui(bank.get(cue), category));
}

/// The cue a pane's last action owes. Each pane that reports an action holds
/// one in its runtime resource. The pane shows no result text: the changed
/// state is the result, and the cue is its only other trace.
#[derive(Default, Debug)]
pub struct ActionCue {
    /// Set by [`raise`](Self::raise), taken by [`tick`](Self::tick) once it
    /// plays. It waits for the sound bank, so a result raised before the bank
    /// lands is heard, not lost; it plays once whether or not the pane body
    /// stands.
    pending: Option<ActionOutcomeType>,
}

impl ActionCue {
    /// The cue not yet played, if any.
    pub fn pending(&self) -> Option<ActionOutcomeType> {
        self.pending
    }

    /// Queue `outcome`'s cue. A second result the same frame replaces the
    /// first, so one frame plays at most one cue.
    pub(crate) fn raise(&mut self, outcome: ActionOutcomeType) {
        self.pending = Some(outcome);
    }

    /// Play the owed cue once `bank` is present.
    pub(crate) fn tick(&mut self, bank: Option<&SoundBank<UiSfx>>, commands: &mut Commands) {
        if bank.is_some() {
            if let Some(outcome) = self.pending.take() {
                play_outcome_cue(commands, bank, outcome);
            }
        }
    }
}

/// A verb named `name` built from `spec`, under `gate`. The caller observes
/// `On<Activate>` on the result. Its action can be refused, so the action's
/// result plays the cue, not the press.
pub(crate) fn gated_verb<'a>(
    parent: &'a mut ChildSpawnerCommands,
    name: String,
    spec: ButtonSpec,
    gate: VerbGate,
) -> EntityCommands<'a> {
    parent.spawn((Name::new(name), button(spec), ButtonCueType::Owned, gate))
}

/// A pane's 3D view: the node its scene image fills and its blips ride on.
/// It takes what the side panel leaves of the row.
fn viewport_node() -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        min_height: px(0),
        position_type: PositionType::Relative,
        overflow: Overflow::clip(),
        ..default()
    }
}

/// The row a viewer pane splits into its view and its side panel.
pub(crate) fn view_split() -> Node {
    Node {
        flex_grow: 1.0,
        min_height: px(0),
        flex_direction: FlexDirection::Row,
        column_gap: px(12),
        ..default()
    }
}

/// Height of the Map and Ship footer, in logical px. Fixed, so a legend or a
/// hint that wraps cannot push the view up.
const PANE_FOOTER_PX: f32 = 96.0;

/// UI width ([`ui_width`]), in logical px, from which the Map and Ship footer lays out
/// on one row: the legend, the hints at half the width and the buttons. Under
/// it the hints take a second row, because at 640 px the legend and the
/// buttons leave them 79 px.
const PANE_FOOTER_SPLIT_PX: f32 = 800.0;

/// Height of the footer under [`PANE_FOOTER_SPLIT_PX`], in logical px: the
/// three legend rows beside the buttons, then up to three lines of hints.
/// Fixed per window width, so a hint set cannot push the view up.
const PANE_FOOTER_NARROW_PX: f32 = 144.0;

/// Which footer zone a node is, so [`fit_pane_footer`] can place it on the
/// footer grid for the window width.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaneFooterZoneType {
    Legend,
    Hints,
    Controls,
}

/// The footer under the Map and Ship panes: the legend, the key hints and the
/// view controls, on one row or with the hints on a second row by the window
/// width ([`fit_pane_footer`]).
#[derive(Component)]
pub(crate) struct InterfacePaneFooter;

/// One footer hint of a key a pane answers to, read from the live
/// [`InputBindings`] by [`refresh_pane_input_hints`]: a verb and the actions
/// whose keyboard binds it names. One node per hint, so a wrapping row breaks
/// between hints and never between a key and its verb.
#[derive(Component)]
pub(crate) struct PaneInputHint {
    verb: &'static str,
    actions: &'static [&'static str],
    /// A key the pane reads raw, outside [`InputBindings`], named after the
    /// bound keys; `None` names the bound keys only.
    raw_key: Option<&'static str>,
    when: HintWhenType,
}

/// When a footer hint shows: only while its key or gesture does something,
/// so the footer is one short line of what applies now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HintWhenType {
    /// Shown always.
    Always,
    /// Shown only while a part is in hand.
    Placing,
    /// Shown only while no part is in hand.
    Selecting,
    /// Shown only while the Ship pane's ship is docked at a station that
    /// refits, where a picked part can be placed.
    Refitting,
    /// Shown only while the selected section has repairable damage.
    Repairable,
    /// Shown only while the selected section takes a binding.
    Rebindable,
    /// Shown only while the selected section can be deleted.
    Deletable,
    /// Shown only while the Map shows live contacts, not the chart.
    MapContacts,
    /// Shown only while the Map shows the sector chart.
    MapChart,
}

/// A pointer gesture in a footer: the raw left click, right drag and wheel
/// both viewers read outside [`InputBindings`], with when it shows.
#[derive(Component)]
pub(crate) struct PanePointerHint(HintWhenType);

/// One footer hint: a verb, the actions whose keys it names, a raw key named
/// after them, and when it shows.
type PaneKeyHint = (
    &'static str,
    &'static [&'static str],
    Option<&'static str>,
    HintWhenType,
);

/// The keys the Map pane answers to. Every name must be registered.
const MAP_KEY_HINTS: &[PaneKeyHint] = &[
    (
        "Turn",
        &["viewer_orbit_left", "viewer_orbit_right"],
        None,
        HintWhenType::Always,
    ),
    (
        "Tilt",
        &["viewer_orbit_up", "viewer_orbit_down"],
        None,
        HintWhenType::Always,
    ),
    (
        "Pan",
        &[
            "viewer_pan_forward",
            "viewer_pan_left",
            "viewer_pan_back",
            "viewer_pan_right",
        ],
        None,
        HintWhenType::Always,
    ),
    (
        "Layer",
        &["map_layer_down", "map_layer_up"],
        None,
        HintWhenType::MapChart,
    ),
    ("Reframe", &["viewer_reframe"], None, HintWhenType::Always),
    (
        "Select",
        &["viewer_prev", "viewer_next"],
        None,
        HintWhenType::MapContacts,
    ),
    ("GOTO", &["map_goto"], None, HintWhenType::MapContacts),
    ("Chart", &["map_chart"], None, HintWhenType::Always),
];

/// The keys the Ship pane answers to. Every name must be registered. The
/// view keys, the mate overlay and the reframe stay bound but unlisted: the
/// footer names the keys that act on the selection or the part in hand, and
/// the Fit and Reset buttons sit beside it.
const SHIP_KEY_HINTS: &[PaneKeyHint] = &[
    ("Pick part", &["ship_pick"], None, HintWhenType::Refitting),
    (
        "Roll part",
        &["ship_part_roll"],
        None,
        HintWhenType::Placing,
    ),
    (
        "Cycle socket",
        &["ship_part_socket"],
        None,
        HintWhenType::Placing,
    ),
    // Q over empty space or Esc empties the hand (ghost.rs `pick_ship_part`,
    // terminal/input.rs `close_surface_from_menu_keys`); Esc stays off the
    // registry.
    (
        "Put away",
        &["ship_pick"],
        Some("Esc"),
        HintWhenType::Placing,
    ),
    (
        "Select",
        &["viewer_prev", "viewer_next"],
        None,
        HintWhenType::Always,
    ),
    ("Repair", &["ship_repair"], None, HintWhenType::Repairable),
    ("Rebind", &["ship_rebind"], None, HintWhenType::Rebindable),
    // The keycap already reads `Delete`.
    ("Remove", &["ship_delete"], None, HintWhenType::Deletable),
];

/// The pointer gestures both viewers read raw, outside [`InputBindings`]:
/// a left click selects or places the part in hand, the right button drags
/// the orbit and the wheel zooms.
const POINTER_HINTS: [(&str, HintWhenType); 4] = [
    ("Click select", HintWhenType::Selecting),
    ("Click place", HintWhenType::Placing),
    ("Right-drag look", HintWhenType::Always),
    ("Wheel zoom", HintWhenType::Always),
];

/// The footer's own node for the window band: a grid, so each zone sits by
/// `grid_column` and `grid_row` and the band changes fields only, never the
/// spawn order. Wide is one row, legend | hints | buttons; narrow is legend |
/// buttons over the hints.
fn footer_node(narrow: bool) -> Node {
    let (height, columns, rows) = if narrow {
        (
            PANE_FOOTER_NARROW_PX,
            vec![GridTrack::fr(1.0), GridTrack::fr(1.0)],
            vec![GridTrack::auto(), GridTrack::auto()],
        )
    } else {
        (
            PANE_FOOTER_PX,
            // The hints track caps at half the footer here, not by a
            // `max_width` on the zone: taffy resolves an item's percent
            // against its own track, a track's fit-content limit against
            // the footer.
            vec![
                GridTrack::fr(1.0),
                GridTrack::fit_content_percent(50.0),
                GridTrack::fr(1.0),
            ],
            vec![GridTrack::auto()],
        )
    };
    Node {
        display: Display::Grid,
        height: px(height),
        flex_shrink: 0.0,
        grid_template_columns: columns,
        grid_template_rows: rows,
        align_items: AlignItems::Center,
        align_content: AlignContent::Center,
        column_gap: px(16),
        row_gap: px(6),
        padding: UiRect::axes(px(12), px(8)),
        border: UiRect::top(px(1)),
        ..default()
    }
}

/// A footer zone's node for the window band. The legend and the buttons keep
/// their content width, so they never clip; the hints wrap in their track
/// instead: at most half the footer wide, or the whole second row when narrow.
fn footer_zone_node(zone: PaneFooterZoneType, narrow: bool) -> Node {
    // No `height: percent(100)`: on the two-row grid that would size the
    // first row to the whole footer and push the hints out of it.
    let side = || Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        align_content: AlignContent::Center,
        column_gap: px(12),
        row_gap: px(6),
        overflow: Overflow::clip(),
        ..default()
    };
    match zone {
        PaneFooterZoneType::Legend => Node {
            grid_column: GridPlacement::start(1),
            grid_row: GridPlacement::start(1),
            ..side()
        },
        PaneFooterZoneType::Hints => {
            let (grid_column, grid_row) = if narrow {
                (GridPlacement::start_span(1, 2), GridPlacement::start(2))
            } else {
                (GridPlacement::start(2), GridPlacement::start(1))
            };
            Node {
                grid_column,
                grid_row,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(4),
                ..default()
            }
        }
        PaneFooterZoneType::Controls => Node {
            grid_column: GridPlacement::start(if narrow { 2 } else { 3 }),
            grid_row: GridPlacement::start(1),
            justify_content: JustifyContent::FlexEnd,
            ..side()
        },
    }
}

/// The footer under a viewer pane: legend left, live key hints center, and
/// clickable view controls right. Selection identity stays in the side panel.
/// Spawned in the wide band; [`fit_pane_footer`] places it for the window.
fn pane_footer(
    body: &mut ChildSpawnerCommands,
    legend: impl FnOnce(&mut ChildSpawnerCommands),
    controls: impl FnOnce(&mut ChildSpawnerCommands),
    keys: &'static [PaneKeyHint],
) {
    body.spawn((
        InterfacePaneFooter,
        footer_node(false),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColorType::TextSecondary, 0.5),
    ))
    .with_children(|footer| {
        footer
            .spawn((
                PaneFooterZoneType::Legend,
                footer_zone_node(PaneFooterZoneType::Legend, false),
            ))
            .with_children(legend);
        footer
            .spawn((
                PaneFooterZoneType::Hints,
                footer_zone_node(PaneFooterZoneType::Hints, false),
            ))
            .with_children(|centre| {
                centre
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        column_gap: px(12),
                        row_gap: px(2),
                        ..default()
                    })
                    .with_children(|hints| {
                        // A hint that waits for its state starts hidden
                        // ([`show_pane_input_hints`]).
                        let display = |when| {
                            if when == HintWhenType::Always {
                                Display::Flex
                            } else {
                                Display::None
                            }
                        };
                        for &(verb, actions, raw_key, when) in keys {
                            hints.spawn((
                                PaneInputHint {
                                    verb,
                                    actions,
                                    raw_key,
                                    when,
                                },
                                text("", TextRole::Body),
                                Node {
                                    display: display(when),
                                    ..default()
                                },
                            ));
                        }
                        for (pointer, when) in POINTER_HINTS {
                            hints.spawn((
                                PanePointerHint(when),
                                text(pointer, TextRole::Label),
                                Node {
                                    display: display(when),
                                    ..default()
                                },
                            ));
                        }
                    });
            });
        footer
            .spawn((
                PaneFooterZoneType::Controls,
                footer_zone_node(PaneFooterZoneType::Controls, false),
            ))
            .with_children(|right| {
                right
                    .spawn(control_row(JustifyContent::FlexEnd))
                    .with_children(controls);
            });
    });
}

/// Place the Map and Ship footer for the window: one row at
/// [`PANE_FOOTER_SPLIT_PX`] and up, else the legend and the buttons on the
/// first row and the hints across the second at [`PANE_FOOTER_NARROW_PX`].
/// Keyed on the window alone, never on a hint set, so the view never jumps
/// on a pick. Writes a `Node` only when a field differs, so an idle frame
/// causes no relayout. Skips when there is no primary window (a headless
/// probe), which leaves the footer in the wide band it spawned in.
pub(crate) fn fit_pane_footer(
    q_window: Query<&Window, With<PrimaryWindow>>,
    scale: Res<UiScale>,
    mut q_footer: Query<(&mut Node, &Children), With<InterfacePaneFooter>>,
    mut q_zone: Query<(&PaneFooterZoneType, &mut Node), Without<InterfacePaneFooter>>,
) {
    let Ok(window) = q_window.single() else {
        return;
    };
    let narrow = ui_width(window, &scale) < PANE_FOOTER_SPLIT_PX;
    for (mut node, zones) in &mut q_footer {
        let target = footer_node(narrow);
        if *node != target {
            *node = target;
        }
        let mut zones = q_zone.iter_many_mut(zones).matched();
        while let Some((zone, mut node)) = zones.fetch_next() {
            let target = footer_zone_node(*zone, narrow);
            if *node != target {
                *node = target;
            }
        }
    }
}

/// Write each footer hint from the live bindings, for the device the player
/// pressed last, when the hint is new or the bindings or the device change,
/// so a rebind or a pad press moves the hint with the key.
///
/// # Panics
///
/// When a hint names an action [`InputBindings`] does not register: the hint
/// tables and [`interface_bindings`](crate::bindings::interface_bindings) are
/// one vocabulary, and a hint for a missing action would name no key.
pub(crate) fn refresh_pane_input_hints(
    bindings: Res<InputBindings>,
    device: Res<InputDeviceType>,
    mut q_hint: Query<(Ref<PaneInputHint>, &mut Text)>,
) {
    for (hint, mut text) in &mut q_hint {
        if !hint.is_added() && !bindings.is_changed() && !device.is_changed() {
            continue;
        }
        let keys = hint
            .actions
            .iter()
            .map(|name| {
                bindings
                    .get(name)
                    .unwrap_or_else(|| {
                        panic!("footer hint names `{name}`, which is not registered")
                    })
                    .hint_display(*device)
            })
            .chain(hint.raw_key.map(str::to_owned))
            .collect::<Vec<_>>()
            .join("/");
        let value = format!("{keys} {}", hint.verb);
        if text.0 != value {
            text.0 = value;
        }
    }
}

/// Show each footer hint by its [`HintWhenType`], from the part in hand,
/// what the Ship pane cached for its selection and its dock, and whether the
/// Map shows its chart. All hide while the parts gallery is open: it holds
/// the keys and names its own.
pub(crate) fn show_pane_input_hints(
    held: Option<Res<HeldPart>>,
    ship: Option<Res<ShipRuntime>>,
    gallery: Option<Res<GalleryState>>,
    map: Option<Res<MapRuntime>>,
    mut q_key: Query<(&PaneInputHint, &mut Node), Without<PanePointerHint>>,
    mut q_pointer: Query<(&PanePointerHint, &mut Node), Without<PaneInputHint>>,
) {
    let placing = held.is_some_and(|held| held.0.is_some());
    // The open gallery holds the keys and names its own.
    let browsing = gallery.is_some_and(|gallery| gallery.open);
    let ship = ship.as_deref();
    // The chart view selects by click and has nothing to fly to.
    let contacts = map.as_ref().is_some_and(|map| !map.chart);
    let chart = map.is_some_and(|map| map.chart);
    let shown = |when: HintWhenType| {
        !browsing
            && match when {
                HintWhenType::Always => true,
                HintWhenType::Placing => placing,
                HintWhenType::Selecting => !placing,
                HintWhenType::Refitting => ship.is_some_and(|ship| ship.refits),
                HintWhenType::Repairable => ship.is_some_and(|ship| ship.panel_repairable),
                HintWhenType::Rebindable => ship.is_some_and(|ship| ship.panel_rebind_enabled),
                HintWhenType::Deletable => {
                    ship.is_some_and(|ship| matches!(ship.delete_verdict(), Some(Ok(_))))
                }
                HintWhenType::MapContacts => contacts,
                HintWhenType::MapChart => chart,
            }
    };
    let nodes = q_key
        .iter_mut()
        .map(|(hint, node)| (hint.when, node))
        .chain(q_pointer.iter_mut().map(|(hint, node)| (hint.0, node)));
    for (when, mut node) in nodes {
        let display = if shown(when) {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}

/// The map: the scene and contact panel beside it, then legend, hints,
/// Reframe and Chart.
fn map_body(body: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    body.spawn(view_split()).with_children(|split| {
        split.spawn((MapViewportMarker, viewport_node(), ImageNode::default()));
        spawn_map_panel(split, icons);
    });
    pane_footer(
        body,
        |legend| {
            legend.spawn((
                MapLegendMarker,
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    column_gap: px(12),
                    row_gap: px(6),
                    ..default()
                },
            ));
        },
        |controls| {
            controls
                .spawn((
                    button(ButtonSpec::control("Reframe")),
                    Name::new("MapReframe"),
                ))
                .observe(on_map_reframe_button);
            controls
                .spawn(map_chart_button())
                .observe(on_map_chart_button);
        },
        MAP_KEY_HINTS,
    );
}

/// The ship: the scene and section panel beside it, then legend, hints, the
/// Catalogue and the view controls.
fn ship_body(body: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    body.spawn((ShipSplitMarker, view_split()))
        .with_children(|split| {
            split
                .spawn((
                    ShipViewportMarker,
                    Name::new("ShipViewport"),
                    viewport_node(),
                    ImageNode::default(),
                ))
                .with_children(spawn_ship_rebind_prompt)
                .observe(on_ship_viewport_enter)
                .observe(on_ship_viewport_move)
                .observe(on_ship_viewport_leave)
                .observe(on_ship_viewport_click);
            spawn_ship_panel(split, icons);
        });
    pane_footer(
        body,
        |legend| {
            legend
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(5),
                    ..default()
                })
                .with_children(|rows| {
                    for icons_in_row in SectionIconType::ALL.chunks(3) {
                        rows.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(8),
                            ..default()
                        })
                        .with_children(|row| {
                            for &icon in icons_in_row {
                                row.spawn(legend_entry()).with_children(|entry| {
                                    entry.spawn(icon_node(icons.section(icon), icon.color(), 18.0));
                                    entry.spawn(text(icon.label(), TextRole::Body));
                                });
                            }
                        });
                    }
                });
        },
        |controls| {
            // First of the three: it shows only at a refit dock, and Fit and
            // Reset keep their places at the right edge with or without it.
            controls.spawn((
                ShipCatalogueButton,
                GalleryAction::Open,
                button(ButtonSpec::control("Catalogue")),
                Name::new("ShipCatalogue"),
            ));
            controls
                .spawn((button(ButtonSpec::control("Fit")), Name::new("ShipFit")))
                .observe(on_ship_fit_button);
            controls
                .spawn((button(ButtonSpec::control("Reset")), Name::new("ShipReset")))
                .observe(on_ship_reset_button);
        },
        SHIP_KEY_HINTS,
    );
}

/// One icon and its word in a legend.
pub(crate) fn legend_entry() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(4),
        ..default()
    }
}

/// One part tile: its category glyph, `count` (`+3`, `2/23` or `23`) and its
/// name. The Inventory strip block and the Ship Salvage block draw it.
///
/// # Panics
///
/// On a part id the catalog does not hold.
pub(crate) fn spawn_part_tile(
    row: &mut ChildSpawnerCommands,
    icons: &InterfaceIcons,
    items: &GameItems,
    item: &ItemDesignId,
    count: &str,
) {
    let design = items.design(item);
    row.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(4.0),
        ..default()
    })
    .with_children(|tile| {
        tile.spawn(icon_node(
            icons.category(design.category),
            category_color(design.category),
            18.0,
        ));
        tile.spawn((
            text(count, TextRole::Value),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
        ));
        tile.spawn((
            text_toned(&design.name, TextRole::Label, UiColorType::TextBody),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
        ));
    });
}

/// Raise the viewer contexts to match the interface: `Viewer` while the
/// interface owns the screen, and the shown pane's own `InterfacePane` context.
/// Both drop under the command modal, where the keyboard is typing.
pub(crate) fn sync_nova_os_contexts(
    pause: Option<Res<State<PauseStates>>>,
    pane: Res<InterfacePaneType>,
    mut active: ResMut<ActiveContexts>,
) {
    let open = pause.is_some_and(|state| *state.get() == PauseStates::Interface);
    ActiveContexts::sync(&mut active, ActionContext::Viewer, open);
    for (each, _) in InterfacePaneType::ALL {
        ActiveContexts::sync(
            &mut active,
            ActionContext::InterfacePane(each.context_id()),
            open && *pane == each,
        );
    }
}

/// Hold the clocks while the TAB interface or the command modal is open.
/// `InterfacePlugin` adds it; a harness without that plugin adds it itself.
///
/// Here, not in the menu: an app without `NovaMenuPlugin` (every example
/// built with game plugins) kept its world running under the interface, and a
/// Map GOTO set on a frozen hull ended on the next fixed tick.
pub fn add_interface_clock_hold(app: &mut App) {
    for surface in [PauseStates::Interface, PauseStates::Commands] {
        app.add_systems(OnEnter(surface), hold_clocks_for_interface);
        app.add_systems(OnExit(surface), release_clocks_for_interface);
    }
}

/// Freeze the simulation for the TAB interface or the command modal: virtual
/// time, and the FixedUpdate flight and physics that follow it. Both states
/// share [`FreezeOwner::Interface`], so a hop between them re-takes a hold
/// that was never dropped.
///
/// GAMEPLAY only. `;` opens the command modal over the main menu as well, and
/// what runs behind it there is the menu's own ambience backdrop - a cinematic
/// the modal is drawn over, not a game the player is being kept out of.
/// Stopping it would leave the front door on a still frame for as long as the
/// modal is up. `Playing` is the whole of what the modal freezes, the editor's
/// build mode included.
fn hold_clocks_for_interface(game: Option<Res<State<GameStates>>>, mut clocks: Clocks) {
    if game.is_some_and(|game| *game.get() != GameStates::Playing) {
        return;
    }
    clocks.hold(FreezeOwner::Interface);
}

/// Drop the interface hold, unless the transition moves straight into the
/// other interface state: `Interface -> Commands` and back keep the world
/// frozen with no frame in between.
fn release_clocks_for_interface(
    mut transitions: MessageReader<StateTransitionEvent<PauseStates>>,
    mut clocks: Clocks,
) {
    let entered = transitions
        .read()
        .last()
        .and_then(|transition| transition.entered);
    if matches!(
        entered,
        Some(PauseStates::Interface | PauseStates::Commands)
    ) {
        return;
    }
    clocks.release(FreezeOwner::Interface);
}
