//! The merged theme registry, the player's selection, and the one system that
//! turns the two into the live [`ActiveUiTheme`].
//!
//! Three resources and one rule:
//!
//! - [`GameUiThemes`] is what the content merge published - every theme every
//!   ENABLED mod declares, overlaid last-wins by id, exactly like the section
//!   and style registries beside it;
//! - [`SelectedUiTheme`] is the player's choice, persisted by Settings as a
//!   stable theme id and nothing else;
//! - [`ActiveUiTheme`] is the resolved result the widgets paint from.
//!
//! The rule: a selection that does not resolve FALLS BACK to `base/phosphor`
//! and says why. Enabling a mod never selects its theme, and disabling the mod
//! whose theme is selected must not leave the game unpaintable - so the
//! fallback is a real runtime condition with defined behaviour, not a way to
//! hide bad authoring. The reason is kept in [`UiThemeDiagnostic`] so the
//! Settings screen can show the player the sentence the log got.

use bevy::prelude::*;

use super::{
    config::{UiThemeConfig, PHOSPHOR_THEME_ID},
    resolve::{resolve_theme, ActiveUiTheme, ThemeIssue},
};

/// Every UI theme the enabled mods declare, in registration order, overlaid
/// last-wins by id - so a mod restyles a base look by declaring its id and
/// ships a new one by declaring a new id.
///
/// Published by the content merge. Empty until content lands, which is why
/// [`ActiveUiTheme`] has a built-in default.
#[derive(Resource, Debug, Clone, Default)]
pub struct GameUiThemes(pub Vec<UiThemeConfig>);

impl GameUiThemes {
    /// The theme with this id, if one is registered.
    pub fn get(&self, id: &str) -> Option<&UiThemeConfig> {
        self.0.iter().find(|theme| theme.id == id)
    }

    /// `(id, display name)` for every registered theme, in registration order -
    /// what the Settings picker lists.
    pub fn listed(&self) -> Vec<(&str, &str)> {
        self.0
            .iter()
            .map(|theme| (theme.id.as_str(), theme.name.as_str()))
            .collect()
    }
}

/// The theme the player chose, by stable id. Persisted by Settings.
///
/// A plain `String`, not an enum: the set of themes is open, and the whole
/// point of the change is that a mod's theme id survives a restart.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct SelectedUiTheme(pub String);

impl Default for SelectedUiTheme {
    fn default() -> Self {
        Self(PHOSPHOR_THEME_ID.to_string())
    }
}

/// Why the selected theme is not the one showing, or `None` when it is.
///
/// Player-visible: Settings draws it beside the theme row, because a player
/// whose theme silently reverted has no way to tell a broken mod from a
/// forgotten setting.
#[derive(Resource, Debug, Clone, Default)]
pub struct UiThemeDiagnostic(pub Option<ThemeIssue>);

/// The system set the theme resolves in. Every widget reconciler runs AFTER it,
/// so a theme flip reaches what is already on screen in the SAME frame rather
/// than one frame late.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UiThemeSystems;

/// Wire the registry resources and the resolver.
pub(crate) fn build(app: &mut App) {
    app.init_resource::<GameUiThemes>()
        .init_resource::<SelectedUiTheme>()
        .init_resource::<UiThemeDiagnostic>()
        .init_resource::<ActiveUiTheme>()
        .add_systems(Update, resolve_active_theme.in_set(UiThemeSystems));
}

/// Resolve the selection against the registry whenever either moves.
///
/// Writes [`ActiveUiTheme`] only when the result actually DIFFERS: the widget
/// reconcilers restyle every live control on `is_changed()`, and a re-merge
/// that republished the same themes would otherwise repaint the whole UI for
/// nothing.
fn resolve_active_theme(
    themes: Res<GameUiThemes>,
    selected: Res<SelectedUiTheme>,
    mut active: ResMut<ActiveUiTheme>,
    mut diagnostic: ResMut<UiThemeDiagnostic>,
) {
    if !themes.is_changed() && !selected.is_changed() {
        return;
    }
    // Before content lands there is nothing to resolve against, and the
    // bootstrap theme is already showing. Resolving an empty registry would
    // report "no theme with this id is registered" on every boot.
    if themes.0.is_empty() {
        return;
    }

    let (resolved, issue) = match resolve_theme(&selected.0, &themes.0) {
        Ok(theme) => (Some(theme), None),
        Err(issue) => {
            warn!("ui theme: {issue}; falling back to {PHOSPHOR_THEME_ID}");
            (
                resolve_theme(PHOSPHOR_THEME_ID, &themes.0).ok(),
                Some(issue),
            )
        }
    };

    if diagnostic.0 != issue {
        diagnostic.0 = issue;
    }
    match resolved {
        Some(theme) if theme != *active => *active = theme,
        Some(_) => {}
        // Both the selection AND base phosphor failed - a base mod whose theme
        // content is broken. The bootstrap theme stays on screen, which is the
        // same look base phosphor is meant to be, and the diagnostic says so.
        None => error!(
            "ui theme: {PHOSPHOR_THEME_ID} does not resolve either; \
             staying on the built-in bootstrap theme"
        ),
    }
}
