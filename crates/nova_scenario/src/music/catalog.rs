//! The music track catalog: what each song is, where it plays and the licence
//! it ships under.

use std::{collections::BTreeMap, fmt, path::Path};

use bevy::{audio::AudioSource, prelude::*};
use nova_gameplay::prelude::AssetRef;

use super::MusicContextType;

/// The one file extension a music file may carry.
///
/// bevy decodes OGG Vorbis with no extra decoder, and a 4 min song at quality
/// 4 is about 3.5 MB. A WAV plays too, but it is ten times the size in the
/// download and in memory, because the game holds each song's file while it
/// plays. FLAC and MP3 need decoders the game does not build.
pub const MUSIC_EXTENSION: &str = "ogg";

/// The authored id of a music track, namespaced like an encounter:
/// `base/stellar_cruise`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct MusicTrackDesignId(pub String);

impl MusicTrackDesignId {
    /// The id as authored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MusicTrackDesignId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for MusicTrackDesignId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// The licence a track ships under. Closed: a track the game ships must be
/// one the repository may hold as a raw file, in every build and every
/// trailer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MusicLicenceType {
    /// CC0 1.0: no attribution is due. The credit is the record of where the
    /// file came from.
    Cc0,
    /// CC BY 3.0: the credit names the creator, the licence and the source.
    CcBy3,
    /// CC BY 4.0: the credit names the creator, the licence and the source.
    CcBy4,
    /// Made for the mod by its own author.
    Own,
}

impl MusicLicenceType {
    /// Whether the credit must link the source the file came from.
    pub const fn needs_source(self) -> bool {
        !matches!(self, Self::Own)
    }
}

/// One song the context rule may play. Every field is required.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct MusicTrackDesign {
    /// Its id.
    pub id: MusicTrackDesignId,
    /// The song, an `.ogg` file: `self://music/<name>.ogg` in the mod that
    /// ships it.
    pub sound: AssetRef<AudioSource>,
    /// The situations whose playlist it joins: `Menu`, `Cruise`, `Threat`,
    /// `Combat` or `Docked`.
    pub contexts: Vec<MusicContextType>,
    /// The song's title.
    pub title: String,
    /// Who made it.
    pub artist: String,
    /// The licence it ships under.
    pub licence: MusicLicenceType,
    /// The attribution line: title, artist, licence and the source URL. The
    /// licence record, kept with the song so the lint can ask for it.
    pub credit: String,
}

/// The merged music tracks, by id. Inserted by the mod merge.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct GameMusicTracks(pub BTreeMap<MusicTrackDesignId, MusicTrackDesign>);

impl GameMusicTracks {
    /// The catalog of `tracks`; a later one with an id replaces an earlier
    /// one.
    pub fn new(tracks: impl IntoIterator<Item = MusicTrackDesign>) -> Self {
        Self(
            tracks
                .into_iter()
                .map(|track| (track.id.clone(), track))
                .collect(),
        )
    }

    /// The track `id`, if the catalog holds it.
    pub fn get(&self, id: &MusicTrackDesignId) -> Option<&MusicTrackDesign> {
        self.0.get(id)
    }

    /// Every track, in id order.
    pub fn iter(&self) -> impl Iterator<Item = &MusicTrackDesign> {
        self.0.values()
    }

    /// The playlist of `context`: every track that names it, in id order.
    pub fn playlist(&self, context: MusicContextType) -> impl Iterator<Item = &MusicTrackDesign> {
        self.iter()
            .filter(move |track| track.contexts.contains(&context))
    }
}

/// Why a music file path is refused, or `None` for an `.ogg` file: the
/// message names the fix, with the `ffmpeg` command that makes the file.
pub fn music_format_fault(path: &str) -> Option<String> {
    let file = Path::new(path.rsplit("://").next().unwrap_or(path));
    let extension = file.extension().and_then(|extension| extension.to_str());
    if extension.is_some_and(|extension| extension.eq_ignore_ascii_case(MUSIC_EXTENSION)) {
        return None;
    }
    let name = file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("song.wav");
    let stem = file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("song");
    Some(format!(
        "music_format: \"{path}\" is not OGG Vorbis; music must be an .ogg file. Convert it: \
         ffmpeg -i {name} -c:a libvorbis -q:a 4 {stem}.ogg"
    ))
}

/// Every fault of `track` on its own, keyed in the message by its id: an
/// empty id, title or artist, no situation, a situation a track cannot play
/// in, a situation named twice, a file that is not `.ogg`, and a missing
/// licence record.
pub fn music_track_faults(track: &MusicTrackDesign) -> Vec<String> {
    let id = &track.id;
    let mut faults = Vec::new();
    if id.0.trim().is_empty() {
        faults.push("a music track has an empty id".to_string());
    }
    if track.title.trim().is_empty() {
        faults.push(format!("the music track '{id}' has an empty title"));
    }
    if track.artist.trim().is_empty() {
        faults.push(format!("the music track '{id}' has an empty artist"));
    }
    if track.contexts.is_empty() {
        faults.push(format!(
            "the music track '{id}' names no situation; it plays in Menu, Cruise, Threat, \
             Combat or Docked"
        ));
    }
    for (at, context) in track.contexts.iter().enumerate() {
        if !context.has_playlist() {
            faults.push(format!(
                "the music track '{id}' names the situation {context:?}; a track plays in \
                 Menu, Cruise, Threat, Combat or Docked"
            ));
        }
        if track.contexts[..at].contains(context) {
            faults.push(format!(
                "the music track '{id}' names the situation {context:?} twice"
            ));
        }
    }
    match track.sound.path() {
        Some(path) => {
            if let Some(fault) = music_format_fault(path) {
                faults.push(format!("the music track '{id}': {fault}"));
            }
        }
        None => faults.push(format!("the music track '{id}' names no sound file")),
    }
    let credit = track.credit.trim();
    if credit.is_empty() {
        faults.push(format!(
            "music_licence: the music track '{id}' has no licence record; its credit is empty"
        ));
    } else if track.licence.needs_source() && !credit.contains("://") {
        faults.push(format!(
            "music_licence: the music track '{id}' is {:?}; its credit must link the source it \
             came from",
            track.licence
        ));
    }
    faults
}
