# Notes

## Reproduction

Chromium under Xvfb :107 on the reporting host, which had ~630 MiB of its
8 GiB free (llama-server 5466 MiB, whisper-server 846 MiB, plus the user's own
capture pipeline). The deployed `/play/` build fails identically to the report:

    INFO  GameAssetsStates::Boot is done
    ERROR Caught rendering error: Out of Memory
          vkAllocateMemory failed with VK_ERROR_OUT_OF_DEVICE_MEMORY
           - While calling [Device].CreateTexture([TextureDescriptor]).
    ERROR Quitting the application due to OutOfMemory RenderError

A `CreateTexture` immediately after Boot, followed by a `WriteTexture` on the
now-invalid texture: an image upload, and the first big one is the skybox.

## The waste

`assets/base/textures/cubemap.png` was 4096x24576 - six stacked 4096 px faces.
Uploaded as Rgba8UnormSrgb that is 6 x 4096 x 4096 x 4 = **402,653,184 bytes**,
and the `.meta` asks for `MAIN_WORLD | RENDER_WORLD`, so the same 402 MB also
sits in the wasm heap. `cubemap_alt.png` is the same shape. MAIN_WORLD cannot be
dropped: `collections.rs` sets the cube `texture_view_descriptor` on the
main-world image, and `nova_ship::camera::skybox` reinterprets it as a fallback.

The source does not earn it. The PNG is 1.4 MB for 100 megapixels: a near-flat
nebula with ~120-250 pixels above 50% brightness per 16.7 megapixel face.

## The change

Both cubemaps are now 1024 px faces (1024x6144): **24 MB** of video memory each,
and 24 MB of heap. Downloads fall from 1.4 MB to 232 KB and from 8.7 MB to
1.5 MB.

Faces are cut apart before scaling so no kernel crosses a cube seam, and each
face is max-pooled at the reduction factor before the box average - the stars are
single bright pixels, and a plain area resize divides each one by 16 into
nothing. The command, per face, was:

    magick <face>.png -colorspace RGB -statistic Maximum 4x4 \
      -filter Box -resize 1024x1024 -colorspace sRGB <out>.png

Fidelity, face 0 of `cubemap.png`:

| measure | 4096 | 1024 |
|---|---|---|
| RGB mean (the nebula) | 0.0036064 | 0.0036591 |
| pixels >= 50% (the stars) | 199 | 42 |

42 beats the 28 a threshold-then-max-pool reference keeps, so no star is lost;
the +1.5% mean is the max filter, invisible at this level.

## A/B, same wasm, same host

Served `dist/` (a local debug wasm) and swapped only the two PNGs:

- 4096 faces: dies at the skybox upload, `Loading` never completes.
- 1024 faces: `Boot` AND `Loading` both complete, the menu renders for 2m12s,
  then the still-tight budget bites on clustering storage buffers.

So the skybox was the first wall and is gone. The rest is the host: a 3D game
does not fit in 630 MiB of free video memory, whatever we do to it.

## Not done

- No live native menu shot: the host never had enough free video memory during
  the session for the game to render at all. The texture comparison above is the
  evidence for the art change.
- Auto-updating an installed portal mod is a separate feature. The two mod parse
  failures in the report are a stale browser cache - the deployed portal serves
  gauntlet 1.12.0 and the-ledger 1.28.0, and those bytes match the working tree
  (sha256 verified against
  `https://alexjercan.github.io/nova-protocol/mods/gauntlet/1.12.0/gauntlet.content.ron`).
  `register_bundles` now names the mod and the file and points at Mods > Explore.
