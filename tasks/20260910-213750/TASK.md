# Web build quits at boot on a GPU device OOM: the 384 MiB skybox cubemap

- STATUS: OPEN
- PRIORITY: 95
- TAGS: v0.13.2

The deployed web build quits at boot on a GPU device OOM, and two cached portal
mods fail to parse. Reported from a live session on an 8 GiB RTX 3060 Ti whose
VRAM was already 7.5 GiB occupied by unrelated local LLM servers.

## Evidence (reproduced locally against the deployed site)

Chromium under Xvfb :107, same host, same VRAM pressure:

    ERROR bevy_render error_handler.rs:132 Caught rendering error: Out of Memory
    vkAllocateMemory failed with VK_ERROR_OUT_OF_DEVICE_MEMORY
     - While calling [Device].CreateTexture([TextureDescriptor]).
    ERROR bevy_render error_handler.rs:79 Quitting the application due to
      OutOfMemory RenderError

The failing CreateTexture lands immediately after `GameAssetsStates::Boot` is
done, followed by a WriteTexture on the invalid texture: this is an image
upload, and the first big one the game does is the skybox.

`assets/base/textures/cubemap.png` is 4096x24576 (six stacked 4096 faces) =
**384 MiB of VRAM**, and its `.meta` keeps `MAIN_WORLD | RENDER_WORLD`, so the
same 384 MiB also sits in the wasm heap. `cubemap_alt.png` is the same size.
The PNG is 1.4 MiB: the content is a near-flat nebula with ~122 pixels above
50% brightness per face. 4096 per face is pure waste.

## Steps

- [x] Downscale both cubemaps so the sparse stars survive the reduction
      (max-pool, not a plain resize). 1024 px faces: 384 MB -> 24 MB.
- [x] Verify the sky still reads right. Per-face nebula mean holds to 1.5% and
      no star is lost; a live render was not possible on this host (see NOTES).
- [x] Name the mod and the file in `register_bundles`' "a content asset was not
      loaded" error - today it says neither, so a stale cached mod is untraceable.
- [x] Re-run the browser A/B under the same VRAM pressure. See NOTES.

## Notes

The two mod parse failures (`missing field hull`, `unknown variant
StoryMessage`) are a STALE BROWSER CACHE, not a repo defect: the deployed
portal serves gauntlet 1.12.0 / the-ledger 1.28.0 and those bytes match the
working tree (sha256 verified). The reporter's IndexedDB holds an older
install. Auto-updating an installed portal mod is a separate feature.
