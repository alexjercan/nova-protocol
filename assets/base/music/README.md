# Music

The songs the music director plays. Each one is a `MusicTrack` row in
`base.content.ron`, which `content gen` writes from
`crates/nova_authoring/src/base_content/music.rs`. Do not edit that file by
hand. The row names the situations whose playlist the song joins and its
licence record. The files are declared in the base bundle's `resources`
list, so a mod reaches them with `dep://base/music/<name>.ogg`.

Every file is OGG Vorbis at quality 4, full length, transcoded from the
author's upload with:

```
ffmpeg -i song.wav -c:a libvorbis -q:a 4 song.ogg
```

The game fades songs in and out at play time; the files carry no fades.
Licences and credits: `credits/CREDITS.md` and `credits/licenses/`.

| File | Plays in | Title | Author | Source | Licence | Changes |
| --- | --- | --- | --- | --- | --- | --- |
| `exploration_theme.ogg` | Menu | Exploration Theme | Cleyton Kauffman | https://opengameart.org/content/exploration-theme | CC0 1.0 | WAV from the zip to OGG |
| `stellar_cruise.ogg` | Cruise | Stellar Cruise | Type 42 | https://opengameart.org/content/distant-sci-fielectronic-music-pack | CC BY 4.0 | WAV to OGG |
| `little_blue_speck.ogg` | Cruise | Little Blue Speck | Type 42 | https://opengameart.org/content/distant-sci-fielectronic-music-pack | CC BY 4.0 | WAV to OGG |
| `uncharted.ogg` | Cruise | Uncharted | Type 42 | https://opengameart.org/content/distant-sci-fielectronic-music-pack | CC BY 4.0 | WAV to OGG |
| `unseen.ogg` | Threat | Unseen | Type 42 | https://opengameart.org/content/distant-sci-fielectronic-music-pack | CC BY 4.0 | WAV to OGG |
| `unknown.ogg` | Threat | Unknown | Type 42 | https://opengameart.org/content/distant-sci-fielectronic-music-pack | CC BY 4.0 | WAV to OGG |
| `intense_electronic_action.ogg` | Combat | Intense Electronic Action Soundtrack | Type 42 | https://opengameart.org/content/intense-electronic-action-soundtrack | CC BY 4.0 | 320 kbit/s MP3 to OGG |
| `the_hunter.ogg` | Combat | The Hunter | Type 42 | https://opengameart.org/content/electronic-soundtracks | CC BY 4.0 | WAV to OGG |
| `ambient_soundtrack.ogg` | Docked | Ambient Soundtrack | Type 42 | https://opengameart.org/content/ambient-soundtrack | CC BY 4.0 | WAV to OGG |
| `hidden_nebula.ogg` | Docked | Hidden Nebula | Type 42 | https://opengameart.org/content/distant-sci-fielectronic-music-pack | CC BY 4.0 | WAV to OGG |
