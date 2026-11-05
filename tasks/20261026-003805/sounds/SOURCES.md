# Torpedo world-sound candidates: sources

Licensed recordings found for the four positional torpedo cues. Nothing here is
generated or synthesized by Nova tooling. Every candidate is CC0 1.0. Each
license was read on the source page itself (Freesound license block, or the
Kenney asset page plus the `License.txt` inside the pack).

Freesound candidates are made from the public HQ OGG preview
(`cdn.freesound.org/previews/...-hq.ogg`), not the original upload, because the
original needs a login. A preview has the same license as the original. Swap in
the lossless original before shipping if you want to drop the lossy stage.

Processing, the same for every row: decode with ffmpeg, downmix to mono, resample
to 44100 Hz. Trim leading silence (start 5 ms before the first sample above
-30 dB of peak, 3 ms fade-in). Cut to the listed length. Raised-cosine fade-out
over the listed tail. Remove DC. Peak-normalize to -3 dBFS. Write 16-bit PCM
WAV. Every file was checked with ffprobe (pcm_s16le, 44100 Hz, 1 ch) and
volumedetect (max -3.0 dB).

The character notes come from the source descriptions and from measured
envelope and spectrum (attack, time to -20/-40 dB, spectral centroid, energy
share below 150 Hz). Nobody has listened to them yet. Audition them before you
pick.

