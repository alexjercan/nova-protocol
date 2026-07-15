# Devlog #2: objectives, enemy AI and better asteroids

v0.1.0 let you build a ship and shoot rocks, which is fun for exactly as long as it takes to run out of things to do. v0.2.0 is about giving the game a reason to exist: objectives to complete, enemies to fight, and asteroids that are not just recoloured spheres. It was also the update where I learned that one small math mistake in one system can ripple out and make the whole game unplayable.

<div class="video-embed">
    <iframe
        src="https://www.youtube-nocookie.com/embed/NBpRYDvL-jM"
        title="Nova Protocol Devlog #2"
        loading="lazy"
        allow="
            accelerometer;
            autoplay;
            clipboard-write;
            encrypted-media;
            gyroscope;
            picture-in-picture;
        "
        allowfullscreen
    ></iframe>
</div>
<span class="video-embed__caption"
    >Devlog #2 - the full v0.2.0 walkthrough on YouTube.</span
>

## Objectives, via an accidental modding system

I could have hardcoded a couple of levels. My brain refused. I grew up modding _The Battle for Wesnoth_, whose simple data language for custom campaigns always stuck with me, so I wanted something data-driven in Nova Protocol too. The result is a small engine of three primitives:

- **Events** fired from the game - `ondestroy` with the object's id and type, for example.
- **Filters** that decide when a handler should react (match by id, by type, or invert with a `NotFilter`).
- **Actions** that do something - read/write scenario variables in a little hashmap on the game world, or switch to another scenario.

A scenario is just a bag of event handlers built from filters and actions. "When an asteroid is destroyed, increment a counter; when the counter hits its target, switch scene" is an entire win condition expressed as data. Yes, this is the observer pattern - but the point was to make it loadable from an asset, since you cannot load Rust functions from JSON, but you _can_ load structs and turn them into logic at runtime. On top of it, a HUD lists the active objectives and <kbd>Enter</kbd> advances to the next scenario when the world resource says there is one. This system is the direct ancestor of the scenario engine the game runs on today.

## Enemy AI (it is very dumb, and that is fine)

v0.2.0 also gets its first other ships. The logic is deliberately minimal: rotate toward the player, and if the alignment is good enough, thrust and shoot. The first version did not understand physics and cheerfully flew off into deep space like it had somewhere better to be. I asked ChatGPT for help and got an AI that at least stopped leaving the map - mostly because it forgot how to use its thrusters.

The interesting failure here is target prediction: the AI points its turret straight at me and still misses, because projectiles inherit the ship's motion and I am moving too. That is not just an AI problem - it is the same problem player auto-aim will have to solve, and it is the reason later versions got proper intercept lead.

## The physics bug that ate a week

Projectiles in v0.1.0 were cubes on rails. Realistically a bullet fired from a moving ship should inherit some of that velocity, so I set out to add the ship's velocity at spawn. Easy, right? Bevy disagreed.

To spawn a projectile in the world I need the turret's `GlobalTransform`, which Bevy only finalises at the end of the frame - so it lags a frame behind. Combined with physics running in `FixedUpdate` while my logic ran in `Update`, the transforms desynced and the whole game went jittery and unplayable. The fix was a trio of counterintuitive moves:

- **Disable interpolation.** It was supposed to smooth motion; it made the jitter worse.
- **Move helper systems to** `PostUpdate`, keeping only input handling in `Update` - gather inputs first, run logic after.
- **Compute the global transform on demand** with `TransformHelper` when firing, which is pricier but only happens per shot.

What actually saved me was tooling. A Graphviz view of the system sets let me see the exact ordering and spot the one system running at the wrong time. Which brings me to...

## Debug tooling and the Great Refactor of Doom

Before touching any of this I upgraded my debugging setup: physics-visualising UI, a "super debug" mode that strips the nice graphics and dumps the system-set graph, and a status bar that shows FPS and version - and, because I over-built it, can literally run shell commands and display their output. Ridiculous, but it paid for itself the moment physics broke.

I also finally split a nearly two-thousand-line `main.rs` into modular simulation and editor plugins. I always tell myself not to refactor mid-prototype and then always do it anyway. It felt like a waste of time, but factoring these systems out into reusable plugins means the next project starts with a copy-paste head start - so, maybe worth it.

## Asteroids that are not spheres

Last, some visual polish. The old sphere asteroids were boring, so I fell down a procedural-generation rabbit hole inspired by Sebastian Lague's planet tutorial (the C# would not copy-paste into Rust, so I followed the ideas instead). I build the base from an **octahedron sphere** rather than a UV sphere, because its triangles are roughly even in size and do not bunch up at the poles, then displace it with noise borrowed from an old project. A photo of an actual rock (yes, I went outside), made seamless and projected on with simple planar UVs, finishes the look. The mesh-explosion algorithm still has a TODO, but the asteroids finally look like asteroids.

## Where v0.2.0 lands

So v0.2.0 ships objectives, a data-driven modding backbone, the first enemy AI, procedural asteroids, and a much stronger debug toolkit - plus hard-won knowledge about Bevy's transform timing. Next up: sharper combat, smarter AI, and more visual polish. Or you can just [go fly something](../../play/).
