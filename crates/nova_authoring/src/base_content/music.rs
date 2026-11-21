//! Built-in MUSIC content: the songs the context rule plays, and the
//! licence record each one ships under.
//!
//! `content gen` serializes this builder into
//! `assets/base/music/base.content.ron`. The files are OGG Vorbis at
//! quality 4, transcoded from the authors' WAV or MP3 uploads; the sources
//! and licences are listed in `assets/base/music/README.md` and
//! `credits/CREDITS.md`.

use nova_gameplay::prelude::AssetRef;
use nova_scenario::prelude::{MusicContextType, MusicLicenceType, MusicTrackDesign};

/// The OpenGameArt pages the songs come from. The licence record is the
/// OpenGameArt copy: the same songs on the authors' own pages carry no
/// open licence.
const EXPLORATION_THEME_PAGE: &str = "https://opengameart.org/content/exploration-theme";
const DISTANT_PACK_PAGE: &str =
    "https://opengameart.org/content/distant-sci-fielectronic-music-pack";
const AMBIENT_SOUNDTRACK_PAGE: &str = "https://opengameart.org/content/ambient-soundtrack";
const INTENSE_ACTION_PAGE: &str =
    "https://opengameart.org/content/intense-electronic-action-soundtrack";
const ELECTRONIC_SOUNDTRACKS_PAGE: &str = "https://opengameart.org/content/electronic-soundtracks";

/// The credit line of a Type 42 song, in the attribution the author asks
/// for, with the licence and the source page.
fn type42_credit(title: &str, page: &str) -> String {
    format!(
        "\"{title}\" - Music by Type 42 - type42.bandcamp.com. Licensed CC BY 4.0 \
         (https://creativecommons.org/licenses/by/4.0/). Source: {page}. Changes: transcoded \
         to OGG Vorbis q4."
    )
}

fn type42(file: &str, title: &str, page: &str, contexts: &[MusicContextType]) -> MusicTrackDesign {
    MusicTrackDesign {
        id: format!("base/{file}").as_str().into(),
        sound: AssetRef::from(format!("self://music/{file}.ogg")),
        contexts: contexts.to_vec(),
        title: title.to_string(),
        artist: "Type 42".to_string(),
        licence: MusicLicenceType::CcBy4,
        credit: type42_credit(title, page),
    }
}

/// The base game's songs: the menu theme, and nine Type 42 songs, two or
/// three for each situation.
pub(crate) fn music_catalog() -> Vec<MusicTrackDesign> {
    use MusicContextType::{Combat, Cruise, Docked, Menu, Threat};
    vec![
        // The theme loops without a seam, so the menu replays it with no
        // gap.
        MusicTrackDesign {
            id: "base/exploration_theme".into(),
            sound: AssetRef::from("self://music/exploration_theme.ogg".to_string()),
            contexts: vec![Menu],
            title: "Exploration Theme".to_string(),
            artist: "Cleyton Kauffman".to_string(),
            licence: MusicLicenceType::Cc0,
            credit: format!(
                "\"Exploration Theme\" by Cleyton Kauffman \
                 (https://soundcloud.com/cleytonkauffman), CC0 1.0. Source: \
                 {EXPLORATION_THEME_PAGE}. Changes: transcoded to OGG Vorbis q4."
            ),
        },
        // Cruise: the steadiest songs of the pack, with a soft top end and
        // little beat, so a long flight does not tire.
        type42(
            "stellar_cruise",
            "Stellar Cruise",
            DISTANT_PACK_PAGE,
            &[Cruise],
        ),
        type42(
            "little_blue_speck",
            "Little Blue Speck",
            DISTANT_PACK_PAGE,
            &[Cruise],
        ),
        type42("uncharted", "Uncharted", DISTANT_PACK_PAGE, &[Cruise]),
        // Threat: sparse and dark, for a contact that has not fired yet.
        type42("unseen", "Unseen", DISTANT_PACK_PAGE, &[Threat]),
        type42("unknown", "Unknown", DISTANT_PACK_PAGE, &[Threat]),
        // Combat: a clear beat at full level.
        type42(
            "intense_electronic_action",
            "Intense Electronic Action Soundtrack",
            INTENSE_ACTION_PAGE,
            &[Combat],
        ),
        type42(
            "the_hunter",
            "The Hunter",
            ELECTRONIC_SOUNDTRACKS_PAGE,
            &[Combat],
        ),
        // Docked: no clear beat, long, so a trade stop seldom hears an end.
        type42(
            "ambient_soundtrack",
            "Ambient Soundtrack",
            AMBIENT_SOUNDTRACK_PAGE,
            &[Docked],
        ),
        type42(
            "hidden_nebula",
            "Hidden Nebula",
            DISTANT_PACK_PAGE,
            &[Docked],
        ),
    ]
}