| Cue | File | Source URL | Author | License | Original name | Duration | Processing | Why it fits |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| torpedo_ignite | `candidates/torpedo_ignite/torpedo_ignite_a.wav` | https://freesound.org/people/gracenew/sounds/857959/ (preview `857959_18666818-hq.ogg`) | gracenew | CC0 1.0 | Rocket Launch | 0.80 s | first 0.80 s of 2.90 s, 200 ms fade | A 15 ms crack goes straight into a steady mid-band burn (centroid ~750 Hz) with no blast tail, so it reads as a motor lighting and not as an explosion. |
| torpedo_ignite | `candidates/torpedo_ignite/torpedo_ignite_b.wav` | https://freesound.org/people/wubitog/sounds/200459/ (preview `200459_3501487-hq.ogg`) | wubitog | CC0 1.0 | Missile / Rocket firing | 0.70 s | 76 ms lead trimmed, first 0.70 s, 200 ms fade | A fast 25 ms onset into a bright hissing burn that drops to -20 dB by 0.33 s, with no sub-bass, so it is smaller and lighter than candidate a. |
| torpedo_ignite | `candidates/torpedo_ignite/torpedo_ignite_c.wav` | https://freesound.org/people/morganpurkis/sounds/398213/ (preview `398213_5937039-hq.ogg`) | morganpurkis | CC0 1.0 | CAS Missile Launching.wav | 0.80 s | first 0.80 s of 1.41 s, 200 ms fade | A 0.1 s whoosh swell into a very bright rocket hiss (centroid ~5.5 kHz); its swell shape is the closest to the cold-launch cue, so it may blur with it. |
| torpedo_dud | `candidates/torpedo_dud/torpedo_dud_a.wav` | https://kenney.nl/assets/sci-fi-sounds | Kenney (kenney.nl) | CC0 1.0 | Sci-Fi Sounds 1.0, `Audio/impactMetal_000.ogg` | 0.45 s | first 0.45 s of 0.63 s, 120 ms fade | An instant dull metal hit centred near 210 Hz with little sub energy (17% below 150 Hz), gone to -40 dB by 0.28 s: a heavy clank with no boom. |
| torpedo_dud | `candidates/torpedo_dud/torpedo_dud_b.wav` | https://freesound.org/people/HenKonen/sounds/682149/ (preview `682149_10938187-hq.ogg`) | HenKonen | CC0 1.0 | Metallic Clank 2.wav | 0.50 s | 14 ms lead trimmed, first 0.50 s, 150 ms fade | A recording of a heavy metal object being handled: a short clank with a thick low thump (67% below 150 Hz) that drops 20 dB in 70 ms. |
| torpedo_dud | `candidates/torpedo_dud/torpedo_dud_c.wav` | https://freesound.org/people/ajanhallinta/sounds/650701/ (preview `650701_5370365-hq.ogg`) | ajanhallinta | CC0 1.0 | MetalHit1.wav | 0.45 s | first 0.45 s of 0.49 s, 100 ms fade | A simple two-stage metal impact near 220 Hz with a slightly longer body than a, still with no explosion tail. |
| torpedo_intercept | `candidates/torpedo_intercept/torpedo_intercept_a.wav` | https://freesound.org/people/ReadeOnly/sounds/186958/ (preview `186958_545448-hq.ogg`) | ReadeOnly | CC0 1.0 | explosion7.wav | 0.25 s | first 0.25 s of 0.31 s, 80 ms fade | A small, weak firework pop that is down 40 dB in 90 ms; the shortest and softest choice, so it should not tire the ear at several per second. |
| torpedo_intercept | `candidates/torpedo_intercept/torpedo_intercept_b.wav` | https://freesound.org/people/Rudmer_Rotteveel/sounds/336011/ (preview `336011_4921277-hq.ogg`) | Rudmer_Rotteveel | CC0 1.0 | Sharp Explosion 4 (of 5) | 0.35 s | first 0.35 s of 1.05 s, 120 ms fade | A single sharp firework crack that drops 20 dB in 20 ms, with a quiet outdoor tail under it; sharper than a and still far below a blast. |
| torpedo_intercept | `candidates/torpedo_intercept/torpedo_intercept_c.wav` | https://freesound.org/people/iliyabylich04/sounds/651332/ (preview `651332_14128260-hq.ogg`) | iliyabylich04 | CC0 1.0 | Firecracker - Single in Ground.wav | 0.25 s | 37 ms lead trimmed, first 0.25 s, 80 ms fade | A very dry firecracker crack with a ground thump, gone in 40 ms; the tightest transient, but it may click when repeated fast. |
| torpedo_rack | `candidates/torpedo_rack/torpedo_rack_a.wav` | https://freesound.org/people/JPolito/sounds/391724/ (preview `391724_149792-hq.ogg`) | JPolito | CC0 1.0 | jp_circuitbox_locked01.flac | 0.60 s | first 0.60 s of 0.74 s, 120 ms fade | A pull on a locked metal circuit-breaker door: a three-part rattle and clunk, which reads like a mechanism seating and locking. |
| torpedo_rack | `candidates/torpedo_rack/torpedo_rack_b.wav` | https://freesound.org/people/P%C3%B3l/sounds/385936/ (preview `385936_7097737-hq.ogg`) | Pól | CC0 1.0 | S011_Gear_Change_Mono.wav | 0.32 s | 175 ms lead trimmed, rest of file (0.32 s), 80 ms fade | A rally-car gearbox clunk: one heavy low mechanical seat (centroid ~360 Hz), the heaviest and most "loaded" of the three. |
| torpedo_rack | `candidates/torpedo_rack/torpedo_rack_c.wav` | https://freesound.org/people/Q.K./sounds/56246/ (preview `56246_91374-hq.ogg`) | Q.K. | CC0 1.0 | Latch_04.wav | 0.28 s | first 0.28 s of 0.29 s, 60 ms fade | A small latch unhooking with a click and a second click; a clean latch, but light for a torpedo-sized rack. |

## Rejected during the search

- Freesound 521377 "Rocket Launch" (Jarusca, CC0): its waveform matches 398213
  (correlation 0.93, same length). I kept the earlier upload, 398213.
- Freesound 458669 "missile-launch.ogg" (jorgerosa, CC0): the description says
  it is remixed from other Freesound sounds it does not name, so the
  provenance cannot be checked.
- Freesound 699823 "Hydra 70 rocket pod launching sound" (hijongpark, CC0): it
  has a double crack (a pod salvo), and the source of the recording is not
  stated.
- Kenney Sci-Fi Sounds `thrusterFire_*` and `explosionCrunch_*` (CC0): the
  thruster loops take 1.1 s to build, and the crunch explosions are 0.8-2 s
  with heavy low end. They are too slow for an ignite and too big for an
  intercept.
- Sonniss GDC bundles: not used, because their license does not allow
  redistributing the raw files.
